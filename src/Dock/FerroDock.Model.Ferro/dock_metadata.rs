// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::{
    collections::ListHandle,
    core::{DockCapabilityPolicyImpl, IDock, IDockable},
};
use ferroui_base::{
    metadata::{MarkupType, MarkupTyped},
    AnyValue,
};
use ferroui_reactiveui::ICommand;
use ferroui_system::component_model::INotifyPropertyChanged;
use std::rc::Rc;
/// Descriptor carrier; values retain the original IDock object.
#[doc(hidden)]
pub trait IDockMetadata {}
ferroui_base::ferro_markup_type!(interface dyn IDockMetadata as "IDock" {
    namespace:"FerroDock.Model.Core",this:Rc<dyn IDock>,handles:[Rc<dyn IDock>,Option<Rc<dyn IDock>>],base:Rc<dyn IDockable>,interfaces:[Rc<dyn IDockable>],
    properties:[
        VisibleDockables:Option<ListHandle<Option<Rc<dyn IDockable>>>>{get:|this:&Rc<dyn IDock>|this.visible_dockables(),try_set:|this:&Rc<dyn IDock>,value:Option<ListHandle<Option<Rc<dyn IDockable>>>>|{this.set_visible_dockables(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        ActiveDockable:Option<Rc<dyn IDockable>>{get:|this:&Rc<dyn IDock>|this.active_dockable(),try_set:|this:&Rc<dyn IDock>,value:Option<Rc<dyn IDockable>>|{this.set_active_dockable(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        DefaultDockable:Option<Rc<dyn IDockable>>{get:|this:&Rc<dyn IDock>|this.default_dockable(),try_set:|this:&Rc<dyn IDock>,value:Option<Rc<dyn IDockable>>|{this.set_default_dockable(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        FocusedDockable:Option<Rc<dyn IDockable>>{get:|this:&Rc<dyn IDock>|this.focused_dockable(),try_set:|this:&Rc<dyn IDock>,value:Option<Rc<dyn IDockable>>|{this.set_focused_dockable(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        IsActive:bool{get:|this:&Rc<dyn IDock>|this.is_active(),try_set:|this:&Rc<dyn IDock>,value:bool|{this.set_is_active(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        OpenedDockablesCount:i32{get:|this:&Rc<dyn IDock>|this.opened_dockables_count(),try_set:|this:&Rc<dyn IDock>,value:i32|{this.set_opened_dockables_count(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        CanCloseLastDockable:bool{get:|this:&Rc<dyn IDock>|this.can_close_last_dockable(),try_set:|this:&Rc<dyn IDock>,value:bool|{this.set_can_close_last_dockable(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        DockCapabilityPolicy:Option<Rc<dyn DockCapabilityPolicyImpl>>{get:|this:&Rc<dyn IDock>|this.dock_capability_policy(),try_set:|this:&Rc<dyn IDock>,value:Option<Rc<dyn DockCapabilityPolicyImpl>>|{this.set_dock_capability_policy(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        CanGoBack:bool{try_get:|this:&Rc<dyn IDock>|Ok::<bool,ferroui_base::metadata::MarkupInvokeError>(this.can_go_back())},
        CanGoForward:bool{try_get:|this:&Rc<dyn IDock>|Ok::<bool,ferroui_base::metadata::MarkupInvokeError>(this.can_go_forward())},
        GoBack:Rc<dyn ICommand>{try_get:|this:&Rc<dyn IDock>|Ok::<Rc<dyn ICommand>,ferroui_base::metadata::MarkupInvokeError>(this.go_back())},
        GoForward:Rc<dyn ICommand>{try_get:|this:&Rc<dyn IDock>|Ok::<Rc<dyn ICommand>,ferroui_base::metadata::MarkupInvokeError>(this.go_forward())},
        Navigate:Rc<dyn ICommand>{try_get:|this:&Rc<dyn IDock>|Ok::<Rc<dyn ICommand>,ferroui_base::metadata::MarkupInvokeError>(this.navigate())},
        Close:Rc<dyn ICommand>{try_get:|this:&Rc<dyn IDock>|Ok::<Rc<dyn ICommand>,ferroui_base::metadata::MarkupInvokeError>(this.close())},
        EnableGlobalDocking:bool{get:|this:&Rc<dyn IDock>|this.enable_global_docking(),try_set:|this:&Rc<dyn IDock>,value:bool|{this.set_enable_global_docking(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
    ],
});
static METADATA: MarkupType = MarkupType {
    notify_property_changed: Some(notifier),
    ..*<dyn IDockMetadata as MarkupTyped>::MARKUP
};
fn notifier(value: &dyn AnyValue) -> Option<&dyn INotifyPropertyChanged> {
    if let Some(model) = value.downcast_ref::<Rc<dyn IDock>>() {
        return model.as_notify_property_changed();
    }
    value
        .downcast_ref::<Option<Rc<dyn IDock>>>()?
        .as_ref()?
        .as_notify_property_changed()
}
pub(crate) fn register() {
    MarkupType::register(&METADATA);
    // The notifier descriptor is the registered identity; emitted members still
    // belong to the original public declaration carrier.
    ferroui_base::__ferro_compiler_metadata! {
        MarkupType::register_rust_paths(&[(&METADATA, "::ferrodock_model_ferro::IDockMetadata", true)]);
    }
}
