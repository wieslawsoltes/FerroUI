use super::egl_consts;

/// The error codes `eglGetError` reports.
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum EglErrors {
    EGL_SUCCESS = egl_consts::EGL_SUCCESS,
    EGL_NOT_INITIALIZED = egl_consts::EGL_NOT_INITIALIZED,
    EGL_BAD_ACCESS = egl_consts::EGL_BAD_ACCESS,
    EGL_BAD_ALLOC = egl_consts::EGL_BAD_ALLOC,
    EGL_BAD_ATTRIBUTE = egl_consts::EGL_BAD_ATTRIBUTE,
    EGL_BAD_CONTEXT = egl_consts::EGL_BAD_CONTEXT,
    EGL_BAD_CONFIG = egl_consts::EGL_BAD_CONFIG,
    EGL_BAD_CURRENT_SURFACE = egl_consts::EGL_BAD_CURRENT_SURFACE,
    EGL_BAD_DISPLAY = egl_consts::EGL_BAD_DISPLAY,
    EGL_BAD_SURFACE = egl_consts::EGL_BAD_SURFACE,
    EGL_BAD_MATCH = egl_consts::EGL_BAD_MATCH,
    EGL_BAD_PARAMETER = egl_consts::EGL_BAD_PARAMETER,
    EGL_BAD_NATIVE_PIXMAP = egl_consts::EGL_BAD_NATIVE_PIXMAP,
    EGL_BAD_NATIVE_WINDOW = egl_consts::EGL_BAD_NATIVE_WINDOW,
    EGL_CONTEXT_LOST = egl_consts::EGL_CONTEXT_LOST,
}

impl EglErrors {
    /// The error with the given code, when the code is a known one.
    pub fn from_code(code: i32) -> Option<EglErrors> {
        Some(match code {
            egl_consts::EGL_SUCCESS => EglErrors::EGL_SUCCESS,
            egl_consts::EGL_NOT_INITIALIZED => EglErrors::EGL_NOT_INITIALIZED,
            egl_consts::EGL_BAD_ACCESS => EglErrors::EGL_BAD_ACCESS,
            egl_consts::EGL_BAD_ALLOC => EglErrors::EGL_BAD_ALLOC,
            egl_consts::EGL_BAD_ATTRIBUTE => EglErrors::EGL_BAD_ATTRIBUTE,
            egl_consts::EGL_BAD_CONTEXT => EglErrors::EGL_BAD_CONTEXT,
            egl_consts::EGL_BAD_CONFIG => EglErrors::EGL_BAD_CONFIG,
            egl_consts::EGL_BAD_CURRENT_SURFACE => EglErrors::EGL_BAD_CURRENT_SURFACE,
            egl_consts::EGL_BAD_DISPLAY => EglErrors::EGL_BAD_DISPLAY,
            egl_consts::EGL_BAD_SURFACE => EglErrors::EGL_BAD_SURFACE,
            egl_consts::EGL_BAD_MATCH => EglErrors::EGL_BAD_MATCH,
            egl_consts::EGL_BAD_PARAMETER => EglErrors::EGL_BAD_PARAMETER,
            egl_consts::EGL_BAD_NATIVE_PIXMAP => EglErrors::EGL_BAD_NATIVE_PIXMAP,
            egl_consts::EGL_BAD_NATIVE_WINDOW => EglErrors::EGL_BAD_NATIVE_WINDOW,
            egl_consts::EGL_CONTEXT_LOST => EglErrors::EGL_CONTEXT_LOST,
            _ => return None,
        })
    }

    /// The name of the error as the EGL headers spell it.
    pub fn name(self) -> &'static str {
        match self {
            EglErrors::EGL_SUCCESS => "EGL_SUCCESS",
            EglErrors::EGL_NOT_INITIALIZED => "EGL_NOT_INITIALIZED",
            EglErrors::EGL_BAD_ACCESS => "EGL_BAD_ACCESS",
            EglErrors::EGL_BAD_ALLOC => "EGL_BAD_ALLOC",
            EglErrors::EGL_BAD_ATTRIBUTE => "EGL_BAD_ATTRIBUTE",
            EglErrors::EGL_BAD_CONTEXT => "EGL_BAD_CONTEXT",
            EglErrors::EGL_BAD_CONFIG => "EGL_BAD_CONFIG",
            EglErrors::EGL_BAD_CURRENT_SURFACE => "EGL_BAD_CURRENT_SURFACE",
            EglErrors::EGL_BAD_DISPLAY => "EGL_BAD_DISPLAY",
            EglErrors::EGL_BAD_SURFACE => "EGL_BAD_SURFACE",
            EglErrors::EGL_BAD_MATCH => "EGL_BAD_MATCH",
            EglErrors::EGL_BAD_PARAMETER => "EGL_BAD_PARAMETER",
            EglErrors::EGL_BAD_NATIVE_PIXMAP => "EGL_BAD_NATIVE_PIXMAP",
            EglErrors::EGL_BAD_NATIVE_WINDOW => "EGL_BAD_NATIVE_WINDOW",
            EglErrors::EGL_CONTEXT_LOST => "EGL_CONTEXT_LOST",
        }
    }
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream project has no tests of the enumeration.
    use super::*;

    #[test]
    fn a_code_maps_to_its_error_and_name() {
        assert_eq!(Some(EglErrors::EGL_BAD_ALLOC), EglErrors::from_code(egl_consts::EGL_BAD_ALLOC));
        assert_eq!("EGL_CONTEXT_LOST", EglErrors::EGL_CONTEXT_LOST.name());
        assert_eq!(egl_consts::EGL_SUCCESS, EglErrors::EGL_SUCCESS as i32);
        assert_eq!(None, EglErrors::from_code(1));
    }
}
