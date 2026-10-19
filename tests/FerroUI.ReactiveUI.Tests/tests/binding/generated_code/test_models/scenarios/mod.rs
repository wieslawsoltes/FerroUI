//! The scenarios of the generated-code tests (`ReactiveUI.Binding.GeneratedCode.TestModels/Scenarios`):
//! one function per observation, which upstream's generator turns into generated code and which
//! here builds the observation over property paths. Only the scenarios that a test uses are
//! ported; each file names the others.
//!
//! [`to_property_scenarios`] holds the view models of `SharedScenarios/ToProperty`, which the
//! generated-code tests of `ToProperty` use.

pub mod to_property_scenarios;
pub mod when_any_observable_scenarios;
pub mod when_any_scenarios;
pub mod when_any_value_extended_scenarios;
pub mod when_any_value_scenarios;
pub mod when_changed_extended_scenarios;
pub mod when_changed_scenarios;
pub mod when_changing_extended_scenarios;
pub mod when_changing_scenarios;

// Not ported: BindCommandScenarios, BindCompatScenarios, BindInteractionScenarios,
// BindOneWayScenarios, BindToScenarios, BindTwoWayScenarios, InvokeCommandScenarios,
// OneWayBindCompatScenarios - the scenarios of the binding tests, which belong to another work
// package.
