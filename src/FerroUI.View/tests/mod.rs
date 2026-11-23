//! Tests of the declarative API: the reactive runtime, the builders and
//! mounting, children that follow state, and the macros.

mod builder_tests;
mod dynamic_tests;
mod macro_tests;
mod model_binding_tests;
mod path_tests;
mod reactive_tests;
mod selector_tests;
mod rerender_tests;
mod resource_tests;
mod route_tests;
mod template_tests;
mod values_tests;
mod api_tests;
mod tools_tests;
mod performance_tests;
mod patterns_tests;
#[cfg(feature = "inking")]
mod inking_tests;
