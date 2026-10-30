// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::controls::IToolDockContent;
use ferrodock_model::core::IDock;
use ferroui_base::{
    metadata::{MarkupType, MarkupTyped},
    AnyValue,
};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::rc::Rc;
/// Native descriptor retaining the original `IToolDockContent` receiver.
#[doc(hidden)]
pub trait IToolDockContentMetadata {}
ferroui_base::ferro_markup_type!(interface dyn IToolDockContentMetadata as "IToolDockContent" {
 namespace:"FerroDock.Model.Controls",this:Rc<dyn IToolDockContent>,handles:[Rc<dyn IToolDockContent>,Option<Rc<dyn IToolDockContent>>],base:Rc<dyn IDock>,interfaces:[Rc<dyn IDock>],
 properties:[ToolTemplate:Option<Rc<dyn ferrodock_model::controls::IToolTemplate>>{get:|this:&Rc<dyn IToolDockContent>|this.tool_template(),set:|this:&Rc<dyn IToolDockContent>,value:Option<Rc<dyn ferrodock_model::controls::IToolTemplate>>|this.set_tool_template(value)},],
 methods:[fn CreateToolFromTemplate()->Option<ferroui_base::BoxedValue> => (|this:&Rc<dyn IToolDockContent>|this.create_tool_from_template().as_ref().map(ferrodock_runtime::from_source_value))],
});
static METADATA: MarkupType = MarkupType {
    notify_property_changed: Some(notifier),
    ..*<dyn IToolDockContentMetadata as MarkupTyped>::MARKUP
};
fn notifier(value: &dyn AnyValue) -> Option<&dyn INotifyPropertyChanged> {
    if let Some(model) = value.downcast_ref::<Rc<dyn IToolDockContent>>() {
        return model.as_notify_property_changed();
    }
    value
        .downcast_ref::<Option<Rc<dyn IToolDockContent>>>()?
        .as_ref()?
        .as_notify_property_changed()
}
pub(crate) fn register() {
    use ferroui_base::data::core::ValueTypes;
    MarkupType::register(&METADATA);
    // The notifier descriptor is the registered identity; emitted members still
    // belong to the original public declaration carrier.
    ferroui_base::__ferro_compiler_metadata! {
        MarkupType::register_rust_paths(&[(&METADATA, "::ferrodock_model_ferro::IToolDockContentMetadata", true)]);
    }
    ValueTypes::register_nullable::<Rc<dyn IToolDockContent>>();
    ValueTypes::register_cast::<Rc<dyn IToolDockContent>, Rc<dyn IDock>>(|v| v.clone());
    use ferrodock_model::core::IDockable;
    ValueTypes::register_cast::<Rc<dyn IToolDockContent>, Rc<dyn IDockable>>(|v| v.clone());
    ValueTypes::register_boxed_cast::<Rc<dyn IDockable>, Rc<dyn IToolDockContent>>(|v| {
        crate::control_contract_registration::dockable(v)?.as_tool_dock_content()
    });
    ValueTypes::register_boxed_cast::<Rc<dyn IDock>, Rc<dyn IToolDockContent>>(|v| {
        crate::control_contract_registration::dockable(v)?.as_tool_dock_content()
    });
}
