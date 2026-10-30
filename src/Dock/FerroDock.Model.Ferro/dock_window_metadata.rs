// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::{
    controls::IRootDock,
    core::{DockWindowOwnerMode, DockWindowState, IDockWindow, IDockable, IFactory, IHostWindow},
};
use ferroui_base::{
    metadata::{MarkupType, MarkupTyped},
    AnyValue,
};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::rc::Rc;
/// Native descriptor retaining the original dock-window model and notifier.
#[doc(hidden)]
pub trait IDockWindowMetadata {}
ferroui_base::ferro_markup_type!(interface dyn IDockWindowMetadata as "IDockWindow"{
    namespace:"FerroDock.Model.Core",this:Rc<dyn IDockWindow>,handles:[Rc<dyn IDockWindow>,Option<Rc<dyn IDockWindow>>],
    properties:[
        Id:String{get:|this:&Rc<dyn IDockWindow>|this.id(),set:|this:&Rc<dyn IDockWindow>,value:String|this.set_id(value)},
        X:f64{get:|this:&Rc<dyn IDockWindow>|this.x(),set:|this:&Rc<dyn IDockWindow>,value:f64|this.set_x(value)},
        Y:f64{get:|this:&Rc<dyn IDockWindow>|this.y(),set:|this:&Rc<dyn IDockWindow>,value:f64|this.set_y(value)},
        Width:f64{get:|this:&Rc<dyn IDockWindow>|this.width(),set:|this:&Rc<dyn IDockWindow>,value:f64|this.set_width(value)},
        Height:f64{get:|this:&Rc<dyn IDockWindow>|this.height(),set:|this:&Rc<dyn IDockWindow>,value:f64|this.set_height(value)},
        WindowState:DockWindowState{get:|this:&Rc<dyn IDockWindow>|this.window_state(),set:|this:&Rc<dyn IDockWindow>,value:DockWindowState|this.set_window_state(value)},
        Topmost:bool{get:|this:&Rc<dyn IDockWindow>|this.topmost(),set:|this:&Rc<dyn IDockWindow>,value:bool|this.set_topmost(value)},
        Title:String{get:|this:&Rc<dyn IDockWindow>|this.title(),set:|this:&Rc<dyn IDockWindow>,value:String|this.set_title(value)},
        OwnerMode:DockWindowOwnerMode{get:|this:&Rc<dyn IDockWindow>|this.owner_mode(),set:|this:&Rc<dyn IDockWindow>,value:DockWindowOwnerMode|this.set_owner_mode(value)},
        ParentWindow:Option<Rc<dyn IDockWindow>>{get:|this:&Rc<dyn IDockWindow>|this.parent_window(),set:|this:&Rc<dyn IDockWindow>,value:Option<Rc<dyn IDockWindow>>|this.set_parent_window(value)},
        IsModal:bool{get:|this:&Rc<dyn IDockWindow>|this.is_modal(),set:|this:&Rc<dyn IDockWindow>,value:bool|this.set_is_modal(value)},
        ShowInTaskbar:Option<bool>{get:|this:&Rc<dyn IDockWindow>|this.show_in_taskbar(),set:|this:&Rc<dyn IDockWindow>,value:Option<bool>|this.set_show_in_taskbar(value)},
        Owner:Option<Rc<dyn IDockable>>{get:|this:&Rc<dyn IDockWindow>|this.owner(),set:|this:&Rc<dyn IDockWindow>,value:Option<Rc<dyn IDockable>>|this.set_owner(value)},
        Factory:Option<Rc<dyn IFactory>>{get:|this:&Rc<dyn IDockWindow>|this.factory(),set:|this:&Rc<dyn IDockWindow>,value:Option<Rc<dyn IFactory>>|this.set_factory(value)},
        Layout:Option<Rc<dyn IRootDock>>{get:|this:&Rc<dyn IDockWindow>|this.layout(),set:|this:&Rc<dyn IDockWindow>,value:Option<Rc<dyn IRootDock>>|this.set_layout(value)},
        Host:Option<Rc<dyn IHostWindow>>{get:|this:&Rc<dyn IDockWindow>|this.host(),set:|this:&Rc<dyn IDockWindow>,value:Option<Rc<dyn IHostWindow>>|this.set_host(value)},
    ],
    methods:[
        fn OnClose()->bool => (|this:&Rc<dyn IDockWindow>|this.on_close()),
        fn OnMoveDragBegin()->bool => (|this:&Rc<dyn IDockWindow>|this.on_move_drag_begin()),
        fn OnMoveDrag() => (|this:&Rc<dyn IDockWindow>|this.on_move_drag()),
        fn OnMoveDragEnd() => (|this:&Rc<dyn IDockWindow>|this.on_move_drag_end()),
        fn Save() => (|this:&Rc<dyn IDockWindow>|this.save()),
        fn Present(bool) => (|this:&Rc<dyn IDockWindow>,dialog|this.present(dialog)),
        fn Exit() => (|this:&Rc<dyn IDockWindow>|this.exit()),
        fn SetActive() => (|this:&Rc<dyn IDockWindow>|this.set_active()),
    ],
});
static METADATA: MarkupType = MarkupType {
    notify_property_changed: Some(notifier),
    ..*<dyn IDockWindowMetadata as MarkupTyped>::MARKUP
};
fn notifier(value: &dyn AnyValue) -> Option<&dyn INotifyPropertyChanged> {
    if let Some(model) = value.downcast_ref::<Rc<dyn IDockWindow>>() {
        return model.as_notify_property_changed();
    }
    value
        .downcast_ref::<Option<Rc<dyn IDockWindow>>>()?
        .as_ref()?
        .as_notify_property_changed()
}
pub(crate) fn register() {
    MarkupType::register(&METADATA);
    // The notifier descriptor is the registered identity; emitted members still
    // belong to the original public declaration carrier.
    ferroui_base::__ferro_compiler_metadata! {
        MarkupType::register_rust_paths(&[(&METADATA, "::ferrodock_model_ferro::IDockWindowMetadata", true)]);
    }
}
