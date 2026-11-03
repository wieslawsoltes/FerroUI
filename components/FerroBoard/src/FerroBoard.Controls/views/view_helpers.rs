use crate::{icons::IconError, themes::ThemeError};
use ferroboard_core::{engine::EngineError, text::Utf16Text};
#[derive(Debug)]
pub(crate) enum ViewError {
    Engine(EngineError),
    Theme(ThemeError),
    Icon(IconError),
    UnpairedSurrogate,
    NullReference,
    ArgumentNull,
    Format,
    TransformUnavailable,
    Owner(Box<dyn std::error::Error>),
    OwnerReleased,
}
impl From<EngineError> for ViewError {
    fn from(e: EngineError) -> Self {
        Self::Engine(e)
    }
}
impl From<ThemeError> for ViewError {
    fn from(e: ThemeError) -> Self {
        Self::Theme(e)
    }
}
impl From<IconError> for ViewError {
    fn from(e: IconError) -> Self {
        Self::Icon(e)
    }
}
impl std::fmt::Display for ViewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ViewError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Owner(error) => Some(error.as_ref()),
            _ => None,
        }
    }
}
pub(crate) fn native_text(value: &Utf16Text) -> Result<String, ViewError> {
    value.to_utf8().map_err(|_| ViewError::UnpairedSurrogate)
}
pub(crate) fn nullable_text(value: Option<Utf16Text>) -> Result<Option<String>, ViewError> {
    value.as_ref().map(native_text).transpose()
}
/// The single string argument String.Format path used for localized paste labels.
/// String arguments ignore format specifications. Alignment counts source UTF-16 units.
pub(crate) fn format_one(
    format: Option<&Utf16Text>,
    text: &Utf16Text,
) -> Result<Utf16Text, ViewError> {
    let f = format.ok_or(ViewError::ArgumentNull)?;
    let f = f.units();
    let mut out = Vec::new();
    let mut i = 0;
    while i < f.len() {
        let c = f[i];
        i += 1;
        if c == b'}' as u16 {
            if f.get(i) == Some(&c) {
                out.push(c);
                i += 1;
                continue;
            }
            return Err(ViewError::Format);
        }
        if c != b'{' as u16 {
            out.push(c);
            continue;
        }
        if f.get(i) == Some(&c) {
            out.push(c);
            i += 1;
            continue;
        }
        let index = number(f, &mut i)?;
        if index != 0 {
            return Err(ViewError::Format);
        }
        spaces(f, &mut i);
        let mut width = 0;
        let mut left = false;
        if f.get(i) == Some(&(b',' as u16)) {
            i += 1;
            spaces(f, &mut i);
            if f.get(i) == Some(&(b'-' as u16)) {
                left = true;
                i += 1;
            }
            width = number(f, &mut i)?;
            spaces(f, &mut i);
        }
        if f.get(i) == Some(&(b':' as u16)) {
            i += 1;
            while let Some(&v) = f.get(i) {
                if v == b'}' as u16 {
                    break;
                }
                if v == b'{' as u16 {
                    return Err(ViewError::Format);
                }
                i += 1;
            }
        }
        if f.get(i) != Some(&(b'}' as u16)) {
            return Err(ViewError::Format);
        }
        i += 1;
        let padding = (width as usize).saturating_sub(text.len());
        if !left {
            out.extend(std::iter::repeat_n(32, padding));
        }
        out.extend_from_slice(text.units());
        if left {
            out.extend(std::iter::repeat_n(32, padding));
        }
    }
    Ok(Utf16Text::from_units(out))
}
fn spaces(f: &[u16], i: &mut usize) {
    while f.get(*i) == Some(&32) {
        *i += 1;
    }
}
fn number(f: &[u16], i: &mut usize) -> Result<u32, ViewError> {
    let mut n = 0;
    let start = *i;
    while let Some(&v) = f.get(*i) {
        if !(48..=57).contains(&v) {
            break;
        }
        if n >= 1_000_000 {
            break;
        }
        n = n * 10 + u32::from(v - 48);
        *i += 1;
    }
    if *i == start {
        Err(ViewError::Format)
    } else {
        Ok(n)
    }
}

/// String.Format accepts a null string argument as empty text.
pub(crate) fn format_one_nullable(
    format: Option<&Utf16Text>,
    text: Option<&Utf16Text>,
) -> Result<Utf16Text, ViewError> {
    let empty = Utf16Text::default();
    format_one(format, text.unwrap_or(&empty))
}
