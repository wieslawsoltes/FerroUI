use crate::entry_points::{gl_entry_points, or_else, GetProcAddress};
use crate::gl_consts::*;
use crate::{GlBasicInfoInterface, GlExtensionEntryPoint, GlMinVersionEntryPoint, GlVersion};
use std::collections::HashSet;
use std::ffi::{c_char, c_void, CString};
use std::ops::Deref;
use std::rc::Rc;

/// What a context is: its version and its extensions.
pub struct GlContextInfo {
    version: GlVersion,
    extensions: HashSet<String>,
}

impl GlContextInfo {
    /// Describes a context.
    pub fn new(version: GlVersion, extensions: HashSet<String>) -> Self {
        Self { version, extensions }
    }

    /// Describes the current context, asking it for its extensions.
    ///
    /// # Safety
    /// See [`GlBasicInfoInterface::new`].
    pub unsafe fn create(version: GlVersion, get_proc_address: &GetProcAddress) -> GlContextInfo {
        // SAFETY: the caller upholds the contract of the loader.
        let basic_info_interface = unsafe { GlBasicInfoInterface::new(get_proc_address) };
        let exts = basic_info_interface.get_extensions();
        GlContextInfo::new(version, exts.into_iter().collect())
    }

    /// The version of the API.
    pub fn version(&self) -> GlVersion {
        self.version
    }

    /// The names of the supported extensions.
    pub fn extensions(&self) -> &HashSet<String> {
        &self.extensions
    }
}

/// GlInterface only includes essential members and members necessary for the
/// framework itself. It is not a general-purpose interface for OpenGL API.
///
/// Use [`GlInterface::get_proc_address`] to get GL procedures you need, or
/// integrate it with third-party GL wrappers.
///
/// The members of [`GlBasicInfoInterface`] are available on the interface
/// too.
pub struct GlInterface {
    base: GlBasicInfoInterface,
    get_proc_address: GetProcAddress,
    version: Option<String>,
    vendor: Option<String>,
    renderer: Option<String>,
    context_info: GlContextInfo,
    gl: GlEntryPoints,
}

impl Deref for GlInterface {
    type Target = GlBasicInfoInterface;

    fn deref(&self) -> &GlBasicInfoInterface {
        &self.base
    }
}

