use crate::media::transformation::{TransformOperations, TransformOperationsBuilder};
use crate::utilities::span_helpers::{try_parse_double, write_double, NumberStyles};
use crate::utilities::{FormatError, MathUtilities};
use crate::Matrix;
use std::rc::Rc;

/// Parses transform strings such as `translate(10px, 5px) rotate(45deg)`.
pub struct TransformParser;

const FUNCTION_MAPPING: [(&str, TransformFunction); 11] = [
    ("translate", TransformFunction::Translate),
    ("translateX", TransformFunction::TranslateX),
    ("translateY", TransformFunction::TranslateY),
    ("scale", TransformFunction::Scale),
    ("scaleX", TransformFunction::ScaleX),
    ("scaleY", TransformFunction::ScaleY),
    ("skew", TransformFunction::Skew),
    ("skewX", TransformFunction::SkewX),
    ("skewY", TransformFunction::SkewY),
    ("rotate", TransformFunction::Rotate),
    ("matrix", TransformFunction::Matrix),
];

const UNIT_MAPPING: [(&str, Unit); 5] = [
    ("deg", Unit::Degree),
    ("grad", Unit::Gradian),
    ("rad", Unit::Radian),
    ("turn", Unit::Turn),
    ("px", Unit::Pixel),
];

impl TransformParser {
    pub fn parse(s: &str) -> Result<Rc<TransformOperations>, FormatError> {
        let invalid_format = || FormatError::from_string(format!("Invalid transform string: '{s}'."));

        if s.is_empty() {
            return Err(invalid_format());
        }

        let mut span = s.trim();

        if span.eq_ignore_ascii_case("none") {
            return Ok(TransformOperations::identity());
        }

        let mut builder = TransformOperations::create_builder(0);

        loop {
            let begin_index = span.find('(');
            let end_index = span.find(')');

            let (Some(begin_index), Some(end_index)) = (begin_index, end_index) else {
                return Err(invalid_format());
            };
            if end_index < begin_index {
                return Err(invalid_format());
            }

            let name_part = span[..begin_index].trim();

            let function = parse_transform_function(name_part);

            if function == TransformFunction::Invalid {
                return Err(invalid_format());
            }

            let value_part = span[begin_index + 1..end_index].trim();

            parse_function(value_part, function, &mut builder)?;

            span = &span[end_index + 1..];

            if span.chars().all(char::is_whitespace) {
                break;
            }
        }

        Ok(builder.build())
    }
}

fn parse_value(mut part: &str) -> Result<UnitValue, FormatError> {
    let mut unit_index = None;

    for (i, c) in part.char_indices() {
        if c.is_ascii_digit() || c == '-' || c == '.' {
            continue;
        }

        unit_index = Some(i);
        break;
    }

    let mut unit = Unit::None;

    if let Some(unit_index) = unit_index {
        let unit_part = &part[unit_index..];

        unit = parse_unit(unit_part)?;

        part = &part[..unit_index];
    }

    let value = try_parse_double(part, NumberStyles::FLOAT).ok_or_else(|| FormatError::invalid_input(part))?;

    Ok(UnitValue { unit, value })
}

fn parse_value_pair(part: &str, left_value: &mut UnitValue, right_value: &mut UnitValue) -> Result<usize, FormatError> {
    if let Some(comma_index) = part.find(',') {
        let left_part = part[..comma_index].trim();
        let right_part = part[comma_index + 1..].trim();

        *left_value = parse_value(left_part)?;
        *right_value = parse_value(right_part)?;

        return Ok(2);
    }

    *left_value = parse_value(part)?;

    Ok(1)
}

fn parse_comma_delimited_values(mut part: &str, out_values: &mut [UnitValue]) -> Result<usize, FormatError> {
    let mut value_index = 0;

    loop {
        if value_index >= out_values.len() {
            return Err(FormatError::new("Too many provided values."));
        }

        let Some(comma_index) = part.find(',') else {
            if !part.chars().all(char::is_whitespace) {
                out_values[value_index] = parse_value(part)?;
                value_index += 1;
            }

            break;
        };

        let value_part = part[..comma_index].trim();

        out_values[value_index] = parse_value(value_part)?;
        value_index += 1;

        part = &part[comma_index + 1..];
    }

    Ok(value_index)
}

