use ferrodock_model::{
    controls::{IDocument, IMdiDocument},
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
/// Native reference to the original Document class and its inherited source state.
#[derive(Clone)]
pub struct DocumentReference {
    base: DockableBaseReference,
}
impl DocumentReference {
    /// Constructs the source class and exposes the same original engine instance.
    pub fn create() -> Self {
        Self::new(ferrodock_model_reactiveui::controls::Document::new())
    }

    /// Creates the original class view with the same enclosing reactive identity.
    pub fn new<T: IDocument + IMdiDocument + IDockingWindowState + IReactiveObject>(
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
impl PartialEq for DocumentReference {
    fn eq(&self, other: &Self) -> bool {
        self.base == other.base
    }
}
impl INotifyPropertyChanged for DocumentReference {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
    fn as_notify_property_changing(&self) -> Option<&dyn INotifyPropertyChanging> {
        Some(self)
    }
}
impl INotifyPropertyChanging for DocumentReference {
    fn property_changing(&self) -> &Event<str> {
        self.base.property_changing()
    }
}
ferroui_base::ferro_markup_type!(class DocumentReference as "Document"{namespace:"FerroDock.Model.ReactiveUI.Controls",this:DocumentReference,handles:[DocumentReference,Option<DocumentReference>],base:DockableBaseReference,interfaces:[Rc<dyn IDocument>,Rc<dyn IMdiDocument>],constructors:[()=>DocumentReference::create],notify_property_changed:DocumentReference,properties:[
    MdiBounds:DockRect{get:|this:&DocumentReference|this.mdi().mdi_bounds(),set:|this:&DocumentReference,value|this.mdi().set_mdi_bounds(value)},
    MdiState:MdiWindowState{get:|this:&DocumentReference|this.mdi().mdi_state(),set:|this:&DocumentReference,value|this.mdi().set_mdi_state(value)},
    MdiZIndex:i32{get:|this:&DocumentReference|this.mdi().mdi_z_index(),set:|this:&DocumentReference,value|this.mdi().set_mdi_z_index(value)},
    IsOpen:bool{get:|this:&DocumentReference|this.model().as_docking_window_state().expect("source lifecycle contract").is_open(),set:|this:&DocumentReference,value|this.model().as_docking_window_state().expect("source lifecycle contract").set_is_open(value)} [IgnoreDataMember],
    IsActive:bool{get:|this:&DocumentReference|this.model().as_docking_window_state().expect("source lifecycle contract").is_active(),set:|this:&DocumentReference,value|this.model().as_docking_window_state().expect("source lifecycle contract").set_is_active(value)} [IgnoreDataMember],
    IsSelected:bool{get:|this:&DocumentReference|this.model().as_docking_window_state().expect("source lifecycle contract").is_selected(),set:|this:&DocumentReference,value|this.model().as_docking_window_state().expect("source lifecycle contract").set_is_selected(value)} [IgnoreDataMember],
]});
pub(crate) fn register() {
    ValueTypes::register_cast::<DocumentReference, super::ReactiveBaseReference>(|value| {
        value.base().base()
    });
    ValueTypes::register_cast::<DocumentReference, super::ReactiveObjectReference>(|value| {
        value.base().base().base()
    });
    MarkupType::register(<DocumentReference as MarkupTyped>::MARKUP);
    ValueTypes::register_nullable::<DocumentReference>();
    ValueTypes::register_cast::<DocumentReference, DockableBaseReference>(|value| value.base());
    ValueTypes::register_cast::<DocumentReference, Rc<dyn IDockable>>(|value| value.model());
    ValueTypes::register_cast::<DocumentReference, Rc<dyn IDocument>>(|value| {
        value
            .model()
            .as_document()
            .expect("source document class contract")
    });
    ValueTypes::register_cast::<DocumentReference, Rc<dyn IMdiDocument>>(|value| {
        value
            .model()
            .as_mdi_document()
            .expect("source MDI class contract")
    });
}
