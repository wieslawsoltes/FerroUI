use ferroui_base::utilities::CultureInfo;
use ferroui_base::data::converters::{cast_value, IMultiValueConverter};
use ferroui_base::data::core::ValueType;
use ferroui_base::data::BindingError;
use ferroui_base::{BoxedValue, Thickness};
use std::rc::Rc;

/// Converts the level of a tree item and an indent to the left margin of the
/// item.
#[derive(Default)]
pub struct TreeViewItemIndentConverter;

impl TreeViewItemIndentConverter {
    /// The shared instance of the converter.
    pub fn instance() -> Rc<TreeViewItemIndentConverter> {
        thread_local! {
            static INSTANCE: Rc<TreeViewItemIndentConverter> = Rc::new(TreeViewItemIndentConverter);
        }
        INSTANCE.with(Rc::clone)
    }
}

impl IMultiValueConverter for TreeViewItemIndentConverter {
    fn convert(
        &self,
        values: &[Option<BoxedValue>],
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        if values.len() > 1 {
            if let (Some(level), Some(indent)) =
                (cast_value::<i32>(values[0].as_ref()), cast_value::<f64>(values[1].as_ref()))
            {
                return Ok(Some(Rc::new(Thickness::new(indent * f64::from(level), 0.0, 0.0, 0.0))));
            }
        }

        Ok(Some(Rc::new(Thickness::uniform(0.0))))
    }
}
