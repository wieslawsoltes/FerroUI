// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::controls::IGridDockSplitter;
use ferrodock_model::controls::ISplitter;
use ferroui_base::{
    metadata::{MarkupType, MarkupTyped},
    AnyValue,
};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::rc::Rc;
/// Native descriptor retaining the original `IGridDockSplitter` receiver.
#[doc(hidden)]
pub trait IGridDockSplitterMetadata {}
ferroui_base::ferro_markup_type!(interface dyn IGridDockSplitterMetadata as "IGridDockSplitter" {
 namespace:"FerroDock.Model.Controls",this:Rc<dyn IGridDockSplitter>,handles:[Rc<dyn IGridDockSplitter>,Option<Rc<dyn IGridDockSplitter>>],base:Rc<dyn ISplitter>,interfaces:[Rc<dyn ISplitter>],
    attributes:[RequiresDataTemplate],
 properties:[ResizeDirection:ferrodock_model::core::GridResizeDirection{get:|this:&Rc<dyn IGridDockSplitter>|this.resize_direction(),set:|this:&Rc<dyn IGridDockSplitter>,value:ferrodock_model::core::GridResizeDirection|this.set_resize_direction(value)},],

});
static METADATA: MarkupType = MarkupType {
    notify_property_changed: Some(notifier),
    ..*<dyn IGridDockSplitterMetadata as MarkupTyped>::MARKUP
};
fn notifier(value: &dyn AnyValue) -> Option<&dyn INotifyPropertyChanged> {
    if let Some(model) = value.downcast_ref::<Rc<dyn IGridDockSplitter>>() {
        return model.as_notify_property_changed();
    }
    value
        .downcast_ref::<Option<Rc<dyn IGridDockSplitter>>>()?
        .as_ref()?
        .as_notify_property_changed()
}
pub(crate) fn register() {
    use ferroui_base::data::core::ValueTypes;
    MarkupType::register(&METADATA);
    // The notifier descriptor is the registered identity; emitted members still
    // belong to the original public declaration carrier.
    ferroui_base::__ferro_compiler_metadata! {
        MarkupType::register_rust_paths(&[(&METADATA, "::ferrodock_model_ferro::IGridDockSplitterMetadata", true)]);
    }
    ValueTypes::register_nullable::<Rc<dyn IGridDockSplitter>>();
    use ferrodock_model::core::IDockable;
    ValueTypes::register_cast::<Rc<dyn IGridDockSplitter>, Rc<dyn ISplitter>>(|v| v.clone());
    ValueTypes::register_cast::<Rc<dyn IGridDockSplitter>, Rc<dyn IDockable>>(|v| v.clone());
    ValueTypes::register_boxed_cast::<Rc<dyn IDockable>, Rc<dyn IGridDockSplitter>>(|v| {
        crate::control_contract_registration::dockable(v)?.as_grid_dock_splitter()
    });
    ValueTypes::register_boxed_cast::<Rc<dyn ISplitter>, Rc<dyn IGridDockSplitter>>(|v| {
        crate::control_contract_registration::dockable(v)?.as_grid_dock_splitter()
    });
}
