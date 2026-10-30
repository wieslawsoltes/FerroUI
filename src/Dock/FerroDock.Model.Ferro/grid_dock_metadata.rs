// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::core::IDock;
use ferrodock_model::{controls::IGridDock, core::IDockable};
use ferroui_base::{
    metadata::{MarkupType, MarkupTyped},
    AnyValue,
};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::rc::Rc;
/// Native descriptor retaining the original IGridDock receiver.
#[doc(hidden)]
pub trait IGridDockMetadata {}
ferroui_base::ferro_markup_type!(interface dyn IGridDockMetadata as "IGridDock" {
    namespace:"FerroDock.Model.Controls",this:Rc<dyn IGridDock>,handles:[Rc<dyn IGridDock>,Option<Rc<dyn IGridDock>>],base:Rc<dyn IDock>,interfaces:[Rc<dyn IDock>],
    attributes:[RequiresDataTemplate],
    properties:[
        ColumnDefinitions:Option<String>{get:|this:&Rc<dyn IGridDock>|this.column_definitions(),try_set:|this:&Rc<dyn IGridDock>,value:Option<String>|{this.set_column_definitions(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        RowDefinitions:Option<String>{get:|this:&Rc<dyn IGridDock>|this.row_definitions(),try_set:|this:&Rc<dyn IGridDock>,value:Option<String>|{this.set_row_definitions(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
    ],
});
static METADATA: MarkupType = MarkupType {
    notify_property_changed: Some(notifier),
    ..*<dyn IGridDockMetadata as MarkupTyped>::MARKUP
};
fn notifier(value: &dyn AnyValue) -> Option<&dyn INotifyPropertyChanged> {
    if let Some(model) = value.downcast_ref::<Rc<dyn IGridDock>>() {
        return model.as_notify_property_changed();
    }
    value
        .downcast_ref::<Option<Rc<dyn IGridDock>>>()?
        .as_ref()?
        .as_notify_property_changed()
}
pub(crate) fn register() {
    use ferroui_base::data::core::ValueTypes;
    MarkupType::register(&METADATA);
    // The notifier descriptor is the registered identity; emitted members still
    // belong to the original public declaration carrier.
    ferroui_base::__ferro_compiler_metadata! {
        MarkupType::register_rust_paths(&[(&METADATA, "::ferrodock_model_ferro::IGridDockMetadata", true)]);
    }
    ValueTypes::register_nullable::<Rc<dyn IGridDock>>();
    ValueTypes::register_cast::<Rc<dyn IGridDock>, Rc<dyn IDock>>(|d| d.clone());
    ValueTypes::register_cast::<Rc<dyn IGridDock>, Rc<dyn IDockable>>(|d| d.clone());
    ValueTypes::register_boxed_cast::<Rc<dyn IDockable>, Rc<dyn IGridDock>>(|v| {
        crate::control_contract_registration::dockable(v)?.as_grid_dock()
    });
    ValueTypes::register_boxed_cast::<Rc<dyn IDock>, Rc<dyn IGridDock>>(|v| {
        crate::control_contract_registration::dockable(v)?.as_grid_dock()
    });
}