gl_entry_points! {
    table GlEntryPoints for GlInterface.gl(get, info: GlContextInfo);

    required safe pub fn clear_stencil(s: i32) = get("glClearStencil");

    required safe pub fn clear_color(r: f32, g: f32, b: f32, a: f32) = get("glClearColor");

    optional safe pub(crate) fn clear_depth_double(value: f64) = get("glClearDepth");

    optional safe pub(crate) fn clear_depth_float(value: f32) = get("glClearDepthf");

    required safe pub fn depth_func(value: i32) = get("glDepthFunc");

    required safe pub fn depth_mask(value: i32) = get("glDepthMask");

    required safe pub fn clear(bits: i32) = get("glClear");

    required safe pub fn viewport(x: i32, y: i32, width: i32, height: i32) = get("glViewport");

    required safe pub fn flush() = get("glFlush");

    required safe pub fn finish() = get("glFinish");

    required unsafe pub fn read_pixels(x: i32, y: i32, width: i32, height: i32, format: i32, type_: i32, data: *mut c_void)
        = get("glReadPixels");

    required unsafe pub fn gen_framebuffers(count: i32, res: *mut i32) = get("glGenFramebuffers");

    required unsafe pub fn delete_framebuffers(count: i32, framebuffers: *const i32) = get("glDeleteFramebuffers");

    required safe pub fn bind_framebuffer(target: i32, fb: i32) = get("glBindFramebuffer");

    required safe pub fn check_framebuffer_status(target: i32) -> i32 = get("glCheckFramebufferStatus");

    optional safe pub fn blit_framebuffer(
        src_x0: i32,
        src_y0: i32,
        src_x1: i32,
        src_y1: i32,
        dst_x0: i32,
        dst_y0: i32,
        dst_x1: i32,
        dst_y1: i32,
        mask: i32,
        filter: i32
    ) = GlMinVersionEntryPoint::get_proc_address(get, info, "glBlitFramebuffer", 3, 0, None);

    required unsafe pub fn gen_renderbuffers(count: i32, res: *mut i32) = get("glGenRenderbuffers");

    required unsafe pub fn delete_renderbuffers(count: i32, renderbuffers: *const i32) = get("glDeleteRenderbuffers");

    required safe pub fn bind_renderbuffer(target: i32, fb: i32) = get("glBindRenderbuffer");

    required safe pub fn renderbuffer_storage(target: i32, internal_format: i32, width: i32, height: i32)
        = get("glRenderbufferStorage");

    required safe pub fn framebuffer_renderbuffer(target: i32, attachment: i32, renderbuffer_target: i32, renderbuffer: i32)
        = get("glFramebufferRenderbuffer");

    required unsafe pub fn gen_textures(count: i32, res: *mut i32) = get("glGenTextures");

    required safe pub fn bind_texture(target: i32, fb: i32) = get("glBindTexture");

    required safe pub fn active_texture(texture: i32) = get("glActiveTexture");

    required unsafe pub fn delete_textures(count: i32, textures: *const i32) = get("glDeleteTextures");

    required unsafe pub fn tex_image_2d(
        target: i32,
        level: i32,
        internal_format: i32,
        width: i32,
        height: i32,
        border: i32,
        format: i32,
        type_: i32,
        data: *const c_void
    ) = get("glTexImage2D");

    required safe pub fn copy_tex_sub_image_2d(
        target: i32,
        level: i32,
        xoffset: i32,
        yoffset: i32,
        x: i32,
        y: i32,
        width: i32,
        height: i32
    ) = get("glCopyTexSubImage2D");

    required safe pub fn tex_parameteri(target: i32, name: i32, value: i32) = get("glTexParameteri");

    required safe pub fn framebuffer_texture_2d(target: i32, attachment: i32, tex_target: i32, texture: i32, level: i32)
        = get("glFramebufferTexture2D");

    required safe pub fn create_shader(shader_type: i32) -> i32 = get("glCreateShader");

    required unsafe pub fn shader_source(shader: i32, count: i32, strings: *const *const c_char, lengths: *const i32)
        = get("glShaderSource");

    required safe pub fn compile_shader(shader: i32) = get("glCompileShader");

    required unsafe pub fn get_shaderiv(shader: i32, name: i32, parameters: *mut i32) = get("glGetShaderiv");

    required unsafe pub fn get_shader_info_log(shader: i32, max_length: i32, length: *mut i32, info_log: *mut c_void)
        = get("glGetShaderInfoLog");

    required safe pub fn create_program() -> i32 = get("glCreateProgram");

    required safe pub fn attach_shader(program: i32, shader: i32) = get("glAttachShader");

    required safe pub fn link_program(program: i32) = get("glLinkProgram");

    required unsafe pub fn get_programiv(program: i32, name: i32, parameters: *mut i32) = get("glGetProgramiv");

    required unsafe pub fn get_program_info_log(program: i32, max_length: i32, len: *mut i32, info_log: *mut c_void)
        = get("glGetProgramInfoLog");

    required unsafe pub fn bind_attrib_location(program: i32, index: i32, name: *const c_char)
        = get("glBindAttribLocation");

    required unsafe pub fn gen_buffers(len: i32, rv: *mut i32) = get("glGenBuffers");

    required safe pub fn bind_buffer(target: i32, buffer: i32) = get("glBindBuffer");

    required unsafe pub fn buffer_data(target: i32, size: isize, data: *const c_void, usage: i32) = get("glBufferData");

    required unsafe pub fn get_attrib_location(program: i32, name: *const c_char) -> i32 = get("glGetAttribLocation");

    required unsafe pub fn vertex_attrib_pointer(
        index: i32,
        size: i32,
        type_: i32,
        normalized: i32,
        stride: i32,
        pointer: *const c_void
    ) = get("glVertexAttribPointer");

    required safe pub fn enable_vertex_attrib_array(index: i32) = get("glEnableVertexAttribArray");

    required safe pub fn use_program(program: i32) = get("glUseProgram");

    required unsafe pub fn draw_arrays(mode: i32, first: i32, count: i32) = get("glDrawArrays");

    required unsafe pub fn draw_elements(mode: i32, count: i32, type_: i32, indices: *const c_void) = get("glDrawElements");

    required unsafe pub fn get_uniform_location(program: i32, name: *const c_char) -> i32 = get("glGetUniformLocation");

    required safe pub fn uniform1f(location: i32, falue: f32) = get("glUniform1f");

    required safe pub fn uniform1i(location: i32, value: i32) = get("glUniform1i");

    /// `transpose` is a `GLboolean`: 0 or 1.
    required unsafe pub fn uniform_matrix4fv(location: i32, count: i32, transpose: u8, value: *const c_void)
        = get("glUniformMatrix4fv");

    required safe pub fn enable(what: i32) = get("glEnable");

    required safe pub fn disable(what: i32) = get("glDisable");

    required unsafe pub fn delete_buffers(count: i32, buffers: *const i32) = get("glDeleteBuffers");

    required safe pub fn delete_program(program: i32) = get("glDeleteProgram");

    required safe pub fn delete_shader(shader: i32) = get("glDeleteShader");

    required unsafe pub fn get_renderbuffer_parameteriv_native(target: i32, name: i32, value: *mut i32)
        = get("glGetRenderbufferParameteriv");

    optional unsafe pub fn delete_vertex_arrays(count: i32, arrays: *const i32) = or_else(
        GlMinVersionEntryPoint::get_proc_address(get, info, "glDeleteVertexArrays", 3, 0, None),
        || GlExtensionEntryPoint::get_proc_address(get, info, "glDeleteVertexArraysOES", "GL_OES_vertex_array_object", None),
    );

    optional safe pub fn bind_vertex_array(array: i32) = or_else(
        GlMinVersionEntryPoint::get_proc_address(get, info, "glBindVertexArray", 3, 0, None),
        || GlExtensionEntryPoint::get_proc_address(get, info, "glBindVertexArrayOES", "GL_OES_vertex_array_object", None),
    );

    optional unsafe pub fn gen_vertex_arrays(n: i32, rv: *mut i32) = or_else(
        GlMinVersionEntryPoint::get_proc_address(get, info, "glGenVertexArrays", 3, 0, None),
        || GlExtensionEntryPoint::get_proc_address(get, info, "glGenVertexArraysOES", "GL_OES_vertex_array_object", None),
    );

    optional safe pub fn read_buffer(buffer: i32) = get("glReadBuffer");

    optional safe pub fn draw_buffer(buffer: i32) = get("glDrawBuffer");

    optional safe pub fn write_buffer(buffer: i32) = get("glWriteBuffer");

    // GL_OES_EGL_image
    optional unsafe pub fn egl_image_target_texture_2d_oes(target: i32, image: *mut c_void)
        = GlExtensionEntryPoint::get_proc_address(get, info, "glEGLImageTargetTexture2DOES", "GL_OES_EGL_image", None);
}

