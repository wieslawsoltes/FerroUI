// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Core host contracts dispatch directly to the original host implementation.
use ferrodock_model::core::{
    DockWindowState, IDock, IDockManagerState, IDockWindow, IHostWindow, IHostWindowState,
};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
use std::rc::Rc;
/// Descriptor of the source host-window contract, independent of native controls.
#[doc(hidden)]
pub trait IHostWindowMetadata {}
/// Descriptor of the source host-state marker contract.
#[doc(hidden)]
pub trait IHostWindowStateMetadata {}
/// Descriptor of the source manager-state marker contract.
#[doc(hidden)]
pub trait IDockManagerStateMetadata {}

ferroui_base::ferro_markup_type!(interface dyn IHostWindowMetadata as "IHostWindow" {
    namespace:"FerroDock.Model.Core",this:Rc<dyn IHostWindow>,handles:[Rc<dyn IHostWindow>,Option<Rc<dyn IHostWindow>>],
    properties:[
        HostWindowState:Option<Rc<dyn IHostWindowState>>{get:|this:&Rc<dyn IHostWindow>|this.host_window_state()},
        IsTracked:bool{get:|this:&Rc<dyn IHostWindow>|this.is_tracked(),set:|this:&Rc<dyn IHostWindow>,value|this.set_is_tracked(value)},
        Window:Option<Rc<dyn IDockWindow>>{get:|this:&Rc<dyn IHostWindow>|this.window(),set:|this:&Rc<dyn IHostWindow>,value|this.set_window(value)},
    ],
    methods:[
        fn Present(bool) => (|this:&Rc<dyn IHostWindow>,dialog|this.present(dialog)),
        fn Exit() => (|this:&Rc<dyn IHostWindow>|this.exit()),
        fn SetPosition(f64,f64) => (|this:&Rc<dyn IHostWindow>,x,y|this.set_position(x,y)),
        fn SetSize(f64,f64) => (|this:&Rc<dyn IHostWindow>,width,height|this.set_size(width,height)),
        fn SetWindowState(DockWindowState) => (|this:&Rc<dyn IHostWindow>,state|this.set_window_state(state)),
        fn GetWindowState()->DockWindowState => (|this:&Rc<dyn IHostWindow>|this.get_window_state()),
        fn SetTitle(Option<String>) => (|this:&Rc<dyn IHostWindow>,title|this.set_title(title)),
        fn SetLayout(Rc<dyn IDock>) => (|this:&Rc<dyn IHostWindow>,layout|this.set_layout(layout)),
        fn SetActive() => (|this:&Rc<dyn IHostWindow>|this.set_active()),
    ],
});
// Original GetPosition/GetSize use out parameters. Their Rust trait adaptations
// remain callable, but do not publish a different signature as source metadata.
ferroui_base::ferro_markup_type!(interface dyn IDockManagerStateMetadata as "IDockManagerState" {
    namespace:"FerroDock.Model.Core",handles:[Rc<dyn IDockManagerState>,Option<Rc<dyn IDockManagerState>>],
});
ferroui_base::ferro_markup_type!(interface dyn IHostWindowStateMetadata as "IHostWindowState" {
    namespace:"FerroDock.Model.Core",handles:[Rc<dyn IHostWindowState>,Option<Rc<dyn IHostWindowState>>],base:Rc<dyn IDockManagerState>,interfaces:[Rc<dyn IDockManagerState>],
});

pub(crate) fn register() {
    MarkupType::register_all(&[
        <dyn IHostWindowMetadata>::MARKUP,
        <dyn IHostWindowStateMetadata>::MARKUP,
        <dyn IDockManagerStateMetadata>::MARKUP,
    ]);
    ValueTypes::register_nullable::<Rc<dyn IHostWindow>>();
    ValueTypes::register_nullable::<Rc<dyn IHostWindowState>>();
    ValueTypes::register_nullable::<Rc<dyn IDockManagerState>>();
    ValueTypes::register_reference_identity::<Rc<dyn IHostWindow>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_reference_identity::<Rc<dyn IHostWindowState>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_reference_identity::<Rc<dyn IDockManagerState>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_cast::<Rc<dyn IHostWindowState>, Rc<dyn IDockManagerState>>(|value| {
        value.clone()
    });
    ferrodock_runtime::register_value_type::<Rc<dyn IHostWindow>>();
    ferrodock_runtime::register_value_type::<Rc<dyn IHostWindowState>>();
    ferrodock_runtime::register_value_type::<Rc<dyn IDockManagerState>>();
}
