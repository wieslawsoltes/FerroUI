use crate::entry_points::{gl_entry_points, GetProcAddress};
use crate::gl_consts;
use std::ffi::{c_char, CStr};

/// The entry points needed to find out what a context is: its strings, its
/// limits and its extensions.
pub struct GlBasicInfoInterface {
    basic: BasicInfoEntryPoints,
}

gl_entry_points! {
    table BasicInfoEntryPoints for GlBasicInfoInterface.basic(get, info: ());

    /// `glGetIntegerv` writing to `rv`.
    required unsafe pub fn get_integerv_native(name: i32, rv: *mut i32) = get("glGetIntegerv");

    /// `glGetFloatv` writing to `rv`.
    required unsafe pub fn get_floatv_native(name: i32, rv: *mut f32) = get("glGetFloatv");

    /// `glGetString`: a string owned by the context, or null.
    required safe pub fn get_string_native(v: i32) -> *const c_char = get("glGetString");

    /// `glGetStringi`: a string owned by the context, or null.
    required safe pub fn get_stringi_native(v: i32, v1: i32) -> *const c_char = get("glGetStringi");

    /// `glGetError`: the oldest pending error, which is thereby consumed.
    required safe pub fn get_error() -> i32 = get("glGetError");
}

/// The number of values the single-value queries reserve room for.
const SCRATCH_VALUES: usize = 256;

impl GlBasicInfoInterface {
    /// Resolves the entry points.
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
    pub unsafe fn new(get_proc_address: &GetProcAddress) -> Self {
        Self { basic: BasicInfoEntryPoints::load(get_proc_address, &()) }
    }

    /// `glGetIntegerv` for a parameter with one value.
    ///
    /// The first value of a parameter with several values is returned.
    /// Parameters whose number of values depends on the context (the lists
    /// of formats) are read with
    /// [`get_integerv_native`](Self::get_integerv_native) and storage of the
    /// right size.
    pub fn get_integerv(&self, name: i32) -> i32 {
        let mut rv = [0i32; SCRATCH_VALUES];
        // SAFETY: the storage holds more values than any parameter with a
        // fixed number of values writes (the largest are the 16 of a
        // matrix); the lists of formats, whose length the context decides,
        // stay far below it in practice.
        unsafe { self.get_integerv_native(name, rv.as_mut_ptr()) };
        rv[0]
    }

    /// `glGetFloatv` for a parameter with one value.
    ///
    /// The first value of a parameter with several values is returned.
    pub fn get_floatv(&self, name: i32) -> f32 {
        let mut rv = [0f32; SCRATCH_VALUES];
        // SAFETY: as in `get_integerv`.
        unsafe { self.get_floatv_native(name, rv.as_mut_ptr()) };
        rv[0]
    }

    fn to_string(ptr: *const c_char) -> Option<String> {
        if ptr.is_null() {
            return None;
        }
        // SAFETY: OpenGL returns static, NUL-terminated strings from
        // `glGetString` and `glGetStringi`.
        Some(unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned())
    }

    /// `glGetString`, `None` when the context has no such string.
    pub fn get_string(&self, v: i32) -> Option<String> {
        Self::to_string(self.get_string_native(v))
    }

    /// `glGetStringi`, `None` when the context has no such string.
    pub fn get_string_indexed(&self, v: i32, index: i32) -> Option<String> {
        Self::to_string(self.get_stringi_native(v, index))
    }

