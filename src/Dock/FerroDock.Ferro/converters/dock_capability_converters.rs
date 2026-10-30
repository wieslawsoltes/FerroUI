// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::{core::DockCapability, DockCapabilityResolver};
use ferroui_base::{
    data::{
        converters::{IMultiValueConverter, IValueConverter},
        core::ValueType,
        BindingError,
    },
    utilities::CultureInfo,
    BoxedValue,
};
use std::rc::Rc;

struct DockCapabilityConverter(DockCapability);
impl DockCapabilityConverter {
    fn convert_dockable(&self, value: Option<&BoxedValue>) -> bool {
        let Some(dockable) = crate::model_projection::dockable(value) else {
            return false;
        };
        let context = DockCapabilityResolver::resolve_operation_dock(Some(dockable.clone()));
        DockCapabilityResolver::is_enabled(Some(dockable), self.0, context)
    }
}
impl IValueConverter for DockCapabilityConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(Some(Rc::new(self.convert_dockable(value))))
    }
    fn convert_back(
        &self,
        _value: Option<&BoxedValue>,
        _target: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Err(BindingError::message("Specified method is not supported."))
    }
}
impl IMultiValueConverter for DockCapabilityConverter {
    fn convert(
        &self,
        values: &[Option<BoxedValue>],
        _target: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        // Additional bindings only invalidate the result; the source resolver owns precedence.
        Ok(Some(Rc::new(self.convert_dockable(
            values.first().and_then(Option::as_ref),
        ))))
    }
}
fn converter(capability: DockCapability) -> Rc<DockCapabilityConverter> {
    ferroui_base::ferro_thread_local! {
        static CLOSE:Rc<DockCapabilityConverter>=Rc::new(DockCapabilityConverter(DockCapability::Close));
        static PIN:Rc<DockCapabilityConverter>=Rc::new(DockCapabilityConverter(DockCapability::Pin));
        static FLOAT:Rc<DockCapabilityConverter>=Rc::new(DockCapabilityConverter(DockCapability::Float));
        static DRAG:Rc<DockCapabilityConverter>=Rc::new(DockCapabilityConverter(DockCapability::Drag));
        static DROP:Rc<DockCapabilityConverter>=Rc::new(DockCapabilityConverter(DockCapability::Drop));
        static DOCK_AS_DOCUMENT:Rc<DockCapabilityConverter>=Rc::new(DockCapabilityConverter(DockCapability::DockAsDocument));
    }
    match capability {
        DockCapability::Close => CLOSE.with(Rc::clone),
        DockCapability::Pin => PIN.with(Rc::clone),
        DockCapability::Float => FLOAT.with(Rc::clone),
        DockCapability::Drag => DRAG.with(Rc::clone),
        DockCapability::Drop => DROP.with(Rc::clone),
        DockCapability::DockAsDocument => DOCK_AS_DOCUMENT.with(Rc::clone),
    }
}
/// Original singleton converters evaluating effective dock capabilities through the model resolver.
pub struct DockCapabilityConverters;
impl DockCapabilityConverters {
    /// Gets CanCloseConverter.
    pub fn can_close_converter() -> Rc<dyn IValueConverter> {
        converter(DockCapability::Close)
    }
    /// Gets CanCloseMultiConverter; the same instance as CanCloseConverter.
    pub fn can_close_multi_converter() -> Rc<dyn IMultiValueConverter> {
        converter(DockCapability::Close)
    }
    /// Gets CanPinConverter.
    pub fn can_pin_converter() -> Rc<dyn IValueConverter> {
        converter(DockCapability::Pin)
    }
    /// Gets CanPinMultiConverter; the same instance as CanPinConverter.
    pub fn can_pin_multi_converter() -> Rc<dyn IMultiValueConverter> {
        converter(DockCapability::Pin)
    }
    /// Gets CanFloatConverter.
    pub fn can_float_converter() -> Rc<dyn IValueConverter> {
        converter(DockCapability::Float)
    }
    /// Gets CanFloatMultiConverter; the same instance as CanFloatConverter.
    pub fn can_float_multi_converter() -> Rc<dyn IMultiValueConverter> {
        converter(DockCapability::Float)
    }
    /// Gets CanDragConverter.
    pub fn can_drag_converter() -> Rc<dyn IValueConverter> {
        converter(DockCapability::Drag)
    }
    /// Gets CanDragMultiConverter; the same instance as CanDragConverter.
    pub fn can_drag_multi_converter() -> Rc<dyn IMultiValueConverter> {
        converter(DockCapability::Drag)
    }
    /// Gets CanDropConverter.
    pub fn can_drop_converter() -> Rc<dyn IValueConverter> {
        converter(DockCapability::Drop)
    }
    /// Gets CanDropMultiConverter; the same instance as CanDropConverter.
    pub fn can_drop_multi_converter() -> Rc<dyn IMultiValueConverter> {
        converter(DockCapability::Drop)
    }
    /// Gets CanDockAsDocumentConverter.
    pub fn can_dock_as_document_converter() -> Rc<dyn IValueConverter> {
        converter(DockCapability::DockAsDocument)
    }
    /// Gets CanDockAsDocumentMultiConverter; the same instance as CanDockAsDocumentConverter.
    pub fn can_dock_as_document_multi_converter() -> Rc<dyn IMultiValueConverter> {
        converter(DockCapability::DockAsDocument)
    }
}
ferroui_base::ferro_markup_type!(static DockCapabilityConverters {
    namespace:"FerroDock.Ferro.Converters",
    fields:[
        CanCloseConverter:Rc<dyn IValueConverter> =>DockCapabilityConverters::can_close_converter,
        CanCloseMultiConverter:Rc<dyn IMultiValueConverter> =>DockCapabilityConverters::can_close_multi_converter,
        CanPinConverter:Rc<dyn IValueConverter> =>DockCapabilityConverters::can_pin_converter,
        CanPinMultiConverter:Rc<dyn IMultiValueConverter> =>DockCapabilityConverters::can_pin_multi_converter,
        CanFloatConverter:Rc<dyn IValueConverter> =>DockCapabilityConverters::can_float_converter,
        CanFloatMultiConverter:Rc<dyn IMultiValueConverter> =>DockCapabilityConverters::can_float_multi_converter,
        CanDragConverter:Rc<dyn IValueConverter> =>DockCapabilityConverters::can_drag_converter,
        CanDragMultiConverter:Rc<dyn IMultiValueConverter> =>DockCapabilityConverters::can_drag_multi_converter,
        CanDropConverter:Rc<dyn IValueConverter> =>DockCapabilityConverters::can_drop_converter,
        CanDropMultiConverter:Rc<dyn IMultiValueConverter> =>DockCapabilityConverters::can_drop_multi_converter,
        CanDockAsDocumentConverter:Rc<dyn IValueConverter> =>DockCapabilityConverters::can_dock_as_document_converter,
        CanDockAsDocumentMultiConverter:Rc<dyn IMultiValueConverter> =>DockCapabilityConverters::can_dock_as_document_multi_converter,
    ],
});
