// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::controls::ManagedDockWindowDocument;
use ferroui_base::{
    data::{
        converters::IValueConverter,
        core::{ValueType, ValueTypes},
        BindingError,
    },
    utilities::CultureInfo,
    BoxedValue,
};
use std::rc::Rc;
/// Original converter reporting whether a managed document hosts a tool dock.
pub struct ManagedDockableIsToolWindowConverter;
impl PartialEq for ManagedDockableIsToolWindowConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl ManagedDockableIsToolWindowConverter {
    /// Creates the stateless source converter.
    pub fn new() -> Rc<Self> {
        Rc::new(Self)
    }
    /// Gets its shared native UI-thread instance.
    pub fn instance() -> Rc<Self> {
        ferroui_base::ferro_thread_local! {static INSTANCE:Rc<ManagedDockableIsToolWindowConverter>=ManagedDockableIsToolWindowConverter::new();}
        INSTANCE.with(Rc::clone)
    }
}
impl IValueConverter for ManagedDockableIsToolWindowConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target: ValueType,
        parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let is_tool = document(value).is_some_and(|d| d.is_tool_window());
        if parameter.and_then(|p| p.downcast_ref::<bool>()).copied() == Some(true) {
            return Ok(Some(Rc::new(!is_tool)));
        }
        if parameter
            .and_then(|p| p.downcast_ref::<String>())
            .is_some_and(|s| s.eq_ignore_ascii_case("Not"))
        {
            return Ok(Some(Rc::new(!is_tool)));
        }
        Ok(Some(Rc::new(is_tool)))
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
/// Original converter returning a managed window's active tool dock.
pub struct ManagedDockableToolDockConverter;
impl PartialEq for ManagedDockableToolDockConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl ManagedDockableToolDockConverter {
    /// Creates the stateless source converter.
    pub fn new() -> Rc<Self> {
        Rc::new(Self)
    }
    /// Gets its shared native UI-thread instance.
    pub fn instance() -> Rc<Self> {
        ferroui_base::ferro_thread_local! {static INSTANCE:Rc<ManagedDockableToolDockConverter>=ManagedDockableToolDockConverter::new();}
        INSTANCE.with(Rc::clone)
    }
}
impl IValueConverter for ManagedDockableToolDockConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(document(value)
            .and_then(|d| d.tool_dock())
            .map(|d| Rc::new(d) as BoxedValue))
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
fn document(value: Option<&BoxedValue>) -> Option<Rc<ManagedDockWindowDocument>> {
    let value = value?;
    if let Some(document) =
        ValueTypes::try_cast(value, ValueType::of::<Rc<ManagedDockWindowDocument>>())
            .and_then(|v| v.downcast_ref::<Rc<ManagedDockWindowDocument>>().cloned())
    {
        return Some(document);
    }
    crate::model_projection::dockable(Some(value))?
        .boxed_reference()
        .downcast_ref::<Rc<ManagedDockWindowDocument>>()
        .cloned()
}
ferroui_base::ferro_markup_type!(class ManagedDockableIsToolWindowConverter{
    namespace:"FerroDock.Ferro.Converters",this:Rc<ManagedDockableIsToolWindowConverter>,handles:[Rc<ManagedDockableIsToolWindowConverter>,Option<Rc<ManagedDockableIsToolWindowConverter>>],interfaces:[Rc<dyn IValueConverter>],constructors:[()=>ManagedDockableIsToolWindowConverter::new],fields:[Instance:Rc<ManagedDockableIsToolWindowConverter> => ManagedDockableIsToolWindowConverter::instance],
});
ferroui_base::ferro_markup_type!(class ManagedDockableToolDockConverter{
    namespace:"FerroDock.Ferro.Converters",this:Rc<ManagedDockableToolDockConverter>,handles:[Rc<ManagedDockableToolDockConverter>,Option<Rc<ManagedDockableToolDockConverter>>],interfaces:[Rc<dyn IValueConverter>],constructors:[()=>ManagedDockableToolDockConverter::new],fields:[Instance:Rc<ManagedDockableToolDockConverter> => ManagedDockableToolDockConverter::instance],
});
