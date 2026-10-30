// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::ReactiveBaseReference;
use ferrodock_model::{
    controls::IRootDock,
    core::{DockWindowOwnerMode, DockWindowState, IDockWindow, IDockable, IFactory, IHostWindow},
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
/// Native class view retaining the original DockWindow and its enclosing reactive identity.
#[derive(Clone)]
pub struct DockWindowReference {
    model: Rc<dyn IDockWindow>,
    base: ReactiveBaseReference,
}
impl DockWindowReference {
    /// Constructs the original concrete source window model.
    pub fn create() -> Self {
        Self::new(ferrodock_model_reactiveui::core::DockWindow::new())
    }
    /// Views an original window or composed source subclass without copying its state.
    pub fn new<T: IDockWindow + IReactiveObject>(model: Rc<T>) -> Self {
        Self {
            base: ReactiveBaseReference::new(model.clone()),
            model,
        }
    }
    /// Gets the same original enclosing window through its source interface.
    pub fn model(&self) -> Rc<dyn IDockWindow> {
        self.model.clone()
    }
    /// Gets the source ReactiveBase view of the same object.
    pub fn base(&self) -> ReactiveBaseReference {
        self.base.clone()
    }
}
impl std::ops::Deref for DockWindowReference {
    type Target = dyn IDockWindow;
    fn deref(&self) -> &Self::Target {
        self.model.as_ref()
    }
}
impl PartialEq for DockWindowReference {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.model, &other.model)
    }
}
impl INotifyPropertyChanged for DockWindowReference {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
    fn as_notify_property_changing(&self) -> Option<&dyn INotifyPropertyChanging> {
        Some(self)
    }
}
impl INotifyPropertyChanging for DockWindowReference {
    fn property_changing(&self) -> &Event<str> {
        self.base.property_changing()
    }
}
ferroui_base::ferro_markup_type!(class DockWindowReference as "DockWindow"{
    namespace:"FerroDock.Model.ReactiveUI.Core",this:DockWindowReference,handles:[DockWindowReference,Option<DockWindowReference>],base:ReactiveBaseReference,interfaces:[Rc<dyn IDockWindow>],constructors:[()=>DockWindowReference::create],notify_property_changed:DockWindowReference,
    properties:[
        Id:String{get:|this:&DockWindowReference|this.id(),set:|this:&DockWindowReference,value:String|this.set_id(value)},
        X:f64{get:|this:&DockWindowReference|this.x(),set:|this:&DockWindowReference,value:f64|this.set_x(value)},
        Y:f64{get:|this:&DockWindowReference|this.y(),set:|this:&DockWindowReference,value:f64|this.set_y(value)},
        Width:f64{get:|this:&DockWindowReference|this.width(),set:|this:&DockWindowReference,value:f64|this.set_width(value)},
        Height:f64{get:|this:&DockWindowReference|this.height(),set:|this:&DockWindowReference,value:f64|this.set_height(value)},
        WindowState:DockWindowState{get:|this:&DockWindowReference|this.window_state(),set:|this:&DockWindowReference,value:DockWindowState|this.set_window_state(value)},
        Topmost:bool{get:|this:&DockWindowReference|this.topmost(),set:|this:&DockWindowReference,value:bool|this.set_topmost(value)},
        Title:String{get:|this:&DockWindowReference|this.title(),set:|this:&DockWindowReference,value:String|this.set_title(value)},
        OwnerMode:DockWindowOwnerMode{get:|this:&DockWindowReference|this.owner_mode(),set:|this:&DockWindowReference,value:DockWindowOwnerMode|this.set_owner_mode(value)},
        ParentWindow:Option<Rc<dyn IDockWindow>>{get:|this:&DockWindowReference|this.parent_window(),set:|this:&DockWindowReference,value:Option<Rc<dyn IDockWindow>>|this.set_parent_window(value)},
        IsModal:bool{get:|this:&DockWindowReference|this.is_modal(),set:|this:&DockWindowReference,value:bool|this.set_is_modal(value)},
        ShowInTaskbar:Option<bool>{get:|this:&DockWindowReference|this.show_in_taskbar(),set:|this:&DockWindowReference,value:Option<bool>|this.set_show_in_taskbar(value)},
        Owner:Option<Rc<dyn IDockable>>{get:|this:&DockWindowReference|this.owner(),set:|this:&DockWindowReference,value:Option<Rc<dyn IDockable>>|this.set_owner(value)} [IgnoreDataMember],
        Factory:Option<Rc<dyn IFactory>>{get:|this:&DockWindowReference|this.factory(),set:|this:&DockWindowReference,value:Option<Rc<dyn IFactory>>|this.set_factory(value)} [IgnoreDataMember],
        Layout:Option<Rc<dyn IRootDock>>{get:|this:&DockWindowReference|this.layout(),set:|this:&DockWindowReference,value:Option<Rc<dyn IRootDock>>|this.set_layout(value)},
        Host:Option<Rc<dyn IHostWindow>>{get:|this:&DockWindowReference|this.host(),set:|this:&DockWindowReference,value:Option<Rc<dyn IHostWindow>>|this.set_host(value)} [IgnoreDataMember],
    ],
    methods:[
        fn OnClose()->bool => (|this:&DockWindowReference|this.on_close()),
        fn OnMoveDragBegin()->bool => (|this:&DockWindowReference|this.on_move_drag_begin()),
        fn OnMoveDrag() => (|this:&DockWindowReference|this.on_move_drag()),
        fn OnMoveDragEnd() => (|this:&DockWindowReference|this.on_move_drag_end()),
        fn Save() => (|this:&DockWindowReference|this.save()),
        fn Present(bool) => (|this:&DockWindowReference,dialog|this.present(dialog)),
        fn Exit() => (|this:&DockWindowReference|this.exit()),
        fn SetActive() => (|this:&DockWindowReference|this.set_active()),
    ],
});
pub(crate) fn register() {
    MarkupType::register(DockWindowReference::MARKUP);
    ValueTypes::register_nullable::<DockWindowReference>();
    ValueTypes::register_nullable::<Rc<dyn IHostWindow>>();
    ValueTypes::register_cast::<DockWindowReference, Rc<dyn IDockWindow>>(|v| v.model());
    ValueTypes::register_cast::<DockWindowReference, ReactiveBaseReference>(|v| v.base());
    ValueTypes::register_cast::<DockWindowReference, super::ReactiveObjectReference>(|v| {
        v.base().base()
    });
    ValueTypes::register_reference_identity::<DockWindowReference>(|v| {
        Rc::as_ptr(&v.model()) as *const ()
    });
    ValueTypes::register_reference::<ferrodock_model_reactiveui::core::DockWindow>();
    ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::core::DockWindow>,
        DockWindowReference,
    >(|v| DockWindowReference::new(v.clone()));
    ValueTypes::register_cast::<
        Rc<ferrodock_model_reactiveui::core::DockWindow>,
        Rc<dyn IDockWindow>,
    >(|v| v.clone());
    ValueTypes::register_boxed_cast::<
        Rc<dyn IDockWindow>,
        Rc<ferrodock_model_reactiveui::core::DockWindow>,
    >(|v| {
        v.downcast_ref::<Rc<dyn IDockWindow>>()?
            .clone()
            .boxed_reference()
            .downcast_ref::<Rc<ferrodock_model_reactiveui::core::DockWindow>>()
            .cloned()
    });
    ValueTypes::register_boxed_cast::<Rc<dyn IDockWindow>, DockWindowReference>(|v| {
        let original = v
            .downcast_ref::<Rc<dyn IDockWindow>>()?
            .clone()
            .boxed_reference();
        if original.as_ref().type_id() == std::any::TypeId::of::<Rc<dyn IDockWindow>>() {
            return None;
        }
        let probe: ferroui_base::BoxedValue = Rc::new(super::model_probe::ModelProbe(original));
        ValueTypes::try_cast(
            &probe,
            ferroui_base::data::core::ValueType::of::<DockWindowReference>(),
        )?
        .downcast_ref::<DockWindowReference>()
        .cloned()
    });
    MarkupType::register_handle::<Rc<ferrodock_model_reactiveui::core::DockWindow>>(
        <DockWindowReference as MarkupTyped>::MARKUP,
    );
}
