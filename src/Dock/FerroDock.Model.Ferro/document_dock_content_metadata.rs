// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::controls::IDocumentDockContent;
use ferrodock_model::core::IDock;
use ferroui_base::{
    metadata::{MarkupType, MarkupTyped},
    AnyValue,
};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::rc::Rc;
/// Native descriptor retaining the original `IDocumentDockContent` receiver.
#[doc(hidden)]
pub trait IDocumentDockContentMetadata {}
ferroui_base::ferro_markup_type!(interface dyn IDocumentDockContentMetadata as "IDocumentDockContent" {
 namespace:"FerroDock.Model.Controls",this:Rc<dyn IDocumentDockContent>,handles:[Rc<dyn IDocumentDockContent>,Option<Rc<dyn IDocumentDockContent>>],base:Rc<dyn IDock>,interfaces:[Rc<dyn IDock>],
 properties:[DocumentTemplate:Option<Rc<dyn ferrodock_model::controls::IDocumentTemplate>>{get:|this:&Rc<dyn IDocumentDockContent>|this.document_template(),set:|this:&Rc<dyn IDocumentDockContent>,value:Option<Rc<dyn ferrodock_model::controls::IDocumentTemplate>>|this.set_document_template(value)},],
 methods:[fn CreateDocumentFromTemplate()->Option<ferroui_base::BoxedValue> => (|this:&Rc<dyn IDocumentDockContent>|this.create_document_from_template().as_ref().map(ferrodock_runtime::from_source_value))],
});
static METADATA: MarkupType = MarkupType {
    notify_property_changed: Some(notifier),
    ..*<dyn IDocumentDockContentMetadata as MarkupTyped>::MARKUP
};
fn notifier(value: &dyn AnyValue) -> Option<&dyn INotifyPropertyChanged> {
    if let Some(model) = value.downcast_ref::<Rc<dyn IDocumentDockContent>>() {
        return model.as_notify_property_changed();
    }
    value
        .downcast_ref::<Option<Rc<dyn IDocumentDockContent>>>()?
        .as_ref()?
        .as_notify_property_changed()
}
pub(crate) fn register() {
    use ferroui_base::data::core::ValueTypes;
    MarkupType::register(&METADATA);
    // The notifier descriptor is the registered identity; emitted members still
    // belong to the original public declaration carrier.
    ferroui_base::__ferro_compiler_metadata! {
        MarkupType::register_rust_paths(&[(&METADATA, "::ferrodock_model_ferro::IDocumentDockContentMetadata", true)]);
    }
    ValueTypes::register_nullable::<Rc<dyn IDocumentDockContent>>();
    ValueTypes::register_cast::<Rc<dyn IDocumentDockContent>, Rc<dyn IDock>>(|v| v.clone());
    use ferrodock_model::core::IDockable;
    ValueTypes::register_cast::<Rc<dyn IDocumentDockContent>, Rc<dyn IDockable>>(|v| v.clone());
    ValueTypes::register_boxed_cast::<Rc<dyn IDockable>, Rc<dyn IDocumentDockContent>>(|v| {
        crate::control_contract_registration::dockable(v)?.as_document_dock_content()
    });
    ValueTypes::register_boxed_cast::<Rc<dyn IDock>, Rc<dyn IDocumentDockContent>>(|v| {
        crate::control_contract_registration::dockable(v)?.as_document_dock_content()
    });
}
