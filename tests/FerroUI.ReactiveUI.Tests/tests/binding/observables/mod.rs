//! The tests of `ReactiveUI.Binding.Tests/Observables`: one module per upstream test file, and the
//! recording observer they share (`TestModels/EmissionRecorder.cs`).

mod applied_change_observable_tests;
mod callback_property_observable_tests;
mod deferred_property_observable_tests;
mod emission_recorder;
mod event_observable_tests;
mod initial_binding_signal_tests;
mod plugin_observation_source_tests;
mod plugin_property_observable_tests;
mod property_changing_observable_initial_emit_serialization_tests;
mod property_changing_observable_tests;
mod property_observable_initial_emit_serialization_tests;
mod property_observable_tests;
mod unchanging_property_observable_tests;
mod view_thread_observable_tests;
