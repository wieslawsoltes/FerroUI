//! Port of `ExampleAppBuilderExtensions.cs`: composes the dependency injection and ReactiveUI setup
//! shared by every platform head.

use crate::App;
use ferroui_controls::AppBuilder;
use ferroui_fonts_inter::AppBuilderExtension;
use ferroui_reactiveui::ReactiveUIBuilder;
use ferroui_reactiveui_controls::AppBuilderExtensions;
use ferroui_splat::IMutableDependencyResolver;

/// Composes the dependency injection and ReactiveUI setup shared by every platform head
/// (`ExampleAppBuilderExtensions`).
///
/// Deviation (DEVIATIONS.md, samples): the C# extension block `extension(AppBuilder builder)` is the
/// trait [`ExampleAppBuilderExt`]; the static class keeps its name, as it names no framework.
pub struct ExampleAppBuilderExtensions;

impl ExampleAppBuilderExtensions {
    /// Creates a builder for [`App`] with the shared showcase setup and no platform backend
    /// (`Create()`). Each head adds its own platform backend.
    pub fn create() -> AppBuilder {
        AppBuilder::configure::<App>().use_showcase()
    }

    /// The configuration of ReactiveUI the showcase adds to the one of FerroUI: the registration of
    /// the views and of the metrics service (`App.RegisterViews(AppLocator.CurrentMutable)`, the
    /// container callback of the reference).
    ///
    /// Deviation (DEVIATIONS.md, samples): the reference resolves its services through the
    /// Microsoft dependency injection container (`UseReactiveUIWithMicrosoftDependencyResolver`),
    /// which has no counterpart in the port; the services are registered in Splat's resolver,
    /// which ReactiveUI's builder of FerroUI configures with the core services
    /// (`WithCoreServices`).
    pub fn configure_reactive_ui(builder: &ReactiveUIBuilder) {
        let _ = builder.with_registration(|resolver: &dyn IMutableDependencyResolver| App::register_views(resolver));
    }
}

/// Extends the application builder with the shared showcase setup (`extension(AppBuilder builder)`).
pub trait ExampleAppBuilderExt {
    /// Adds the Inter font, ReactiveUI with the services of FerroUI, and the view registrations
    /// (`UseShowcase()`); the same builder, for chaining.
    fn use_showcase(&self) -> AppBuilder;
}

impl ExampleAppBuilderExt for AppBuilder {
    fn use_showcase(&self) -> AppBuilder {
        self.with_inter_font().use_reactive_ui(ExampleAppBuilderExtensions::configure_reactive_ui)
    }
}
