//! Context-independent two-pass OpenGL renderer. The caller owns the current GL context.
use glow::HasContext;
use std::rc::Rc;

pub const DATA_WIDTH: usize = 1024;

pub fn pack_program(values: &[f32]) -> Result<(Vec<f32>, i32), String> {
    let (data, rows, _) = pack_program_with_split(values)?;
    Ok((data, rows))
}

fn pack_program_with_split(values: &[f32]) -> Result<(Vec<f32>, i32, i32), String> {
    if values.is_empty() || values.iter().any(|v| !v.is_finite()) {
        return Err("display list must contain finite floats and its op-count header".into());
    }
    let split = dither_ir::program::bloom_index(values)
        .ok_or("display-list operation headers are invalid")?;
    let rows = values.len().div_ceil(DATA_WIDTH);
    let mut padded = vec![0.0; rows * DATA_WIDTH];
    padded[..values.len()].copy_from_slice(values);
    Ok((
        padded,
        i32::try_from(rows).map_err(|_| "display list too large")?,
        i32::try_from(split).map_err(|_| "display list too large")?,
    ))
}

/// Destination rectangle in bottom-left GL coordinates; grid rows stay top-left aligned.
pub fn blit_rect(w: i32, h: i32, cell: i32, scale: i32, fb_h: i32) -> [i32; 4] {
    let cellpx = cell.saturating_mul(scale);
    [0, fb_h - h * cellpx, w * cellpx, fb_h]
}

pub struct Renderer {
    gl: Rc<glow::Context>,
    program: glow::Program,
    vao: glow::VertexArray,
    data: glow::Texture,
    textures: [glow::Texture; 2],
    fbos: [glow::Framebuffer; 2],
    size: (i32, i32),
}

impl Renderer {
    pub(crate) fn context(&self) -> &glow::Context {
        &self.gl
    }

