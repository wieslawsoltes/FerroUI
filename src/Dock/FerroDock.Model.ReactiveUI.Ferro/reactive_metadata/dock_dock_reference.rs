// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::DockBaseReference;
use ferrodock_model::{
    controls::IDockDock,
    core::{IDock, IDockable},
};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
use ferroui_reactiveui::IReactiveObject;
use ferroui_system::{
    component_model::{INotifyPropertyChanged, INotifyPropertyChanging},
    events::Event,
};
use std::rc::Rc;
/// Identity-preserving native class view of the original DockDock model.
#[derive(Clone)]
pub struct DockDockReference {
    model: Rc<dyn IDockDock>,
    base: DockBaseReference,
}
impl DockDockReference {
    /// Constructs the original concrete model, preserving its source defaults and behavior.
    pub fn create() -> Self {
        Self::new(ferrodock_model_reactiveui::controls::DockDock::new())
    }
    /// Views an original model or composed source subclass without copying its state.
    pub fn new<T: IDockDock + IReactiveObject>(model: Rc<T>) -> Self {
        Self {
            base: DockBaseReference::new(model.clone()),
            model,
        }
    }
    /// Gets the same enclosing source model through its original contract.
    pub fn model(&self) -> Rc<dyn IDockDock> {
        self.model.clone()
    }
    /// Gets the inherited source class view of the same enclosing object.
    pub fn base(&self) -> DockBaseReference {
        self.base.clone()
    }
}
impl PartialEq for DockDockReference {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.model, &other.model)
    }
}
impl INotifyPropertyChanged for DockDockReference {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
    fn as_notify_property_changing(&self) -> Option<&dyn INotifyPropertyChanging> {
        Some(self)
    }
}
impl INotifyPropertyChanging for DockDockReference {
    fn property_changing(&self) -> &Event<str> {
        self.base.property_changing()
    }
}
ferroui_base::ferro_markup_type!(class DockDockReference as "DockDock" {
 namespace: "FerroDock.Model.ReactiveUI.Controls", this: DockDockReference,
 handles: [DockDockReference, Option<DockDockReference>], base: DockBaseReference,
 interfaces: [Rc<dyn IDockDock>],
 constructors: [() => DockDockReference::create], notify_property_changed: DockDockReference,
 properties: [
  LastChildFill: bool { get: |this: &DockDockReference| this.model.last_child_fill(), set: |this: &DockDockReference, value| this.model.set_last_child_fill(value) },
 ],
});
pub(crate) fn register() {
    MarkupType::register(DockDockReference::MARKUP);
    ValueTypes::register_nullable::<Rc<dyn IDockDock>>();
    ValueTypes::register_cast::<Rc<dyn IDockDock>, Rc<dyn IDockable>>(|model| model.clone());
    ValueTypes::register_cast::<Rc<dyn IDockDock>, Rc<dyn IDock>>(|model| model.clone());
    ValueTypes::register_nullable::<DockDockReference>();
    ValueTypes::register_cast::<DockDockReference, super::DockBaseReference>(|value| value.base());
    ValueTypes::register_cast::<DockDockReference, super::DockableBaseReference>(|value| {
        value.base().base()
    });
    ValueTypes::register_cast::<DockDockReference, super::ReactiveBaseReference>(|value| {
        value.base().base().base()
    });
    ValueTypes::register_cast::<DockDockReference, super::ReactiveObjectReference>(|value| {
        value.base().base().base().base()
    });
    ValueTypes::register_cast::<DockDockReference, Rc<dyn IDockable>>(|value| value.model());
    ValueTypes::register_cast::<DockDockReference, Rc<dyn IDock>>(|value| value.model());
    ValueTypes::register_cast::<DockDockReference, Rc<dyn IDockDock>>(|value| value.model());
    ValueTypes::register_reference_identity::<DockDockReference>(|value| {
        Rc::as_ptr(&value.model()) as *const ()
    });
}
