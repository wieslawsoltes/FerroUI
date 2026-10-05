//! A scripted OpenGL implementation for the tests of this crate.
//!
//! The functions record their calls and answer from per-thread state, so
//! every test (each runs on its own thread) sees a fresh implementation.

use crate::entry_points::GetProcAddress;
use crate::gl_consts::*;
use crate::{GlInterface, GlVersion};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::ffi::{c_char, c_void, CStr, CString};
use std::rc::Rc;

#[derive(Default)]
struct State {
    calls: Vec<String>,
    removed: HashSet<String>,
    integers: HashMap<i32, i32>,
    floats: HashMap<i32, f32>,
    strings: HashMap<i32, Option<CString>>,
    indexed_extensions: Vec<CString>,
    error: i32,
    next_name: i32,
    shader_source: Option<String>,
    shader_status: i32,
    shader_log: String,
    program_status: i32,
    program_log: String,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    STATE.with(|state| f(&mut state.borrow_mut()))
}

fn record(call: impl Into<String>) {
    with(|state| state.calls.push(call.into()));
}

fn gen_names(kind: &str, count: i32, res: *mut i32) {
    record(format!("{kind}({count})"));
    for i in 0..count.max(0) as usize {
        let name = with(|state| {
            state.next_name += 1;
            state.next_name
        });
        // SAFETY: the callers of the crate pass storage for `count` names.
        unsafe { *res.add(i) = name };
    }
}

fn delete_names(kind: &str, count: i32, names: *const i32) {
    // SAFETY: the callers of the crate pass `count` names.
    let names = (0..count.max(0) as usize).map(|i| unsafe { *names.add(i) }.to_string()).collect::<Vec<_>>();
    record(format!("{kind}({})", names.join(",")));
}

fn write_log(log: &str, max_length: i32, length: *mut i32, info_log: *mut c_void) {
    let bytes = log.as_bytes();
    let count = bytes.len().min(max_length.max(0) as usize);
    // SAFETY: the callers of the crate pass a buffer of `max_length` bytes
    // and storage for the length.
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), info_log as *mut u8, count);
        *length = count as i32;
    }
}

fn name_of(name: *const c_char) -> String {
    // SAFETY: the callers of the crate pass NUL-terminated names.
    unsafe { CStr::from_ptr(name) }.to_string_lossy().into_owned()
}

extern "system" fn get_integerv(name: i32, rv: *mut i32) {
    let value = with(|state| {
        if name == GL_NUM_EXTENSIONS {
            state.indexed_extensions.len() as i32
        } else {
            state.integers.get(&name).copied().unwrap_or(0)
        }
    });
    // SAFETY: the callers of the crate pass storage for one value.
    unsafe { *rv = value };
}

extern "system" fn get_floatv(name: i32, rv: *mut f32) {
    let value = with(|state| state.floats.get(&name).copied().unwrap_or(0.0));
    // SAFETY: the callers of the crate pass storage for one value.
    unsafe { *rv = value };
}

extern "system" fn get_string(v: i32) -> *const c_char {
    with(|state| match state.strings.get(&v) {
        Some(Some(value)) => value.as_ptr(),
        Some(None) => {
            if v == GL_EXTENSIONS {
                state.error = GL_INVALID_ENUM;
            }
            std::ptr::null()
        }
        None if v == GL_EXTENSIONS => c"".as_ptr(),
        None => std::ptr::null(),
    })
}

extern "system" fn get_stringi(v: i32, index: i32) -> *const c_char {
    record(format!("glGetStringi({index})"));
    with(|state| {
        if v != GL_EXTENSIONS {
            return std::ptr::null();
        }
        state.indexed_extensions.get(index as usize).map_or(std::ptr::null(), |e| e.as_ptr())
    })
}

extern "system" fn get_error() -> i32 {
    with(|state| std::mem::replace(&mut state.error, GL_NO_ERROR))
}

extern "system" fn clear_depth(value: f64) {
    record(format!("glClearDepth({value})"));
}

