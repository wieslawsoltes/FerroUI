//! ferroui-reactiveui-controls
//!
//! The integration of ReactiveUI (`ferroui-reactiveui`) with FerroUI, ported from the integration
//! of ReactiveUI with the reference UI framework, together with the matching parts of
//! ReactiveUI.Binding and ReactiveUI.Primitives:
//!
//! - the scheduler of the user-interface thread, [`FerroScheduler`], which the application sets as
//!   `RxSchedulers::main_thread_scheduler`, and [`FerroScheduler::install_on_ui_thread`], which
//!   makes the dispatcher's synchronization context the context of the user-interface thread;
//! - the property observation of FerroUI objects ([`FerroObjectObservableForProperty`]), the
//!   command binding of FerroUI controls ([`FerroCreatesCommandBinding`], with
//!   [`ReactiveCommandAdapter`], a command of ReactiveUI as a command of FerroUI) and the view
//!   thread invoker ([`FerroViewThreadInvoker`]), which [`FerroBindingModule`] (or
//!   [`register_binding_integration`]) registers;
//! - the activation of the views of FerroUI ([`FerroActivationForViewFetcher`]);
//! - the properties of FerroUI objects as signals ([`FerroObjectReactiveExtensions`]);
//! - property paths through the properties of FerroUI objects ([`ferro_property`],
//!   [`ferro_prop!`]) and the lookup of the FerroUI object behind an untyped object of ReactiveUI
//!   ([`FerroObjectHandles`]);
//! - the reactive views, the view hosts, the data-template hook, suspension and `use_reactive_ui`
//!   on the application builder.

mod ferro_activation_for_view_fetcher;
mod ferro_binding_builder_extensions;
mod ferro_binding_module;
mod ferro_creates_command_binding;
mod ferro_object_handles;
mod ferro_object_observable_for_property;
mod ferro_object_reactive_extensions;
mod ferro_property_path;
mod ferro_property_subscription;
mod ferro_scheduler;
mod ferro_view_thread_invoker;
mod reactive_command_adapter;

pub use ferro_activation_for_view_fetcher::FerroActivationForViewFetcher;
pub use ferro_binding_builder_extensions::{FerroAppBuilderExtensions, FerroBindingBuilderExtensions};
pub use ferro_binding_module::{register_binding_integration, FerroBindingModule};
pub use ferro_creates_command_binding::{control_target, control_type, FerroCreatesCommandBinding};
pub use ferro_object_handles::FerroObjectHandles;
pub use ferro_object_observable_for_property::FerroObjectObservableForProperty;
pub use ferro_object_reactive_extensions::{FerroObjectReactiveExtensions, FerroPropertySignal};
pub use ferro_property_path::{ferro_property, ferro_property_owner, observe_ferro_property};
pub use ferro_property_subscription::FerroPropertySubscription;
pub use ferro_scheduler::FerroScheduler;
pub use ferro_view_thread_invoker::FerroViewThreadInvoker;
pub use reactive_command_adapter::{boxed_parameter, parameter_as_any, ReactiveCommandAdapter, ReactiveCommandParameter};

mod app_builder_extensions;
mod bindable_view;
mod auto_data_template_binding_hook;
mod auto_suspend_helper;
mod navigation_target;
mod reactive_user_control;
mod reactive_user_control_base;
mod reactive_view;
mod reactive_window;
mod reactive_window_base;
mod register_types;
mod routed_view_host;
mod rust_paths;
mod splat_app;
mod startup_configuration;
mod subscription_errors;
mod view_control;
mod view_host_navigation;
mod view_model_property_sync;
mod view_model_value;
mod view_model_view_host;
mod view_registrar;

pub use app_builder_extensions::{AppBuilderExtensions, ReactiveUIBuilderFerroExtensions};
pub use auto_data_template_binding_hook::AutoDataTemplateBindingHook;
pub use bindable_view::{element_control, element_property, owned_element_property, BindableView};
pub use auto_suspend_helper::AutoSuspendHelper;
pub use navigation_target::NavigationTarget;
pub use reactive_user_control::ReactiveUserControl;
pub use reactive_user_control_base::ReactiveUserControlBase;
pub use reactive_view::{IReactiveViewClass, ITypedReactiveViewClass, ReactiveView, ReactiveViewFor, ViewModelFilter};
pub use reactive_window::ReactiveWindow;
pub use reactive_window_base::ReactiveWindowBase;
pub use register_types::{register_types, ASSEMBLY, REACTIVEUI_XML_NAMESPACE};
pub use routed_view_host::RoutedViewHost;
pub use splat_app::SplatApp;
pub use startup_configuration::StartupConfiguration;
pub use subscription_errors::SubscriptionErrors;
pub use view_control::{register_view_control, view_control, ControlViewFor, IControlViewFor, IViewControl, ViewLink};
pub use view_host_navigation::{ViewForValue, ViewHostNavigation};
pub use view_model_property_sync::ViewModelPropertySync;
pub use view_model_value::{is_view_model_of, register_view_model, same_view_model, same_view_model_object, value_of_optional_view_model, value_of_view_model, view_model_of_optional, view_model_of_value, UntypedViewModel};
pub use view_model_view_host::ViewModelViewHost;
pub use view_registrar::{ViewAssembly, ViewAssemblyType, ViewRegistrar};

/// What the macros of the crate expand to, so that a crate that uses them needs no other
/// dependency.
#[doc(hidden)]
pub mod __private {
    pub use ferroui_base::Ref;
    pub use ferroui_system::component_model::property_name_of;
}

#[cfg(test)]
mod tests;
