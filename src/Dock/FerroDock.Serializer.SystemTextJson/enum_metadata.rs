// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::{error, JsonErrorKind, JsonTypeInfo, JsonValue};
use std::rc::Rc;
impl JsonTypeInfo {
    /// Uses the source default numeric JSON format for an Int32-backed enumeration.
    /// The compiled conversions retain the actual enum/flags representation, including
    /// undefined bits; names and quoted numbers are not accepted by this codec.
    /// The enum converter is independent of any supplied Int32 property converter.
    pub fn numeric_enum<T: 'static>(
        bits: impl Fn(&T) -> i32 + 'static,
        from_bits: impl Fn(i32) -> T + 'static,
    ) -> Rc<Self> {
        Self::converter(
            move |value, _| Ok(JsonValue::number(i64::from(bits(value)))),
            move |value, _| {
                if let JsonValue::Number(number) = value {
                    if let Ok(bits) = number.as_str().parse::<i32>() {
                        return Ok(Some(from_bits(bits)));
                    }
                }
                Err(error(
                    JsonErrorKind::InvalidJson,
                    "Expected an Int32 numeric enumeration value.",
                ))
            },
        )
    }
}