extern "system" fn clear_depthf(value: f32) {
    record(format!("glClearDepthf({value})"));
}

extern "system" fn clear(bits: i32) {
    record(format!("glClear({bits})"));
}

extern "system" fn viewport(x: i32, y: i32, width: i32, height: i32) {
    record(format!("glViewport({x},{y},{width},{height})"));
}

extern "system" fn flush() {
    record("glFlush");
}

extern "system" fn bind_framebuffer(target: i32, fb: i32) {
    record(format!("glBindFramebuffer({target},{fb})"));
}

#[allow(clippy::too_many_arguments)]
extern "system" fn blit_framebuffer(_: i32, _: i32, _: i32, _: i32, _: i32, _: i32, _: i32, _: i32, _: i32, _: i32) {
    record("glBlitFramebuffer");
}

extern "system" fn gen_framebuffers(count: i32, res: *mut i32) {
    gen_names("glGenFramebuffers", count, res);
}

extern "system" fn delete_framebuffers(count: i32, names: *const i32) {
    delete_names("glDeleteFramebuffers", count, names);
}

extern "system" fn gen_renderbuffers(count: i32, res: *mut i32) {
    gen_names("glGenRenderbuffers", count, res);
}

extern "system" fn delete_renderbuffers(count: i32, names: *const i32) {
    delete_names("glDeleteRenderbuffers", count, names);
}

extern "system" fn gen_textures(count: i32, res: *mut i32) {
    gen_names("glGenTextures", count, res);
}

extern "system" fn delete_textures(count: i32, names: *const i32) {
    delete_names("glDeleteTextures", count, names);
}

extern "system" fn gen_buffers(count: i32, res: *mut i32) {
    gen_names("glGenBuffers", count, res);
}

extern "system" fn delete_buffers(count: i32, names: *const i32) {
    delete_names("glDeleteBuffers", count, names);
}

extern "system" fn gen_vertex_arrays(count: i32, res: *mut i32) {
    gen_names("glGenVertexArrays", count, res);
    with(|state| state.calls.last_mut().unwrap().truncate("glGenVertexArrays".len()));
}

extern "system" fn bind_vertex_array(_: i32) {
    record("glBindVertexArray");
}

extern "system" fn delete_vertex_arrays(_: i32, _: *const i32) {
    record("glDeleteVertexArrays");
}

extern "system" fn gen_vertex_arrays_oes(count: i32, res: *mut i32) {
    gen_names("glGenVertexArraysOES", count, res);
    with(|state| state.calls.last_mut().unwrap().truncate("glGenVertexArraysOES".len()));
}

extern "system" fn bind_vertex_array_oes(_: i32) {
    record("glBindVertexArrayOES");
}

extern "system" fn delete_vertex_arrays_oes(_: i32, _: *const i32) {
    record("glDeleteVertexArraysOES");
}

extern "system" fn shader_source(_shader: i32, count: i32, strings: *const *const c_char, lengths: *const i32) {
    assert_eq!(1, count);
    // SAFETY: the crate passes one string and its length.
    let source = unsafe { std::slice::from_raw_parts(*strings as *const u8, *lengths as usize) };
    with(|state| state.shader_source = Some(String::from_utf8_lossy(source).into_owned()));
}

extern "system" fn compile_shader(_shader: i32) {}

extern "system" fn get_shaderiv(_shader: i32, name: i32, parameters: *mut i32) {
    let value = with(|state| match name {
        GL_COMPILE_STATUS => state.shader_status,
        GL_INFO_LOG_LENGTH => state.shader_log.len() as i32,
        _ => 0,
    });
    // SAFETY: the callers of the crate pass storage for one value.
    unsafe { *parameters = value };
}

extern "system" fn get_shader_info_log(_shader: i32, max_length: i32, length: *mut i32, info_log: *mut c_void) {
    record(format!("glGetShaderInfoLog({max_length})"));
    let log = with(|state| state.shader_log.clone());
    write_log(&log, max_length, length, info_log);
}

