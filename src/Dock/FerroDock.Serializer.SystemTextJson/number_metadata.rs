// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::{error, JsonErrorKind, JsonNumber, JsonTypeInfo, JsonValue};
use ferrodock_model::core::SerializationResult;
use std::{fmt::Display, rc::Rc, str::FromStr};

pub(crate) fn numbers() -> Vec<Rc<JsonTypeInfo>> {
    vec![
        integer::<i8>(),
        integer::<u8>(),
        integer::<i16>(),
        integer::<u16>(),
        integer::<i32>(),
        integer::<u32>(),
        integer::<i64>(),
        integer::<u64>(),
        floating(
            |v: f64| write_float(v, named(v), 17),
            f64::from_bits(0xfff8_0000_0000_0000),
            f64::INFINITY,
            f64::NEG_INFINITY,
        ),
        floating(
            |v: f32| write_float(v, named(f64::from(v)), 9),
            f32::from_bits(0xffc0_0000),
            f32::INFINITY,
            f32::NEG_INFINITY,
        ),
    ]
}

fn integer<T: Copy + Display + FromStr + 'static>() -> Rc<JsonTypeInfo> {
    JsonTypeInfo::converter(
        |value: &T, _| Ok(JsonValue::Number(JsonNumber::parse(&value.to_string())?)),
        |value, _| {
            if let JsonValue::Number(value) = value {
                if let Ok(value) = value.as_str().parse::<T>() {
                    return Ok(Some(value));
                }
            }
            Err(error(
                JsonErrorKind::InvalidJson,
                "Expected an integer within the declared type's range.",
            ))
        },
    )
}

fn floating<T: Copy + FromStr + 'static>(
    encode: impl Fn(T) -> SerializationResult<JsonValue> + 'static,
    nan: T,
    infinity: T,
    negative_infinity: T,
) -> Rc<JsonTypeInfo> {
    JsonTypeInfo::converter(
        move |value: &T, _| encode(*value),
        move |value, _| {
            let result = match value {
                JsonValue::Number(value) => value.as_str().parse::<T>().ok(),
                JsonValue::String(value) => match value.as_str() {
                    "NaN" => Some(nan),
                    "Infinity" => Some(infinity),
                    "-Infinity" => Some(negative_infinity),
                    _ => None,
                },
                _ => None,
            };
            result.map(Some).ok_or_else(|| {
                error(
                    JsonErrorKind::InvalidJson,
                    "Expected a floating-point number or a named floating-point literal.",
                )
            })
        },
    )
}

fn named(value: f64) -> Option<&'static str> {
    if value.is_nan() {
        Some("NaN")
    } else if value == f64::INFINITY {
        Some("Infinity")
    } else if value == f64::NEG_INFINITY {
        Some("-Infinity")
    } else {
        None
    }
}

fn write_float<T: ryu::Float>(
    value: T,
    named: Option<&str>,
    precision: i32,
) -> SerializationResult<JsonValue> {
    if let Some(name) = named {
        return Ok(JsonValue::String(name.into()));
    }
    let mut buffer = ryu::Buffer::new();
    let shortest = buffer.format_finite(value);
    let text = general_format(shortest, precision);
    Ok(JsonValue::Number(JsonNumber::parse(&text)?))
}

// The source general format uses shortest roundtrip digits, fixed notation from
// exponent -4 through precision-1, and signed, at least two-digit exponents.
fn general_format(shortest: &str, precision: i32) -> String {
    let (sign, unsigned) = shortest
        .strip_prefix('-')
        .map_or(("", shortest), |v| ("-", v));
    let (mantissa, power) = unsigned.split_once('e').map_or((unsigned, 0), |(v, e)| {
        (v, e.parse::<i32>().expect("finite formatter exponent"))
    });
    let point = mantissa.find('.').unwrap_or(mantissa.len()) as i32;
    let mut digits: Vec<u8> = mantissa.bytes().filter(|v| *v != b'.').collect();
    let leading = digits.iter().position(|v| *v != b'0');
    let Some(leading) = leading else {
        return format!("{sign}0");
    };
    let exponent = power + point - leading as i32 - 1;
    digits.drain(..leading);
    while digits.last() == Some(&b'0') {
        digits.pop();
    }
    let mut out = String::with_capacity(32);
    out.push_str(sign);
    if exponent < -4 || exponent >= precision {
        out.push(char::from(digits[0]));
        if digits.len() > 1 {
            out.push('.');
            for digit in &digits[1..] {
                out.push(char::from(*digit));
            }
        }
        out.push('E');
        out.push(if exponent < 0 { '-' } else { '+' });
        let exponent = exponent.unsigned_abs();
        if exponent < 10 {
            out.push('0');
        }
        out.push_str(&exponent.to_string());
    } else {
        let point = exponent + 1;
        if point <= 0 {
            out.push_str("0.");
            for _ in 0..-point {
                out.push('0');
            }
            for digit in digits {
                out.push(char::from(digit));
            }
        } else {
            for i in 0..(point as usize).max(digits.len()) {
                if i == point as usize {
                    out.push('.');
                }
                out.push(char::from(digits.get(i).copied().unwrap_or(b'0')));
            }
        }
    }
    out
}
