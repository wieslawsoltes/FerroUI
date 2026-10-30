// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::DockBaseReference;
use ferrodock_model::core::Orientation;
use ferrodock_model::{
    controls::IWrapDock,
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
/// Identity-preserving native class view of the original WrapDock model.
#[derive(Clone)]
pub struct WrapDockReference {
    model: Rc<dyn IWrapDock>,
    base: DockBaseReference,
}
impl WrapDockReference {
    /// Constructs the original concrete model, preserving its source defaults and behavior.
    pub fn create() -> Self {
        Self::new(ferrodock_model_reactiveui::controls::WrapDock::new())
    }
    /// Views an original model or composed source subclass without copying its state.
    pub fn new<T: IWrapDock + IReactiveObject>(model: Rc<T>) -> Self {
        Self {
            base: DockBaseReference::new(model.clone()),
            model,
        }
    }
    /// Gets the same enclosing source model through its original contract.
    pub fn model(&self) -> Rc<dyn IWrapDock> {
        self.model.clone()
    }
    /// Gets the inherited source class view of the same enclosing object.
    pub fn base(&self) -> DockBaseReference {
        self.base.clone()
    }
}
impl PartialEq for WrapDockReference {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.model, &other.model)
    }
}
impl INotifyPropertyChanged for WrapDockReference {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
    fn as_notify_property_changing(&self) -> Option<&dyn INotifyPropertyChanging> {
        Some(self)
    }
}
impl INotifyPropertyChanging for WrapDockReference {
    fn property_changing(&self) -> &Event<str> {
        self.base.property_changing()
    }
}
ferroui_base::ferro_markup_type!(class WrapDockReference as "WrapDock" {
 namespace: "FerroDock.Model.ReactiveUI.Controls", this: WrapDockReference,
 handles: [WrapDockReference, Option<WrapDockReference>], base: DockBaseReference,
 interfaces: [Rc<dyn IWrapDock>],
 constructors: [() => WrapDockReference::create], notify_property_changed: WrapDockReference,
 properties: [
  Orientation: Orientation { get: |this: &WrapDockReference| this.model.orientation(), set: |this: &WrapDockReference, value| this.model.set_orientation(value) },
 ],
});
pub(crate) fn register() {
    MarkupType::register(WrapDockReference::MARKUP);
    ValueTypes::register_nullable::<Rc<dyn IWrapDock>>();
    ValueTypes::register_cast::<Rc<dyn IWrapDock>, Rc<dyn IDockable>>(|model| model.clone());
    ValueTypes::register_cast::<Rc<dyn IWrapDock>, Rc<dyn IDock>>(|model| model.clone());
    ValueTypes::register_nullable::<WrapDockReference>();
    ValueTypes::register_cast::<WrapDockReference, super::DockBaseReference>(|value| value.base());
    ValueTypes::register_cast::<WrapDockReference, super::DockableBaseReference>(|value| {
        value.base().base()
    });
    ValueTypes::register_cast::<WrapDockReference, super::ReactiveBaseReference>(|value| {
        value.base().base().base()
    });
    ValueTypes::register_cast::<WrapDockReference, super::ReactiveObjectReference>(|value| {
        value.base().base().base().base()
    });
    ValueTypes::register_cast::<WrapDockReference, Rc<dyn IDockable>>(|value| value.model());
    ValueTypes::register_cast::<WrapDockReference, Rc<dyn IDock>>(|value| value.model());
    ValueTypes::register_cast::<WrapDockReference, Rc<dyn IWrapDock>>(|value| value.model());
    ValueTypes::register_reference_identity::<WrapDockReference>(|value| {
        Rc::as_ptr(&value.model()) as *const ()
    });
}
