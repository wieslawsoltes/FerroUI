// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::{controls::IDocumentContent, core::IDockable};
use ferroui_base::{
    metadata::{MarkupType, MarkupTyped},
    AnyValue,
};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::rc::Rc;
/// Native descriptor retaining the original IDocumentContent receiver.
#[doc(hidden)]
pub trait IDocumentContentMetadata {}
ferroui_base::ferro_markup_type!(interface dyn IDocumentContentMetadata as "IDocumentContent" {
    namespace:"FerroDock.Model.Controls",this:Rc<dyn IDocumentContent>,handles:[Rc<dyn IDocumentContent>,Option<Rc<dyn IDocumentContent>>],base:Rc<dyn IDockable>,interfaces:[Rc<dyn IDockable>],
    attributes:[RequiresDataTemplate],
    properties:[
        Content:Option<ferroui_base::BoxedValue>{get:|this:&Rc<dyn IDocumentContent>|this.content() .as_ref().map(ferrodock_runtime::from_source_value),try_set:|this:&Rc<dyn IDocumentContent>,value:Option<ferroui_base::BoxedValue>|{this.set_content(value.and_then(ferroui_base::data::core::ValueTypes::normalize).map(ferrodock_runtime::to_source_value));Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
    ],
});
static METADATA: MarkupType = MarkupType {
    notify_property_changed: Some(notifier),
    ..*<dyn IDocumentContentMetadata as MarkupTyped>::MARKUP
};
fn notifier(value: &dyn AnyValue) -> Option<&dyn INotifyPropertyChanged> {
    if let Some(model) = value.downcast_ref::<Rc<dyn IDocumentContent>>() {
        return model.as_notify_property_changed();
    }
    value
        .downcast_ref::<Option<Rc<dyn IDocumentContent>>>()?
        .as_ref()?
        .as_notify_property_changed()
}
pub(crate) fn register() {
    use ferroui_base::data::core::ValueTypes;
    MarkupType::register(&METADATA);
    // The notifier descriptor is the registered identity; emitted members still
    // belong to the original public declaration carrier.
    ferroui_base::__ferro_compiler_metadata! {
        MarkupType::register_rust_paths(&[(&METADATA, "::ferrodock_model_ferro::IDocumentContentMetadata", true)]);
    }
    ValueTypes::register_nullable::<Rc<dyn IDocumentContent>>();
    ValueTypes::register_cast::<Rc<dyn IDocumentContent>, Rc<dyn IDockable>>(|d| d.clone());
    ValueTypes::register_boxed_cast::<Rc<dyn IDockable>, Rc<dyn IDocumentContent>>(|v| {
        crate::control_contract_registration::dockable(v)?.as_document_content()
    });
}
