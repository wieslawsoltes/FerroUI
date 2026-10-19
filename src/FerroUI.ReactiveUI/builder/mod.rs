//! The builder of a ReactiveUI application (`ReactiveUI.Shared/Builder`, with
//! `Interfaces/IViewModule.cs` and `Mixins/BuilderMixins*.cs`): Splat's application builder with
//! the services, schedulers, converters and state of ReactiveUI, the replacement of the former
//! `RxApp` initialization.
//!
//! ```
//! use ferroui_reactiveui::RxAppBuilder;
//!
//! RxAppBuilder::reset_for_testing();
//! RxAppBuilder::create_reactive_ui_builder().with_core_services().build_app();
//! RxAppBuilder::ensure_initialized();
//! ```
//!
//! Not ported: `WithViewsFromAssembly(Assembly)`, which scans an assembly for views (reflection).

mod builder_mixins;
mod builder_mixins_instance;
mod i_reactive_ui_builder;
mod i_reactive_ui_instance;
mod i_view_module;
mod reactive_ui_builder;
mod reactive_ui_builder_build;
mod rx_app_builder;
mod rx_app_builder_mixins;

pub use builder_mixins::ReactiveUIBuilderMixins;
pub use builder_mixins_instance::{ReactiveUIInstanceMixins, ResolveServices};
pub use i_reactive_ui_builder::{IReactiveUIBuilder, ReactiveUIBuilderExt, ResolverAction};
pub use i_reactive_ui_instance::IReactiveUIInstance;
pub use i_view_module::IViewModule;
pub use reactive_ui_builder::ReactiveUIBuilder;
pub use rx_app_builder::RxAppBuilder;
pub use rx_app_builder_mixins::RxAppBuilderMixins;
