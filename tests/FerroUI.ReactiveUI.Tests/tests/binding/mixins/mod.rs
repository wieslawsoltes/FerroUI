//! The tests of `ReactiveUI.Binding.Tests/Mixins`, one module per upstream test file.
//!
//! `RefusalAssertions.cs` holds the assertion of the refusal tests and no test; it is not ported,
//! as the refusal tests are not.

mod observed_property_tests;
mod to_property_dispatch_stub_tests;
mod when_any_dynamic_tests;
mod when_any_observable_refusal_tests;
mod when_any_observable_wide_arity_tests;
mod when_any_refusal_tests;
mod when_any_value_refusal_tests;
mod when_any_value_wide_arity_tests;
mod when_any_wide_arity_tests;
mod when_changed_refusal_tests;
mod when_changed_wide_arity_tests;
mod when_changing_refusal_tests;
mod when_changing_wide_arity_tests;
mod invoke_command_tests;
mod binding_dispatch_stub_tests;
mod indexer_unsafe_binding_tests;
mod poco_link_unsafe_binding_tests;
mod queued_two_way_binding_settling_tests;
mod scheduler_unsafe_binding_tests;
mod trigger_update_unsafe_binding_tests;
mod view_model_rooted_unsafe_binding_tests;
