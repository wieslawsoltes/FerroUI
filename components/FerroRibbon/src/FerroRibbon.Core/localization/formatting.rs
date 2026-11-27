//! UTF-16 composite parser adapted from the selected FerroUI formatter. Numeric
//! type/culture dispatch stays a service so Core never imports the UI Base crate.
use super::{
    RibbonCultureInfo, RibbonLocalizationError as Error, RibbonLocalizationErrorKind as Kind,
};
use crate::{RibbonString, collections::RibbonArray, model::RibbonObject, text::empty};
use std::{any::Any, rc::Rc, sync::Arc};
pub type RibbonCustomFormatter = dyn Fn(
    Option<&RibbonString>,
    &RibbonFormatArgument,
    &Arc<dyn RibbonCultureInfo>,
) -> Result<Option<RibbonString>, Error>;
pub trait RibbonFormatObject: Any {
    /// Explicit IFormattable/ToString dispatch on the original object. A plain
    /// object implementation ignores `format`; null ToString remains empty text.
    fn format(
        &self,
        format: Option<&RibbonString>,
        culture: &Arc<dyn RibbonCultureInfo>,
    ) -> Result<Option<RibbonString>, Error>;
}
#[derive(Clone, Default)]
pub enum RibbonFormatArgument {
    #[default]
    Null,
    String(RibbonString),
    Bool(bool),
    Char(u16),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U64(u64),
    F32(f32),
    F64(f64),
    Object(Rc<dyn RibbonFormatObject>),
    Native(RibbonObject),
}
impl From<&str> for RibbonFormatArgument {
    fn from(s: &str) -> Self {
        Self::String(s.into())
    }
}
impl From<RibbonString> for RibbonFormatArgument {
    fn from(s: RibbonString) -> Self {
        Self::String(s)
    }
}
impl From<i32> for RibbonFormatArgument {
    fn from(v: i32) -> Self {
        Self::I32(v)
    }
}
fn malformed() -> Error {
    Error::new(
        Kind::Format,
        None,
        "Input string was not in a correct format.",
    )
}
fn next(units: &[u16], position: &mut usize) -> Result<u16, Error> {
    *position += 1;
    units.get(*position).copied().ok_or_else(malformed)
}
fn digit(c: u16) -> bool {
    (48..=57).contains(&c)
}
/// Shared port implementation of source String.Format, preserving UTF-16 and culture dispatch.
#[doc(hidden)]
pub fn format_composite(
    culture: Arc<dyn RibbonCultureInfo>,
    pattern: &RibbonString,
    args: &RibbonArray<RibbonFormatArgument>,
) -> Result<RibbonString, Error> {
    let custom = culture.custom_formatter();
    let units = pattern.as_utf16();
    let mut out = vec![];
    let mut position = 0;
    loop {
        let start = position;
        while position < units.len() && !matches!(units[position], 123 | 125) {
            position += 1;
        }
        out.extend_from_slice(&units[start..position]);
        if position >= units.len() {
            return Ok(if out.is_empty() {
                empty()
            } else {
                RibbonString::from_utf16(out)
            });
        }
        let brace = units[position];
        let mut ch = next(units, &mut position)?;
        if ch == brace {
            out.push(brace);
            position += 1;
            continue;
        }
        if brace != 123 || !digit(ch) {
            return Err(malformed());
        }
        let mut index = (ch - 48) as usize;
        ch = next(units, &mut position)?;
        let mut width = 0;
        let mut left = false;
        let mut format = None;
        if ch != 125 {
            while digit(ch) && index < 1_000_000 {
                index = index * 10 + (ch - 48) as usize;
                ch = next(units, &mut position)?;
            }
            while ch == 32 {
                ch = next(units, &mut position)?;
            }
            if ch == 44 {
                loop {
                    ch = next(units, &mut position)?;
                    if ch != 32 {
                        break;
                    }
                }
                if ch == 45 {
                    left = true;
                    ch = next(units, &mut position)?;
                }
                if !digit(ch) {
                    return Err(malformed());
                }
                width = (ch - 48) as usize;
                ch = next(units, &mut position)?;
                while digit(ch) && width < 1_000_000 {
                    width = width * 10 + (ch - 48) as usize;
                    ch = next(units, &mut position)?;
                }
                while ch == 32 {
                    ch = next(units, &mut position)?;
                }
            }
            if ch != 125 {
                if ch != 58 {
                    return Err(malformed());
                }
                let start = position + 1;
                loop {
                    ch = next(units, &mut position)?;
                    if ch == 125 {
                        break;
                    }
                    if ch == 123 {
                        return Err(malformed());
                    }
                }
                if start < position {
                    format = Some(RibbonString::from(&units[start..position]));
                }
            }
        }
        position += 1;
        if index >= args.len() {
            return Err(Error::new(
                Kind::Format,
                None,
                "Index (zero based) must be greater than or equal to zero and less than the size of the argument list.",
            ));
        }
        // Read at each use: a formatting callback can mutate later array slots.
        let arg = args.get(index);
        let mut text = custom
            .as_ref()
            .map(|custom| custom(format.as_ref(), &arg, &culture))
            .transpose()?
            .flatten();
        if text.is_none() {
            text = match &arg {
                RibbonFormatArgument::Null => None,
                RibbonFormatArgument::String(v) => Some(v.clone()),
                RibbonFormatArgument::Bool(v) => Some(if *v { "True" } else { "False" }.into()),
                RibbonFormatArgument::Char(c) => Some(RibbonString::from_utf16(vec![*c])),
                RibbonFormatArgument::Object(v) => v.format(format.as_ref(), &culture)?,
                _ => culture.format_numeric(&arg, format.as_ref())?,
            };
        }
        let text = text.unwrap_or_else(empty);
        let padding = width.saturating_sub(text.len());
        if !left {
            out.extend(std::iter::repeat_n(32, padding));
        }
        out.extend_from_slice(text.as_utf16());
        if left {
            out.extend(std::iter::repeat_n(32, padding));
        }
    }
}
pub(super) fn basic_numeric(
    arg: &RibbonFormatArgument,
    format: Option<&RibbonString>,
    invariant: bool,
) -> Result<Option<RibbonString>, Error> {
    if format.is_some_and(|v| !v.is_empty()) {
        return Err(Error::new(
            Kind::Unavailable,
            None,
            "Formatted numbers require a faithful culture formatter",
        ));
    }
    macro_rules! signed {
        ($v:expr) => {
            if invariant || $v >= 0 {
                Ok(Some($v.to_string().into()))
            } else {
                Err(Error::new(
                    Kind::Unavailable,
                    None,
                    "Negative signs require a faithful culture formatter",
                ))
            }
        };
    }
    match arg {
        RibbonFormatArgument::I8(v) => signed!(*v),
        RibbonFormatArgument::I16(v) => signed!(*v),
        RibbonFormatArgument::I32(v) => signed!(*v),
        RibbonFormatArgument::I64(v) => signed!(*v),
        RibbonFormatArgument::U64(v) => Ok(Some(v.to_string().into())),
        RibbonFormatArgument::F64(v) if invariant => Ok(Some(
            ferroui_system::invariant_numbers::InvariantF64(*v)
                .to_string()
                .into(),
        )),
        _ => Err(Error::new(
            Kind::Unavailable,
            None,
            "This argument requires a faithful culture formatter",
        )),
    }
}
