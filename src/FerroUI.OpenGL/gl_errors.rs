use crate::gl_consts;

/// The error codes `glGetError` reports.
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum GlErrors {
    GL_NO_ERROR = gl_consts::GL_NO_ERROR,
    GL_INVALID_ENUM = gl_consts::GL_INVALID_ENUM,
    GL_INVALID_VALUE = gl_consts::GL_INVALID_VALUE,
    GL_INVALID_OPERATION = gl_consts::GL_INVALID_OPERATION,
    GL_INVALID_FRAMEBUFFER_OPERATION = gl_consts::GL_INVALID_FRAMEBUFFER_OPERATION,
    GL_STACK_OVERFLOW = gl_consts::GL_STACK_OVERFLOW,
    GL_STACK_UNDERFLOW = gl_consts::GL_STACK_UNDERFLOW,
    GL_OUT_OF_MEMORY = gl_consts::GL_OUT_OF_MEMORY,
    GL_CONTEXT_LOST = gl_consts::GL_CONTEXT_LOST,
}

impl GlErrors {
    /// The error with the given code, when the code is a known one.
    pub fn from_code(code: i32) -> Option<GlErrors> {
        Some(match code {
            gl_consts::GL_NO_ERROR => GlErrors::GL_NO_ERROR,
            gl_consts::GL_INVALID_ENUM => GlErrors::GL_INVALID_ENUM,
            gl_consts::GL_INVALID_VALUE => GlErrors::GL_INVALID_VALUE,
            gl_consts::GL_INVALID_OPERATION => GlErrors::GL_INVALID_OPERATION,
            gl_consts::GL_INVALID_FRAMEBUFFER_OPERATION => GlErrors::GL_INVALID_FRAMEBUFFER_OPERATION,
            gl_consts::GL_STACK_OVERFLOW => GlErrors::GL_STACK_OVERFLOW,
            gl_consts::GL_STACK_UNDERFLOW => GlErrors::GL_STACK_UNDERFLOW,
            gl_consts::GL_OUT_OF_MEMORY => GlErrors::GL_OUT_OF_MEMORY,
            gl_consts::GL_CONTEXT_LOST => GlErrors::GL_CONTEXT_LOST,
            _ => return None,
        })
    }

    /// The name of the error as the OpenGL headers spell it.
    pub fn name(self) -> &'static str {
        match self {
            GlErrors::GL_NO_ERROR => "GL_NO_ERROR",
            GlErrors::GL_INVALID_ENUM => "GL_INVALID_ENUM",
            GlErrors::GL_INVALID_VALUE => "GL_INVALID_VALUE",
            GlErrors::GL_INVALID_OPERATION => "GL_INVALID_OPERATION",
            GlErrors::GL_INVALID_FRAMEBUFFER_OPERATION => "GL_INVALID_FRAMEBUFFER_OPERATION",
            GlErrors::GL_STACK_OVERFLOW => "GL_STACK_OVERFLOW",
            GlErrors::GL_STACK_UNDERFLOW => "GL_STACK_UNDERFLOW",
            GlErrors::GL_OUT_OF_MEMORY => "GL_OUT_OF_MEMORY",
            GlErrors::GL_CONTEXT_LOST => "GL_CONTEXT_LOST",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_map_to_the_header_values() {
        assert_eq!(0, GlErrors::GL_NO_ERROR as i32);
        assert_eq!(0x0500, GlErrors::GL_INVALID_ENUM as i32);
        assert_eq!(0x0506, GlErrors::GL_INVALID_FRAMEBUFFER_OPERATION as i32);
        assert_eq!(0x0507, GlErrors::GL_CONTEXT_LOST as i32);
    }

    #[test]
    fn every_error_round_trips_through_its_code() {
        for error in [
            GlErrors::GL_NO_ERROR,
            GlErrors::GL_INVALID_ENUM,
            GlErrors::GL_INVALID_VALUE,
            GlErrors::GL_INVALID_OPERATION,
            GlErrors::GL_INVALID_FRAMEBUFFER_OPERATION,
            GlErrors::GL_STACK_OVERFLOW,
            GlErrors::GL_STACK_UNDERFLOW,
            GlErrors::GL_OUT_OF_MEMORY,
            GlErrors::GL_CONTEXT_LOST,
        ] {
            assert_eq!(Some(error), GlErrors::from_code(error as i32));
        }
        assert_eq!(None, GlErrors::from_code(0x1234));
    }
}
