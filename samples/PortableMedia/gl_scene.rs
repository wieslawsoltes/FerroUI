//! Client-owned GLES scene; the framework only schedules and leases its textures.
use ferroui_base::{media::Stretch, Point, Size};
use ferroui_media::{frames::*, MediaError, MediaErrorKind, RendererViewport};
use ferroui_media_opengl::GlTextureRender;
use ferroui_opengl::{GlInterface, IGlContext, SharedFrameTexture, SharedGlTextureRetirementQueue};
use std::{ffi::c_void, rc::Rc, sync::Mutex};
fn error(s: impl Into<String>) -> MediaError {
    MediaError::new(MediaErrorKind::NativeFailure, s)
}
struct Pipeline {
    program: i32,
    vao: i32,
    context: u64,
    generation: u64,
    queue: SharedGlTextureRetirementQueue,
}
impl Drop for Pipeline {
    fn drop(&mut self) {
        let (program, vao) = (self.program, self.vao);
        self.queue.retire_native(move |gl| {
            if let Some(gl) = gl {
                unsafe { gl.delete_vertex_arrays(1, &vao) }
                gl.delete_program(program);
            }
        });
    }
}
#[derive(Default)]
pub struct Scene(Mutex<Option<Pipeline>>);
const VERTEX:&str="#version 300 es\nprecision highp float;uniform mat4 uvMap;out vec2 uv;void main(){vec2 q[6]=vec2[6](vec2(0,0),vec2(1,0),vec2(1,1),vec2(0,0),vec2(1,1),vec2(0,1));vec2 p=q[gl_VertexID];mat4 m=mat4(1.4,0.,0.,.35,-.25,-1.35,0.,.15,0.,0.,1.,0.,-.55,.7,0.,1.);gl_Position=m*vec4(p,0,1);uv=(uvMap*vec4(p,0,1)).xy;}";
const FRAGMENT:&str="#version 300 es\nprecision highp float;uniform sampler2D source;uniform int rec709;in vec2 uv;out vec4 color;void main(){vec3 c=texture(source,uv).rgb;if(rec709!=0){c=pow(max(c,vec3(0)),vec3(2.4));c=mix(12.92*c,1.055*pow(c,vec3(1./2.4))-.055,step(vec3(.0031308),c));}color=vec4(c*.85,.85);}";
fn pipeline(context: &Rc<dyn IGlContext>) -> Result<Pipeline, MediaError> {
    let gl = context.gl_interface();
    let v = gl.create_shader(0x8b31);
    let f = gl.create_shader(0x8b30);
    if let Some(e) = gl
        .compile_shader_and_get_error(v, VERTEX)
        .or_else(|| gl.compile_shader_and_get_error(f, FRAGMENT))
    {
        gl.delete_shader(v);
        gl.delete_shader(f);
        return Err(error(e));
    }
    let program = gl.create_program();
    gl.attach_shader(program, v);
    gl.attach_shader(program, f);
    gl.delete_shader(v);
    gl.delete_shader(f);
    if let Some(e) = gl.link_program_and_get_error(program) {
        gl.delete_program(program);
        return Err(error(e));
    }
    let mut vao = 0;
    unsafe { gl.gen_vertex_arrays(1, &mut vao) };
    Ok(Pipeline {
        program,
        vao,
        generation: context.shared_frame_context_generation(),
        context: context
            .shared_frame_context_id()
            .ok_or_else(|| error("No admitted GL context"))?,
        queue: context
            .shared_frame_retirement_queue()
            .ok_or_else(|| error("No retirement queue"))?,
    })
}
// Restore every state touched, including masks, enables and unit-zero binding.
struct State<'a> {
    gl: &'a GlInterface,
    program: i32,
    vao: i32,
    draw: i32,
    read: i32,
    active: i32,
    texture: i32,
    viewport: [i32; 4],
    mask: [i32; 4],
    clear: [f32; 4],
    enables: Vec<(i32, bool)>,
    color_mask: unsafe extern "system" fn(u8, u8, u8, u8),
}
impl<'a> State<'a> {
    fn capture(gl: &'a GlInterface) -> Result<Self, MediaError> {
        let p = gl.get_proc_address("glColorMask");
        if p.is_null() {
            return Err(error("Missing glColorMask"));
        }
        let color_mask = unsafe {
            std::mem::transmute::<*const c_void, unsafe extern "system" fn(u8, u8, u8, u8)>(p)
        };
        let active = gl.get_integerv(0x84e0);
        gl.active_texture(0x84c0);
        let texture = gl.get_integerv(0x8069);
        let (mut viewport, mut mask, mut clear) = ([0; 4], [0; 4], [0.; 4]);
        unsafe {
            gl.get_integerv_native(0x0ba2, viewport.as_mut_ptr());
            gl.get_integerv_native(0x0c23, mask.as_mut_ptr());
            gl.get_floatv_native(0x0c22, clear.as_mut_ptr());
        }
        let enables = [
            0x0be2, 0x0b71, 0x0b90, 0x0c11, 0x0b44, 0x8c89, 0x809e, 0x80a0,
        ]
        .into_iter()
        .map(|e| (e, gl.get_integerv(e) != 0))
        .collect();
        Ok(Self {
            gl,
            program: gl.get_integerv(0x8b8d),
            vao: gl.get_integerv(0x85b5),
            draw: gl.get_integerv(0x8ca6),
            read: gl.get_integerv(0x8caa),
            active,
            texture,
            viewport,
            mask,
            clear,
            enables,
            color_mask,
        })
    }
}
impl Drop for State<'_> {
    fn drop(&mut self) {
        let gl = self.gl;
        gl.use_program(self.program);
        gl.bind_vertex_array(self.vao);
        gl.bind_framebuffer(0x8ca9, self.draw);
        gl.bind_framebuffer(0x8ca8, self.read);
        gl.viewport(
            self.viewport[0],
            self.viewport[1],
            self.viewport[2],
            self.viewport[3],
        );
        gl.clear_color(self.clear[0], self.clear[1], self.clear[2], self.clear[3]);
        unsafe {
            (self.color_mask)(
                self.mask[0] as u8,
                self.mask[1] as u8,
                self.mask[2] as u8,
                self.mask[3] as u8,
            )
        }
        for &(e, on) in &self.enables {
            if on {
                gl.enable(e)
            } else {
                gl.disable(e)
            }
        }
        gl.bind_texture(0x0de1, self.texture);
        gl.active_texture(self.active);
    }
}
impl GlTextureRender for Scene {
    fn draw(
        &self,
        context: &Rc<dyn IGlContext>,
        output: u32,
        viewport: RendererViewport,
        inputs: &[FrameLease],
    ) -> Result<(), MediaError> {
        let frame = inputs
            .first()
            .ok_or_else(|| error("The scene requires one shared input"))?;
        let m = frame.metadata();
        if m.alpha != FrameAlpha::Opaque
            || !matches!(m.color, FrameColor::Srgb | FrameColor::Rec709Full)
        {
            return Err(error("This scene admits opaque SDR inputs"));
        }
        let input = frame
            .resource::<SharedFrameTexture>()
            .ok_or_else(|| error("Input is not an admitted GL texture"))?;
        let layout = ferroui_media_controls::frame_layout(m, Size::new(1., 1.), Stretch::Fill)
            .ok_or_else(|| error("No frame layout"))?;
        let inverse = layout.transform.invert();
        let map = |x, y| {
            let p = inverse.transform(Point::new(x, y));
            let u = p.x / m.width as f64;
            let v = p.y / m.height as f64;
            (u as f32, if m.top_left_origin { v } else { 1. - v } as f32)
        };
        let (a, b, c) = (map(0., 0.), map(1., 0.), map(0., 1.));
        let uv = [
            b.0 - a.0,
            b.1 - a.1,
            0.,
            0.,
            c.0 - a.0,
            c.1 - a.1,
            0.,
            0.,
            0.,
            0.,
            1.,
            0.,
            a.0,
            a.1,
            0.,
            1.,
        ];
        let mut cached = self.0.lock().unwrap();
        if cached.as_ref().is_none_or(|p| {
            Some(p.context) != context.shared_frame_context_id()
                || p.generation != context.shared_frame_context_generation()
        }) {
            *cached = Some(pipeline(context)?);
        }
        let pipeline = cached.as_ref().unwrap();
        let gl = context.gl_interface();
        let state = State::capture(&gl)?;
        for &(e, _) in &state.enables {
            gl.disable(e)
        }
        unsafe { (state.color_mask)(1, 1, 1, 1) };
        let fbo = gl.gen_framebuffer();
        gl.bind_framebuffer(0x8d40, fbo);
        gl.framebuffer_texture_2d(0x8d40, 0x8ce0, 0x0de1, output as i32, 0);
        let result = (|| {
            if gl.check_framebuffer_status(0x8d40) != 0x8cd5 {
                return Err(error("Incomplete scene framebuffer"));
            }
            gl.viewport(0, 0, viewport.width as i32, viewport.height as i32);
            gl.clear_color(0., 0., 0., 0.);
            gl.clear(0x4000);
            gl.use_program(pipeline.program);
            gl.bind_vertex_array(pipeline.vao);
            gl.bind_texture(0x0de1, input.texture() as i32);
            unsafe {
                let name = |s: &[u8]| gl.get_uniform_location(pipeline.program, s.as_ptr().cast());
                gl.uniform1i(name(b"source\0"), 0);
                gl.uniform1i(
                    name(b"rec709\0"),
                    (m.color == FrameColor::Rec709Full) as i32,
                );
                gl.uniform_matrix4fv(name(b"uvMap\0"), 1, 0, uv.as_ptr().cast());
                gl.draw_arrays(4, 0, 6);
            }
            if m.color == FrameColor::Rec709Full {
                frame
                    .counters()
                    .gpu_conversion(GpuTransferReason::ColorSpaceConversion);
            }
            if gl.get_error() != 0 {
                return Err(error("Scene GL draw failed"));
            }
            Ok(())
        })();
        drop(state);
        gl.delete_framebuffer(fbo);
        result
    }
}
