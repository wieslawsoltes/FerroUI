//! Port of `TypeSystem/TypeSystemHelpers.cs`.

use std::rc::Rc;

use crate::ast::{IXamlLineInfo, XamlConstantNode};
use crate::exceptions::{XamlError, XamlResult};

use super::{IXamlField, IXamlType, XamlValue};

pub struct TypeSystemHelpers;

fn invalid_literal_cast(literal: &XamlValue, target: &str) -> XamlError {
    XamlError::invalid_cast(format!(
        "Invalid cast from '{}' to '{target}'.",
        literal.type_name()
    ))
}

fn overflow(target: &str) -> XamlError {
    XamlError::internal(
        "OverflowException",
        format!("Value was either too large or too small for an {target}."),
    )
}

fn format_error(input: &str) -> XamlError {
    XamlError::internal(
        "FormatException",
        format!("The input string '{input}' was not in a correct format."),
    )
}

/// `Math.Round`-style conversion used by `Convert.ChangeType` for floating point sources.
fn round_half_even(v: f64) -> f64 {
    let r = v.round();
    if (v - v.trunc()).abs() == 0.5 && r % 2.0 != 0.0 {
        r - v.signum()
    } else {
        r
    }
}

fn literal_to_i128(literal: &XamlValue, target: &str) -> XamlResult<i128> {
    Ok(match literal {
        XamlValue::Boolean(v) => *v as i128,
        XamlValue::Char(v) => *v as u32 as i128,
        XamlValue::SByte(v) => *v as i128,
        XamlValue::Byte(v) => *v as i128,
        XamlValue::Int16(v) => *v as i128,
        XamlValue::UInt16(v) => *v as i128,
        XamlValue::Int32(v) => *v as i128,
        XamlValue::UInt32(v) => *v as i128,
        XamlValue::Int64(v) => *v as i128,
        XamlValue::UInt64(v) => *v as i128,
        XamlValue::Single(v) => round_half_even(*v as f64) as i128,
        XamlValue::Double(v) => round_half_even(*v) as i128,
        XamlValue::String(s) => {
            TypeSystemHelpers::parse_integer(s)?.ok_or_else(|| overflow(target))?
        }
        _ => return Err(invalid_literal_cast(literal, target)),
    })
}

impl TypeSystemHelpers {
    pub fn convert_literal_to_int(literal: &XamlValue) -> XamlResult<i32> {
        if let XamlValue::UInt32(ui) = literal {
            return Ok(*ui as i32);
        }
        i32::try_from(literal_to_i128(literal, "Int32")?).map_err(|_| overflow("Int32"))
    }

    pub fn convert_literal_to_long(literal: &XamlValue) -> XamlResult<i64> {
        if let XamlValue::UInt64(ui) = literal {
            return Ok(*ui as i64);
        }
        i64::try_from(literal_to_i128(literal, "Int64")?).map_err(|_| overflow("Int64"))
    }

    pub fn get_literal_field_constant_node(
        field: &dyn IXamlField,
        info: &dyn IXamlLineInfo,
    ) -> XamlResult<Rc<XamlConstantNode>> {
        XamlConstantNode::new(
            info,
            field.field_type(),
            Self::get_literal_field_constant_value(field)?,
        )
    }

    /// Upstream additionally unwraps boxed .NET enums returned by the reflection based type
    /// system; [`XamlValue`] cannot hold one, so the value is returned as is.
    pub fn get_literal_field_constant_value(field: &dyn IXamlField) -> XamlResult<XamlValue> {
        field.get_literal_value()
    }

    pub fn try_get_enum_value_node(
        enum_type: &Rc<dyn IXamlType>,
        value: &str,
        line_info: &dyn IXamlLineInfo,
        ignore_case: bool,
    ) -> XamlResult<Option<Rc<XamlConstantNode>>> {
        match Self::try_get_enum_value(enum_type, value, ignore_case)? {
            Some(constant) => Ok(Some(XamlConstantNode::new(
                line_info,
                enum_type.clone(),
                constant,
            )?)),
            None => Ok(None),
        }
    }