extern "system" fn link_program(_program: i32) {}

extern "system" fn get_programiv(_program: i32, name: i32, parameters: *mut i32) {
    let value = with(|state| match name {
        GL_LINK_STATUS => state.program_status,
        GL_INFO_LOG_LENGTH => state.program_log.len() as i32,
        _ => 0,
    });
    // SAFETY: the callers of the crate pass storage for one value.
    unsafe { *parameters = value };
}

extern "system" fn get_program_info_log(_program: i32, max_length: i32, length: *mut i32, info_log: *mut c_void) {
    let log = with(|state| state.program_log.clone());
    write_log(&log, max_length, length, info_log);
}

extern "system" fn bind_attrib_location(program: i32, index: i32, name: *const c_char) {
    record(format!("glBindAttribLocation({program},{index},{})", name_of(name)));
}

extern "system" fn get_attrib_location(program: i32, name: *const c_char) -> i32 {
    record(format!("glGetAttribLocation({program},{})", name_of(name)));
    7
}

extern "system" fn get_uniform_location(program: i32, name: *const c_char) -> i32 {
    record(format!("glGetUniformLocation({program},{})", name_of(name)));
    7
}

/// Stands in for the entry points the tests never call. Only its address is
/// used: an entry point has to resolve to something for the table to load.
extern "system" fn never_called() {
    unreachable!("an entry point without a scripted implementation was called");
}

fn resolve(name: &str) -> *const c_void {
    macro_rules! table {
        ($($gl:literal => $f:expr),* $(,)?) => {
            match name {
                $($gl => $f as *const c_void,)*
                _ => never_called as *const c_void,
            }
        };
    }
    table! {
        "glGetIntegerv" => get_integerv as extern "system" fn(i32, *mut i32),
        "glGetFloatv" => get_floatv as extern "system" fn(i32, *mut f32),
        "glGetString" => get_string as extern "system" fn(i32) -> *const c_char,
        "glGetStringi" => get_stringi as extern "system" fn(i32, i32) -> *const c_char,
        "glGetError" => get_error as extern "system" fn() -> i32,
        "glClearDepth" => clear_depth as extern "system" fn(f64),
        "glClearDepthf" => clear_depthf as extern "system" fn(f32),
        "glClear" => clear as extern "system" fn(i32),
        "glViewport" => viewport as extern "system" fn(i32, i32, i32, i32),
        "glFlush" => flush as extern "system" fn(),
        "glBindFramebuffer" => bind_framebuffer as extern "system" fn(i32, i32),
        "glBlitFramebuffer" => blit_framebuffer as extern "system" fn(i32, i32, i32, i32, i32, i32, i32, i32, i32, i32),
        "glGenFramebuffers" => gen_framebuffers as extern "system" fn(i32, *mut i32),
        "glDeleteFramebuffers" => delete_framebuffers as extern "system" fn(i32, *const i32),
        "glGenRenderbuffers" => gen_renderbuffers as extern "system" fn(i32, *mut i32),
        "glDeleteRenderbuffers" => delete_renderbuffers as extern "system" fn(i32, *const i32),
        "glGenTextures" => gen_textures as extern "system" fn(i32, *mut i32),
        "glDeleteTextures" => delete_textures as extern "system" fn(i32, *const i32),
        "glGenBuffers" => gen_buffers as extern "system" fn(i32, *mut i32),
        "glDeleteBuffers" => delete_buffers as extern "system" fn(i32, *const i32),
        "glGenVertexArrays" => gen_vertex_arrays as extern "system" fn(i32, *mut i32),
        "glBindVertexArray" => bind_vertex_array as extern "system" fn(i32),
        "glDeleteVertexArrays" => delete_vertex_arrays as extern "system" fn(i32, *const i32),
        "glGenVertexArraysOES" => gen_vertex_arrays_oes as extern "system" fn(i32, *mut i32),
        "glBindVertexArrayOES" => bind_vertex_array_oes as extern "system" fn(i32),
        "glDeleteVertexArraysOES" => delete_vertex_arrays_oes as extern "system" fn(i32, *const i32),
        "glShaderSource" => shader_source as extern "system" fn(i32, i32, *const *const c_char, *const i32),
        "glCompileShader" => compile_shader as extern "system" fn(i32),
        "glGetShaderiv" => get_shaderiv as extern "system" fn(i32, i32, *mut i32),
        "glGetShaderInfoLog" => get_shader_info_log as extern "system" fn(i32, i32, *mut i32, *mut c_void),
        "glLinkProgram" => link_program as extern "system" fn(i32),
        "glGetProgramiv" => get_programiv as extern "system" fn(i32, i32, *mut i32),
        "glGetProgramInfoLog" => get_program_info_log as extern "system" fn(i32, i32, *mut i32, *mut c_void),
        "glBindAttribLocation" => bind_attrib_location as extern "system" fn(i32, i32, *const c_char),
        "glGetAttribLocation" => get_attrib_location as extern "system" fn(i32, *const c_char) -> i32,
        "glGetUniformLocation" => get_uniform_location as extern "system" fn(i32, *const c_char) -> i32,
    }
}

