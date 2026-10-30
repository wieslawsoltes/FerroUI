// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::ReactiveObjectReference;
use ferroui_reactiveui::IReactiveObject;
use ferroui_system::{
    component_model::{INotifyPropertyChanged, INotifyPropertyChanging},
    events::Event,
};
use std::rc::Rc;
/// Native reference to the source abstract ReactiveBase without copying its state.
#[derive(Clone, PartialEq)]
pub struct ReactiveBaseReference {
    base: ReactiveObjectReference,
}
impl ReactiveBaseReference {
    /// Declares the enclosing model's original ReactiveBase inheritance.
    pub fn new<T: IReactiveObject>(model: Rc<T>) -> Self {
        Self {
            base: ReactiveObjectReference::new(model),
        }
    }
    /// Gets the original ReactiveObject base reference.
    pub fn base(&self) -> ReactiveObjectReference {
        self.base.clone()
    }
}
impl INotifyPropertyChanged for ReactiveBaseReference {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
    fn as_notify_property_changing(&self) -> Option<&dyn INotifyPropertyChanging> {
        Some(self)
    }
}
impl INotifyPropertyChanging for ReactiveBaseReference {
    fn property_changing(&self) -> &Event<str> {
        self.base.property_changing()
    }
}
ferroui_base::ferro_markup_type!(class ReactiveBaseReference as "ReactiveBase"{namespace:"FerroDock.Model.ReactiveUI.Core",this:ReactiveBaseReference,handles:[ReactiveBaseReference,Option<ReactiveBaseReference>],base:ReactiveObjectReference,notify_property_changed:ReactiveBaseReference});
pub(crate) fn register() {
    use ferroui_base::{
        data::core::ValueTypes,
        metadata::{MarkupType, MarkupTyped},
    };
    MarkupType::register(<ReactiveBaseReference as MarkupTyped>::MARKUP);
    ValueTypes::register_nullable::<ReactiveBaseReference>();
    ValueTypes::register_cast::<ReactiveBaseReference, ReactiveObjectReference>(|value| {
        value.base()
    });
}