impl GlInterface {
    /// # Safety
    /// See [`GlBasicInfoInterface::new`].
    unsafe fn with_info(info: GlContextInfo, get_proc_address: GetProcAddress) -> Self {
        // SAFETY: the caller upholds the contract of the loader.
        let base = unsafe { GlBasicInfoInterface::new(&get_proc_address) };
        let version = base.get_string(GL_VERSION);
        let renderer = base.get_string(GL_RENDERER);
        let vendor = base.get_string(GL_VENDOR);
        let gl = GlEntryPoints::load(&get_proc_address, &info);
        Self { base, get_proc_address, version, vendor, renderer, context_info: info, gl }
    }

    /// Resolves the entry points of the current context, which implements
    /// `version` of the API.
    ///
    /// # Safety
    /// `get_proc_address` must return, for the name of an OpenGL entry point,
    /// either null or the address of that entry point in an OpenGL
    /// implementation whose calling convention is the platform's default
    /// one. The interface must only be used on a thread on which a context
    /// of that implementation is current.
    ///
    /// # Panics
    /// Panics when a required entry point cannot be resolved.
    pub unsafe fn new(version: GlVersion, get_proc_address: GetProcAddress) -> Self {
        // SAFETY: the caller upholds the contract of the loader.
        unsafe {
            let info = GlContextInfo::create(version, &get_proc_address);
            Self::with_info(info, get_proc_address)
        }
    }

