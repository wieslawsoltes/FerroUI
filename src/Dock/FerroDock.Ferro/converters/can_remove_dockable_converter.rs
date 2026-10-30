// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferroui_base::{
    data::{converters::IMultiValueConverter, core::ValueType, BindingError},
    utilities::CultureInfo,
    BoxedValue,
};
use std::rc::Rc;
/// Determines whether the original dock permits removing its last visible dockable.
pub struct CanRemoveDockableConverter;
impl PartialEq for CanRemoveDockableConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl CanRemoveDockableConverter {
    /// Creates the stateless source converter.
    pub fn new() -> Rc<Self> {
        Rc::new(Self)
    }
    /// Gets the shared native UI-thread instance.
    pub fn instance() -> Rc<Self> {
        ferroui_base::ferro_thread_local! {static INSTANCE:Rc<CanRemoveDockableConverter>=CanRemoveDockableConverter::new();}
        INSTANCE.with(Rc::clone)
    }
}
impl IMultiValueConverter for CanRemoveDockableConverter {
    fn convert(
        &self,
        values: &[Option<BoxedValue>],
        _target: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        if values.len() == 1 {
            if let Some(dock) =
                crate::model_projection::dockable(values[0].as_ref()).and_then(|d| d.as_dock())
            {
                return Ok(Some(Rc::new(
                    dock.can_close_last_dockable() || dock.opened_dockables_count() > 1,
                )));
            }
        }
        if values.len() >= 2 {
            if let (Some(can_close_last), Some(count)) = (
                values[0].as_ref().and_then(|v| v.downcast_ref::<bool>()),
                values[1].as_ref().and_then(|v| v.downcast_ref::<i32>()),
            ) {
                return Ok(Some(Rc::new(*can_close_last || *count > 1)));
            }
        }
        Ok(Some(Rc::new(true)))
    }
}
ferroui_base::ferro_markup_type!(class CanRemoveDockableConverter {
    namespace:"FerroDock.Ferro.Converters",this:Rc<CanRemoveDockableConverter>,handles:[Rc<CanRemoveDockableConverter>,Option<Rc<CanRemoveDockableConverter>>],interfaces:[Rc<dyn IMultiValueConverter>],constructors:[()=>CanRemoveDockableConverter::new],fields:[Instance:Rc<CanRemoveDockableConverter> =>CanRemoveDockableConverter::instance],
});