    pub fn try_get_enum_value(
        enum_type: &Rc<dyn IXamlType>,
        value: &str,
        ignore_case: bool,
    ) -> XamlResult<Option<XamlValue>> {
        if let Ok(Some(parsed)) = Self::parse_integer(value) {
            if let Ok(parsed_long) = i64::try_from(parsed) {
                let enum_type_name = enum_type.get_enum_underlying_type()?.name();
                return Ok(Some(
                    if enum_type_name == "Int32" || enum_type_name == "UInt32" {
                        XamlValue::Int32(parsed_long as i32)
                    } else {
                        XamlValue::Int64(parsed_long)
                    },
                ));
            }
        }

        let is_flags = enum_type
            .custom_attributes()
            .iter()
            .any(|a| a.type_().name() == "FlagsAttribute");
        let values: Vec<String> = if is_flags {
            value
                .split(',')
                .map(|x| dotnet_trim(x).to_string())
                .collect()
        } else {
            vec![value.to_string()]
        };

        let mut cv: Option<XamlValue> = None;
        let fields = enum_type.fields();
        for (c, name) in values.iter().enumerate() {
            let enum_value_field = fields.iter().find(|f| {
                if ignore_case {
                    ordinal_ignore_case_equals(&f.name(), name)
                } else {
                    f.name() == *name
                }
            });
            let Some(enum_value_field) = enum_value_field else {
                return Ok(None);
            };
            let enum_value = Self::get_literal_field_constant_value(&**enum_value_field)?;
            cv = Some(if c == 0 {
                enum_value
            } else {
                // `cv` is always set after the first iteration.
                or(cv.as_ref().unwrap_or(&XamlValue::Null), &enum_value)?
            });
        }

        Ok(cv)
    }

    pub fn parse_constant_if_type_allows(
        s: &str,
        type_: &Rc<dyn IXamlType>,
        info: &dyn IXamlLineInfo,
    ) -> XamlResult<Option<Rc<XamlConstantNode>>> {
        if type_.namespace().as_deref() != Some("System") {
            return Ok(None);
        }

        let parsed = (|| -> XamlResult<Option<XamlValue>> {
            fn int<T: TryFrom<i128>>(s: &str, target: &str) -> XamlResult<T> {
                let v = TypeSystemHelpers::parse_integer(s)?.ok_or_else(|| overflow(target))?;
                T::try_from(v).map_err(|_| overflow(target))
            }
            Ok(Some(match type_.name().as_str() {
                "Byte" => XamlValue::Byte(int(s, "unsigned byte")?),
                "SByte" => XamlValue::SByte(int(s, "signed byte")?),
                "Int16" => XamlValue::Int16(int(s, "Int16")?),
                "UInt16" => XamlValue::UInt16(int(s, "UInt16")?),
                "Int32" => XamlValue::Int32(int(s, "Int32")?),
                "UInt32" => XamlValue::UInt32(int(s, "UInt32")?),
                "Int64" => XamlValue::Int64(int(s, "Int64")?),
                "UInt64" => XamlValue::UInt64(int(s, "UInt64")?),
                "Single" => XamlValue::Single(Self::parse_float(s)? as f32),
                "Double" => XamlValue::Double(Self::parse_float(s)?),
                "Boolean" => {
                    let trimmed = dotnet_trim(s).trim_matches('\0');
                    let trimmed = dotnet_trim(trimmed);
                    if trimmed.eq_ignore_ascii_case("true") {
                        XamlValue::Boolean(true)
                    } else if trimmed.eq_ignore_ascii_case("false") {
                        XamlValue::Boolean(false)
                    } else {
                        return Err(XamlError::internal(
                            "FormatException",
                            format!("String '{s}' was not recognized as a valid Boolean."),
                        ));
                    }
                }
                "Char" => {
                    let mut chars = s.chars();
                    match (chars.next(), chars.next()) {
                        (Some(c), None) if c.len_utf16() == 1 => XamlValue::Char(c),
                        _ => {
                            return Err(XamlError::internal(
                                "FormatException",
                                "String must be exactly one character long.",
                            ))
                        }
                    }
                }
                _ => return Ok(None),
            }))
        })();

        match parsed {
            Ok(Some(r)) => Ok(Some(XamlConstantNode::new(info, type_.clone(), r)?)),
            Ok(None) => Ok(None),
            Err(XamlError::Internal(e)) if e.type_name == "FormatException" => {
                Err(XamlError::parse_exception_with_inner(
                    e.message.clone(),
                    Some(info),
                    XamlError::Internal(e),
                ))
            }
            Err(e) => Err(e),
        }
    }

