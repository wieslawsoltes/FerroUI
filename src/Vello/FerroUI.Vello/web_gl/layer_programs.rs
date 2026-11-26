//! The programs of the layers with an effect on the WebGL2 context of a canvas: the programs of
//! `gpu/effect_passes.rs` in GLSL ES 3.0 (step 3 of the GPU route, docs/porting/gpu-rendering.md,
//! #623).
//!
//! Each program is compiled once per context, when a layer first needs it, and takes its values
//! as uniforms for each pass. A pass reads the content of a layer from a texture of the pool, as
//! the copy from the drawing buffer left it (its first row the bottom one of the area), and
//! writes the result into another texture of the pool upright (its first row the top one), the
//! way the renderer samples a texture it is given. Every pixel of the target is written (the
//! pixels outside the area are transparent), so nothing is cleared first.
//!
//! The renderer of `vello_gpu` sets what it draws with itself; a pass sets what it needs, and
//! leaves the state it changes as it found it where the renderer may not set it again (the
//! tests of the framebuffer, the viewport), and unbound otherwise.

use super::frame_textures::VelloWebGlTexture;
use ferroui_base::media::{ColorMatrix, ColorMatrixAlphaMode};
use std::rc::Rc;
use web_sys::{WebGl2RenderingContext as Gl, WebGlProgram, WebGlShader, WebGlUniformLocation, WebGlVertexArrayObject};

/// The vertex stage of the passes: a triangle over the target, without attributes.
const VERTEX: &str = r#"#version 300 es
void main() {
    vec2 uv = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2));
    gl_Position = vec4(uv * 2.0 - 1.0, 0.0, 1.0);
}
"#;

/// A colour matrix (`ColorMatrix::apply_premultiplied`; `COLOR_MATRIX_WGSL` of
/// `gpu/effect_passes.rs`): the matrix applied to the colour un-premultiplied and the result
/// premultiplied again, or to the premultiplied colour as it is with the colour clamped to the
/// alpha.
const COLOR_MATRIX: &str = r#"#version 300 es
precision highp float;
precision highp int;

uniform sampler2D u_content;
// What red, green, blue and alpha add to each output channel, then the offsets (the rows of
// `ColorMatrix`).
uniform vec4 u_rows[5];
// x: 1 to apply the matrix to the premultiplied colour as it is, 0 to un-premultiply it first;
// y, z: the size of the area in pixels.
uniform vec4 u_mode;

out vec4 o_color;

vec4 apply_matrix(vec4 c) {
    vec4 m = u_rows[4] + c.r * u_rows[0] + c.g * u_rows[1] + c.b * u_rows[2] + c.a * u_rows[3];
    return clamp(m, vec4(0.0), vec4(1.0));
}

void main() {
    ivec2 pixel = ivec2(floor(gl_FragCoord.xy));
    if (float(pixel.x) >= u_mode.y || float(pixel.y) >= u_mode.z) {
        o_color = vec4(0.0);
        return;
    }
    // The row `pixel.y` of the result is the row `pixel.y` from the top of the area, which the
    // copy from the drawing buffer left at `height - 1 - pixel.y`.
    vec4 c = texelFetch(u_content, ivec2(pixel.x, int(u_mode.z) - 1 - pixel.y), 0);
    if (u_mode.x > 0.5) {
        vec4 result = apply_matrix(c);
        o_color = vec4(min(result.rgb, vec3(result.a)), result.a);
        return;
    }
    vec3 straight = vec3(0.0);
    if (c.a > 0.0) {
        straight = min(c.rgb / c.a, vec3(1.0));
    }
    vec4 result = apply_matrix(vec4(straight, c.a));
    o_color = vec4(result.rgb * result.a, result.a);
}
"#;

/// The colour matrix program and the places of its uniforms.
struct ColorMatrixProgram {
    program: WebGlProgram,
    content: Option<WebGlUniformLocation>,
    rows: Option<WebGlUniformLocation>,
    mode: Option<WebGlUniformLocation>,
}

/// The programs of a context, compiled when first needed. A program that does not compile is
/// not compiled again: its layers are drawn on the processor, and the failure is logged once.
#[derive(Default)]
pub(crate) struct LayerPrograms {
    vertex_array: Option<WebGlVertexArrayObject>,
    color_matrix: Option<Result<ColorMatrixProgram, ()>>,
}

fn compile(gl: &Gl, kind: u32, source: &str) -> Result<WebGlShader, String> {
    let shader = gl.create_shader(kind).ok_or("the context did not create a shader")?;
    gl.shader_source(&shader, source);
    gl.compile_shader(&shader);
    if gl.get_shader_parameter(&shader, Gl::COMPILE_STATUS).as_bool().unwrap_or(false) {
        return Ok(shader);
    }
    let log = gl.get_shader_info_log(&shader).unwrap_or_default();
    gl.delete_shader(Some(&shader));
    Err(log)
}

