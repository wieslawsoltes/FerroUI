use ferroui_base::utilities::CultureInfo;
use crate::shapes::Rectangle;
use crate::{ColumnDefinition, Control, Grid, GridLength, GridUnitType, RowDefinition};
use ferroui_base::data::converters::{cast_value, IMultiValueConverter};
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::BindingError;
use ferroui_base::media::{Brushes, IBrush, VisualBrush};
use ferroui_base::{BoxedValue, FerroProperty};
use std::rc::Rc;

/// Creates the opacity mask of a group box border: a brush which leaves a
/// gap in the top edge of the border where the header is drawn.
///
/// The values are the header width, the border width and the border height;
/// the parameter is the width of the line to the left of the header.
///
/// Derived from the converter of the same name in the WPF sources (.NET
/// Foundation, MIT license).
#[derive(Default)]
pub struct BorderGapMaskConverter;

impl BorderGapMaskConverter {
    pub fn new() -> Self {
        Self
    }

    /// A grid length in pixels, or an error where the value cannot be one.
    fn pixels(value: f64) -> Result<GridLength, BindingError> {
        if value < 0.0 || value.is_nan() || value.is_infinite() {
            return Err(BindingError::message("Invalid value"));
        }
        Ok(GridLength::new(value, GridUnitType::Pixel))
    }
}

impl IMultiValueConverter for BorderGapMaskConverter {
    fn convert(
        &self,
        values: &[Option<BoxedValue>],
        _target_type: ValueType,
        parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        //
        // Parameter validation
        //
        if parameter.cloned().and_then(ValueTypes::normalize).is_none() || values.len() != 3 {
            return Ok(Some(FerroProperty::unset_value()));
        }

        let (Some(header_width), Some(border_width), Some(border_height)) = (
            cast_value::<f64>(values[0].as_ref()),
            cast_value::<f64>(values[1].as_ref()),
            cast_value::<f64>(values[2].as_ref()),
        ) else {
            return Ok(Some(FerroProperty::unset_value()));
        };

        let parameter_number = cast_value::<f64>(parameter);
        let parameter_text = cast_value::<String>(parameter);
        if parameter_number.is_none() && parameter_text.is_none() {
            return Ok(Some(FerroProperty::unset_value()));
        }

        //
        // Conversion
        //

        // Doesn't make sense to have a grid with 0 as width or height.
        if border_width == 0.0 || border_height == 0.0 {
            return Ok(None);
        }

        // Width of the line to the left of the header, to be used to set the
        // width of the first column of the grid.
        let line_width = match (parameter_text, parameter_number) {
            (Some(text), _) => text.trim().parse::<f64>().map_err(|_| {
                BindingError::message(format!("The input string '{text}' was not in a correct format."))
            })?,
            (None, Some(number)) => number,
            (None, None) => return Ok(Some(FerroProperty::unset_value())),
        };

        let grid = Grid::new();
        grid.set_width(border_width);
        grid.set_height(border_height);

        let col_def1 = ColumnDefinition::new();
        let col_def2 = ColumnDefinition::new();
        let col_def3 = ColumnDefinition::new();
        col_def1.set_width(Self::pixels(line_width)?);
        col_def2.set_width(Self::pixels(header_width)?);
        col_def3.set_width(GridLength::new(1.0, GridUnitType::Star));
        grid.column_definitions().add(col_def1);
        grid.column_definitions().add(col_def2);
        grid.column_definitions().add(col_def3);
        let row_def1 = RowDefinition::new();
        let row_def2 = RowDefinition::new();
        row_def1.set_height(Self::pixels(border_height / 2.0)?);
        row_def2.set_height(GridLength::new(1.0, GridUnitType::Star));
        grid.row_definitions().add(row_def1);
        grid.row_definitions().add(row_def2);

        let rect_column1 = Rectangle::new();
        let rect_column2 = Rectangle::new();
        let rect_column3 = Rectangle::new();
        let black: Option<Rc<dyn IBrush>> = Some(Brushes::black());
        rect_column1.set_fill(black.clone());
        rect_column2.set_fill(black.clone());
        rect_column3.set_fill(black);

        Grid::set_row_span(&rect_column1, 2);
        Grid::set_row(&rect_column1, 0);
        Grid::set_column(&rect_column1, 0);

        Grid::set_row(&rect_column2, 1);
        Grid::set_column(&rect_column2, 1);

        Grid::set_row_span(&rect_column3, 2);
        Grid::set_row(&rect_column3, 0);
        Grid::set_column(&rect_column3, 2);

        grid.children().add(rect_column1.upcast::<Control>());
        grid.children().add(rect_column2.upcast::<Control>());
        grid.children().add(rect_column3.upcast::<Control>());

        let brush: Rc<dyn IBrush> = VisualBrush::with_visual(grid).into();
        Ok(Some(Rc::new(Some(brush))))
    }
}