fn parse_function(
    function_part: &str,
    function: TransformFunction,
    builder: &mut TransformOperationsBuilder,
) -> Result<(), FormatError> {
    match function {
        TransformFunction::Scale | TransformFunction::ScaleX | TransformFunction::ScaleY => {
            let mut scale_x = UnitValue::ONE;
            let mut scale_y = UnitValue::ONE;

            let count = parse_value_pair(function_part, &mut scale_x, &mut scale_y)?;

            if count != 1 && (function == TransformFunction::ScaleX || function == TransformFunction::ScaleY) {
                return Err(format_invalid_value_count(function, 1));
            }

            verify_zero_or_unit(function, &scale_x, Unit::None)?;
            verify_zero_or_unit(function, &scale_y, Unit::None)?;

            if function == TransformFunction::ScaleY {
                scale_y = scale_x;
                scale_x = UnitValue::ONE;
            } else if function == TransformFunction::Scale && count == 1 {
                scale_y = scale_x;
            }

            builder.append_scale(scale_x.value, scale_y.value);
        }
        TransformFunction::Skew | TransformFunction::SkewX | TransformFunction::SkewY => {
            let mut skew_x = UnitValue::ZERO;
            let mut skew_y = UnitValue::ZERO;

            let count = parse_value_pair(function_part, &mut skew_x, &mut skew_y)?;

            if count != 1 && (function == TransformFunction::SkewX || function == TransformFunction::SkewY) {
                return Err(format_invalid_value_count(function, 1));
            }

            verify_zero_or_angle(function, &skew_x)?;
            verify_zero_or_angle(function, &skew_y)?;

            if function == TransformFunction::SkewY {
                skew_y = skew_x;
                skew_x = UnitValue::ZERO;
            }

            builder.append_skew(to_radians(&skew_x), to_radians(&skew_y));
        }
        TransformFunction::Rotate => {
            let mut angle = UnitValue::ZERO;
            let mut unused = UnitValue::ZERO;

            let count = parse_value_pair(function_part, &mut angle, &mut unused)?;

            if count != 1 {
                return Err(format_invalid_value_count(function, 1));
            }

            verify_zero_or_angle(function, &angle)?;

            builder.append_rotate(to_radians(&angle));
        }
        TransformFunction::Translate | TransformFunction::TranslateX | TransformFunction::TranslateY => {
            let mut translate_x = UnitValue::ZERO;
            let mut translate_y = UnitValue::ZERO;

            let count = parse_value_pair(function_part, &mut translate_x, &mut translate_y)?;

            if count != 1 && (function == TransformFunction::TranslateX || function == TransformFunction::TranslateY)
            {
                return Err(format_invalid_value_count(function, 1));
            }

            verify_zero_or_unit(function, &translate_x, Unit::Pixel)?;
            verify_zero_or_unit(function, &translate_y, Unit::Pixel)?;

            if function == TransformFunction::TranslateY {
                translate_y = translate_x;
                translate_x = UnitValue::ZERO;
            }

            builder.append_translate(translate_x.value, translate_y.value);
        }
        TransformFunction::Matrix => {
            let mut values = [UnitValue::ZERO; 6];

            let count = parse_comma_delimited_values(function_part, &mut values)?;

            if count != 6 {
                return Err(format_invalid_value_count(function, 6));
            }

            for value in &values {
                verify_zero_or_unit(function, value, Unit::None)?;
            }

            let matrix = Matrix::new(
                values[0].value,
                values[1].value,
                values[2].value,
                values[3].value,
                values[4].value,
                values[5].value,
            );

            builder.append_matrix(matrix);
        }
        TransformFunction::Invalid => {}
    }

    Ok(())
}

fn verify_zero_or_unit(function: TransformFunction, value: &UnitValue, unit: Unit) -> Result<(), FormatError> {
    let is_zero = value.unit == Unit::None && value.value == 0.0;

    if !is_zero && value.unit != unit {
        return Err(format_invalid_value(function, value));
    }
    Ok(())
}

fn verify_zero_or_angle(function: TransformFunction, value: &UnitValue) -> Result<(), FormatError> {
    if value.value != 0.0 && !is_angle_unit(value.unit) {
        return Err(format_invalid_value(function, value));
    }
    Ok(())
}

fn is_angle_unit(unit: Unit) -> bool {
    matches!(unit, Unit::Radian | Unit::Gradian | Unit::Degree | Unit::Turn)
}

fn format_invalid_value(function: TransformFunction, value: &UnitValue) -> FormatError {
    let unit_string = if value.unit == Unit::None { String::new() } else { format!("{:?}", value.unit) };
    let mut number = String::new();
    let _ = write_double(&mut number, value.value);

    FormatError::from_string(format!("Invalid value {number} {unit_string} for {function:?}"))
}

fn format_invalid_value_count(function: TransformFunction, count: usize) -> FormatError {
    FormatError::from_string(format!("Invalid format. {function:?} expects {count} value(s)."))
}

fn parse_unit(part: &str) -> Result<Unit, FormatError> {
    for (name, unit) in UNIT_MAPPING {
        if part.eq_ignore_ascii_case(name) {
            return Ok(unit);
        }
    }

    Err(FormatError::from_string(format!("Invalid unit: {part}")))
}

fn parse_transform_function(part: &str) -> TransformFunction {
    for (name, transform_function) in FUNCTION_MAPPING {
        if part.eq_ignore_ascii_case(name) {
            return transform_function;
        }
    }

    TransformFunction::Invalid
}

fn to_radians(value: &UnitValue) -> f64 {
    match value.unit {
        Unit::Radian => value.value,
        Unit::Gradian => MathUtilities::grad2rad(value.value),
        Unit::Degree => MathUtilities::deg2rad(value.value),
        Unit::Turn => MathUtilities::turn2rad(value.value),
        _ => value.value,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Unit {
    None,
    Pixel,
    Radian,
    Gradian,
    Degree,
    Turn,
}

#[derive(Clone, Copy, Debug)]
struct UnitValue {
    unit: Unit,
    value: f64,
}

impl UnitValue {
    const ZERO: UnitValue = UnitValue { unit: Unit::None, value: 0.0 };
    const ONE: UnitValue = UnitValue { unit: Unit::None, value: 1.0 };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TransformFunction {
    Invalid,
    Translate,
    TranslateX,
    TranslateY,
    Scale,
    ScaleX,
    ScaleY,
    Skew,
    SkewX,
    SkewY,
    Rotate,
    Matrix,
}