fn link(gl: &Gl, fragment: &str) -> Result<WebGlProgram, String> {
    let vertex = compile(gl, Gl::VERTEX_SHADER, VERTEX)?;
    let fragment = match compile(gl, Gl::FRAGMENT_SHADER, fragment) {
        Ok(fragment) => fragment,
        Err(error) => {
            gl.delete_shader(Some(&vertex));
            return Err(error);
        }
    };
    let program = gl.create_program().ok_or("the context did not create a program")?;
    gl.attach_shader(&program, &vertex);
    gl.attach_shader(&program, &fragment);
    gl.link_program(&program);
    // The program keeps what it was linked from.
    gl.delete_shader(Some(&vertex));
    gl.delete_shader(Some(&fragment));
    if gl.get_program_parameter(&program, Gl::LINK_STATUS).as_bool().unwrap_or(false) {
        return Ok(program);
    }
    let log = gl.get_program_info_log(&program).unwrap_or_default();
    gl.delete_program(Some(&program));
    Err(log)
}

impl LayerPrograms {
    /// Whether the colour matrix program is there, compiled now if it was not.
    pub(crate) fn prepare_color_matrix(&mut self, gl: &Gl) -> bool {
        self.color_matrix(gl).is_some()
    }

    fn color_matrix(&mut self, gl: &Gl) -> Option<&ColorMatrixProgram> {
        let program = self.color_matrix.get_or_insert_with(|| match link(gl, COLOR_MATRIX) {
            Ok(program) => Ok(ColorMatrixProgram {
                content: gl.get_uniform_location(&program, "u_content"),
                rows: gl.get_uniform_location(&program, "u_rows"),
                mode: gl.get_uniform_location(&program, "u_mode"),
                program,
            }),
            Err(error) => {
                super::log_render_failure(&format!("the colour matrix program did not compile: {error}"));
                Err(())
            }
        });
        program.as_ref().ok()
    }

    /// Writes `target` with the program that is in use, its uniforms set, reading `source` as
    /// texture unit 0.
    fn draw(&mut self, gl: &Gl, source: &VelloWebGlTexture, target: &VelloWebGlTexture) {
        if self.vertex_array.is_none() {
            self.vertex_array = gl.create_vertex_array();
        }
        let tests = [Gl::BLEND, Gl::SCISSOR_TEST, Gl::DEPTH_TEST, Gl::STENCIL_TEST];
        let enabled: Vec<bool> = tests.iter().map(|test| gl.is_enabled(*test)).collect();
        for test in tests {
            gl.disable(test);
        }

        gl.bind_framebuffer(Gl::FRAMEBUFFER, Some(&target.framebuffer));
        gl.viewport(0, 0, target.width as i32, target.height as i32);
        gl.active_texture(Gl::TEXTURE0);
        gl.bind_texture(Gl::TEXTURE_2D, Some(&source.texture));
        gl.bind_vertex_array(self.vertex_array.as_ref());
        gl.draw_arrays(Gl::TRIANGLES, 0, 3);

        gl.bind_vertex_array(None);
        gl.bind_texture(Gl::TEXTURE_2D, None);
        gl.use_program(None);
        gl.bind_framebuffer(Gl::FRAMEBUFFER, None);
        gl.viewport(0, 0, gl.drawing_buffer_width(), gl.drawing_buffer_height());
        for (test, enabled) in tests.iter().zip(enabled) {
            if enabled {
                gl.enable(*test);
            }
        }
    }

    /// The colour matrix pass: `matrix` applied to the `width` by `height` pixels of the content
    /// in `source`, written into `target`. `false` when the program is not there.
    pub(crate) fn color_matrix_pass(
        &mut self,
        gl: &Gl,
        source: &Rc<VelloWebGlTexture>,
        target: &Rc<VelloWebGlTexture>,
        (width, height): (u32, u32),
        matrix: &ColorMatrix,
        alpha_mode: ColorMatrixAlphaMode,
    ) -> bool {
        let Some(program) = self.color_matrix(gl) else {
            return false;
        };
        let mut rows = [0.0_f32; 20];
        for (index, row) in matrix.rows.iter().enumerate() {
            rows[index * 4..index * 4 + 4].copy_from_slice(row);
        }
        let as_it_is = if alpha_mode == ColorMatrixAlphaMode::Straight { 1.0 } else { 0.0 };
        gl.use_program(Some(&program.program));
        gl.uniform1i(program.content.as_ref(), 0);
        gl.uniform4fv_with_f32_array(program.rows.as_ref(), &rows);
        gl.uniform4f(program.mode.as_ref(), as_it_is, width as f32, height as f32, 0.0);
        self.draw(gl, source, target);
        true
    }

    /// Deletes the programs: the context is released.
    pub(crate) fn dispose(&mut self, gl: &Gl) {
        if let Some(Ok(program)) = self.color_matrix.take() {
            gl.delete_program(Some(&program.program));
        }
        if let Some(vertex_array) = self.vertex_array.take() {
            gl.delete_vertex_array(Some(&vertex_array));
        }
    }
}
