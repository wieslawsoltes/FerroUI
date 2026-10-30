// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Native root metadata over the original model handle and its optional System notifier.
use ferrodock_model::{
    collections::ListHandle,
    controls::{IRootDock, IToolDock},
    core::{
        DockCapabilityPolicyImpl, DockFloatingWindowHostMode, IDock, IDockWindow, IDockable,
        PinnedDockDisplayMode,
    },
};
use ferroui_base::{
    metadata::{MarkupType, MarkupTyped},
    AnyValue,
};
use ferroui_reactiveui::ICommand;
use ferroui_system::component_model::INotifyPropertyChanged;
use std::rc::Rc;
/// Descriptor carrier retaining the original root model.
#[doc(hidden)]
pub trait IRootDockMetadata {}
ferroui_base::ferro_markup_type!(interface dyn IRootDockMetadata as "IRootDock" {
    namespace:"FerroDock.Model.Controls",this:Rc<dyn IRootDock>,handles:[Rc<dyn IRootDock>,Option<Rc<dyn IRootDock>>],base:Rc<dyn IDock>,interfaces:[Rc<dyn IDock>],
    attributes:[RequiresDataTemplate],
    properties:[
        IsFocusableRoot:bool{get:|this:&Rc<dyn IRootDock>|this.is_focusable_root(),try_set:|this:&Rc<dyn IRootDock>,value:bool|{this.set_is_focusable_root(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        HiddenDockables:Option<ListHandle<Option<Rc<dyn IDockable>>>>{get:|this:&Rc<dyn IRootDock>|this.hidden_dockables(),try_set:|this:&Rc<dyn IRootDock>,value:Option<ListHandle<Option<Rc<dyn IDockable>>>>|{this.set_hidden_dockables(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        LeftPinnedDockables:Option<ListHandle<Option<Rc<dyn IDockable>>>>{get:|this:&Rc<dyn IRootDock>|this.left_pinned_dockables(),try_set:|this:&Rc<dyn IRootDock>,value:Option<ListHandle<Option<Rc<dyn IDockable>>>>|{this.set_left_pinned_dockables(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        RightPinnedDockables:Option<ListHandle<Option<Rc<dyn IDockable>>>>{get:|this:&Rc<dyn IRootDock>|this.right_pinned_dockables(),try_set:|this:&Rc<dyn IRootDock>,value:Option<ListHandle<Option<Rc<dyn IDockable>>>>|{this.set_right_pinned_dockables(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        TopPinnedDockables:Option<ListHandle<Option<Rc<dyn IDockable>>>>{get:|this:&Rc<dyn IRootDock>|this.top_pinned_dockables(),try_set:|this:&Rc<dyn IRootDock>,value:Option<ListHandle<Option<Rc<dyn IDockable>>>>|{this.set_top_pinned_dockables(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        BottomPinnedDockables:Option<ListHandle<Option<Rc<dyn IDockable>>>>{get:|this:&Rc<dyn IRootDock>|this.bottom_pinned_dockables(),try_set:|this:&Rc<dyn IRootDock>,value:Option<ListHandle<Option<Rc<dyn IDockable>>>>|{this.set_bottom_pinned_dockables(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        PinnedDock:Option<Rc<dyn IToolDock>>{get:|this:&Rc<dyn IRootDock>|this.pinned_dock(),try_set:|this:&Rc<dyn IRootDock>,value:Option<Rc<dyn IToolDock>>|{this.set_pinned_dock(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        PinnedDockDisplayMode:PinnedDockDisplayMode{get:|this:&Rc<dyn IRootDock>|this.pinned_dock_display_mode(),try_set:|this:&Rc<dyn IRootDock>,value:PinnedDockDisplayMode|{this.set_pinned_dock_display_mode(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        Window:Option<Rc<dyn IDockWindow>>{get:|this:&Rc<dyn IRootDock>|this.window(),try_set:|this:&Rc<dyn IRootDock>,value:Option<Rc<dyn IDockWindow>>|{this.set_window(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        Windows:Option<ListHandle<Option<Rc<dyn IDockWindow>>>>{get:|this:&Rc<dyn IRootDock>|this.windows(),try_set:|this:&Rc<dyn IRootDock>,value:Option<ListHandle<Option<Rc<dyn IDockWindow>>>>|{this.set_windows(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        FloatingWindowHostMode:DockFloatingWindowHostMode{get:|this:&Rc<dyn IRootDock>|this.floating_window_host_mode(),try_set:|this:&Rc<dyn IRootDock>,value:DockFloatingWindowHostMode|{this.set_floating_window_host_mode(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        RootDockCapabilityPolicy:Option<Rc<dyn DockCapabilityPolicyImpl>>{get:|this:&Rc<dyn IRootDock>|this.root_dock_capability_policy(),try_set:|this:&Rc<dyn IRootDock>,value:Option<Rc<dyn DockCapabilityPolicyImpl>>|{this.set_root_dock_capability_policy(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        ShowWindows:Rc<dyn ICommand>{try_get:|this:&Rc<dyn IRootDock>|Ok::<Rc<dyn ICommand>,ferroui_base::metadata::MarkupInvokeError>(this.show_windows())},
        ExitWindows:Rc<dyn ICommand>{try_get:|this:&Rc<dyn IRootDock>|Ok::<Rc<dyn ICommand>,ferroui_base::metadata::MarkupInvokeError>(this.exit_windows())},
        EnableAdaptiveGlobalDockTargets:bool{get:|this:&Rc<dyn IRootDock>|this.enable_adaptive_global_dock_targets(),try_set:|this:&Rc<dyn IRootDock>,value:bool|{this.set_enable_adaptive_global_dock_targets(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
    ],
});
static METADATA: MarkupType = MarkupType {
    notify_property_changed: Some(notifier),
    ..*<dyn IRootDockMetadata as MarkupTyped>::MARKUP
};
fn notifier(value: &dyn AnyValue) -> Option<&dyn INotifyPropertyChanged> {
    if let Some(root) = value.downcast_ref::<Rc<dyn IRootDock>>() {
        return root.as_notify_property_changed();
    }
    value
        .downcast_ref::<Option<Rc<dyn IRootDock>>>()?
        .as_ref()?
        .as_notify_property_changed()
}
pub(crate) fn register() {
    MarkupType::register(&METADATA);
    // The notifier descriptor is the registered identity; emitted members still
    // belong to the original public declaration carrier.
    ferroui_base::__ferro_compiler_metadata! {
        MarkupType::register_rust_paths(&[(&METADATA, "::ferrodock_model_ferro::IRootDockMetadata", true)]);
    }
}
