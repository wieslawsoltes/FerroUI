// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::DockBaseReference;
use ferrodock_model::core::Orientation;
use ferrodock_model::{
    controls::IStackDock,
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
/// Identity-preserving native class view of the original StackDock model.
#[derive(Clone)]
pub struct StackDockReference {
    model: Rc<dyn IStackDock>,
    base: DockBaseReference,
}
impl StackDockReference {
    /// Constructs the original concrete model, preserving its source defaults and behavior.
    pub fn create() -> Self {
        Self::new(ferrodock_model_reactiveui::controls::StackDock::new())
    }
    /// Views an original model or composed source subclass without copying its state.
    pub fn new<T: IStackDock + IReactiveObject>(model: Rc<T>) -> Self {
        Self {
            base: DockBaseReference::new(model.clone()),
            model,
        }
    }
    /// Gets the same enclosing source model through its original contract.
    pub fn model(&self) -> Rc<dyn IStackDock> {
        self.model.clone()
    }
    /// Gets the inherited source class view of the same enclosing object.
    pub fn base(&self) -> DockBaseReference {
        self.base.clone()
    }
}
impl PartialEq for StackDockReference {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.model, &other.model)
    }
}
impl INotifyPropertyChanged for StackDockReference {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
    fn as_notify_property_changing(&self) -> Option<&dyn INotifyPropertyChanging> {
        Some(self)
    }
}
impl INotifyPropertyChanging for StackDockReference {
    fn property_changing(&self) -> &Event<str> {
        self.base.property_changing()
    }
}
ferroui_base::ferro_markup_type!(class StackDockReference as "StackDock" {
 namespace: "FerroDock.Model.ReactiveUI.Controls", this: StackDockReference,
 handles: [StackDockReference, Option<StackDockReference>], base: DockBaseReference,
 interfaces: [Rc<dyn IStackDock>],
 constructors: [() => StackDockReference::create], notify_property_changed: StackDockReference,
 properties: [
  Orientation: Orientation { get: |this: &StackDockReference| this.model.orientation(), set: |this: &StackDockReference, value| this.model.set_orientation(value) },
  Spacing: f64 { get: |this: &StackDockReference| this.model.spacing(), set: |this: &StackDockReference, value| this.model.set_spacing(value) },
 ],
});
pub(crate) fn register() {
    MarkupType::register(StackDockReference::MARKUP);
    ValueTypes::register_nullable::<Rc<dyn IStackDock>>();
    ValueTypes::register_cast::<Rc<dyn IStackDock>, Rc<dyn IDockable>>(|model| model.clone());
    ValueTypes::register_cast::<Rc<dyn IStackDock>, Rc<dyn IDock>>(|model| model.clone());
    ValueTypes::register_nullable::<StackDockReference>();
    ValueTypes::register_cast::<StackDockReference, super::DockBaseReference>(|value| value.base());
    ValueTypes::register_cast::<StackDockReference, super::DockableBaseReference>(|value| {
        value.base().base()
    });
    ValueTypes::register_cast::<StackDockReference, super::ReactiveBaseReference>(|value| {
        value.base().base().base()
    });
    ValueTypes::register_cast::<StackDockReference, super::ReactiveObjectReference>(|value| {
        value.base().base().base().base()
    });
    ValueTypes::register_cast::<StackDockReference, Rc<dyn IDockable>>(|value| value.model());
    ValueTypes::register_cast::<StackDockReference, Rc<dyn IDock>>(|value| value.model());
    ValueTypes::register_cast::<StackDockReference, Rc<dyn IStackDock>>(|value| value.model());
    ValueTypes::register_reference_identity::<StackDockReference>(|value| {
        Rc::as_ptr(&value.model()) as *const ()
    });
}
