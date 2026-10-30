// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::core::IDock;
use ferrodock_model::{controls::IProportionalDock, core::IDockable};
use ferroui_base::{
    metadata::{MarkupType, MarkupTyped},
    AnyValue,
};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::rc::Rc;
/// Native descriptor retaining the original IProportionalDock receiver.
#[doc(hidden)]
pub trait IProportionalDockMetadata {}
ferroui_base::ferro_markup_type!(interface dyn IProportionalDockMetadata as "IProportionalDock" {
    namespace:"FerroDock.Model.Controls",this:Rc<dyn IProportionalDock>,handles:[Rc<dyn IProportionalDock>,Option<Rc<dyn IProportionalDock>>],base:Rc<dyn IDock>,interfaces:[Rc<dyn IDock>,Rc<dyn ferrodock_model::core::IGlobalTarget>],
    attributes:[RequiresDataTemplate],
    properties:[
        Orientation:ferrodock_model::core::Orientation{get:|this:&Rc<dyn IProportionalDock>|this.orientation(),try_set:|this:&Rc<dyn IProportionalDock>,value:ferrodock_model::core::Orientation|{this.set_orientation(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
    ],
});
static METADATA: MarkupType = MarkupType {
    notify_property_changed: Some(notifier),
    ..*<dyn IProportionalDockMetadata as MarkupTyped>::MARKUP
};
fn notifier(value: &dyn AnyValue) -> Option<&dyn INotifyPropertyChanged> {
    if let Some(model) = value.downcast_ref::<Rc<dyn IProportionalDock>>() {
        return model.as_notify_property_changed();
    }
    value
        .downcast_ref::<Option<Rc<dyn IProportionalDock>>>()?
        .as_ref()?
        .as_notify_property_changed()
}
pub(crate) fn register() {
    use ferroui_base::data::core::ValueTypes;
    MarkupType::register(&METADATA);
    // The notifier descriptor is the registered identity; emitted members still
    // belong to the original public declaration carrier.
    ferroui_base::__ferro_compiler_metadata! {
        MarkupType::register_rust_paths(&[(&METADATA, "::ferrodock_model_ferro::IProportionalDockMetadata", true)]);
    }
    ValueTypes::register_nullable::<Rc<dyn IProportionalDock>>();
    ValueTypes::register_cast::<Rc<dyn IProportionalDock>, Rc<dyn IDock>>(|d| d.clone());
    ValueTypes::register_cast::<Rc<dyn IProportionalDock>, Rc<dyn IDockable>>(|d| d.clone());
    ValueTypes::register_boxed_cast::<Rc<dyn IDockable>, Rc<dyn IProportionalDock>>(|v| {
        crate::control_contract_registration::dockable(v)?.as_proportional_dock()
    });
    ValueTypes::register_boxed_cast::<Rc<dyn IDock>, Rc<dyn IProportionalDock>>(|v| {
        crate::control_contract_registration::dockable(v)?.as_proportional_dock()
    });
}