    /// The `GL_VERSION` string of the context.
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    /// The `GL_VENDOR` string of the context.
    pub fn vendor(&self) -> Option<&str> {
        self.vendor.as_deref()
    }

    /// The `GL_RENDERER` string of the context.
    pub fn renderer(&self) -> Option<&str> {
        self.renderer.as_deref()
    }

    /// The version and the extensions of the context.
    pub fn context_info(&self) -> &GlContextInfo {
        &self.context_info
    }

    /// Returns an OpenGL function by name.
    ///
    /// The result is the address of the function, which can be cast to a
    /// function pointer of its signature; null when the context does not
    /// have the function.
    pub fn get_proc_address(&self, proc: &str) -> *const c_void {
        (self.get_proc_address)(proc)
    }

    /// `glClearDepth` or, where only that exists, `glClearDepthf`.
    pub fn clear_depth(&self, value: f32) {
        if self.is_clear_depth_double_available() {
            self.clear_depth_double(value as f64);
        } else if self.is_clear_depth_float_available() {
            self.clear_depth_float(value);
        }
    }

    /// Creates one framebuffer object.
    pub fn gen_framebuffer(&self) -> i32 {
        let mut rv = 0;
        // SAFETY: `rv` is valid for the one name written.
        unsafe { self.gen_framebuffers(1, &mut rv) };
        rv
    }

    /// Deletes one framebuffer object.
    pub fn delete_framebuffer(&self, fb: i32) {
        // SAFETY: `fb` is valid for the one name read.
        unsafe { self.delete_framebuffers(1, &fb) };
    }

    /// Creates one renderbuffer object.
    pub fn gen_renderbuffer(&self) -> i32 {
        let mut rv = 0;
        // SAFETY: `rv` is valid for the one name written.
        unsafe { self.gen_renderbuffers(1, &mut rv) };
        rv
    }

    /// Deletes one renderbuffer object.
    pub fn delete_renderbuffer(&self, renderbuffer: i32) {
        // SAFETY: `renderbuffer` is valid for the one name read.
        unsafe { self.delete_renderbuffers(1, &renderbuffer) };
    }

    /// Creates one texture object.
    pub fn gen_texture(&self) -> i32 {
        let mut rv = 0;
        // SAFETY: `rv` is valid for the one name written.
        unsafe { self.gen_textures(1, &mut rv) };
        rv
    }

    /// Deletes one texture object.
    pub fn delete_texture(&self, texture: i32) {
        // SAFETY: `texture` is valid for the one name read.
        unsafe { self.delete_textures(1, &texture) };
    }

    /// Replaces the source of a shader.
    pub fn shader_source_string(&self, shader: i32, source: &str) {
        let ptr = source.as_ptr() as *const c_char;
        let len = source.len() as i32;
        // SAFETY: one string of `len` bytes is passed with its explicit
        // length, so no terminator is required; both locals outlive the call.
        unsafe { self.shader_source(shader, 1, &ptr, &len) };
    }

    /// Compiles `source` as the shader. Returns the compiler's log when the
    /// compilation fails and `None` when it succeeds.
    pub fn compile_shader_and_get_error(&self, shader: i32, source: &str) -> Option<String> {
        self.shader_source_string(shader, source);
        self.compile_shader(shader);
        let mut compiled = 0;
        // SAFETY: `compiled` is valid for the one value written.
        unsafe { self.get_shaderiv(shader, GL_COMPILE_STATUS, &mut compiled) };
        if compiled != 0 {
            return None;
        }
        let mut log_length = 0;
        // SAFETY: `log_length` is valid for the one value written.
        unsafe { self.get_shaderiv(shader, GL_INFO_LOG_LENGTH, &mut log_length) };
        if log_length == 0 {
            log_length = 4096;
        }
        let mut log_data = vec![0u8; log_length.max(0) as usize];
        let mut len = 0;
        // SAFETY: the buffer holds `log_length` bytes, the most OpenGL writes.
        unsafe { self.get_shader_info_log(shader, log_length, &mut len, log_data.as_mut_ptr() as *mut c_void) };
        Some(Self::log_to_string(&log_data, len))
    }

