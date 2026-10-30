// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::DockBaseReference;
use ferrodock_model::{
    controls::IUniformGridDock,
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
/// Identity-preserving native class view of the original UniformGridDock model.
#[derive(Clone)]
pub struct UniformGridDockReference {
    model: Rc<dyn IUniformGridDock>,
    base: DockBaseReference,
}
impl UniformGridDockReference {
    /// Constructs the original concrete model, preserving its source defaults and behavior.
    pub fn create() -> Self {
        Self::new(ferrodock_model_reactiveui::controls::UniformGridDock::new())
    }
    /// Views an original model or composed source subclass without copying its state.
    pub fn new<T: IUniformGridDock + IReactiveObject>(model: Rc<T>) -> Self {
        Self {
            base: DockBaseReference::new(model.clone()),
            model,
        }
    }
    /// Gets the same enclosing source model through its original contract.
    pub fn model(&self) -> Rc<dyn IUniformGridDock> {
        self.model.clone()
    }
    /// Gets the inherited source class view of the same enclosing object.
    pub fn base(&self) -> DockBaseReference {
        self.base.clone()
    }
}
impl PartialEq for UniformGridDockReference {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.model, &other.model)
    }
}
impl INotifyPropertyChanged for UniformGridDockReference {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
    fn as_notify_property_changing(&self) -> Option<&dyn INotifyPropertyChanging> {
        Some(self)
    }
}
impl INotifyPropertyChanging for UniformGridDockReference {
    fn property_changing(&self) -> &Event<str> {
        self.base.property_changing()
    }
}
ferroui_base::ferro_markup_type!(class UniformGridDockReference as "UniformGridDock" {
 namespace: "FerroDock.Model.ReactiveUI.Controls", this: UniformGridDockReference,
 handles: [UniformGridDockReference, Option<UniformGridDockReference>], base: DockBaseReference,
 interfaces: [Rc<dyn IUniformGridDock>],
 constructors: [() => UniformGridDockReference::create], notify_property_changed: UniformGridDockReference,
 properties: [
  Rows: i32 { get: |this: &UniformGridDockReference| this.model.rows(), set: |this: &UniformGridDockReference, value| this.model.set_rows(value) },
  Columns: i32 { get: |this: &UniformGridDockReference| this.model.columns(), set: |this: &UniformGridDockReference, value| this.model.set_columns(value) },
 ],
});
pub(crate) fn register() {
    MarkupType::register(UniformGridDockReference::MARKUP);
    ValueTypes::register_nullable::<Rc<dyn IUniformGridDock>>();
    ValueTypes::register_cast::<Rc<dyn IUniformGridDock>, Rc<dyn IDockable>>(|model| model.clone());
    ValueTypes::register_cast::<Rc<dyn IUniformGridDock>, Rc<dyn IDock>>(|model| model.clone());
    ValueTypes::register_nullable::<UniformGridDockReference>();
    ValueTypes::register_cast::<UniformGridDockReference, super::DockBaseReference>(|value| {
        value.base()
    });
    ValueTypes::register_cast::<UniformGridDockReference, super::DockableBaseReference>(|value| {
        value.base().base()
    });
    ValueTypes::register_cast::<UniformGridDockReference, super::ReactiveBaseReference>(|value| {
        value.base().base().base()
    });
    ValueTypes::register_cast::<UniformGridDockReference, super::ReactiveObjectReference>(
        |value| value.base().base().base().base(),
    );
    ValueTypes::register_cast::<UniformGridDockReference, Rc<dyn IDockable>>(|value| value.model());
    ValueTypes::register_cast::<UniformGridDockReference, Rc<dyn IDock>>(|value| value.model());
    ValueTypes::register_cast::<UniformGridDockReference, Rc<dyn IUniformGridDock>>(|value| {
        value.model()
    });
    ValueTypes::register_reference_identity::<UniformGridDockReference>(|value| {
        Rc::as_ptr(&value.model()) as *const ()
    });
}