    /// # Safety
    /// `gl` must refer to the context currently bound on this thread. The same context
    /// must be current for every method and when this renderer is dropped.
    pub unsafe fn new(gl: Rc<glow::Context>) -> Result<Self, String> {
        unsafe {
            let version = if gl.version().is_embedded {
                "#version 300 es\n"
            } else {
                "#version 330 core\n"
            };
            let program = gl.create_program()?;
            let vertex = format!(
                "{version}void main() {{ vec2 p = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2)); gl_Position = vec4(p * 2.0 - 1.0, 0.0, 1.0); }}"
            );
            let fragment = format!("{version}{}", include_str!("../shaders/dither.glsl"));
            for (kind, source) in [
                (glow::VERTEX_SHADER, vertex),
                (glow::FRAGMENT_SHADER, fragment),
            ] {
                let shader = gl.create_shader(kind)?;
                gl.shader_source(shader, &source);
                gl.compile_shader(shader);
                if !gl.get_shader_compile_status(shader) {
                    let log = gl.get_shader_info_log(shader);
                    gl.delete_shader(shader);
                    gl.delete_program(program);
                    return Err(log);
                }
                gl.attach_shader(program, shader);
                gl.delete_shader(shader);
            }
            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                let log = gl.get_program_info_log(program);
                gl.delete_program(program);
                return Err(log);
            }
            let vao = gl.create_vertex_array()?;
            let data = gl.create_texture()?;
            let textures = [gl.create_texture()?, gl.create_texture()?];
            let fbos = [gl.create_framebuffer()?, gl.create_framebuffer()?];
            for texture in [data, textures[0], textures[1]] {
                gl.bind_texture(glow::TEXTURE_2D, Some(texture));
                gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_MIN_FILTER,
                    glow::NEAREST as i32,
                );
                gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_MAG_FILTER,
                    glow::NEAREST as i32,
                );
                gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_WRAP_S,
                    glow::CLAMP_TO_EDGE as i32,
                );
                gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_WRAP_T,
                    glow::CLAMP_TO_EDGE as i32,
                );
            }
            Ok(Self {
                gl,
                program,
                vao,
                data,
                textures,
                fbos,
                size: (0, 0),
            })
        }
    }

    /// Render at grid resolution; `straight` is for PNG parity rather than compositing.
    /// # Safety
    /// The renderer's GL context must be current.
    pub unsafe fn render(
        &mut self,
        values: &[f32],
        w: i32,
        h: i32,
        straight: bool,
    ) -> Result<(), String> {
        unsafe {
            let gl = &self.gl;
            let max = gl.get_parameter_i32(glow::MAX_TEXTURE_SIZE);
            let (data, rows, split) = pack_program_with_split(values)?;
            if w <= 0 || h <= 0 || w > max || h > max || rows > max || DATA_WIDTH as i32 > max {
                return Err("grid or program exceeds GL texture limits".into());
            }
            if self.size != (w, h) {
                for i in 0..2 {
                    gl.bind_texture(glow::TEXTURE_2D, Some(self.textures[i]));
                    gl.tex_image_2d(
                        glow::TEXTURE_2D,
                        0,
                        glow::RGBA8 as i32,
                        w,
                        h,
                        0,
                        glow::RGBA,
                        glow::UNSIGNED_BYTE,
                        glow::PixelUnpackData::Slice(None),
                    );
                    gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fbos[i]));
                    gl.framebuffer_texture_2d(
                        glow::FRAMEBUFFER,
                        glow::COLOR_ATTACHMENT0,
                        glow::TEXTURE_2D,
                        Some(self.textures[i]),
                        0,
                    );
                    if gl.check_framebuffer_status(glow::FRAMEBUFFER) != glow::FRAMEBUFFER_COMPLETE
                    {
                        return Err("RGBA8 framebuffer is incomplete".into());
                    }
                }
                self.size = (w, h);
            }
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.data));
            let bytes = std::slice::from_raw_parts(data.as_ptr().cast::<u8>(), data.len() * 4);
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::R32F as i32,
                DATA_WIDTH as i32,
                rows,
                0,
                glow::RED,
                glow::FLOAT,
                glow::PixelUnpackData::Slice(Some(bytes)),
            );
            gl.use_program(Some(self.program));
            gl.bind_vertex_array(Some(self.vao));
            gl.disable(glow::BLEND);
            gl.disable(glow::DEPTH_TEST);
            gl.disable(glow::STENCIL_TEST);
            gl.disable(glow::SCISSOR_TEST);
            gl.disable(glow::DITHER);
            if !gl.version().is_embedded {
                gl.disable(glow::FRAMEBUFFER_SRGB);
            }
            gl.color_mask(true, true, true, true);
            gl.viewport(0, 0, w, h);
            gl.uniform_1_i32(gl.get_uniform_location(self.program, "uData").as_ref(), 0);
            gl.uniform_1_i32(gl.get_uniform_location(self.program, "uBase").as_ref(), 1);
            gl.uniform_1_i32(
                gl.get_uniform_location(self.program, "uBloomIndex")
                    .as_ref(),
                split,
            );
            gl.uniform_2_i32(
                gl.get_uniform_location(self.program, "uGrid").as_ref(),
                w,
                h,
            );
            gl.uniform_1_i32(
                gl.get_uniform_location(self.program, "uStraight").as_ref(),
                i32::from(straight),
            );
            // Keep both samplers complete even in pass 0 (GL validates active samplers).
            gl.active_texture(glow::TEXTURE1);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.textures[1]));
            for pass in 0..2 {
                gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fbos[pass]));
                gl.uniform_1_i32(
                    gl.get_uniform_location(self.program, "uPass").as_ref(),
                    pass as i32,
                );
                gl.draw_arrays(glow::TRIANGLES, 0, 3);
                gl.bind_texture(glow::TEXTURE_2D, Some(self.textures[0]));
            }
            let error = gl.get_error();
            if error != glow::NO_ERROR {
                return Err(format!("GL render error 0x{error:x}"));
            }
            Ok(())
        }
    }

    /// # Safety
    /// Context must be current; `target` must belong to it (None is its default framebuffer).
    pub unsafe fn blit(
        &self,
        target: Option<glow::Framebuffer>,
        fb_w: i32,
        fb_h: i32,
        cell: i32,
        scale: i32,
    ) {
        unsafe {
            let gl = &self.gl;
            gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, target);
            gl.viewport(0, 0, fb_w, fb_h);
            gl.clear_color(0.0, 0.0, 0.0, 0.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
            gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(self.fbos[1]));
            let [x0, y0, x1, y1] = blit_rect(self.size.0, self.size.1, cell, scale, fb_h);
            gl.blit_framebuffer(
                0,
                0,
                self.size.0,
                self.size.1,
                x0,
                y0,
                x1,
                y1,
                glow::COLOR_BUFFER_BIT,
                glow::NEAREST,
            );
            gl.bind_framebuffer(glow::FRAMEBUFFER, target);
        }
    }

    /// Read straight/premultiplied output (as chosen in render), row-major from top left.
    /// # Safety
    /// Context must be current and render must have succeeded.
    pub unsafe fn read_rgba(&self) -> Vec<u8> {
        unsafe {
            let (w, h) = self.size;
            let mut pixels = vec![0; (w * h * 4) as usize];
            self.gl
                .bind_framebuffer(glow::READ_FRAMEBUFFER, Some(self.fbos[1]));
            self.gl.pixel_store_i32(glow::PACK_ALIGNMENT, 1);
            self.gl.read_pixels(
                0,
                0,
                w,
                h,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(Some(&mut pixels)),
            );
            let stride = w as usize * 4;
            for y in 0..h as usize / 2 {
                let (top, rest) = pixels.split_at_mut((h as usize - 1 - y) * stride);
                top[y * stride..(y + 1) * stride].swap_with_slice(&mut rest[..stride]);
            }
            pixels
        }
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_program(self.program);
            self.gl.delete_vertex_array(self.vao);
            self.gl.delete_texture(self.data);
            for texture in self.textures {
                self.gl.delete_texture(texture);
            }
            for fbo in self.fbos {
                self.gl.delete_framebuffer(fbo);
            }
        }
    }
}