/// The scripted implementation of the current test thread.
pub(crate) struct FakeGl {
    version: GlVersion,
}

impl FakeGl {
    /// Resets the implementation of this thread.
    pub(crate) fn install(version: GlVersion) -> FakeGl {
        with(|state| *state = State::default());
        FakeGl { version }
    }

    pub(crate) fn version(&self) -> GlVersion {
        self.version
    }

    /// A loader that resolves every entry point that has not been removed.
    pub(crate) fn loader(&self) -> GetProcAddress {
        Rc::new(|name: &str| {
            if with(|state| state.removed.contains(name)) {
                std::ptr::null()
            } else {
                resolve(name)
            }
        })
    }

    /// The interface over the implementation; the calls made while it is
    /// constructed are not kept.
    pub(crate) fn interface(&self) -> Rc<GlInterface> {
        // SAFETY: the loader resolves to functions of this module with the
        // signatures of the entry points they stand in for; the ones without
        // an implementation are never called by the tests.
        let gl = unsafe { GlInterface::new(self.version, self.loader()) };
        self.clear_calls();
        Rc::new(gl)
    }

    /// Makes the loader return null for the entry point.
    pub(crate) fn remove(&self, name: &str) {
        with(|state| state.removed.insert(name.to_string()));
    }

    pub(crate) fn set_integer(&self, name: i32, value: i32) {
        with(|state| state.integers.insert(name, value));
    }

    pub(crate) fn set_float(&self, name: i32, value: f32) {
        with(|state| state.floats.insert(name, value));
    }

    /// Scripts `glGetString`. `None` for `GL_EXTENSIONS` behaves like a core
    /// profile: the query fails with `GL_INVALID_ENUM`.
    pub(crate) fn set_string(&self, name: i32, value: Option<&str>) {
        with(|state| state.strings.insert(name, value.map(|v| CString::new(v).unwrap())));
    }

    pub(crate) fn set_indexed_extensions(&self, extensions: &[&str]) {
        with(|state| state.indexed_extensions = extensions.iter().map(|e| CString::new(*e).unwrap()).collect());
    }

    pub(crate) fn set_error(&self, error: i32) {
        with(|state| state.error = error);
    }

    pub(crate) fn set_shader_result(&self, status: i32, log: &str) {
        with(|state| {
            state.shader_status = status;
            state.shader_log = log.to_string();
        });
    }

    pub(crate) fn set_program_result(&self, status: i32, log: &str) {
        with(|state| {
            state.program_status = status;
            state.program_log = log.to_string();
        });
    }

    pub(crate) fn shader_source(&self) -> Option<String> {
        with(|state| state.shader_source.clone())
    }

    pub(crate) fn calls(&self) -> Vec<String> {
        with(|state| state.calls.clone())
    }

    pub(crate) fn clear_calls(&self) {
        with(|state| state.calls.clear());
    }
}