    /// Links the program. Returns the linker's log when linking fails and
    /// `None` when it succeeds.
    pub fn link_program_and_get_error(&self, program: i32) -> Option<String> {
        self.link_program(program);
        let mut compiled = 0;
        // SAFETY: `compiled` is valid for the one value written.
        unsafe { self.get_programiv(program, GL_LINK_STATUS, &mut compiled) };
        if compiled != 0 {
            return None;
        }
        let mut log_length = 0;
        // SAFETY: `log_length` is valid for the one value written.
        unsafe { self.get_programiv(program, GL_INFO_LOG_LENGTH, &mut log_length) };
        let mut log_data = vec![0u8; log_length.max(0) as usize];
        let mut len = 0;
        // SAFETY: the buffer holds `log_length` bytes, the most OpenGL writes.
        unsafe { self.get_program_info_log(program, log_length, &mut len, log_data.as_mut_ptr() as *mut c_void) };
        Some(Self::log_to_string(&log_data, len))
    }

    fn log_to_string(log_data: &[u8], len: i32) -> String {
        let len = (len.max(0) as usize).min(log_data.len());
        String::from_utf8_lossy(&log_data[..len]).into_owned()
    }

    fn c_string(value: &str) -> CString {
        CString::new(value).unwrap_or_else(|_| panic!("The name must not contain a NUL character."))
    }

    /// Binds a vertex attribute of the program to a location.
    pub fn bind_attrib_location_string(&self, program: i32, index: i32, name: &str) {
        let b = Self::c_string(name);
        // SAFETY: `b` is a NUL-terminated string that outlives the call.
        unsafe { self.bind_attrib_location(program, index, b.as_ptr()) };
    }

    /// Creates one buffer object.
    pub fn gen_buffer(&self) -> i32 {
        let mut rv = 0;
        // SAFETY: `rv` is valid for the one name written.
        unsafe { self.gen_buffers(1, &mut rv) };
        rv
    }

    /// The location of a vertex attribute of the program.
    pub fn get_attrib_location_string(&self, program: i32, name: &str) -> i32 {
        let b = Self::c_string(name);
        // SAFETY: `b` is a NUL-terminated string that outlives the call.
        unsafe { self.get_attrib_location(program, b.as_ptr()) }
    }

    /// The location of a uniform of the program.
    pub fn get_uniform_location_string(&self, program: i32, name: &str) -> i32 {
        let b = Self::c_string(name);
        // SAFETY: `b` is a NUL-terminated string that outlives the call.
        unsafe { self.get_uniform_location(program, b.as_ptr()) }
    }

    /// Deletes one buffer object.
    pub fn delete_buffer(&self, buffer: i32) {
        // SAFETY: `buffer` is valid for the one name read.
        unsafe { self.delete_buffers(1, &buffer) };
    }

    /// `glGetRenderbufferParameteriv` for a parameter with one value.
    pub fn get_renderbuffer_parameteriv(&self, target: i32, name: i32) -> i32 {
        let mut value = 0;
        // SAFETY: `value` is valid for the one value written.
        unsafe { self.get_renderbuffer_parameteriv_native(target, name, &mut value) };
        value
    }

    /// Deletes one vertex array object.
    pub fn delete_vertex_array(&self, array: i32) {
        // SAFETY: `array` is valid for the one name read.
        unsafe { self.delete_vertex_arrays(1, &array) };
    }

    /// Creates one vertex array object.
    pub fn gen_vertex_array(&self) -> i32 {
        let mut rv = 0;
        // SAFETY: `rv` is valid for the one name written.
        unsafe { self.gen_vertex_arrays(1, &mut rv) };
        rv
    }

