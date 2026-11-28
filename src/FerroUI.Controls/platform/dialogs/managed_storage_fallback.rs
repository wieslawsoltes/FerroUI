//! The last storage provider of a window on a platform whose file pickers can be missing (X11 and
//! Wayland without a portal or GTK). Not from upstream (#612).
//!
//! The managed file chooser is found through the theme: the unit of a theme that styles the
//! chooser also holds, under [`MANAGED_STORAGE_FALLBACK_KEY`], the factory of its storage
//! provider. A backend names neither, so an application whose themes do not keep that unit does
//! not link the chooser, and cannot open it without its template.

use super::IStorageProviderFactory;
use crate::TopLevel;
use ferroui_base::controls::ResourceKey;
use ferroui_base::logging::{LogEventLevel, Logger};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::platform::storage::{IStorageProvider, NoopStorageProvider};
use ferroui_base::Ref;
use std::rc::Rc;

/// The key of the factory of the managed storage provider in a theme.
pub const MANAGED_STORAGE_FALLBACK_KEY: &str = "ManagedStorageFallback";

/// The managed storage provider of `top_level`, from the factory its theme holds. Without one,
/// a provider with no pickers, which still finds files and folders by path: a picker call then
/// ends in the error of a storage provider with nothing suitable.
pub fn managed_storage_fallback(top_level: &Ref<TopLevel>) -> Rc<dyn IStorageProvider> {
    let key = ResourceKey::from(MANAGED_STORAGE_FALLBACK_KEY);
    let found = top_level.try_find_resource(&key, top_level.actual_theme_variant().as_ref());
    match found.and_then(|value| from_markup_value::<Rc<dyn IStorageProviderFactory>>(&value)) {
        Some(factory) => factory.create_provider(top_level),
        None => {
            if cfg!(debug_assertions) {
                if let Some(logger) = Logger::try_get(LogEventLevel::Warning, "Platform") {
                    logger.log(
                        None,
                        "No file picker is available: the platform has none, and the themes of the application \
                         do not keep ManagedFileChooser (add it to `theme-roots`).",
                    );
                }
            }
            Rc::new(NoopStorageProvider)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{TestServices, UnitTestApplication};
    use crate::Window;
    use ferroui_base::platform::storage::{FallbackStorageProvider, FilePickerOpenOptions, StorageProviderFactory};
    use ferroui_base::threading::Dispatcher;
    use ferroui_base::{BoxedValue, Upcast};
    use std::cell::{Cell, RefCell};

    /// A factory of providers that answer as a managed one would.
    struct Factory {
        made: Rc<Cell<u32>>,
    }

    impl IStorageProviderFactory for Factory {
        fn create_provider(&self, _top_level: &Ref<TopLevel>) -> Rc<dyn IStorageProvider> {
            self.made.set(self.made.get() + 1);
            Rc::new(NoopStorageProvider)
        }
    }

    #[test]
    fn the_factory_of_the_theme_makes_the_provider() {
        let _scope = UnitTestApplication::start(TestServices::styled_window());
        let window = Window::new();
        let made = Rc::new(Cell::new(0));
        let factory: Rc<dyn IStorageProviderFactory> = Rc::new(Factory { made: made.clone() });
        let value: BoxedValue = Rc::new(factory);
        window.resources().add(ResourceKey::from(MANAGED_STORAGE_FALLBACK_KEY), Some(value));

        let top_level: Ref<TopLevel> = window.upcast();
        let _provider = managed_storage_fallback(&top_level);
        assert_eq!(made.get(), 1);
    }

    /// A theme of the application that has no chooser: no picker, no panic, and the error of a
    /// storage provider with nothing suitable.
    #[test]
    fn without_a_chooser_in_the_themes_a_picker_call_reports_that_none_is_available() {
        let _scope = UnitTestApplication::start(TestServices::styled_window());
        let top_level: Ref<TopLevel> = Window::new().upcast();

        let provider = managed_storage_fallback(&top_level);
        assert!(!provider.can_open() && !provider.can_save() && !provider.can_pick_folder());

        let last: StorageProviderFactory = {
            let top_level = top_level.clone();
            Rc::new(move || {
                let provider = managed_storage_fallback(&top_level);
                Box::pin(std::future::ready(Some(provider)))
            })
        };
        let chain = FallbackStorageProvider::new(vec![last]);
        let picked = chain.open_file_picker_async(FilePickerOpenOptions::new());
        let result = Rc::new(RefCell::new(None));
        let sink = result.clone();
        Dispatcher::ui_thread().spawn_local(async move {
            *sink.borrow_mut() = Some(picked.await.map(|files| files.len()));
        });
        Dispatcher::ui_thread().run_jobs(None);
        let error = result.borrow_mut().take().expect("the call finished").expect_err("no provider is suitable");
        assert_eq!(error.to_string(), "Unable to select a suitable storage provider");
    }
}
