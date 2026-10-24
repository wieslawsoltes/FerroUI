//! Port of `Diagnostics/Converters/BrushSelectorConveter.cs` (the misspelt name is upstream's).

use ferroui_base::data::converters::{cast_value, IValueConverter};
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::{BindingError, BindingOperations};
use ferroui_base::media::IBrush;
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, DirectProperty, FerroObject,
    FerroObjectImpl, FerroProperty, Ref,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Converts a brush to [`brush`](Self::brush) when it is the brush of the parameter (the same
/// brush, or a solid color brush of the same color, transform and opacity), and to null
/// otherwise: the brush that marks the selected one of a row of sample brushes.
#[repr(C)]
pub struct BrushSelectorConveter {
    base: FerroObject,
    brush: RefCell<Option<Rc<dyn IBrush>>>,
}

ferro_class!(BrushSelectorConveter: FerroObject);
ferro_class_info!(BrushSelectorConveter {
    new: BrushSelectorConveter::new,
    interfaces: [Rc<dyn IValueConverter> => BrushSelectorConveter::as_value_converter],
});

impl FerroObjectImpl for BrushSelectorConveter {}

ferro_properties! {
    impl BrushSelectorConveter {
        /// Defines the [`brush`](BrushSelectorConveter::brush) property.
        pub fn brush_property() -> DirectProperty<BrushSelectorConveter, Option<Rc<dyn IBrush>>> {
            FerroProperty::register_direct::<BrushSelectorConveter, _>(
                "Brush",
                |o| o.brush(),
                Some(|o, v| o.set_brush(v)),
                None,
            )
        }
    }
}

impl BrushSelectorConveter {
    pub fn construct() -> Self {
        Self { base: FerroObject::construct(), brush: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The converter as its contract (the class implements it in the original).
    pub fn as_value_converter(this: Ref<Self>) -> Rc<dyn IValueConverter> {
        Rc::new(BrushSelectorConveterHandle(this))
    }

    /// The brush the selected brush converts to.
    ///
    /// A plain property upstream: setting it raises no change notification.
    pub fn brush(&self) -> Option<Rc<dyn IBrush>> {
        self.brush.borrow().clone()
    }

    pub fn set_brush(&self, value: Option<Rc<dyn IBrush>>) {
        *self.brush.borrow_mut() = value;
    }

    /// `Convert`.
    pub fn convert(&self, value: Option<&BoxedValue>, parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        let value = value.cloned().and_then(ValueTypes::normalize);
        let parameter = parameter.cloned().and_then(ValueTypes::normalize);
        let value_brush = cast_value::<Rc<dyn IBrush>>(value.as_ref());
        let parameter_brush = cast_value::<Rc<dyn IBrush>>(parameter.as_ref());

        // `ReferenceEquals(value, parameter)`: two nulls, or the same object.
        let reference_equals = match (&value, &parameter) {
            (None, None) => true,
            (Some(_), Some(_)) => match (&value_brush, &parameter_brush) {
                (Some(a), Some(b)) => a.reference_id() == b.reference_id(),
                _ => ValueTypes::identity_equals(value.as_ref(), parameter.as_ref()),
            },
            _ => false,
        };

        if reference_equals {
            return self.brush().map(|brush| Rc::new(brush) as BoxedValue);
        }

        if let (Some(a), Some(b)) = (&value_brush, &parameter_brush) {
            if let (Some(a), Some(b)) = (a.as_solid_color_brush(), b.as_solid_color_brush()) {
                if a.color() == b.color() && a.transform() == b.transform() && b.opacity() == a.opacity() {
                    return self.brush().map(|brush| Rc::new(brush) as BoxedValue);
                }
            }
        }

        None
    }
}

/// The value converter contract of a [`BrushSelectorConveter`].
struct BrushSelectorConveterHandle(Ref<BrushSelectorConveter>);

impl IValueConverter for BrushSelectorConveterHandle {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(self.0.convert(value, parameter))
    }

    fn convert_back(
        &self,
        _value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(Some(BindingOperations::do_nothing()))
    }
}
