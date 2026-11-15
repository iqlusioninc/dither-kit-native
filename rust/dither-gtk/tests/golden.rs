//! The shader gate is mandatory on Linux: missing EGL/Mesa is an error, never a skip.
#![cfg_attr(not(target_os = "linux"), allow(dead_code, unused_imports))]
use dither_gtk::renderer::Renderer;
use glow::HasContext;
use khronos_egl as egl;
use std::{path::PathBuf, rc::Rc};

struct EglHeadless {
    egl: egl::DynamicInstance<egl::EGL1_5>,
    display: egl::Display,
    context: egl::Context,
    surface: egl::Surface,
    gl: Rc<glow::Context>,
}
impl EglHeadless {
    fn new(gles: bool) -> Self {
        unsafe {
            let egl = egl::DynamicInstance::<egl::EGL1_5>::load_required()
                .expect("load EGL 1.5 (install libegl-mesa0 libegl-dev)");
            let display = egl
                .get_platform_display(0x31DD, std::ptr::null_mut(), &[egl::ATTRIB_NONE])
                .expect("EGL_PLATFORM_SURFACELESS_MESA");
            egl.initialize(display)
                .expect("initialize Mesa surfaceless display");
            egl.bind_api(if gles {
                egl::OPENGL_ES_API
            } else {
                egl::OPENGL_API
            })
            .expect("bind GL API");
            let config = egl
                .choose_first_config(
                    display,
                    &[
                        egl::SURFACE_TYPE,
                        egl::PBUFFER_BIT,
                        egl::RENDERABLE_TYPE,
                        if gles {
                            egl::OPENGL_ES3_BIT
                        } else {
                            egl::OPENGL_BIT
                        },
                        egl::RED_SIZE,
                        8,
                        egl::GREEN_SIZE,
                        8,
                        egl::BLUE_SIZE,
                        8,
                        egl::ALPHA_SIZE,
                        8,
                        egl::NONE,
                    ],
                )
                .expect("choose RGBA8 config")
                .expect("RGBA8 config available");
            let attributes = if gles {
                vec![egl::CONTEXT_CLIENT_VERSION, 3, egl::NONE]
            } else {
                vec![
                    egl::CONTEXT_MAJOR_VERSION,
                    3,
                    egl::CONTEXT_MINOR_VERSION,
                    3,
                    egl::CONTEXT_OPENGL_PROFILE_MASK,
                    egl::CONTEXT_OPENGL_CORE_PROFILE_BIT,
                    egl::NONE,
                ]
            };
            let context = egl
                .create_context(display, config, None, &attributes)
                .expect("create GL3.3/GLES3 context");
            let surface = egl
                .create_pbuffer_surface(
                    display,
                    config,
                    &[egl::WIDTH, 1, egl::HEIGHT, 1, egl::NONE],
                )
                .expect("create pbuffer");
            egl.make_current(display, Some(surface), Some(surface), Some(context))
                .expect("make GL context current");
            let gl = Rc::new(glow::Context::from_loader_function(|name| {
                egl.get_proc_address(name)
                    .map(|f| f as *const () as *const _)
                    .unwrap_or(std::ptr::null())
            }));
            eprintln!(
                "{} / {}",
                gl.get_parameter_string(glow::RENDERER),
                gl.get_parameter_string(glow::VERSION)
            );
            Self {
                egl,
                display,
                context,
                surface,
                gl,
            }
        }
    }
}
impl Drop for EglHeadless {
    fn drop(&mut self) {
        self.egl
            .make_current(self.display, None, None, None)
            .unwrap();
        self.egl
            .destroy_surface(self.display, self.surface)
            .unwrap();
        self.egl
            .destroy_context(self.display, self.context)
            .unwrap();
        self.egl.terminate(self.display).unwrap();
    }
}
#[cfg(target_os = "linux")]
type Headless = EglHeadless;
#[cfg(target_os = "macos")]
type Headless = macos::CglHeadless;
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
type Headless = EglHeadless;
static GPU_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
fn compare_all(gles: bool) {
    let _lock = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let headless = Headless::new(gles);
    let mut renderer =
        unsafe { Renderer::new(headless.gl.clone()) }.expect("compile/link shader interpreter");
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
    let document: serde_json::Value =
        serde_json::from_slice(&std::fs::read(fixtures.join("scenes.json")).unwrap()).unwrap();
    let scenes = document["scenes"].as_array().unwrap();
    assert_eq!(scenes.len(), 39, "all shared scenes must be checked");
    for scene in scenes {
        let name = scene["name"].as_str().unwrap();
        let w = scene["w"].as_i64().unwrap() as i32;
        let h = scene["h"].as_i64().unwrap() as i32;
        let data = scene["program"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap() as f32)
            .collect::<Vec<_>>();
        unsafe { renderer.render(&data, w, h, true) }.unwrap_or_else(|e| panic!("{name}: {e}"));
        let actual = unsafe { renderer.read_rgba() };
        let decoder = png::Decoder::new(
            std::fs::File::open(fixtures.join("png").join(format!("{name}.png"))).unwrap(),
        );
        let mut reader = decoder.read_info().unwrap();
        let mut expected = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut expected).unwrap();
        assert_eq!((info.width, info.height), (w as u32, h as u32));
        assert_eq!(info.color_type, png::ColorType::Rgba);
        expected.truncate(info.buffer_size());
        assert_eq!(actual.len(), expected.len());
        let mut differing = 0usize;
        let mut max_delta = 0u8;
        for (a, e) in actual
            .as_chunks::<4>()
            .0
            .iter()
            .zip(expected.as_chunks::<4>().0)
        {
            let delta = a.iter().zip(e).map(|(a, e)| a.abs_diff(*e)).max().unwrap();
            max_delta = max_delta.max(delta);
            differing += usize::from(delta != 0);
        }
        eprintln!(
            "{name}: {differing}/{} differing pixels, max channel delta {max_delta}",
            w * h
        );
        assert!(
            max_delta <= 2,
            "{name}: max channel delta {max_delta} exceeds 2"
        );
        assert!(
            differing * 100 <= (w * h) as usize,
            "{name}: {differing}/{} pixels differ, exceeds 1%",
            w * h
        );
    }
    drop(renderer); // Its context is still current here.
}
#[test]
#[cfg(target_os = "linux")]
fn desktop_gl_matches_all_39_goldens() {
    compare_all(false);
}
#[test]
#[cfg(target_os = "linux")]
fn gles_matches_all_39_goldens() {
    compare_all(true);
}

