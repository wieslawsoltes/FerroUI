//! Native metadata for the original framework-independent recycling contracts.
//! The empty carrier traits have no runtime objects or behavioral role: their
//! descriptors expose the ORIGINAL model trait handles. This keeps UI metadata
//! out of the model crate and satisfies Rust's orphan rule for MarkupTyped.
use ferrodock_controls_recycling_model::{IControlRecycling, IControlRecyclingIdProvider};
use std::rc::Rc;

/// Metadata carrier for the original IControlRecycling contract handles.
#[doc(hidden)]
pub trait IControlRecyclingMetadata {}
/// Metadata carrier for the original ID provider contract handles.
#[doc(hidden)]
pub trait IControlRecyclingIdProviderMetadata {}
ferroui_base::ferro_markup_type!(interface dyn IControlRecyclingMetadata as "IControlRecycling" {
    namespace:"FerroUI.Controls.Recycling.Model",
    this:Rc<dyn IControlRecycling>,handles:[Rc<dyn IControlRecycling>,Option<Rc<dyn IControlRecycling>>],
    properties:[TryToUseIdAsKey:bool{get:|this:&Rc<dyn IControlRecycling>|this.try_to_use_id_as_key(),set:|this:&Rc<dyn IControlRecycling>,value|this.set_try_to_use_id_as_key(value)}],
    methods:[
        fn Add(ferroui_base::BoxedValue,ferroui_base::BoxedValue) => (|this:&Rc<dyn IControlRecycling>,data,control|this.add(ferrodock_runtime::to_source_value(data),ferrodock_runtime::to_source_value(control))),
        fn Build(Option<ferroui_base::BoxedValue>,Option<ferroui_base::BoxedValue>,Option<ferroui_base::BoxedValue>)->Option<ferroui_base::BoxedValue> => (|this:&Rc<dyn IControlRecycling>,data:Option<ferroui_base::BoxedValue>,existing:Option<ferroui_base::BoxedValue>,parent:Option<ferroui_base::BoxedValue>|this.build(data.map(ferrodock_runtime::to_source_value),existing.map(ferrodock_runtime::to_source_value),parent.map(ferrodock_runtime::to_source_value)).as_ref().map(ferrodock_runtime::from_source_value)),
        fn Clear() => (|this:&Rc<dyn IControlRecycling>|this.clear()),
    ],
});
ferroui_base::ferro_markup_type!(interface dyn IControlRecyclingIdProviderMetadata as "IControlRecyclingIdProvider" {
    namespace:"FerroUI.Controls.Recycling.Model",
    this:Rc<dyn IControlRecyclingIdProvider>,handles:[Rc<dyn IControlRecyclingIdProvider>,Option<Rc<dyn IControlRecyclingIdProvider>>],
    methods:[fn GetControlRecyclingId()->Option<String> => (|this:&Rc<dyn IControlRecyclingIdProvider>|this.get_control_recycling_id())],
});

// TryGetValue retains a source out parameter; native by-reference method metadata
// remains unported. Do not expose a different return signature as that source method.
