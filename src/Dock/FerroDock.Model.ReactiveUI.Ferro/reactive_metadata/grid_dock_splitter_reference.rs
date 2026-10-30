// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::DockableBaseReference;
use ferrodock_model::controls::ISplitter;
use ferrodock_model::core::GridResizeDirection;
use ferrodock_model::{controls::IGridDockSplitter, core::IDockable};
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
/// Identity-preserving native class view of the original GridDockSplitter model.
#[derive(Clone)]
pub struct GridDockSplitterReference {
    model: Rc<dyn IGridDockSplitter>,
    base: DockableBaseReference,
}
impl GridDockSplitterReference {
    /// Constructs the original concrete model, preserving its source defaults and behavior.
    pub fn create() -> Self {
        Self::new(ferrodock_model_reactiveui::controls::GridDockSplitter::new())
    }
    /// Views an original model or composed source subclass without copying its state.
    pub fn new<T: IGridDockSplitter + IReactiveObject>(model: Rc<T>) -> Self {
        Self {
            base: DockableBaseReference::new(model.clone()),
            model,
        }
    }
    /// Gets the same enclosing source model through its original contract.
    pub fn model(&self) -> Rc<dyn IGridDockSplitter> {
        self.model.clone()
    }
    /// Gets the inherited source class view of the same enclosing object.
    pub fn base(&self) -> DockableBaseReference {
        self.base.clone()
    }
}
impl PartialEq for GridDockSplitterReference {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.model, &other.model)
    }
}
impl INotifyPropertyChanged for GridDockSplitterReference {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
    fn as_notify_property_changing(&self) -> Option<&dyn INotifyPropertyChanging> {
        Some(self)
    }
}
impl INotifyPropertyChanging for GridDockSplitterReference {
    fn property_changing(&self) -> &Event<str> {
        self.base.property_changing()
    }
}
ferroui_base::ferro_markup_type!(class GridDockSplitterReference as "GridDockSplitter" {
 namespace: "FerroDock.Model.ReactiveUI.Controls", this: GridDockSplitterReference,
 handles: [GridDockSplitterReference, Option<GridDockSplitterReference>], base: DockableBaseReference,
 interfaces: [Rc<dyn IGridDockSplitter>,Rc<dyn ISplitter>],
 constructors: [() => GridDockSplitterReference::create], notify_property_changed: GridDockSplitterReference,
 properties: [
  ResizeDirection: GridResizeDirection { get: |this: &GridDockSplitterReference| this.model.resize_direction(), set: |this: &GridDockSplitterReference, value| this.model.set_resize_direction(value) },
 ],
});
pub(crate) fn register() {
    MarkupType::register(GridDockSplitterReference::MARKUP);
    ValueTypes::register_nullable::<Rc<dyn IGridDockSplitter>>();
    ValueTypes::register_cast::<Rc<dyn IGridDockSplitter>, Rc<dyn IDockable>>(|model| {
        model.clone()
    });
    ValueTypes::register_nullable::<GridDockSplitterReference>();
    ValueTypes::register_cast::<GridDockSplitterReference, super::DockableBaseReference>(|value| {
        value.base()
    });
    ValueTypes::register_cast::<GridDockSplitterReference, super::ReactiveBaseReference>(|value| {
        value.base().base()
    });
    ValueTypes::register_cast::<GridDockSplitterReference, super::ReactiveObjectReference>(
        |value| value.base().base().base(),
    );
    ValueTypes::register_cast::<GridDockSplitterReference, Rc<dyn IDockable>>(|value| {
        value.model()
    });
    ValueTypes::register_cast::<GridDockSplitterReference, Rc<dyn IGridDockSplitter>>(|value| {
        value.model()
    });
    ValueTypes::register_cast::<GridDockSplitterReference, Rc<dyn ISplitter>>(|value| {
        value.model()
    });
    ValueTypes::register_reference_identity::<GridDockSplitterReference>(|value| {
        Rc::as_ptr(&value.model()) as *const ()
    });
}
