use ferrodock_model::{
    controls::{IMdiDocument, ITool},
    core::{DockRect, IDockable, IDockingWindowState, MdiWindowState},
};
// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::DockableBaseReference;
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
/// Native reference to the original Tool class and its inherited source state.
#[derive(Clone)]
pub struct ToolReference {
    base: DockableBaseReference,
}
impl ToolReference {
    /// Constructs the source class and exposes the same original engine instance.
    pub fn create() -> Self {
        Self::new(ferrodock_model_reactiveui::controls::Tool::new())
    }

    /// Creates the original class view with the same enclosing reactive identity.
    pub fn new<T: ITool + IMdiDocument + IDockingWindowState + IReactiveObject>(
        model: Rc<T>,
    ) -> Self {
        Self {
            base: DockableBaseReference::new(model),
        }
    }
    /// Gets the source base-class reference.
    pub fn base(&self) -> DockableBaseReference {
        self.base.clone()
    }
    /// Gets the original enclosing model.
    pub fn model(&self) -> Rc<dyn IDockable> {
        self.base.model()
    }
    fn mdi(&self) -> Rc<dyn IMdiDocument> {
        self.model()
            .as_mdi_document()
            .expect("source MDI class contract")
    }
}
impl PartialEq for ToolReference {
    fn eq(&self, other: &Self) -> bool {
        self.base == other.base
    }
}
impl INotifyPropertyChanged for ToolReference {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
    fn as_notify_property_changing(&self) -> Option<&dyn INotifyPropertyChanging> {
        Some(self)
    }
}
impl INotifyPropertyChanging for ToolReference {
    fn property_changing(&self) -> &Event<str> {
        self.base.property_changing()
    }
}
ferroui_base::ferro_markup_type!(class ToolReference as "Tool"{namespace:"FerroDock.Model.ReactiveUI.Controls",this:ToolReference,handles:[ToolReference,Option<ToolReference>],base:DockableBaseReference,interfaces:[Rc<dyn ITool>,Rc<dyn IMdiDocument>],constructors:[()=>ToolReference::create],notify_property_changed:ToolReference,properties:[
MdiBounds:DockRect{get:|this:&ToolReference|this.mdi().mdi_bounds(),set:|this:&ToolReference,value|this.mdi().set_mdi_bounds(value)},
MdiState:MdiWindowState{get:|this:&ToolReference|this.mdi().mdi_state(),set:|this:&ToolReference,value|this.mdi().set_mdi_state(value)},
MdiZIndex:i32{get:|this:&ToolReference|this.mdi().mdi_z_index(),set:|this:&ToolReference,value|this.mdi().set_mdi_z_index(value)},
IsOpen:bool{get:|this:&ToolReference|this.model().as_docking_window_state().expect("source lifecycle contract").is_open(),set:|this:&ToolReference,value|this.model().as_docking_window_state().expect("source lifecycle contract").set_is_open(value)},
IsActive:bool{get:|this:&ToolReference|this.model().as_docking_window_state().expect("source lifecycle contract").is_active(),set:|this:&ToolReference,value|this.model().as_docking_window_state().expect("source lifecycle contract").set_is_active(value)},
IsSelected:bool{get:|this:&ToolReference|this.model().as_docking_window_state().expect("source lifecycle contract").is_selected(),set:|this:&ToolReference,value|this.model().as_docking_window_state().expect("source lifecycle contract").set_is_selected(value)},
]});
pub(crate) fn register() {
    ValueTypes::register_cast::<ToolReference, super::ReactiveBaseReference>(|value| {
        value.base().base()
    });
    ValueTypes::register_cast::<ToolReference, super::ReactiveObjectReference>(|value| {
        value.base().base().base()
    });
    MarkupType::register(<ToolReference as MarkupTyped>::MARKUP);
    ValueTypes::register_nullable::<ToolReference>();
    ValueTypes::register_cast::<ToolReference, DockableBaseReference>(|value| value.base());
    ValueTypes::register_cast::<ToolReference, Rc<dyn IDockable>>(|value| value.model());
    ValueTypes::register_cast::<ToolReference, Rc<dyn ITool>>(|value| {
        value.model().as_tool().expect("source tool class contract")
    });
    ValueTypes::register_cast::<ToolReference, Rc<dyn IMdiDocument>>(|value| {
        value
            .model()
            .as_mdi_document()
            .expect("source MDI class contract")
    });
}
