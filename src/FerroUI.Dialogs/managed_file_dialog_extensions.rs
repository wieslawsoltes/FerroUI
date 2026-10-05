use crate::{ManagedFileDialogOptions, ManagedStorageProvider};
use ferroui_base::platform::storage::IStorageProvider;
use ferroui_base::{FerroLocator, LocatorExtensions, ObjectType, Ref, Upcast};
use ferroui_controls::platform::{IMountedVolumeInfoProvider, IStorageProviderFactory};
use ferroui_controls::{AppBuilder, ContentControl, TopLevel, Window};
use std::rc::Rc;

/// The factory of the managed storage providers: one per top level, with
/// the options the application registered.
pub(crate) struct ManagedStorageProviderFactory {
    options: Option<ManagedFileDialogOptions>,
}

impl ManagedStorageProviderFactory {
    pub(crate) fn new(options: Option<ManagedFileDialogOptions>) -> Self {
        Self { options }
    }
}

impl IStorageProviderFactory for ManagedStorageProviderFactory {
    fn create_provider(&self, top_level: &Ref<TopLevel>) -> Rc<dyn IStorageProvider> {
        Rc::new(ManagedStorageProvider::new(Some(top_level), self.options.clone()))
    }
}

/// The managed dialogs extension of the application builder: replaces the
/// file pickers of the platform with the managed file chooser.
pub trait ManagedFileDialogExtensions {
    /// Uses the managed file chooser for the file and folder pickers of
    /// every top level, shown in a window (or in a popup over the content
    /// of a top level that is not a window).
    fn use_managed_system_dialogs(&self) -> AppBuilder;

    /// Uses the managed file chooser for the file and folder pickers of
    /// every top level, shown in a new `TWindow`.
    ///
    /// The class model states the `Window` constraint of the original; its
    /// `new()` constraint is checked when the extension is used.
    ///
    /// # Panics
    /// Panics when `TWindow` has no parameterless constructor.
    fn use_managed_system_dialogs_with<TWindow: ObjectType + Upcast<Window>>(&self) -> AppBuilder;
}

/// The options registered with the locator, with `custom_root_factory` as
/// their content root factory when both are given.
fn prepare_options(
    options_override: Option<ManagedFileDialogOptions>,
    custom_root_factory: Option<Rc<dyn Fn() -> Ref<ContentControl>>>,
) -> Option<ManagedFileDialogOptions> {
    let mut options =
        options_override.or_else(|| FerroLocator::current().get_service::<ManagedFileDialogOptions>().map(|o| (*o).clone()));
    if let (Some(options), Some(custom_root_factory)) = (options.as_mut(), custom_root_factory) {
        options.set_content_root_factory(Some(custom_root_factory));
    }

    options
}

fn use_managed_system_dialogs(builder: &AppBuilder, custom_factory: Option<Rc<dyn Fn() -> Ref<ContentControl>>>) -> AppBuilder {
    builder.after_setup(move |_| {
        let options = prepare_options(None, custom_factory.clone());
        FerroLocator::current_mutable()
            .bind::<dyn IStorageProviderFactory>()
            .to_constant(Rc::new(ManagedStorageProviderFactory::new(options.clone())));
        if let Some(custom_volume_info_provider) = options.as_ref().and_then(|options| options.custom_volume_info_provider()) {
            FerroLocator::current_mutable()
                .bind::<dyn IMountedVolumeInfoProvider>()
                .to_constant(custom_volume_info_provider.clone());
        }
    });
    builder.clone()
}

impl ManagedFileDialogExtensions for AppBuilder {
    fn use_managed_system_dialogs(&self) -> AppBuilder {
        use_managed_system_dialogs(self, None)
    }

    fn use_managed_system_dialogs_with<TWindow: ObjectType + Upcast<Window>>(&self) -> AppBuilder {
        let constructor = TWindow::TYPE
            .default_constructor()
            .unwrap_or_else(|| panic!("{} has no parameterless constructor.", TWindow::TYPE.name()));
        use_managed_system_dialogs(
            self,
            Some(Rc::new(move || {
                constructor().cast::<Window>().expect("the window type derives from Window").upcast::<ContentControl>()
            })),
        )
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the upstream project has no tests.
    use super::*;
    use ferroui_controls::platform::MountedVolumeInfo;
    use ferroui_base::collections::FerroList;
    use ferroui_base::reactive::{Disposable, IDisposable};

    struct NoVolumes;

    impl IMountedVolumeInfoProvider for NoVolumes {
        fn listen(&self, _mounted_drives: FerroList<MountedVolumeInfo>) -> Rc<dyn IDisposable> {
            Disposable::empty()
        }
    }

    /// Isolates the dispatcher and the service locator.
    struct Scope {
        locator: Rc<dyn IDisposable>,
        _dispatcher: ferroui_base::threading::UnitTestDispatcherScope,
    }

    fn scope() -> Scope {
        let dispatcher = ferroui_base::threading::Dispatcher::unit_test_scope();
        Scope { locator: FerroLocator::enter_scope(), _dispatcher: dispatcher }
    }

    impl Drop for Scope {
        fn drop(&mut self) {
            self.locator.dispose();
        }
    }

    #[test]
    fn prepare_options_prefers_the_override_and_applies_the_root_factory() {
        let _scope = scope();
        let mut registered = ManagedFileDialogOptions::new();
        registered.set_allow_directory_selection(true);
        let factory: Rc<dyn Fn() -> Ref<ContentControl>> = Rc::new(ContentControl::new);

        let prepared = prepare_options(Some(registered.clone()), Some(factory.clone())).unwrap();
        assert!(prepared.allow_directory_selection());
        assert!(std::ptr::addr_eq(Rc::as_ptr(prepared.content_root_factory().unwrap()), Rc::as_ptr(&factory)));

        // Without options, the root factory alone gives none.
        assert!(prepare_options(None, Some(factory.clone())).is_none());

        // The options registered with the locator are the default.
        FerroLocator::current_mutable().bind::<ManagedFileDialogOptions>().to_constant(Rc::new(registered));
        let prepared = prepare_options(None, Some(factory)).unwrap();
        assert!(prepared.allow_directory_selection());
        assert!(prepared.content_root_factory().is_some());
    }

    #[test]
    fn use_managed_system_dialogs_registers_the_factory_and_the_volume_provider_after_setup() {
        let _scope = scope();
        let mut options = ManagedFileDialogOptions::new();
        let provider: Rc<dyn IMountedVolumeInfoProvider> = Rc::new(NoVolumes);
        options.set_custom_volume_info_provider(Some(provider.clone()));
        FerroLocator::current_mutable().bind::<ManagedFileDialogOptions>().to_constant(Rc::new(options));

        let builder = AppBuilder::configure::<ferroui_controls::Application>().use_managed_system_dialogs();
        assert!(FerroLocator::current().get_service::<dyn IStorageProviderFactory>().is_none());
        (builder.after_setup_callback())(&builder);

        assert!(FerroLocator::current().get_service::<dyn IStorageProviderFactory>().is_some());
        let registered = FerroLocator::current().get_service::<dyn IMountedVolumeInfoProvider>().unwrap();
        assert!(std::ptr::addr_eq(Rc::as_ptr(&registered), Rc::as_ptr(&provider)));
    }
}
