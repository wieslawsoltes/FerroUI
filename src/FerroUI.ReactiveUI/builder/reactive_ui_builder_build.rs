//! The building of `ReactiveUIBuilder` (`ReactiveUI.Shared/Builder/ReactiveUIBuilder.Build.cs`).
//!
//! Upstream's `WithInstance` overloads of the builder are those of every built application,
//! [`ReactiveUIInstanceMixins::with_instance`](super::ReactiveUIInstanceMixins::with_instance),
//! which the builder has as an [`IReactiveUIInstance`](super::IReactiveUIInstance).

use super::{ReactiveUIBuilder, RxAppBuilder};
use crate::{BindingConverters, MessageBus};
use ferroui_splat::{AppBuilder, IAppBuilder};

impl ReactiveUIBuilder {
    /// Builds the application once, with its core services and registrations, then initializes
    /// the state of ReactiveUI: the limits of the caches, the handler of the errors, the
    /// suspension host, the converters of the bindings and the message bus; and marks ReactiveUI as
    /// initialized ([`RxAppBuilder::ensure_initialized`]). A second build returns the application
    /// as it is (`Build()`).
    ///
    /// # Panics
    /// When the application has no resolver (`InvalidOperationException`).
    pub fn build(&self) -> &Self {
        let already_built = AppBuilder::has_been_built();
        let app_instance = IAppBuilder::build(self);
        assert!(app_instance.current().is_some(), "InvalidOperationException: Failed to create ReactiveUIInstance instance");

        if already_built {
            return self;
        }

        self.initialize_static_state();
        BindingConverters::set_service(self.converter_service.clone());

        if let Some(message_bus) = self.state.message_bus.borrow().clone() {
            MessageBus::set_current(message_bus);
        }

        RxAppBuilder::mark_as_initialized();
        self
    }

    /// Builds the application (`BuildApp()`), as [`build`](Self::build) does.
    pub fn build_app(&self) -> &Self {
        self.build()
    }
}
