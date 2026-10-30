// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferroui_reactiveui::IReactiveObject;
use ferroui_system::{
    component_model::{INotifyPropertyChanged, INotifyPropertyChanging},
    events::Event,
};
use std::rc::Rc;
/// A source ReactiveObject reference retaining the actual enclosing model.
#[derive(Clone)]
pub struct ReactiveObjectReference {
    object: Rc<dyn IReactiveObject>,
}
impl ReactiveObjectReference {
    pub(crate) fn reference_id(&self) -> *const () {
        Rc::as_ptr(&self.object) as *const ()
    }
    /// Creates an identity-preserving reference to an original reactive object.
    pub fn new<T: IReactiveObject>(object: Rc<T>) -> Self {
        Self { object }
    }
}
impl PartialEq for ReactiveObjectReference {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.object, &other.object)
    }
}
impl INotifyPropertyChanged for ReactiveObjectReference {
    fn property_changed(&self) -> &Event<str> {
        self.object.property_changed()
    }
    fn as_notify_property_changing(&self) -> Option<&dyn INotifyPropertyChanging> {
        Some(self)
    }
}
impl INotifyPropertyChanging for ReactiveObjectReference {
    fn property_changing(&self) -> &Event<str> {
        self.object.property_changing()
    }
}
ferroui_base::ferro_markup_type!(class ReactiveObjectReference as "ReactiveObject"{namespace:"ReactiveUI.Reactive",this:ReactiveObjectReference,handles:[ReactiveObjectReference,Option<ReactiveObjectReference>],notify_property_changed:ReactiveObjectReference});
