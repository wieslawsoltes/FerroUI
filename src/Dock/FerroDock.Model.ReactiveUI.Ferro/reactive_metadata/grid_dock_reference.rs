// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::DockBaseReference;
use ferrodock_model::{
    controls::IGridDock,
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
/// Identity-preserving native class view of the original GridDock model.
#[derive(Clone)]
pub struct GridDockReference {
    model: Rc<dyn IGridDock>,
    base: DockBaseReference,
}
impl GridDockReference {
    /// Constructs the original concrete model, preserving its source defaults and behavior.
    pub fn create() -> Self {
        Self::new(ferrodock_model_reactiveui::controls::GridDock::new())
    }
    /// Views an original model or composed source subclass without copying its state.
    pub fn new<T: IGridDock + IReactiveObject>(model: Rc<T>) -> Self {
        Self {
            base: DockBaseReference::new(model.clone()),
            model,
        }
    }
    /// Gets the same enclosing source model through its original contract.
    pub fn model(&self) -> Rc<dyn IGridDock> {
        self.model.clone()
    }
    /// Gets the inherited source class view of the same enclosing object.
    pub fn base(&self) -> DockBaseReference {
        self.base.clone()
    }
}
impl PartialEq for GridDockReference {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.model, &other.model)
    }
}
impl INotifyPropertyChanged for GridDockReference {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
    fn as_notify_property_changing(&self) -> Option<&dyn INotifyPropertyChanging> {
        Some(self)
    }
}
impl INotifyPropertyChanging for GridDockReference {
    fn property_changing(&self) -> &Event<str> {
        self.base.property_changing()
    }
}
ferroui_base::ferro_markup_type!(class GridDockReference as "GridDock" {
 namespace: "FerroDock.Model.ReactiveUI.Controls", this: GridDockReference,
 handles: [GridDockReference, Option<GridDockReference>], base: DockBaseReference,
 interfaces: [Rc<dyn IGridDock>],
 constructors: [() => GridDockReference::create], notify_property_changed: GridDockReference,
 properties: [
  ColumnDefinitions: Option<String> { get: |this: &GridDockReference| this.model.column_definitions(), set: |this: &GridDockReference, value| this.model.set_column_definitions(value) },
  RowDefinitions: Option<String> { get: |this: &GridDockReference| this.model.row_definitions(), set: |this: &GridDockReference, value| this.model.set_row_definitions(value) },
 ],
});
pub(crate) fn register() {
    MarkupType::register(GridDockReference::MARKUP);
    ValueTypes::register_nullable::<Rc<dyn IGridDock>>();
    ValueTypes::register_cast::<Rc<dyn IGridDock>, Rc<dyn IDockable>>(|model| model.clone());
    ValueTypes::register_cast::<Rc<dyn IGridDock>, Rc<dyn IDock>>(|model| model.clone());
    ValueTypes::register_nullable::<GridDockReference>();
    ValueTypes::register_cast::<GridDockReference, super::DockBaseReference>(|value| value.base());
    ValueTypes::register_cast::<GridDockReference, super::DockableBaseReference>(|value| {
        value.base().base()
    });
    ValueTypes::register_cast::<GridDockReference, super::ReactiveBaseReference>(|value| {
        value.base().base().base()
    });
    ValueTypes::register_cast::<GridDockReference, super::ReactiveObjectReference>(|value| {
        value.base().base().base().base()
    });
    ValueTypes::register_cast::<GridDockReference, Rc<dyn IDockable>>(|value| value.model());
    ValueTypes::register_cast::<GridDockReference, Rc<dyn IDock>>(|value| value.model());
    ValueTypes::register_cast::<GridDockReference, Rc<dyn IGridDock>>(|value| value.model());
    ValueTypes::register_reference_identity::<GridDockReference>(|value| {
        Rc::as_ptr(&value.model()) as *const ()
    });
}
