// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::core::IDock;
use ferrodock_model::{controls::IDockDock, core::IDockable};
use ferroui_base::{
    metadata::{MarkupType, MarkupTyped},
    AnyValue,
};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::rc::Rc;
/// Native descriptor retaining the original IDockDock receiver.
#[doc(hidden)]
pub trait IDockDockMetadata {}
ferroui_base::ferro_markup_type!(interface dyn IDockDockMetadata as "IDockDock" {
    namespace:"FerroDock.Model.Controls",this:Rc<dyn IDockDock>,handles:[Rc<dyn IDockDock>,Option<Rc<dyn IDockDock>>],base:Rc<dyn IDock>,interfaces:[Rc<dyn IDock>],
    attributes:[RequiresDataTemplate],
    properties:[
        LastChildFill:bool{get:|this:&Rc<dyn IDockDock>|this.last_child_fill(),try_set:|this:&Rc<dyn IDockDock>,value:bool|{this.set_last_child_fill(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
    ],
});
static METADATA: MarkupType = MarkupType {
    notify_property_changed: Some(notifier),
    ..*<dyn IDockDockMetadata as MarkupTyped>::MARKUP
};
fn notifier(value: &dyn AnyValue) -> Option<&dyn INotifyPropertyChanged> {
    if let Some(model) = value.downcast_ref::<Rc<dyn IDockDock>>() {
        return model.as_notify_property_changed();
    }
    value
        .downcast_ref::<Option<Rc<dyn IDockDock>>>()?
        .as_ref()?
        .as_notify_property_changed()
}
pub(crate) fn register() {
    use ferroui_base::data::core::ValueTypes;
    MarkupType::register(&METADATA);
    // The notifier descriptor is the registered identity; emitted members still
    // belong to the original public declaration carrier.
    ferroui_base::__ferro_compiler_metadata! {
        MarkupType::register_rust_paths(&[(&METADATA, "::ferrodock_model_ferro::IDockDockMetadata", true)]);
    }
    ValueTypes::register_nullable::<Rc<dyn IDockDock>>();
    ValueTypes::register_cast::<Rc<dyn IDockDock>, Rc<dyn IDock>>(|d| d.clone());
    ValueTypes::register_cast::<Rc<dyn IDockDock>, Rc<dyn IDockable>>(|d| d.clone());
    ValueTypes::register_boxed_cast::<Rc<dyn IDockable>, Rc<dyn IDockDock>>(|v| {
        crate::control_contract_registration::dockable(v)?.as_dock_dock()
    });
    ValueTypes::register_boxed_cast::<Rc<dyn IDock>, Rc<dyn IDockDock>>(|v| {
        crate::control_contract_registration::dockable(v)?.as_dock_dock()
    });
}
