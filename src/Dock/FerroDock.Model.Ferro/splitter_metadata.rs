// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::controls::ISplitter;
use ferrodock_model::core::IDockable;
use ferroui_base::{
    metadata::{MarkupType, MarkupTyped},
    AnyValue,
};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::rc::Rc;
/// Native descriptor retaining the original `ISplitter` receiver.
#[doc(hidden)]
pub trait ISplitterMetadata {}
ferroui_base::ferro_markup_type!(interface dyn ISplitterMetadata as "ISplitter" {
 namespace:"FerroDock.Model.Controls",this:Rc<dyn ISplitter>,handles:[Rc<dyn ISplitter>,Option<Rc<dyn ISplitter>>],base:Rc<dyn IDockable>,interfaces:[Rc<dyn IDockable>],
 properties:[],

});
static METADATA: MarkupType = MarkupType {
    notify_property_changed: Some(notifier),
    ..*<dyn ISplitterMetadata as MarkupTyped>::MARKUP
};
fn notifier(value: &dyn AnyValue) -> Option<&dyn INotifyPropertyChanged> {
    if let Some(model) = value.downcast_ref::<Rc<dyn ISplitter>>() {
        return model.as_notify_property_changed();
    }
    value
        .downcast_ref::<Option<Rc<dyn ISplitter>>>()?
        .as_ref()?
        .as_notify_property_changed()
}
pub(crate) fn register() {
    use ferroui_base::data::core::ValueTypes;
    MarkupType::register(&METADATA);
    // The notifier descriptor is the registered identity; emitted members still
    // belong to the original public declaration carrier.
    ferroui_base::__ferro_compiler_metadata! {
        MarkupType::register_rust_paths(&[(&METADATA, "::ferrodock_model_ferro::ISplitterMetadata", true)]);
    }
    ValueTypes::register_nullable::<Rc<dyn ISplitter>>();
    ValueTypes::register_cast::<Rc<dyn ISplitter>, Rc<dyn IDockable>>(|v| v.clone());
    ValueTypes::register_boxed_cast::<Rc<dyn IDockable>, Rc<dyn ISplitter>>(|v| {
        crate::control_contract_registration::dockable(v)?.as_splitter()
    });
}
