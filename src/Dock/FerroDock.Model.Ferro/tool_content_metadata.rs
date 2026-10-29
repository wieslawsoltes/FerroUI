// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::{controls::IToolContent, core::IDockable};
use ferroui_base::{
    metadata::{MarkupType, MarkupTyped},
    AnyValue,
};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::rc::Rc;
/// Native descriptor retaining the original IToolContent receiver.
#[doc(hidden)]
pub trait IToolContentMetadata {}
ferroui_base::ferro_markup_type!(interface dyn IToolContentMetadata as "IToolContent" {
    namespace:"FerroDock.Model.Controls",this:Rc<dyn IToolContent>,handles:[Rc<dyn IToolContent>,Option<Rc<dyn IToolContent>>],base:Rc<dyn IDockable>,interfaces:[Rc<dyn IDockable>],
    attributes:[RequiresDataTemplate],
    properties:[
        Content:Option<ferroui_base::BoxedValue>{get:|this:&Rc<dyn IToolContent>|this.content() .as_ref().map(ferrodock_runtime::from_source_value),try_set:|this:&Rc<dyn IToolContent>,value:Option<ferroui_base::BoxedValue>|{this.set_content(value.and_then(ferroui_base::data::core::ValueTypes::normalize).map(ferrodock_runtime::to_source_value));Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
    ],
});
static METADATA: MarkupType = MarkupType {
    notify_property_changed: Some(notifier),
    ..*<dyn IToolContentMetadata as MarkupTyped>::MARKUP
};
fn notifier(value: &dyn AnyValue) -> Option<&dyn INotifyPropertyChanged> {
    if let Some(model) = value.downcast_ref::<Rc<dyn IToolContent>>() {
        return model.as_notify_property_changed();
    }
    value
        .downcast_ref::<Option<Rc<dyn IToolContent>>>()?
        .as_ref()?
        .as_notify_property_changed()
}
pub(crate) fn register() {
    use ferroui_base::data::core::ValueTypes;
    MarkupType::register(&METADATA);
    // The notifier descriptor is the registered identity; emitted members still
    // belong to the original public declaration carrier.
    ferroui_base::__ferro_compiler_metadata! {
        MarkupType::register_rust_paths(&[(&METADATA, "::ferrodock_model_ferro::IToolContentMetadata", true)]);
    }
    ValueTypes::register_nullable::<Rc<dyn IToolContent>>();
    ValueTypes::register_cast::<Rc<dyn IToolContent>, Rc<dyn IDockable>>(|d| d.clone());
    ValueTypes::register_boxed_cast::<Rc<dyn IDockable>, Rc<dyn IToolContent>>(|v| {
        crate::control_contract_registration::dockable(v)?.as_tool_content()
    });
}
