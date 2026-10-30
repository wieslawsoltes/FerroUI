// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferroui_base::{
    data::{converters::IValueConverter, core::ValueType, BindingError},
    utilities::CultureInfo,
    BoxedValue,
};
use std::rc::Rc;

/// Original OwnerIsToolDockConverter binding converter.
pub struct OwnerIsToolDockConverter;
impl PartialEq for OwnerIsToolDockConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl OwnerIsToolDockConverter {
    /// Creates the original stateless converter.
    pub fn new() -> Rc<Self> {
        Rc::new(Self)
    }
    /// Gets the shared converter on the native UI thread.
    pub fn instance() -> Rc<Self> {
        ferroui_base::ferro_thread_local! {static INSTANCE:Rc<OwnerIsToolDockConverter>=OwnerIsToolDockConverter::new();}
        INSTANCE.with(Rc::clone)
    }
}
impl IValueConverter for OwnerIsToolDockConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(Some(Rc::new(
            crate::model_projection::dockable(value).is_some_and(|v| v.as_tool_dock().is_some()),
        )))
    }
    fn convert_back(
        &self,
        _value: Option<&BoxedValue>,
        _target: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Err(BindingError::message(
            "The method or operation is not implemented.",
        ))
    }
}
ferroui_base::ferro_markup_type!(class OwnerIsToolDockConverter {
    namespace:"FerroDock.Ferro.Converters",this:Rc<OwnerIsToolDockConverter>,handles:[Rc<OwnerIsToolDockConverter>,Option<Rc<OwnerIsToolDockConverter>>],interfaces:[Rc<dyn IValueConverter>],constructors:[()=>OwnerIsToolDockConverter::new],fields:[Instance:Rc<OwnerIsToolDockConverter> =>OwnerIsToolDockConverter::instance],
});

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_converter_tests_tool_dock_type_and_rejects_other_values() {
        use ferrodock_model_reactiveui::controls::{DocumentDock, Tool, ToolDock};
        let c = OwnerIsToolDockConverter::new();
        for (value, expected) in [
            (None, false),
            (
                Some(crate::model_projection::to_dockable_value(ToolDock::new())),
                true,
            ),
            (
                Some(crate::model_projection::to_dockable_value(
                    DocumentDock::new(),
                )),
                false,
            ),
            (
                Some(crate::model_projection::to_dockable_value(Tool::new())),
                false,
            ),
            (Some(Rc::new(true) as BoxedValue), false),
        ] {
            let result = c
                .convert(
                    value.as_ref(),
                    ValueType::of::<bool>(),
                    None,
                    &CultureInfo::invariant_culture(),
                )
                .unwrap()
                .unwrap();
            assert_eq!(result.downcast_ref::<bool>(), Some(&expected));
        }
        assert!(c
            .convert_back(
                None,
                ValueType::of::<bool>(),
                None,
                &CultureInfo::invariant_culture()
            )
            .is_err());
    }
}
