// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::DockBaseReference;
use ferrodock_model::core::ILocalTarget;
use ferrodock_model::core::{Alignment, GripMode};
use ferrodock_model::{
    controls::IToolDock,
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
/// Identity-preserving native class view of the original ToolDock model.
#[derive(Clone)]
pub struct ToolDockReference {
    model: Rc<dyn IToolDock>,
    base: DockBaseReference,
}
impl ToolDockReference {
    /// Constructs the original concrete model, preserving its source defaults and behavior.
    pub fn create() -> Self {
        Self::new(ferrodock_model_reactiveui::controls::ToolDock::new())
    }
    /// Views an original model or composed source subclass without copying its state.
    pub fn new<T: IToolDock + IReactiveObject>(model: Rc<T>) -> Self {
        Self {
            base: DockBaseReference::new(model.clone()),
            model,
        }
    }
    /// Gets the same enclosing source model through its original contract.
    pub fn model(&self) -> Rc<dyn IToolDock> {
        self.model.clone()
    }
    /// Gets the inherited source class view of the same enclosing object.
    pub fn base(&self) -> DockBaseReference {
        self.base.clone()
    }
}
impl PartialEq for ToolDockReference {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.model, &other.model)
    }
}
impl INotifyPropertyChanged for ToolDockReference {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
    fn as_notify_property_changing(&self) -> Option<&dyn INotifyPropertyChanging> {
        Some(self)
    }
}
impl INotifyPropertyChanging for ToolDockReference {
    fn property_changing(&self) -> &Event<str> {
        self.base.property_changing()
    }
}
ferroui_base::ferro_markup_type!(class ToolDockReference as "ToolDock" {
 namespace: "FerroDock.Model.ReactiveUI.Controls", this: ToolDockReference,
 handles: [ToolDockReference, Option<ToolDockReference>], base: DockBaseReference,
 interfaces: [Rc<dyn IToolDock>,Rc<dyn ILocalTarget>],
 constructors: [() => ToolDockReference::create], notify_property_changed: ToolDockReference,
 properties: [
  Alignment: Alignment { get: |this: &ToolDockReference| this.model.alignment(), set: |this: &ToolDockReference, value| this.model.set_alignment(value) },
  IsExpanded: bool { get: |this: &ToolDockReference| this.model.is_expanded(), set: |this: &ToolDockReference, value| this.model.set_is_expanded(value) },
  AutoHide: bool { get: |this: &ToolDockReference| this.model.auto_hide(), set: |this: &ToolDockReference, value| this.model.set_auto_hide(value) },
  GripMode: GripMode { get: |this: &ToolDockReference| this.model.grip_mode(), set: |this: &ToolDockReference, value| this.model.set_grip_mode(value) },
 ],
 methods: [fn AddTool(Option<Rc<dyn IDockable>>) => (|this: &ToolDockReference, tool| this.model.add_tool(tool))],
});
pub(crate) fn register() {
    MarkupType::register(ToolDockReference::MARKUP);
    ValueTypes::register_nullable::<Rc<dyn IToolDock>>();
    ValueTypes::register_cast::<Rc<dyn IToolDock>, Rc<dyn IDockable>>(|model| model.clone());
    ValueTypes::register_cast::<Rc<dyn IToolDock>, Rc<dyn IDock>>(|model| model.clone());
    ValueTypes::register_nullable::<ToolDockReference>();
    ValueTypes::register_cast::<ToolDockReference, super::DockBaseReference>(|value| value.base());
    ValueTypes::register_cast::<ToolDockReference, super::DockableBaseReference>(|value| {
        value.base().base()
    });
    ValueTypes::register_cast::<ToolDockReference, super::ReactiveBaseReference>(|value| {
        value.base().base().base()
    });
    ValueTypes::register_cast::<ToolDockReference, super::ReactiveObjectReference>(|value| {
        value.base().base().base().base()
    });
    ValueTypes::register_cast::<ToolDockReference, Rc<dyn IDockable>>(|value| value.model());
    ValueTypes::register_cast::<ToolDockReference, Rc<dyn IDock>>(|value| value.model());
    ValueTypes::register_cast::<ToolDockReference, Rc<dyn IToolDock>>(|value| value.model());
    ValueTypes::register_cast::<ToolDockReference, Rc<dyn ILocalTarget>>(|value| value.model());
    ValueTypes::register_reference_identity::<ToolDockReference>(|value| {
        Rc::as_ptr(&value.model()) as *const ()
    });
}
