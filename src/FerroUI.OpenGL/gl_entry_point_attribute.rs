use crate::entry_points::GetProcAddress;
use crate::{GlContextInfo, GlProfileType};
use std::ffi::c_void;
use std::ptr;

/// Resolves an entry point that is part of the API from a given version on.
pub struct GlMinVersionEntryPoint;

impl GlMinVersionEntryPoint {
    /// The address of `entry` when the context is at least of version
    /// `min_version_major.min_version_minor` (and of `profile`, when given);
    /// null otherwise.
    pub fn get_proc_address(
        get_proc_address: &GetProcAddress,
        context: &GlContextInfo,
        entry: &str,
        min_version_major: i32,
        min_version_minor: i32,
        profile: Option<GlProfileType>,
    ) -> *const c_void {
        if profile.is_some_and(|profile| context.version().type_() != profile) {
            return ptr::null();
        }
        if context.version().major() < min_version_major {
            return ptr::null();
        }
        if context.version().major() == min_version_major && context.version().minor() < min_version_minor {
            return ptr::null();
        }
        get_proc_address(entry)
    }
}

/// Resolves an entry point that an extension provides.
pub struct GlExtensionEntryPoint;

impl GlExtensionEntryPoint {
    /// The address of `entry` when the context supports `extension` (and is
    /// of `profile`, when given); null otherwise.
    pub fn get_proc_address(
        get_proc_address: &GetProcAddress,
        context: &GlContextInfo,
        entry: &str,
        extension: &str,
        profile: Option<GlProfileType>,
    ) -> *const c_void {
        // Ignore different profile type
        if profile.is_some_and(|profile| profile != context.version().type_()) {
            return ptr::null();
        }

        // Check if extension is supported by the current context
        if !context.extensions().contains(extension) {
            return ptr::null();
        }

        get_proc_address(entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GlVersion;
    use std::cell::RefCell;
    use std::collections::HashSet;
    use std::rc::Rc;

    fn recording_loader() -> (GetProcAddress, Rc<RefCell<Vec<String>>>) {
        let asked = Rc::new(RefCell::new(Vec::new()));
        let log = asked.clone();
        let loader: GetProcAddress = Rc::new(move |name: &str| {
            log.borrow_mut().push(name.to_string());
            ptr::NonNull::<c_void>::dangling().as_ptr() as *const c_void
        });
        (loader, asked)
    }

    fn context(type_: GlProfileType, major: i32, minor: i32, extensions: &[&str]) -> GlContextInfo {
        GlContextInfo::new(
            GlVersion::new(type_, major, minor),
            extensions.iter().map(|e| e.to_string()).collect::<HashSet<_>>(),
        )
    }

    #[test]
    fn min_version_resolves_from_the_version_on() {
        let (loader, asked) = recording_loader();

        for (major, minor, expected) in [(2, 1, false), (3, 0, true), (3, 3, true), (4, 0, true)] {
            let info = context(GlProfileType::OpenGL, major, minor, &[]);
            let addr = GlMinVersionEntryPoint::get_proc_address(&loader, &info, "glBlitFramebuffer", 3, 0, None);
            assert_eq!(expected, !addr.is_null(), "{major}.{minor}");
        }
        assert_eq!(3, asked.borrow().len());
    }

    #[test]
    fn min_version_compares_the_minor_version_only_for_the_same_major() {
        let (loader, _) = recording_loader();

        let older = context(GlProfileType::OpenGL, 3, 1, &[]);
        let newer_major = context(GlProfileType::OpenGL, 4, 0, &[]);

        assert!(GlMinVersionEntryPoint::get_proc_address(&loader, &older, "glFoo", 3, 2, None).is_null());
        assert!(!GlMinVersionEntryPoint::get_proc_address(&loader, &newer_major, "glFoo", 3, 2, None).is_null());
    }

    #[test]
    fn min_version_ignores_a_different_profile() {
        let (loader, asked) = recording_loader();
        let info = context(GlProfileType::OpenGLES, 3, 0, &[]);

        let addr = GlMinVersionEntryPoint::get_proc_address(
            &loader,
            &info,
            "glFoo",
            3,
            0,
            Some(GlProfileType::OpenGL),
        );

        assert!(addr.is_null());
        assert!(asked.borrow().is_empty());
        assert!(!GlMinVersionEntryPoint::get_proc_address(&loader, &info, "glFoo", 3, 0, Some(GlProfileType::OpenGLES))
            .is_null());
    }

    #[test]
    fn extension_resolves_only_when_the_extension_is_present() {
        let (loader, asked) = recording_loader();
        let with = context(GlProfileType::OpenGLES, 2, 0, &["GL_OES_vertex_array_object"]);
        let without = context(GlProfileType::OpenGLES, 2, 0, &["GL_OES_EGL_image"]);

        assert!(!GlExtensionEntryPoint::get_proc_address(
            &loader,
            &with,
            "glBindVertexArrayOES",
            "GL_OES_vertex_array_object",
            None
        )
        .is_null());
        assert!(GlExtensionEntryPoint::get_proc_address(
            &loader,
            &without,
            "glBindVertexArrayOES",
            "GL_OES_vertex_array_object",
            None
        )
        .is_null());
        assert_eq!(vec!["glBindVertexArrayOES".to_string()], *asked.borrow());
    }

    #[test]
    fn extension_ignores_a_different_profile() {
        let (loader, asked) = recording_loader();
        let info = context(GlProfileType::OpenGL, 4, 0, &["GL_OES_EGL_image"]);

        let addr = GlExtensionEntryPoint::get_proc_address(
            &loader,
            &info,
            "glEGLImageTargetTexture2DOES",
            "GL_OES_EGL_image",
            Some(GlProfileType::OpenGLES),
        );

        assert!(addr.is_null());
        assert!(asked.borrow().is_empty());
    }
}
