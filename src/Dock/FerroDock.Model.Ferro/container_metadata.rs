//! Native metadata over original generated-container values and shared object conversion.
use std::rc::Rc;
/// Core-owned descriptor for the original optional generated-container metadata contract.
#[doc(hidden)]
pub trait IDockItemContainerMetadataDescriptor {}
ferroui_base::ferro_markup_type!(interface dyn IDockItemContainerMetadataDescriptor as "IDockItemContainerMetadata" {
    namespace:"FerroDock.Model.Core",
    this: Rc<dyn ferrodock_model::core::IDockItemContainerMetadata>,
    handles:[Rc<dyn ferrodock_model::core::IDockItemContainerMetadata>,Option<Rc<dyn ferrodock_model::core::IDockItemContainerMetadata>>],
    properties:[
        ItemContainerTheme:Option<ferroui_base::BoxedValue>{
            get:|this:&Rc<dyn ferrodock_model::core::IDockItemContainerMetadata>|this.item_container_theme().as_ref().map(ferrodock_runtime::from_source_value),
            set:|this:&Rc<dyn ferrodock_model::core::IDockItemContainerMetadata>,value:Option<ferroui_base::BoxedValue>|this.set_item_container_theme(value.map(ferrodock_runtime::to_source_value))
        },
        ItemTemplateSelector:Option<ferroui_base::BoxedValue>{
            get:|this:&Rc<dyn ferrodock_model::core::IDockItemContainerMetadata>|this.item_template_selector().as_ref().map(ferrodock_runtime::from_source_value),
            set:|this:&Rc<dyn ferrodock_model::core::IDockItemContainerMetadata>,value:Option<ferroui_base::BoxedValue>|this.set_item_template_selector(value.map(ferrodock_runtime::to_source_value))
        }
    ]
});

pub(crate) fn register() {
    use ferrodock_model::core::IDockItemContainerMetadata;
    use ferroui_base::{
        data::core::ValueTypes,
        metadata::{MarkupType, MarkupTyped},
    };
    MarkupType::register(<dyn IDockItemContainerMetadataDescriptor>::MARKUP);
    ValueTypes::register_nullable::<Rc<dyn IDockItemContainerMetadata>>();
    ValueTypes::register_reference_identity::<Rc<dyn IDockItemContainerMetadata>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
}
