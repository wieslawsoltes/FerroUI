// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::controls::IToolTemplate;
use ferroui_base::{
    metadata::{MarkupType, MarkupTyped},
    AnyValue,
};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::rc::Rc;
/// Native descriptor retaining the original `IToolTemplate` receiver.
#[doc(hidden)]
pub trait IToolTemplateMetadata {}
ferroui_base::ferro_markup_type!(interface dyn IToolTemplateMetadata as "IToolTemplate" {
 namespace:"FerroDock.Model.Controls",this:Rc<dyn IToolTemplate>,handles:[Rc<dyn IToolTemplate>,Option<Rc<dyn IToolTemplate>>],
 properties:[Content:Option<ferroui_base::BoxedValue>{get:|this:&Rc<dyn IToolTemplate>|this.content().as_ref().map(ferrodock_runtime::from_source_value),set:|this:&Rc<dyn IToolTemplate>,value:Option<ferroui_base::BoxedValue>|this.set_content(value.and_then(ferroui_base::data::core::ValueTypes::normalize).map(ferrodock_runtime::to_source_value))},],

});
static METADATA: MarkupType = MarkupType {
    notify_property_changed: Some(notifier),
    ..*<dyn IToolTemplateMetadata as MarkupTyped>::MARKUP
};
fn notifier(value: &dyn AnyValue) -> Option<&dyn INotifyPropertyChanged> {
    if let Some(model) = value.downcast_ref::<Rc<dyn IToolTemplate>>() {
        return model.as_notify_property_changed();
    }
    value
        .downcast_ref::<Option<Rc<dyn IToolTemplate>>>()?
        .as_ref()?
        .as_notify_property_changed()
}
pub(crate) fn register() {
    use ferroui_base::data::core::ValueTypes;
    MarkupType::register(&METADATA);
    // The notifier descriptor is the registered identity; emitted members still
    // belong to the original public declaration carrier.
    ferroui_base::__ferro_compiler_metadata! {
        MarkupType::register_rust_paths(&[(&METADATA, "::ferrodock_model_ferro::IToolTemplateMetadata", true)]);
    }
    ValueTypes::register_nullable::<Rc<dyn IToolTemplate>>();
}
