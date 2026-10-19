//! `SplatApp` (`Shared/SplatApp.cs`): configures the support of FerroUI on a ReactiveUI builder
//! and builds the Splat application once.
//!
//! Every dependency injection integration reaches this from the application builder's
//! post-platform-setup callback. The callback can run more than once in a process that configures
//! several builders, and a second build would fail, so the build is guarded.

use crate::app_builder_extensions::ReactiveUIBuilderFerroExtensions;
use crate::FerroScheduler;
use ferroui_reactiveui::{ReactiveUIBuilder, RxAppBuilderMixins};
use ferroui_splat::{AppBuilder as SplatAppBuilder, AppLocator};

/// Configures the support of FerroUI on a ReactiveUI builder and builds the Splat application once
/// (`SplatApp`).
pub struct SplatApp;

impl SplatApp {
    /// Creates the ReactiveUI builder of FerroUI over the current resolver, applies the caller's
    /// configuration and builds it, unless an application was built already
    /// (`BuildWithFerro`).
    pub fn build_with_ferro(with_reactive_ui_builder: Option<&dyn Fn(&ReactiveUIBuilder)>) {
        FerroScheduler::install_on_ui_thread();

        let rxui_builder = AppLocator::current_mutable().create_reactive_ui_builder();
        let _ = rxui_builder.with_ferro();
        if let Some(with_reactive_ui_builder) = with_reactive_ui_builder {
            with_reactive_ui_builder(&rxui_builder);
        }

        if SplatAppBuilder::has_been_built() {
            return;
        }

        let _ = rxui_builder.build_app();
    }
}
