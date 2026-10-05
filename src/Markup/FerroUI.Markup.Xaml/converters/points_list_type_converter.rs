//! Port of `Converters/PointsListTypeConverter.cs`.

use super::type_converter::{text_of, type_converter_markup};
use super::{ITypeDescriptorContext, TypeConverter};
use crate::XamlLoadException;
use ferroui_base::data::core::ValueType;
use ferroui_base::utilities::{CultureInfo, FormatError, SpanStringTokenizer};
use ferroui_base::{BoxedValue, Point};
use std::rc::Rc;

/// Converts text to a list of points: coordinates separated by commas or
/// whitespace, two per point.
pub struct PointsListTypeConverter;

impl PointsListTypeConverter {
    /// Parses a list of points.
    pub fn parse(text: &str) -> Result<Vec<Point>, FormatError> {
        let mut points = Vec::new();

        let mut tokenizer = SpanStringTokenizer::with_message(text, "Invalid PointsList.");
        while let Some(x) = tokenizer.try_read_double()? {
            points.push(Point::new(x, tokenizer.read_double()?));
        }
        tokenizer.finish()?;

        Ok(points)
    }
}

impl TypeConverter for PointsListTypeConverter {
    fn can_convert_from(&self, _context: Option<&Rc<dyn ITypeDescriptorContext>>, source_type: ValueType) -> bool {
        source_type.is_string()
    }

    fn convert_from(
        &self,
        _context: Option<&Rc<dyn ITypeDescriptorContext>>,
        _culture: Option<&CultureInfo>,
        value: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, XamlLoadException> {
        let points = Self::parse(text_of(value)?).map_err(|e| XamlLoadException::with_message(e.to_string()))?;
        Ok(Some(Rc::new(points)))
    }
}

type_converter_markup!(PointsListTypeConverter);
