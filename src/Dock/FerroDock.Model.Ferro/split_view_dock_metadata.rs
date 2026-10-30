// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::core::IDock;
use ferrodock_model::{controls::ISplitViewDock, core::IDockable};
use ferroui_base::{
    metadata::{MarkupType, MarkupTyped},
    AnyValue,
};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::rc::Rc;
/// Native descriptor retaining the original ISplitViewDock receiver.
#[doc(hidden)]
pub trait ISplitViewDockMetadata {}
ferroui_base::ferro_markup_type!(interface dyn ISplitViewDockMetadata as "ISplitViewDock" {
    namespace:"FerroDock.Model.Controls",this:Rc<dyn ISplitViewDock>,handles:[Rc<dyn ISplitViewDock>,Option<Rc<dyn ISplitViewDock>>],base:Rc<dyn IDock>,interfaces:[Rc<dyn IDock>],
    attributes:[RequiresDataTemplate],
    properties:[
        CompactPaneLength:f64{get:|this:&Rc<dyn ISplitViewDock>|this.compact_pane_length(),try_set:|this:&Rc<dyn ISplitViewDock>,value:f64|{this.set_compact_pane_length(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        DisplayMode:ferrodock_model::core::SplitViewDisplayMode{get:|this:&Rc<dyn ISplitViewDock>|this.display_mode(),try_set:|this:&Rc<dyn ISplitViewDock>,value:ferrodock_model::core::SplitViewDisplayMode|{this.set_display_mode(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        IsPaneOpen:bool{get:|this:&Rc<dyn ISplitViewDock>|this.is_pane_open(),try_set:|this:&Rc<dyn ISplitViewDock>,value:bool|{this.set_is_pane_open(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        OpenPaneLength:f64{get:|this:&Rc<dyn ISplitViewDock>|this.open_pane_length(),try_set:|this:&Rc<dyn ISplitViewDock>,value:f64|{this.set_open_pane_length(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        PanePlacement:ferrodock_model::core::SplitViewPanePlacement{get:|this:&Rc<dyn ISplitViewDock>|this.pane_placement(),try_set:|this:&Rc<dyn ISplitViewDock>,value:ferrodock_model::core::SplitViewPanePlacement|{this.set_pane_placement(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        UseLightDismissOverlayMode:bool{get:|this:&Rc<dyn ISplitViewDock>|this.use_light_dismiss_overlay_mode(),try_set:|this:&Rc<dyn ISplitViewDock>,value:bool|{this.set_use_light_dismiss_overlay_mode(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        PaneDockable:Option<Rc<dyn IDockable>>{get:|this:&Rc<dyn ISplitViewDock>|this.pane_dockable(),try_set:|this:&Rc<dyn ISplitViewDock>,value:Option<Rc<dyn IDockable>>|{this.set_pane_dockable(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        ContentDockable:Option<Rc<dyn IDockable>>{get:|this:&Rc<dyn ISplitViewDock>|this.content_dockable(),try_set:|this:&Rc<dyn ISplitViewDock>,value:Option<Rc<dyn IDockable>>|{this.set_content_dockable(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
    ],
    methods:[fn OpenPane() => (|this:&Rc<dyn ISplitViewDock>|this.open_pane()),fn ClosePane() => (|this:&Rc<dyn ISplitViewDock>|this.close_pane()),fn TogglePane() => (|this:&Rc<dyn ISplitViewDock>|this.toggle_pane())],
});
static METADATA: MarkupType = MarkupType {
    notify_property_changed: Some(notifier),
    ..*<dyn ISplitViewDockMetadata as MarkupTyped>::MARKUP
};
fn notifier(value: &dyn AnyValue) -> Option<&dyn INotifyPropertyChanged> {
    if let Some(model) = value.downcast_ref::<Rc<dyn ISplitViewDock>>() {
        return model.as_notify_property_changed();
    }
    value
        .downcast_ref::<Option<Rc<dyn ISplitViewDock>>>()?
        .as_ref()?
        .as_notify_property_changed()
}
pub(crate) fn register() {
    use ferroui_base::data::core::ValueTypes;
    MarkupType::register(&METADATA);
    // The notifier descriptor is the registered identity; emitted members still
    // belong to the original public declaration carrier.
    ferroui_base::__ferro_compiler_metadata! {
        MarkupType::register_rust_paths(&[(&METADATA, "::ferrodock_model_ferro::ISplitViewDockMetadata", true)]);
    }
    ValueTypes::register_nullable::<Rc<dyn ISplitViewDock>>();
    ValueTypes::register_cast::<Rc<dyn ISplitViewDock>, Rc<dyn IDock>>(|d| d.clone());
    ValueTypes::register_cast::<Rc<dyn ISplitViewDock>, Rc<dyn IDockable>>(|d| d.clone());
    ValueTypes::register_boxed_cast::<Rc<dyn IDockable>, Rc<dyn ISplitViewDock>>(|v| {
        crate::control_contract_registration::dockable(v)?.as_split_view_dock()
    });
    ValueTypes::register_boxed_cast::<Rc<dyn IDock>, Rc<dyn ISplitViewDock>>(|v| {
        crate::control_contract_registration::dockable(v)?.as_split_view_dock()
    });
}
