// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::controls::IProportionalDockSplitter;
use ferrodock_model::controls::ISplitter;
use ferroui_base::{
    metadata::{MarkupType, MarkupTyped},
    AnyValue,
};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::rc::Rc;
/// Native descriptor retaining the original `IProportionalDockSplitter` receiver.
#[doc(hidden)]
pub trait IProportionalDockSplitterMetadata {}
ferroui_base::ferro_markup_type!(interface dyn IProportionalDockSplitterMetadata as "IProportionalDockSplitter" {
 namespace:"FerroDock.Model.Controls",this:Rc<dyn IProportionalDockSplitter>,handles:[Rc<dyn IProportionalDockSplitter>,Option<Rc<dyn IProportionalDockSplitter>>],base:Rc<dyn ISplitter>,interfaces:[Rc<dyn ISplitter>],
    attributes:[RequiresDataTemplate],
 properties:[CanResize:bool{get:|this:&Rc<dyn IProportionalDockSplitter>|this.can_resize(),set:|this:&Rc<dyn IProportionalDockSplitter>,value:bool|this.set_can_resize(value)},
ResizePreview:bool{get:|this:&Rc<dyn IProportionalDockSplitter>|this.resize_preview(),set:|this:&Rc<dyn IProportionalDockSplitter>,value:bool|this.set_resize_preview(value)},],

});
static METADATA: MarkupType = MarkupType {
    notify_property_changed: Some(notifier),
    ..*<dyn IProportionalDockSplitterMetadata as MarkupTyped>::MARKUP
};
fn notifier(value: &dyn AnyValue) -> Option<&dyn INotifyPropertyChanged> {
    if let Some(model) = value.downcast_ref::<Rc<dyn IProportionalDockSplitter>>() {
        return model.as_notify_property_changed();
    }
    value
        .downcast_ref::<Option<Rc<dyn IProportionalDockSplitter>>>()?
        .as_ref()?
        .as_notify_property_changed()
}
pub(crate) fn register() {
    use ferroui_base::data::core::ValueTypes;
    MarkupType::register(&METADATA);
    // The notifier descriptor is the registered identity; emitted members still
    // belong to the original public declaration carrier.
    ferroui_base::__ferro_compiler_metadata! {
        MarkupType::register_rust_paths(&[(&METADATA, "::ferrodock_model_ferro::IProportionalDockSplitterMetadata", true)]);
    }
    ValueTypes::register_nullable::<Rc<dyn IProportionalDockSplitter>>();
    use ferrodock_model::core::IDockable;
    ValueTypes::register_cast::<Rc<dyn IProportionalDockSplitter>, Rc<dyn ISplitter>>(|v| {
        v.clone()
    });
    ValueTypes::register_cast::<Rc<dyn IProportionalDockSplitter>, Rc<dyn IDockable>>(|v| {
        v.clone()
    });
    ValueTypes::register_boxed_cast::<Rc<dyn IDockable>, Rc<dyn IProportionalDockSplitter>>(|v| {
        crate::control_contract_registration::dockable(v)?.as_proportional_dock_splitter()
    });
    ValueTypes::register_boxed_cast::<Rc<dyn ISplitter>, Rc<dyn IProportionalDockSplitter>>(|v| {
        crate::control_contract_registration::dockable(v)?.as_proportional_dock_splitter()
    });
}
