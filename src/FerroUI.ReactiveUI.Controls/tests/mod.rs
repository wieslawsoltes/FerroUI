//! The ported tests of the integration: one module per test file of the references, on the unit
//! test application of the framework.
//!
//! - The tests of ReactiveUI.Binding's integration (the property observer, the command binder, the
//!   view thread invoker, the module and the builder extensions).
//! - Of the tests of the integration: the activation fetcher, the reactive extensions of objects,
//!   the surface of the extension types and the scheduler of the user-interface thread.
//! - The tests of the command adapter and of the scheduler's queue, which are the port's own.

// The helpers of the tests of the binding integration, the activation, the scheduler and the
// adapter.
pub(crate) mod binding_support;

mod ferro_binding_builder_extensions_tests;
mod ferro_binding_module_tests;
mod ferro_creates_command_binding_tests;
mod ferro_object_observable_for_property_tests;
mod ferro_view_thread_invoker_tests;

mod ferro_activation_for_view_fetcher_tests;
mod ferro_object_reactive_extensions_behavior_tests;
mod ferro_ui_thread_tests_main;
mod reactive_extensions_tests;

mod ferro_scheduler_tests;
mod reactive_command_adapter_tests;

// The tests of the views, the view hosts, the hooks and the builder, and their helpers.
pub(crate) mod view_support;
mod app_builder_extensions_overloads_tests;
mod app_builder_extensions_tests;
mod app_builder_extensions_views_api_tests;
mod auto_data_template_binding_hook_tests;
mod auto_suspend_helper_shutdown_tests;
mod auto_suspend_helper_tests;
mod reactive_controls_tests;
mod reactive_user_control_activation_tests;
mod reactive_view_controls_binding_tests;
mod reactive_window_activation_tests;
mod reactive_window_user_control_binding_tests;
mod view_hosts_navigation_tests;
mod app_builder_extensions_registration_tests;
mod not_ported;
