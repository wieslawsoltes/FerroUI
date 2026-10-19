//! `RxAppBuilder` (`ReactiveUI.Shared/Builder/RxAppBuilder.cs`).

use super::ReactiveUIBuilder;
use crate::RxSchedulers;
use ferroui_splat::{AppBuilder, AppLocator, ModernDependencyResolver};
use std::cell::Cell;
use std::rc::Rc;

ferroui_system::ferro_thread_local! {
    /// Whether an application was built with a ReactiveUI builder (`_hasBeenInitialized`).
    static HAS_BEEN_INITIALIZED: Cell<bool> = const { Cell::new(false) };
}

/// The message of [`RxAppBuilder::ensure_initialized`].
const NOT_INITIALIZED_MESSAGE: &str = "ReactiveUI has not been initialized. You must initialize ReactiveUI using the builder pattern. \
See https://www.reactiveui.net/docs/handbook/rxappbuilder.html for migration guidance.\n\n\
Example:\n\
RxAppBuilder.CreateReactiveUIBuilder()\n    .WithCoreServices()\n    .WithPlatformServices()\n    .BuildApp();";

/// The start of the configuration of ReactiveUI (`RxAppBuilder`): the builder over the current
/// locator, and the check that an application was built.
///
/// Deviation (DEVIATIONS.md, ReactiveUI): the state of the initialization is per thread, as the
/// locator of Splat is (`docs/porting/reactive.md`, section 5.1).
pub struct RxAppBuilder;

impl RxAppBuilder {
    /// A builder over the current resolver of Splat's [`AppLocator`] (`CreateReactiveUIBuilder()`).
    pub fn create_reactive_ui_builder() -> ReactiveUIBuilder {
        ReactiveUIBuilder::new(AppLocator::current_mutable(), Some(AppLocator::current()))
    }

    /// Checks that ReactiveUI was initialized by building an application (`EnsureInitialized`), the
    /// replacement of upstream's former `RxApp.EnsureInitialized()`.
    ///
    /// # Panics
    /// When no application was built (`InvalidOperationException`), with the message that shows
    /// how to build one.
    pub fn ensure_initialized() {
        if !HAS_BEEN_INITIALIZED.with(Cell::get) {
            panic!("InvalidOperationException: {NOT_INITIALIZED_MESSAGE}");
        }
    }

    /// Resets the state of the initialization, for tests (`ResetForTesting`): the schedulers, the
    /// state of Splat's builder, a new locator and the initialization flag.
    ///
    /// Deviation (DEVIATIONS.md, ReactiveUI): upstream's method is internal; it is public and hidden
    /// from the documentation here, so that the tests, in their own crate, reach it.
    #[doc(hidden)]
    pub fn reset_for_testing() {
        RxSchedulers::reset_for_testing();

        AppBuilder::reset_builder_state_for_tests();

        AppLocator::set_locator(Rc::new(ModernDependencyResolver::new()));

        // Deviation (DEVIATIONS.md, ReactiveUI): upstream also clears the cache of the activation
        // fetchers of `ViewForMixins` (`ResetActivationFetcherCacheForTesting`); the port of
        // `ViewForMixins` finds the fetcher each time and keeps no cache to clear.

        HAS_BEEN_INITIALIZED.with(|flag| flag.set(false));
    }

    /// Marks ReactiveUI as initialized; the build of the builder calls it (`MarkAsInitialized`).
    pub(crate) fn mark_as_initialized() {
        HAS_BEEN_INITIALIZED.with(|flag| flag.set(true));
    }
}
