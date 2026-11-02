use crate::egl::{EglErrors, EglInterface};
use crate::{GlErrors, GlInterface};
use std::fmt;

/// A failure reported by OpenGL or by the code driving it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenGlError {
    message: String,
    error_code: Option<i32>,
}

/// The name of [`OpenGlError`] before 0.5.
#[deprecated(since = "0.5.0", note = "use `OpenGlError`")]
pub type OpenGlException = OpenGlError;

impl OpenGlError {
    /// A failure without an OpenGL error code.
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into(), error_code: None }
    }

    fn with_error_code(message: String, error_code: i32) -> Self {
        Self { message, error_code: Some(error_code) }
    }

    /// The message of the failure.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// The OpenGL error code, when the failure was reported by OpenGL.
    pub fn error_code(&self) -> Option<i32> {
        self.error_code
    }

    /// The failure of the function `func_name`, with the pending error of
    /// the context behind `gl`.
    pub fn get_formatted_exception(func_name: &str, gl: &GlInterface) -> OpenGlError {
        Self::get_formatted_exception_for_code(func_name, gl.get_error())
    }

    /// The failure of the function `func_name`, with the pending error of
    /// `egl`.
    pub fn get_formatted_exception_for_egl(func_name: &str, egl: &EglInterface) -> OpenGlError {
        Self::get_formatted_egl_exception(func_name, egl.get_error())
    }

    /// The failure of the function `func_name` with the given EGL error code.
    pub fn get_formatted_egl_exception(func_name: &str, error_code: i32) -> OpenGlError {
        // An error code without a name leaves the place of the name empty.
        let error_name = EglErrors::from_code(error_code).map_or("", EglErrors::name);
        OpenGlError::with_error_code(
            format!("{func_name} failed with error {error_name} (0x{error_code:08X})"),
            error_code,
        )
    }

    /// The failure of the function `func_name` with the given OpenGL error
    /// code.
    pub fn get_formatted_exception_for_code(func_name: &str, error_code: i32) -> OpenGlError {
        // An error code without a name leaves the place of the name empty.
        let error_name = GlErrors::from_code(error_code).map_or("", GlErrors::name);
        OpenGlError::with_error_code(
            format!("{func_name} failed with error {error_name} (0x{error_code:08X})"),
            error_code,
        )
    }
}

impl fmt::Display for OpenGlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for OpenGlError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::FakeGl;
    use crate::{gl_consts, GlProfileType, GlVersion};

    #[test]
    fn a_plain_failure_has_no_error_code() {
        let exception = OpenGlError::new("Unable to make the context current");

        assert_eq!("Unable to make the context current", exception.message());
        assert_eq!("Unable to make the context current", exception.to_string());
        assert_eq!(None, exception.error_code());
    }

    #[test]
    fn a_known_code_is_named() {
        let exception = OpenGlError::get_formatted_exception_for_code("glBindTexture", gl_consts::GL_INVALID_ENUM);

        assert_eq!("glBindTexture failed with error GL_INVALID_ENUM (0x00000500)", exception.message());
        assert_eq!(Some(0x500), exception.error_code());
    }

    #[test]
    fn a_known_egl_code_is_named() {
        let exception =
            OpenGlError::get_formatted_egl_exception("eglCreateContext", crate::egl::egl_consts::EGL_BAD_MATCH);

        assert_eq!("eglCreateContext failed with error EGL_BAD_MATCH (0x00003009)", exception.message());
        assert_eq!(Some(0x3009), exception.error_code());
    }

    #[test]
    fn an_unknown_code_has_no_name() {
        let exception = OpenGlError::get_formatted_exception_for_code("glFoo", 0x1ABC);

        assert_eq!("glFoo failed with error  (0x00001ABC)", exception.message());
        assert_eq!(Some(0x1ABC), exception.error_code());
    }

    #[test]
    fn the_pending_error_of_the_context_is_consumed() {
        let fake = FakeGl::install(GlVersion::new(GlProfileType::OpenGLES, 3, 0));
        let gl = fake.interface();
        fake.set_error(gl_consts::GL_OUT_OF_MEMORY);

        let exception = OpenGlError::get_formatted_exception("glTexImage2D", &gl);

        assert_eq!("glTexImage2D failed with error GL_OUT_OF_MEMORY (0x00000505)", exception.message());
        assert_eq!(gl_consts::GL_NO_ERROR, gl.get_error());
    }
}
