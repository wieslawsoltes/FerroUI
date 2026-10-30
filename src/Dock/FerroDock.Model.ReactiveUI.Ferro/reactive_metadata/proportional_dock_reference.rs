// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::DockBaseReference;
use ferrodock_model::core::IGlobalTarget;
use ferrodock_model::core::Orientation;
use ferrodock_model::{
    controls::IProportionalDock,
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
/// Identity-preserving native class view of the original ProportionalDock model.
#[derive(Clone)]
pub struct ProportionalDockReference {
    model: Rc<dyn IProportionalDock>,
    base: DockBaseReference,
}
impl ProportionalDockReference {
    /// Constructs the original concrete model, preserving its source defaults and behavior.
    pub fn create() -> Self {
        Self::new(ferrodock_model_reactiveui::controls::ProportionalDock::new())
    }
    /// Views an original model or composed source subclass without copying its state.
    pub fn new<T: IProportionalDock + IReactiveObject>(model: Rc<T>) -> Self {
        Self {
            base: DockBaseReference::new(model.clone()),
            model,
        }
    }
    /// Gets the same enclosing source model through its original contract.
    pub fn model(&self) -> Rc<dyn IProportionalDock> {
        self.model.clone()
    }
    /// Gets the inherited source class view of the same enclosing object.
    pub fn base(&self) -> DockBaseReference {
        self.base.clone()
    }
}
impl PartialEq for ProportionalDockReference {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.model, &other.model)
    }
}
impl INotifyPropertyChanged for ProportionalDockReference {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
    fn as_notify_property_changing(&self) -> Option<&dyn INotifyPropertyChanging> {
        Some(self)
    }
}
impl INotifyPropertyChanging for ProportionalDockReference {
    fn property_changing(&self) -> &Event<str> {
        self.base.property_changing()
    }
}
ferroui_base::ferro_markup_type!(class ProportionalDockReference as "ProportionalDock" {
 namespace: "FerroDock.Model.ReactiveUI.Controls", this: ProportionalDockReference,
 handles: [ProportionalDockReference, Option<ProportionalDockReference>], base: DockBaseReference,
 interfaces: [Rc<dyn IProportionalDock>,Rc<dyn IGlobalTarget>],
 constructors: [() => ProportionalDockReference::create], notify_property_changed: ProportionalDockReference,
 properties: [
  Orientation: Orientation { get: |this: &ProportionalDockReference| this.model.orientation(), set: |this: &ProportionalDockReference, value| this.model.set_orientation(value) },
 ],
});
pub(crate) fn register() {
    MarkupType::register(ProportionalDockReference::MARKUP);
    ValueTypes::register_nullable::<Rc<dyn IProportionalDock>>();
    ValueTypes::register_cast::<Rc<dyn IProportionalDock>, Rc<dyn IDockable>>(|model| {
        model.clone()
    });
    ValueTypes::register_cast::<Rc<dyn IProportionalDock>, Rc<dyn IDock>>(|model| model.clone());
    ValueTypes::register_nullable::<ProportionalDockReference>();
    ValueTypes::register_cast::<ProportionalDockReference, super::DockBaseReference>(|value| {
        value.base()
    });
    ValueTypes::register_cast::<ProportionalDockReference, super::DockableBaseReference>(|value| {
        value.base().base()
    });
    ValueTypes::register_cast::<ProportionalDockReference, super::ReactiveBaseReference>(|value| {
        value.base().base().base()
    });
    ValueTypes::register_cast::<ProportionalDockReference, super::ReactiveObjectReference>(
        |value| value.base().base().base().base(),
    );
    ValueTypes::register_cast::<ProportionalDockReference, Rc<dyn IDockable>>(|value| {
        value.model()
    });
    ValueTypes::register_cast::<ProportionalDockReference, Rc<dyn IDock>>(|value| value.model());
    ValueTypes::register_cast::<ProportionalDockReference, Rc<dyn IProportionalDock>>(|value| {
        value.model()
    });
    ValueTypes::register_cast::<ProportionalDockReference, Rc<dyn IGlobalTarget>>(|value| {
        value.model()
    });
    ValueTypes::register_reference_identity::<ProportionalDockReference>(|value| {
        Rc::as_ptr(&value.model()) as *const ()
    });
}
