//! Port of `Helpers/ParseHelper.cs`.

use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::BoxedValue;
use std::rc::Rc;

/// Parses text into the values of the types of the framework that have a `Parse(string)`.
///
/// Deviation (DEVIATIONS.md, Behaviors): the reference keeps a table of the `Parse` methods of
/// 40 framework types (brushes, colors, geometries, thicknesses, transforms, cursors, key
/// gestures, ..). Every one of them states its parser in its markup metadata (`parse:`), which is
/// what the text conversion of the untyped values uses, so the table is that conversion.
pub struct ParseHelper;

impl ParseHelper {
    /// Parses `s` into a value of `target_type`; `None` when the type has no parser or the text
    /// does not parse.
    pub fn invoke_parse(s: &str, target_type: ValueType) -> Option<BoxedValue> {
        let text: BoxedValue = Rc::new(s.to_string());
        ValueTypes::try_parse_text(&text, target_type)
    }
}
