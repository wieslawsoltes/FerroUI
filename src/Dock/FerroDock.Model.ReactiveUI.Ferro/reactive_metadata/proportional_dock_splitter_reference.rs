// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::DockableBaseReference;
use ferrodock_model::controls::ISplitter;
use ferrodock_model::{controls::IProportionalDockSplitter, core::IDockable};
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
/// Identity-preserving native class view of the original ProportionalDockSplitter model.
#[derive(Clone)]
pub struct ProportionalDockSplitterReference {
    model: Rc<dyn IProportionalDockSplitter>,
    base: DockableBaseReference,
}
impl ProportionalDockSplitterReference {
    /// Constructs the original concrete model, preserving its source defaults and behavior.
    pub fn create() -> Self {
        Self::new(ferrodock_model_reactiveui::controls::ProportionalDockSplitter::new())
    }
    /// Views an original model or composed source subclass without copying its state.
    pub fn new<T: IProportionalDockSplitter + IReactiveObject>(model: Rc<T>) -> Self {
        Self {
            base: DockableBaseReference::new(model.clone()),
            model,
        }
    }
    /// Gets the same enclosing source model through its original contract.
    pub fn model(&self) -> Rc<dyn IProportionalDockSplitter> {
        self.model.clone()
    }
    /// Gets the inherited source class view of the same enclosing object.
    pub fn base(&self) -> DockableBaseReference {
        self.base.clone()
    }
}
impl PartialEq for ProportionalDockSplitterReference {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.model, &other.model)
    }
}
impl INotifyPropertyChanged for ProportionalDockSplitterReference {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
    fn as_notify_property_changing(&self) -> Option<&dyn INotifyPropertyChanging> {
        Some(self)
    }
}
impl INotifyPropertyChanging for ProportionalDockSplitterReference {
    fn property_changing(&self) -> &Event<str> {
        self.base.property_changing()
    }
}
ferroui_base::ferro_markup_type!(class ProportionalDockSplitterReference as "ProportionalDockSplitter" {
 namespace: "FerroDock.Model.ReactiveUI.Controls", this: ProportionalDockSplitterReference,
 handles: [ProportionalDockSplitterReference, Option<ProportionalDockSplitterReference>], base: DockableBaseReference,
 interfaces: [Rc<dyn IProportionalDockSplitter>,Rc<dyn ISplitter>],
 constructors: [() => ProportionalDockSplitterReference::create], notify_property_changed: ProportionalDockSplitterReference,
 properties: [
  CanResize: bool { get: |this: &ProportionalDockSplitterReference| this.model.can_resize(), set: |this: &ProportionalDockSplitterReference, value| this.model.set_can_resize(value) },
  ResizePreview: bool { get: |this: &ProportionalDockSplitterReference| this.model.resize_preview(), set: |this: &ProportionalDockSplitterReference, value| this.model.set_resize_preview(value) },
 ],
});
pub(crate) fn register() {
    MarkupType::register(ProportionalDockSplitterReference::MARKUP);
    ValueTypes::register_nullable::<Rc<dyn IProportionalDockSplitter>>();
    ValueTypes::register_cast::<Rc<dyn IProportionalDockSplitter>, Rc<dyn IDockable>>(|model| {
        model.clone()
    });
    ValueTypes::register_nullable::<ProportionalDockSplitterReference>();
    ValueTypes::register_cast::<ProportionalDockSplitterReference, super::DockableBaseReference>(
        |value| value.base(),
    );
    ValueTypes::register_cast::<ProportionalDockSplitterReference, super::ReactiveBaseReference>(
        |value| value.base().base(),
    );
    ValueTypes::register_cast::<ProportionalDockSplitterReference, super::ReactiveObjectReference>(
        |value| value.base().base().base(),
    );
    ValueTypes::register_cast::<ProportionalDockSplitterReference, Rc<dyn IDockable>>(|value| {
        value.model()
    });
    ValueTypes::register_cast::<ProportionalDockSplitterReference, Rc<dyn IProportionalDockSplitter>>(
        |value| value.model(),
    );
    ValueTypes::register_cast::<ProportionalDockSplitterReference, Rc<dyn ISplitter>>(|value| {
        value.model()
    });
    ValueTypes::register_reference_identity::<ProportionalDockSplitterReference>(|value| {
        Rc::as_ptr(&value.model()) as *const ()
    });
}
