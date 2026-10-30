// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::core::IDock;
use ferrodock_model::{controls::IStackDock, core::IDockable};
use ferroui_base::{
    metadata::{MarkupType, MarkupTyped},
    AnyValue,
};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::rc::Rc;
/// Native descriptor retaining the original IStackDock receiver.
#[doc(hidden)]
pub trait IStackDockMetadata {}
ferroui_base::ferro_markup_type!(interface dyn IStackDockMetadata as "IStackDock" {
    namespace:"FerroDock.Model.Controls",this:Rc<dyn IStackDock>,handles:[Rc<dyn IStackDock>,Option<Rc<dyn IStackDock>>],base:Rc<dyn IDock>,interfaces:[Rc<dyn IDock>],
    attributes:[RequiresDataTemplate],
    properties:[
        Orientation:ferrodock_model::core::Orientation{get:|this:&Rc<dyn IStackDock>|this.orientation(),try_set:|this:&Rc<dyn IStackDock>,value:ferrodock_model::core::Orientation|{this.set_orientation(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        Spacing:f64{get:|this:&Rc<dyn IStackDock>|this.spacing(),try_set:|this:&Rc<dyn IStackDock>,value:f64|{this.set_spacing(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
    ],
});
static METADATA: MarkupType = MarkupType {
    notify_property_changed: Some(notifier),
    ..*<dyn IStackDockMetadata as MarkupTyped>::MARKUP
};
fn notifier(value: &dyn AnyValue) -> Option<&dyn INotifyPropertyChanged> {
    if let Some(model) = value.downcast_ref::<Rc<dyn IStackDock>>() {
        return model.as_notify_property_changed();
    }
    value
        .downcast_ref::<Option<Rc<dyn IStackDock>>>()?
        .as_ref()?
        .as_notify_property_changed()
}
pub(crate) fn register() {
    use ferroui_base::data::core::ValueTypes;
    MarkupType::register(&METADATA);
    // The notifier descriptor is the registered identity; emitted members still
    // belong to the original public declaration carrier.
    ferroui_base::__ferro_compiler_metadata! {
        MarkupType::register_rust_paths(&[(&METADATA, "::ferrodock_model_ferro::IStackDockMetadata", true)]);
    }
    ValueTypes::register_nullable::<Rc<dyn IStackDock>>();
    ValueTypes::register_cast::<Rc<dyn IStackDock>, Rc<dyn IDock>>(|d| d.clone());
    ValueTypes::register_cast::<Rc<dyn IStackDock>, Rc<dyn IDockable>>(|d| d.clone());
    ValueTypes::register_boxed_cast::<Rc<dyn IDockable>, Rc<dyn IStackDock>>(|v| {
        crate::control_contract_registration::dockable(v)?.as_stack_dock()
    });
    ValueTypes::register_boxed_cast::<Rc<dyn IDock>, Rc<dyn IStackDock>>(|v| {
        crate::control_contract_registration::dockable(v)?.as_stack_dock()
    });
}
