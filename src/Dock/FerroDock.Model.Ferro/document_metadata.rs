// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::controls::IDocument;
use ferrodock_model::core::{IDockable, ILocalTarget};
use ferroui_base::{
    metadata::{MarkupType, MarkupTyped},
    AnyValue,
};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::rc::Rc;
/// Native descriptor retaining the original `IDocument` receiver.
#[doc(hidden)]
pub trait IDocumentMetadata {}
ferroui_base::ferro_markup_type!(interface dyn IDocumentMetadata as "IDocument" {
 namespace:"FerroDock.Model.Controls",this:Rc<dyn IDocument>,handles:[Rc<dyn IDocument>,Option<Rc<dyn IDocument>>],base:Rc<dyn IDockable>,interfaces:[Rc<dyn IDockable>,Rc<dyn ILocalTarget>],
 properties:[],

});
static METADATA: MarkupType = MarkupType {
    notify_property_changed: Some(notifier),
    ..*<dyn IDocumentMetadata as MarkupTyped>::MARKUP
};
fn notifier(value: &dyn AnyValue) -> Option<&dyn INotifyPropertyChanged> {
    if let Some(model) = value.downcast_ref::<Rc<dyn IDocument>>() {
        return model.as_notify_property_changed();
    }
    value
        .downcast_ref::<Option<Rc<dyn IDocument>>>()?
        .as_ref()?
        .as_notify_property_changed()
}
pub(crate) fn register() {
    use ferroui_base::data::core::ValueTypes;
    MarkupType::register(&METADATA);
    // The notifier descriptor is the registered identity; emitted members still
    // belong to the original public declaration carrier.
    ferroui_base::__ferro_compiler_metadata! {
        MarkupType::register_rust_paths(&[(&METADATA, "::ferrodock_model_ferro::IDocumentMetadata", true)]);
    }
    ValueTypes::register_nullable::<Rc<dyn IDocument>>();
    ValueTypes::register_cast::<Rc<dyn IDocument>, Rc<dyn IDockable>>(|v| v.clone());
    ValueTypes::register_boxed_cast::<Rc<dyn IDockable>, Rc<dyn IDocument>>(|v| {
        crate::control_contract_registration::dockable(v)?.as_document()
    });
}