    /// The names of the extensions of the context.
    pub fn get_extensions(&self) -> Vec<String> {
        // On some (generally older) versions of OpenGL, GL_EXTENSIONS is a space-separated list of available extensions.
        // For example:
        // https://learn.microsoft.com/en-us/windows/win32/opengl/glgetstring
        // https://docs.gl/gl2/glGetString
        // https://registry.khronos.org/OpenGL-Refpages/es3.0/html/glGetString.xhtml
        if let Some(sp) = self.get_string(gl_consts::GL_EXTENSIONS) {
            return sp.split(' ').map(str::to_string).collect();
        }
        // The OpenGL version is not such a version.
        // Consume the GL_INVALID_ENUM error from the GetString call above.
        self.get_error();

        // For other (generally newer) versions
        let count = self.get_integerv(gl_consts::GL_NUM_EXTENSIONS);
        let mut rv = Vec::with_capacity(count.max(0) as usize);
        for c in 0..count {
            if let Some(extension) = self.get_string_indexed(gl_consts::GL_EXTENSIONS, c) {
                rv.push(extension);
            }
        }

        rv
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::FakeGl;
    use crate::{GlProfileType, GlVersion};

    #[test]
    fn single_values_are_read_through_the_out_parameter() {
        let fake = FakeGl::install(GlVersion::new(GlProfileType::OpenGLES, 3, 0));
        fake.set_integer(gl_consts::GL_FRAMEBUFFER_BINDING, 7);
        fake.set_float(0x0B21, 2.5);
        // SAFETY: the loader of the fake resolves to functions of this crate.
        let basic = unsafe { GlBasicInfoInterface::new(&fake.loader()) };

        assert_eq!(7, basic.get_integerv(gl_consts::GL_FRAMEBUFFER_BINDING));
        assert_eq!(2.5, basic.get_floatv(0x0B21));
    }

    #[test]
    fn strings_are_copied_and_missing_ones_are_none() {
        let fake = FakeGl::install(GlVersion::new(GlProfileType::OpenGLES, 3, 0));
        fake.set_string(gl_consts::GL_VENDOR, Some("Fake Vendor"));
        fake.set_string(gl_consts::GL_RENDERER, None);
        // SAFETY: as above.
        let basic = unsafe { GlBasicInfoInterface::new(&fake.loader()) };

        assert_eq!(Some("Fake Vendor".to_string()), basic.get_string(gl_consts::GL_VENDOR));
        assert_eq!(None, basic.get_string(gl_consts::GL_RENDERER));
    }

    #[test]
    fn extensions_come_from_the_space_separated_string_when_there_is_one() {
        let fake = FakeGl::install(GlVersion::new(GlProfileType::OpenGLES, 2, 0));
        fake.set_string(gl_consts::GL_EXTENSIONS, Some("GL_OES_vertex_array_object GL_OES_EGL_image"));
        // SAFETY: as above.
        let basic = unsafe { GlBasicInfoInterface::new(&fake.loader()) };

        assert_eq!(vec!["GL_OES_vertex_array_object", "GL_OES_EGL_image"], basic.get_extensions());
        assert!(!fake.calls().iter().any(|call| call.starts_with("glGetStringi")));
    }

    #[test]
    fn extensions_are_enumerated_when_the_string_is_not_available() {
        let fake = FakeGl::install(GlVersion::new(GlProfileType::OpenGL, 4, 1));
        fake.set_string(gl_consts::GL_EXTENSIONS, None);
        fake.set_indexed_extensions(&["GL_ARB_one", "GL_ARB_two", "GL_ARB_three"]);
        // SAFETY: as above.
        let basic = unsafe { GlBasicInfoInterface::new(&fake.loader()) };

        assert_eq!(vec!["GL_ARB_one", "GL_ARB_two", "GL_ARB_three"], basic.get_extensions());
        // The GL_INVALID_ENUM of the failed string query has been consumed.
        assert_eq!(gl_consts::GL_NO_ERROR, basic.get_error());
    }

    #[test]
    #[should_panic(expected = "Unable to find an entry point named 'get_stringi_native'")]
    fn a_missing_required_entry_point_fails_the_construction() {
        let fake = FakeGl::install(GlVersion::new(GlProfileType::OpenGLES, 3, 0));
        fake.remove("glGetStringi");
        // SAFETY: as above.
        let _ = unsafe { GlBasicInfoInterface::new(&fake.loader()) };
    }
}