    /// Creates the interface from a loader that takes a NUL-terminated name.
    ///
    /// # Safety
    /// See [`GlInterface::new`]; `get_proc_address` must in addition accept a
    /// pointer to a NUL-terminated name that is only valid during the call.
    pub unsafe fn from_native_utf8_get_proc_address(
        version: GlVersion,
        get_proc_address: impl Fn(*const c_char) -> *const c_void + 'static,
    ) -> GlInterface {
        let loader: GetProcAddress = Rc::new(move |s: &str| {
            let name = Self::c_string(s);
            get_proc_address(name.as_ptr())
        });
        // SAFETY: the caller upholds the contract of the loader.
        unsafe { GlInterface::new(version, loader) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::FakeGl;
    use crate::GlProfileType;

    fn gles3() -> (FakeGl, Rc<GlInterface>) {
        let fake = FakeGl::install(GlVersion::new(GlProfileType::OpenGLES, 3, 0));
        let gl = fake.interface();
        (fake, gl)
    }

    #[test]
    fn the_strings_and_the_context_info_are_read_at_construction() {
        let fake = FakeGl::install(GlVersion::new(GlProfileType::OpenGLES, 3, 0));
        fake.set_string(GL_VERSION, Some("OpenGL ES 3.0 Fake"));
        fake.set_string(GL_VENDOR, Some("Fake Vendor"));
        fake.set_string(GL_RENDERER, Some("Fake Renderer"));
        fake.set_string(GL_EXTENSIONS, Some("GL_EXT_a GL_EXT_b"));

        let gl = fake.interface();

        assert_eq!(Some("OpenGL ES 3.0 Fake"), gl.version());
        assert_eq!(Some("Fake Vendor"), gl.vendor());
        assert_eq!(Some("Fake Renderer"), gl.renderer());
        assert_eq!(GlVersion::new(GlProfileType::OpenGLES, 3, 0), gl.context_info().version());
        assert!(gl.context_info().extensions().contains("GL_EXT_a"));
        assert!(gl.context_info().extensions().contains("GL_EXT_b"));
        assert_eq!(2, gl.context_info().extensions().len());
    }

    #[test]
    fn the_basic_info_members_are_available_on_the_interface() {
        let (fake, gl) = gles3();
        fake.set_integer(GL_TEXTURE_BINDING_2D, 11);

        assert_eq!(11, gl.get_integerv(GL_TEXTURE_BINDING_2D));
        assert_eq!(GL_NO_ERROR, gl.get_error());
    }

    #[test]
    fn get_proc_address_forwards_to_the_loader() {
        let (fake, gl) = gles3();

        assert!(!gl.get_proc_address("glClear").is_null());
        fake.remove("glNotThere");
        assert!(gl.get_proc_address("glNotThere").is_null());
    }

    #[test]
    #[should_panic(expected = "Unable to find an entry point named 'bind_framebuffer'")]
    fn a_missing_required_entry_point_fails_the_construction() {
        let fake = FakeGl::install(GlVersion::new(GlProfileType::OpenGLES, 3, 0));
        fake.remove("glBindFramebuffer");
        fake.interface();
    }

    #[test]
    fn a_missing_optional_entry_point_is_reported_as_unavailable() {
        let fake = FakeGl::install(GlVersion::new(GlProfileType::OpenGLES, 3, 0));
        fake.remove("glReadBuffer");
        fake.remove("glDrawBuffer");
        fake.remove("glWriteBuffer");
        let gl = fake.interface();

        assert!(!gl.is_read_buffer_available());
        assert!(!gl.is_draw_buffer_available());
        assert!(!gl.is_write_buffer_available());
    }

    #[test]
    #[should_panic(expected = "Unable to find an entry point named 'read_buffer'")]
    fn calling_a_missing_optional_entry_point_fails() {
        let fake = FakeGl::install(GlVersion::new(GlProfileType::OpenGLES, 3, 0));
        fake.remove("glReadBuffer");
        let gl = fake.interface();

        gl.read_buffer(GL_BACK);
    }

    #[test]
    fn blit_framebuffer_needs_version_three() {
        let es2 = FakeGl::install(GlVersion::new(GlProfileType::OpenGLES, 2, 0)).interface();
        assert!(!es2.is_blit_framebuffer_available());

        let (fake, es3) = gles3();
        assert!(es3.is_blit_framebuffer_available());
        es3.blit_framebuffer(0, 0, 4, 4, 0, 0, 4, 4, GL_COLOR_BUFFER_BIT, GL_LINEAR);
        assert!(fake.calls().contains(&"glBlitFramebuffer".to_string()));
    }

    #[test]
    fn vertex_arrays_come_from_the_core_api_from_version_three() {
        let (fake, gl) = gles3();

        assert!(gl.is_gen_vertex_arrays_available());
        assert!(gl.is_bind_vertex_array_available());
        assert!(gl.is_delete_vertex_arrays_available());
        let array = gl.gen_vertex_array();
        gl.bind_vertex_array(array);
        gl.delete_vertex_array(array);

        assert_eq!(
            vec!["glGenVertexArrays", "glBindVertexArray", "glDeleteVertexArrays"],
            fake.calls().iter().filter(|c| c.contains("VertexArray")).map(String::as_str).collect::<Vec<_>>()
        );
    }

    #[test]
    fn vertex_arrays_fall_back_to_the_extension_before_version_three() {
        let fake = FakeGl::install(GlVersion::new(GlProfileType::OpenGLES, 2, 0));
        fake.set_string(GL_EXTENSIONS, Some("GL_OES_vertex_array_object"));
        let gl = fake.interface();

        assert!(gl.is_gen_vertex_arrays_available());
        let array = gl.gen_vertex_array();
        gl.bind_vertex_array(array);
        gl.delete_vertex_array(array);

        assert_eq!(
            vec!["glGenVertexArraysOES", "glBindVertexArrayOES", "glDeleteVertexArraysOES"],
            fake.calls().iter().filter(|c| c.contains("VertexArray")).map(String::as_str).collect::<Vec<_>>()
        );
    }

    #[test]
    fn vertex_arrays_are_unavailable_without_the_version_or_the_extension() {
        let gl = FakeGl::install(GlVersion::new(GlProfileType::OpenGLES, 2, 0)).interface();

        assert!(!gl.is_gen_vertex_arrays_available());
        assert!(!gl.is_bind_vertex_array_available());
        assert!(!gl.is_delete_vertex_arrays_available());
    }

    #[test]
    fn the_egl_image_entry_point_needs_its_extension() {
        let without = FakeGl::install(GlVersion::new(GlProfileType::OpenGLES, 3, 0)).interface();
        assert!(!without.is_egl_image_target_texture_2d_oes_available());

        let fake = FakeGl::install(GlVersion::new(GlProfileType::OpenGLES, 3, 0));
        fake.set_string(GL_EXTENSIONS, Some("GL_OES_EGL_image"));
        assert!(fake.interface().is_egl_image_target_texture_2d_oes_available());
    }

    #[test]
    fn clear_depth_prefers_the_double_entry_point() {
        let (fake, gl) = gles3();

        gl.clear_depth(0.5);

        assert_eq!(vec!["glClearDepth(0.5)"], fake.calls());
    }

    #[test]
    fn clear_depth_falls_back_to_the_float_entry_point() {
        let fake = FakeGl::install(GlVersion::new(GlProfileType::OpenGLES, 3, 0));
        fake.remove("glClearDepth");
        let gl = fake.interface();
        fake.clear_calls();

        gl.clear_depth(0.25);

        assert_eq!(vec!["glClearDepthf(0.25)"], fake.calls());
    }

    #[test]
    fn clear_depth_does_nothing_without_either_entry_point() {
        let fake = FakeGl::install(GlVersion::new(GlProfileType::OpenGLES, 3, 0));
        fake.remove("glClearDepth");
        fake.remove("glClearDepthf");
        let gl = fake.interface();
        fake.clear_calls();

        gl.clear_depth(1.0);

        assert!(fake.calls().is_empty());
    }

    #[test]
    fn single_object_helpers_create_and_delete_one_name() {
        let (fake, gl) = gles3();

        let framebuffer = gl.gen_framebuffer();
        let renderbuffer = gl.gen_renderbuffer();
        let texture = gl.gen_texture();
        let buffer = gl.gen_buffer();
        assert!(framebuffer > 0 && renderbuffer > framebuffer && texture > renderbuffer && buffer > texture);

        gl.delete_framebuffer(framebuffer);
        gl.delete_renderbuffer(renderbuffer);
        gl.delete_texture(texture);
        gl.delete_buffer(buffer);

        assert_eq!(
            vec![
                "glGenFramebuffers(1)".to_string(),
                "glGenRenderbuffers(1)".to_string(),
                "glGenTextures(1)".to_string(),
                "glGenBuffers(1)".to_string(),
                format!("glDeleteFramebuffers({framebuffer})"),
                format!("glDeleteRenderbuffers({renderbuffer})"),
                format!("glDeleteTextures({texture})"),
                format!("glDeleteBuffers({buffer})"),
            ],
            fake.calls()
        );
    }

    #[test]
    fn plain_calls_reach_the_entry_point_with_their_arguments() {
        let (fake, gl) = gles3();

        gl.viewport(1, 2, 30, 40);
        gl.bind_framebuffer(GL_FRAMEBUFFER, 5);
        gl.clear(GL_COLOR_BUFFER_BIT);

        assert_eq!(
            vec![
                "glViewport(1,2,30,40)".to_string(),
                format!("glBindFramebuffer({GL_FRAMEBUFFER},5)"),
                format!("glClear({GL_COLOR_BUFFER_BIT})")
            ],
            fake.calls()
        );
    }

    #[test]
    fn a_successful_compilation_reports_no_error() {
        let (fake, gl) = gles3();
        fake.set_shader_result(1, "");

        assert_eq!(None, gl.compile_shader_and_get_error(3, "void main() {}"));
        assert_eq!(Some("void main() {}".to_string()), fake.shader_source());
    }

    #[test]
    fn a_failed_compilation_reports_the_log() {
        let (fake, gl) = gles3();
        fake.set_shader_result(0, "0:1: syntax error");

        assert_eq!(Some("0:1: syntax error".to_string()), gl.compile_shader_and_get_error(3, "broken"));
    }

    #[test]
    fn a_failed_compilation_without_a_log_length_reads_into_a_default_buffer() {
        let (fake, gl) = gles3();
        fake.set_shader_result(0, "");

        assert_eq!(Some(String::new()), gl.compile_shader_and_get_error(3, "broken"));
        assert!(fake.calls().contains(&"glGetShaderInfoLog(4096)".to_string()));
    }

    #[test]
    fn linking_reports_the_log_only_on_failure() {
        let (fake, gl) = gles3();

        fake.set_program_result(1, "");
        assert_eq!(None, gl.link_program_and_get_error(9));

        fake.set_program_result(0, "unresolved symbol");
        assert_eq!(Some("unresolved symbol".to_string()), gl.link_program_and_get_error(9));
    }

    #[test]
    fn names_are_passed_as_terminated_strings() {
        let (fake, gl) = gles3();

        gl.bind_attrib_location_string(4, 0, "aPos");
        assert_eq!(7, gl.get_attrib_location_string(4, "aTexCoord"));
        assert_eq!(7, gl.get_uniform_location_string(4, "uMatrix"));

        assert_eq!(
            vec!["glBindAttribLocation(4,0,aPos)", "glGetAttribLocation(4,aTexCoord)", "glGetUniformLocation(4,uMatrix)"],
            fake.calls()
        );
    }

    #[test]
    fn a_loader_taking_native_names_receives_terminated_strings() {
        let fake = FakeGl::install(GlVersion::new(GlProfileType::OpenGLES, 3, 0));
        let loader = fake.loader();
        let native = move |name: *const c_char| {
            // SAFETY: the interface passes a NUL-terminated name.
            let name = unsafe { std::ffi::CStr::from_ptr(name) }.to_str().unwrap().to_string();
            loader(&name)
        };

        // SAFETY: the loader of the fake resolves to functions of this crate.
        let gl = unsafe { GlInterface::from_native_utf8_get_proc_address(fake.version(), native) };

        gl.flush();
        assert!(fake.calls().contains(&"glFlush".to_string()));
    }
}