    /// Integer parsing with the `NumberStyles.Integer` rules of the invariant culture:
    /// optional surrounding whitespace and an optional leading sign.
    /// `Ok(None)` means the value does not fit any 64-bit integer (`OverflowException`).
    pub(crate) fn parse_integer(s: &str) -> XamlResult<Option<i128>> {
        let t = dotnet_trim(s);
        let (negative, digits) = match t.as_bytes().first() {
            Some(b'-') => (true, &t[1..]),
            Some(b'+') => (false, &t[1..]),
            _ => (false, t),
        };
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return Err(format_error(s));
        }
        let digits = digits.trim_start_matches('0');
        if digits.len() > 20 {
            return Ok(None);
        }
        let magnitude: i128 = if digits.is_empty() {
            0
        } else {
            digits.parse().map_err(|_| format_error(s))?
        };
        Ok(Some(if negative { -magnitude } else { magnitude }))
    }

    /// Floating point parsing with the `NumberStyles.Float | NumberStyles.AllowThousands` rules
    /// of the invariant culture.
    pub(crate) fn parse_float(s: &str) -> XamlResult<f64> {
        let t = dotnet_trim(s);
        if t.eq_ignore_ascii_case("NaN") {
            return Ok(f64::NAN);
        }
        let (sign, body) = match t.as_bytes().first() {
            Some(b'-') => (-1.0, &t[1..]),
            Some(b'+') => (1.0, &t[1..]),
            _ => (1.0, t),
        };
        if body.eq_ignore_ascii_case("Infinity") || body == "\u{221E}" {
            return Ok(sign * f64::INFINITY);
        }
        if body.is_empty()
            || !body
                .bytes()
                .all(|b| b.is_ascii_digit() || matches!(b, b'.' | b',' | b'e' | b'E' | b'+' | b'-'))
        {
            return Err(format_error(s));
        }
        // Thousands separators are only allowed in the integral part.
        let (mantissa, exponent) = match body.find(['e', 'E']) {
            Some(i) => (&body[..i], &body[i..]),
            None => (body, ""),
        };
        let (integral, fraction) = match mantissa.find('.') {
            Some(i) => (&mantissa[..i], &mantissa[i..]),
            None => (mantissa, ""),
        };
        if fraction.contains(',') || exponent.contains(',') || mantissa.contains(['+', '-']) {
            return Err(format_error(s));
        }
        let integral: String = integral.chars().filter(|c| *c != ',').collect();
        if integral.is_empty() && fraction.len() <= 1 {
            return Err(format_error(s));
        }
        let normalized = format!("{integral}{fraction}{exponent}");
        normalized
            .parse::<f64>()
            .map(|v| sign * v)
            .map_err(|_| format_error(s))
    }
}

/// `string.Trim()`: trims Unicode white space.
fn dotnet_trim(s: &str) -> &str {
    s.trim_matches(char::is_whitespace)
}

/// `StringComparer.OrdinalIgnoreCase.Equals`.
fn ordinal_ignore_case_equals(a: &str, b: &str) -> bool {
    let mut x = a.chars().flat_map(char::to_uppercase);
    let mut y = b.chars().flat_map(char::to_uppercase);
    loop {
        match (x.next(), y.next()) {
            (None, None) => return true,
            (Some(p), Some(q)) if p == q => {}
            _ => return false,
        }
    }
}

fn or(l: &XamlValue, r: &XamlValue) -> XamlResult<XamlValue> {
    use XamlValue::*;
    // C# promotes the small integer types to Int32 when applying `|`.
    Ok(match (l, r) {
        (Byte(a), Byte(b)) => Int32((*a | *b) as i32),
        (SByte(a), SByte(b)) => Int32((*a | *b) as i32),
        (UInt16(a), UInt16(b)) => Int32((*a | *b) as i32),
        (Int16(a), Int16(b)) => Int32((*a | *b) as i32),
        (UInt32(a), UInt32(b)) => UInt32(*a | *b),
        (Int32(a), Int32(b)) => Int32(*a | *b),
        (UInt64(a), UInt64(b)) => UInt64(*a | *b),
        (Int64(a), Int64(b)) => Int64(*a | *b),
        (
            Byte(_) | SByte(_) | UInt16(_) | Int16(_) | UInt32(_) | Int32(_) | UInt64(_) | Int64(_),
            _,
        ) => {
            return Err(XamlError::invalid_cast(format!(
                "Unable to cast object of type '{}' to type '{}'.",
                r.type_name(),
                l.type_name()
            )))
        }
        _ => {
            return Err(XamlError::argument(format!(
                "Unsupported type {}",
                l.type_name()
            )))
        }
    })
}
