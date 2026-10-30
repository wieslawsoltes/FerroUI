// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::core::IDock;
use ferrodock_model::{controls::IUniformGridDock, core::IDockable};
use ferroui_base::{
    metadata::{MarkupType, MarkupTyped},
    AnyValue,
};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::rc::Rc;
/// Native descriptor retaining the original IUniformGridDock receiver.
#[doc(hidden)]
pub trait IUniformGridDockMetadata {}
ferroui_base::ferro_markup_type!(interface dyn IUniformGridDockMetadata as "IUniformGridDock" {
    namespace:"FerroDock.Model.Controls",this:Rc<dyn IUniformGridDock>,handles:[Rc<dyn IUniformGridDock>,Option<Rc<dyn IUniformGridDock>>],base:Rc<dyn IDock>,interfaces:[Rc<dyn IDock>],
    attributes:[RequiresDataTemplate],
    properties:[
        Rows:i32{get:|this:&Rc<dyn IUniformGridDock>|this.rows(),try_set:|this:&Rc<dyn IUniformGridDock>,value:i32|{this.set_rows(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        Columns:i32{get:|this:&Rc<dyn IUniformGridDock>|this.columns(),try_set:|this:&Rc<dyn IUniformGridDock>,value:i32|{this.set_columns(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
    ],
});
static METADATA: MarkupType = MarkupType {
    notify_property_changed: Some(notifier),
    ..*<dyn IUniformGridDockMetadata as MarkupTyped>::MARKUP
};
fn notifier(value: &dyn AnyValue) -> Option<&dyn INotifyPropertyChanged> {
    if let Some(model) = value.downcast_ref::<Rc<dyn IUniformGridDock>>() {
        return model.as_notify_property_changed();
    }
    value
        .downcast_ref::<Option<Rc<dyn IUniformGridDock>>>()?
        .as_ref()?
        .as_notify_property_changed()
}
pub(crate) fn register() {
    use ferroui_base::data::core::ValueTypes;
    MarkupType::register(&METADATA);
    // The notifier descriptor is the registered identity; emitted members still
    // belong to the original public declaration carrier.
    ferroui_base::__ferro_compiler_metadata! {
        MarkupType::register_rust_paths(&[(&METADATA, "::ferrodock_model_ferro::IUniformGridDockMetadata", true)]);
    }
    ValueTypes::register_nullable::<Rc<dyn IUniformGridDock>>();
    ValueTypes::register_cast::<Rc<dyn IUniformGridDock>, Rc<dyn IDock>>(|d| d.clone());
    ValueTypes::register_cast::<Rc<dyn IUniformGridDock>, Rc<dyn IDockable>>(|d| d.clone());
    ValueTypes::register_boxed_cast::<Rc<dyn IDockable>, Rc<dyn IUniformGridDock>>(|v| {
        crate::control_contract_registration::dockable(v)?.as_uniform_grid_dock()
    });
    ValueTypes::register_boxed_cast::<Rc<dyn IDock>, Rc<dyn IUniformGridDock>>(|v| {
        crate::control_contract_registration::dockable(v)?.as_uniform_grid_dock()
    });
}