#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    use std::ffi::c_void;
    #[link(name = "OpenGL", kind = "framework")]
    unsafe extern "C" {
        fn CGLChoosePixelFormat(
            attrs: *const u32,
            format: *mut *mut c_void,
            count: *mut i32,
        ) -> i32;
        fn CGLDestroyPixelFormat(format: *mut c_void) -> i32;
        fn CGLCreateContext(
            format: *mut c_void,
            shared: *mut c_void,
            context: *mut *mut c_void,
        ) -> i32;
        fn CGLSetCurrentContext(context: *mut c_void) -> i32;
        fn CGLDestroyContext(context: *mut c_void) -> i32;
    }
    pub struct CglHeadless {
        context: *mut c_void,
        pub gl: Rc<glow::Context>,
        _library: libloading::Library,
    }
    impl CglHeadless {
        pub fn new(gles: bool) -> Self {
            assert!(!gles, "CGL only supports desktop GL");
            unsafe {
                let (mut format, mut count, mut context) =
                    (std::ptr::null_mut(), 0, std::ptr::null_mut());
                // SDK CGLTypes.h: OpenGLProfile=99, GL4_Core=0x4100.
                assert_eq!(
                    CGLChoosePixelFormat([99, 0x4100, 0].as_ptr(), &mut format, &mut count),
                    0
                );
                assert!(count > 0 && !format.is_null());
                assert_eq!(
                    CGLCreateContext(format, std::ptr::null_mut(), &mut context),
                    0
                );
                CGLDestroyPixelFormat(format);
                assert_eq!(CGLSetCurrentContext(context), 0);
                let library =
                    libloading::Library::new("/System/Library/Frameworks/OpenGL.framework/OpenGL")
                        .unwrap();
                let gl = Rc::new(glow::Context::from_loader_function(|name| {
                    library
                        .get::<unsafe extern "C" fn()>(format!("{name}\0").as_bytes())
                        .map(|f| *f as *const () as *const c_void)
                        .unwrap_or(std::ptr::null())
                }));
                eprintln!(
                    "{} / {}",
                    gl.get_parameter_string(glow::RENDERER),
                    gl.get_parameter_string(glow::VERSION)
                );
                Self {
                    context,
                    gl,
                    _library: library,
                }
            }
        }
    }
    impl Drop for CglHeadless {
        fn drop(&mut self) {
            unsafe {
                CGLSetCurrentContext(std::ptr::null_mut());
                CGLDestroyContext(self.context);
            }
        }
    }
}
#[test]
#[cfg(target_os = "macos")]
fn macos_gl_matches_all_39_goldens() {
    compare_all(false);
}

#[test]
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn premultiplied_output_and_nearest_top_left_blit() {
    let _lock = GPU_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let headless = Headless::new(false);
    let gl = &headless.gl;
    unsafe {
        let mut renderer = Renderer::new(gl.clone()).unwrap();
        let mut program = dither_gtk::ir::Program::new();
        for (x, y, c) in [
            (0, 0, [200, 100, 50, 128]),
            (1, 0, [0, 255, 0, 255]),
            (0, 1, [0, 0, 255, 255]),
            (1, 1, [255, 0, 0, 255]),
        ] {
            program.line(
                (x, y),
                (x, y),
                dither_gtk::ir::Rgba::new(c[0], c[1], c[2], c[3]),
                0,
            );
        }
        renderer.render(&program.to_vec(), 2, 2, false).unwrap();
        assert_eq!(&renderer.read_rgba()[..4], &[100, 50, 25, 128]);
        let texture = gl.create_texture().unwrap();
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGBA8 as i32,
            8,
            7,
            0,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(None),
        );
        let target = gl.create_framebuffer().unwrap();
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(target));
        gl.framebuffer_texture_2d(
            glow::FRAMEBUFFER,
            glow::COLOR_ATTACHMENT0,
            glow::TEXTURE_2D,
            Some(texture),
            0,
        );
        assert_eq!(
            gl.check_framebuffer_status(glow::FRAMEBUFFER),
            glow::FRAMEBUFFER_COMPLETE
        );
        renderer.blit(Some(target), 8, 7, 2, 1);
        let mut pixels = vec![0u8; 8 * 7 * 4];
        gl.read_pixels(
            0,
            0,
            8,
            7,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelPackData::Slice(Some(&mut pixels)),
        );
        let colors = [
            [100, 50, 25, 128],
            [0, 255, 0, 255],
            [0, 0, 255, 255],
            [255, 0, 0, 255],
        ];
        for top_y in 0..7 {
            for x in 0..8 {
                let at = ((6 - top_y) * 8 + x) * 4;
                let expected = if top_y < 4 && x < 4 {
                    colors[(top_y / 2) * 2 + x / 2]
                } else {
                    [0, 0, 0, 0]
                };
                assert_eq!(&pixels[at..at + 4], &expected, "({x},{top_y})");
            }
        }
        gl.delete_framebuffer(target);
        gl.delete_texture(texture);
    }
}
