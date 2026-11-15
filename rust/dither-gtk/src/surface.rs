use crate::renderer::Renderer;
use dither_ir::program::Program;
use glow::HasContext;
use gtk::{glib, prelude::*};
use std::{
    cell::{Cell, RefCell},
    ffi::c_void,
    rc::Rc,
};

type Encoder = dyn Fn(usize, usize) -> Program;

/// A GLArea whose display list is rebuilt from its allocated grid dimensions.
#[derive(Clone)]
pub struct DitherSurface {
    area: gtk::GLArea,
    cell: Rc<Cell<i32>>,
    encode: Rc<RefCell<Box<Encoder>>>,
}

impl DitherSurface {
    pub fn new(cell: i32, encode: impl Fn(usize, usize) -> Program + 'static) -> Self {
        let area = gtk::GLArea::builder()
            .auto_render(true)
            .has_stencil_buffer(false)
            .hexpand(true)
            .vexpand(true)
            .build();
        area.set_required_version(3, 3);
        area.update_property(&[gtk::accessible::Property::Label("Dither visualization")]);
        let this = Self {
            area,
            cell: Rc::new(Cell::new(cell.max(1))),
            encode: Rc::new(RefCell::new(Box::new(encode))),
        };
        let renderer: Rc<RefCell<Option<Renderer>>> = Rc::default();
        let r = renderer.clone();
        this.area.connect_realize(move |area| {
            area.make_current();
            if area.error().is_some() {
                return;
            }
            let result = unsafe { epoxy_context().and_then(|gl| Renderer::new(Rc::new(gl))) };
            match result {
                Ok(value) => *r.borrow_mut() = Some(value),
                Err(error) => area.set_error(Some(&glib::Error::new(
                    gtk::gdk::GLError::NotAvailable,
                    &error,
                ))),
            }
        });
        let r = renderer.clone();
        this.area.connect_unrealize(move |area| {
            area.make_current();
            // Delete GL resources while their owning context is current.
            r.borrow_mut().take();
        });
        let cell = this.cell.clone();
        let encoder = this.encode.clone();
        this.area.connect_render(move |area, _| {
            let mut storage = renderer.borrow_mut();
            let Some(renderer) = storage.as_mut() else {
                return glib::Propagation::Proceed;
            };
            let c = cell.get();
            let w = area.width() / c;
            let h = area.height() / c;
            if w <= 0 || h <= 0 {
                return glib::Propagation::Stop;
            }
            let scale = area.scale_factor();
            unsafe {
                // GTK's target is a private framebuffer, never assume framebuffer zero.
                area.attach_buffers();
                let gl = renderer_gl(renderer);
                let id = gl.get_parameter_i32(glow::DRAW_FRAMEBUFFER_BINDING) as u32;
                let target = std::num::NonZeroU32::new(id).map(glow::NativeFramebuffer);
                let program = encoder.borrow()(w as usize, h as usize);
                match renderer.render(&program.to_vec(), w, h, false) {
                    Ok(()) => renderer.blit(
                        target,
                        area.width() * scale,
                        area.height() * scale,
                        c,
                        scale,
                    ),
                    Err(error) => area.set_error(Some(&glib::Error::new(
                        gtk::gdk::GLError::NotAvailable,
                        &error,
                    ))),
                }
            }
            glib::Propagation::Stop
        });
        this.area.connect_resize(|area, _, _| area.queue_render());
        this
    }
    pub(crate) fn downgrade(&self) -> WeakDitherSurface {
        WeakDitherSurface {
            area: self.area.downgrade(),
            cell: self.cell.clone(),
            encode: Rc::downgrade(&self.encode),
        }
    }
    pub fn widget(&self) -> &gtk::GLArea {
        &self.area
    }
    pub fn cell(&self) -> i32 {
        self.cell.get()
    }
    pub fn set_cell(&self, cell: i32) {
        self.cell.set(cell.max(1));
        self.queue_render();
    }
    pub fn set_encoder(&self, encode: impl Fn(usize, usize) -> Program + 'static) {
        *self.encode.borrow_mut() = Box::new(encode);
        self.queue_render();
    }
    pub fn set_accessible_label(&self, label: &str) {
        self.area
            .update_property(&[gtk::accessible::Property::Label(label)]);
    }
    pub fn queue_render(&self) {
        self.area.queue_render();
    }
}

pub(crate) struct WeakDitherSurface {
    area: glib::WeakRef<gtk::GLArea>,
    cell: Rc<Cell<i32>>,
    encode: std::rc::Weak<RefCell<Box<Encoder>>>,
}
impl WeakDitherSurface {
    pub(crate) fn upgrade(&self) -> Option<DitherSurface> {
        Some(DitherSurface {
            area: self.area.upgrade()?,
            cell: self.cell.clone(),
            encode: self.encode.upgrade()?,
        })
    }
}

// The epoxy dispatch table follows the current context, including GL/GLES selection.
// Keep the library alive for the lifetime of every returned function pointer.
unsafe fn epoxy_context() -> Result<glow::Context, String> {
    unsafe {
        #[cfg(target_os = "macos")]
        let names = [
            "libepoxy.0.dylib",
            "/opt/homebrew/lib/libepoxy.0.dylib",
            "/usr/local/lib/libepoxy.0.dylib",
        ];
        #[cfg(target_os = "linux")]
        let names = ["libepoxy.so.0", "libepoxy.so", "libepoxy.so.0"];
        #[cfg(target_os = "windows")]
        let names = ["libepoxy-0.dll", "epoxy-0.dll", "libepoxy.dll"];
        static LIBRARY: std::sync::OnceLock<Result<libloading::Library, String>> =
            std::sync::OnceLock::new();
        let library = LIBRARY
            .get_or_init(|| {
                names
                    .iter()
                    .find_map(|name| libloading::Library::new(name).ok())
                    .ok_or_else(|| "unable to load libepoxy".into())
            })
            .as_ref()
            .map_err(Clone::clone)?;
        Ok(glow::Context::from_loader_function(|name| {
            let symbol = format!("epoxy_{name}\0");
            library
                .get::<*const *const c_void>(symbol.as_bytes())
                .map(|pointer| **pointer)
                .unwrap_or(std::ptr::null())
        }))
    }
}

fn renderer_gl(renderer: &Renderer) -> &glow::Context {
    renderer.context()
}
