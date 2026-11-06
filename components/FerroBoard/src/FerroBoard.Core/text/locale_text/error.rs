use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CollationError {
    UnsupportedPlatform,
    InvalidLocale,
    InputTooLong,
    SystemLocaleUnavailable,
    Native { operation: &'static str, code: i32 },
}
impl fmt::Display for CollationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedPlatform => {
                write!(f, "native collation is not implemented on this platform")
            }
            Self::InvalidLocale => write!(f, "native locale contains a NUL byte"),
            Self::InputTooLong => write!(f, "UTF-16 input exceeds the native signed length limit"),
            Self::SystemLocaleUnavailable => {
                write!(f, "the system did not supply a preferred locale")
            }
            Self::Native { operation, code } => {
                write!(f, "native {operation} failed with code {code}")
            }
        }
    }
}
impl std::error::Error for CollationError {}
