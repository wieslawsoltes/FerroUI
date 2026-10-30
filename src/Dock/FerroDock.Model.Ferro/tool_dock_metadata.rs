// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::{
    controls::IToolDock,
    core::{Alignment, GripMode, IDock, IDockable},
};
use ferroui_base::{
    metadata::{MarkupType, MarkupTyped},
    AnyValue,
};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::rc::Rc;
/// Descriptor carrier; values retain the original IToolDock object.
#[doc(hidden)]
pub trait IToolDockMetadata {}
ferroui_base::ferro_markup_type!(interface dyn IToolDockMetadata as "IToolDock" {
    namespace:"FerroDock.Model.Controls",this:Rc<dyn IToolDock>,handles:[Rc<dyn IToolDock>,Option<Rc<dyn IToolDock>>],base:Rc<dyn IDock>,interfaces:[Rc<dyn IDock>,Rc<dyn ferrodock_model::core::ILocalTarget>],
    attributes:[RequiresDataTemplate],
    properties:[
        Alignment:Alignment{get:|this:&Rc<dyn IToolDock>|this.alignment(),try_set:|this:&Rc<dyn IToolDock>,value:Alignment|{this.set_alignment(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        IsExpanded:bool{get:|this:&Rc<dyn IToolDock>|this.is_expanded(),try_set:|this:&Rc<dyn IToolDock>,value:bool|{this.set_is_expanded(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        AutoHide:bool{get:|this:&Rc<dyn IToolDock>|this.auto_hide(),try_set:|this:&Rc<dyn IToolDock>,value:bool|{this.set_auto_hide(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        GripMode:GripMode{get:|this:&Rc<dyn IToolDock>|this.grip_mode(),try_set:|this:&Rc<dyn IToolDock>,value:GripMode|{this.set_grip_mode(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
    ],
    methods:[
        fn AddTool(Option<Rc<dyn IDockable>>) => (|this:&Rc<dyn IToolDock>,value|this.add_tool(value)),
    ],
});
static METADATA: MarkupType = MarkupType {
    notify_property_changed: Some(notifier),
    ..*<dyn IToolDockMetadata as MarkupTyped>::MARKUP
};
fn notifier(value: &dyn AnyValue) -> Option<&dyn INotifyPropertyChanged> {
    if let Some(model) = value.downcast_ref::<Rc<dyn IToolDock>>() {
        return model.as_notify_property_changed();
    }
    value
        .downcast_ref::<Option<Rc<dyn IToolDock>>>()?
        .as_ref()?
        .as_notify_property_changed()
}
pub(crate) fn register() {
    MarkupType::register(&METADATA);
    // The notifier descriptor is the registered identity; emitted members still
    // belong to the original public declaration carrier.
    ferroui_base::__ferro_compiler_metadata! {
        MarkupType::register_rust_paths(&[(&METADATA, "::ferrodock_model_ferro::IToolDockMetadata", true)]);
    }
}
