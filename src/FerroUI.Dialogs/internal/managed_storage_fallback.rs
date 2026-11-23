use crate::ManagedStorageProvider;
use ferroui_base::data::core::{ValueTypeRegistration, ValueTypes};
use ferroui_base::platform::storage::IStorageProvider;
use ferroui_base::{ferro_markup_type, Ref};
use ferroui_controls::platform::IStorageProviderFactory;
use ferroui_controls::TopLevel;
use std::rc::Rc;

/// The factory of the managed storage provider that a theme holds beside the template of the
/// chooser, under `MANAGED_STORAGE_FALLBACK_KEY`: what the backends fall back to without a portal
/// or GTK. Only a kept unit of a theme constructs it, so an application without that unit does
/// not link the managed file chooser (#612).
// Made only through `new`, which registers its cast to the factory contract.
#[non_exhaustive]
pub struct ManagedStorageFallback;

impl PartialEq for ManagedStorageFallback {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl ManagedStorageFallback {
    pub fn new() -> Rc<Self> {
        static CASTS: std::sync::Once = std::sync::Once::new();
        CASTS.call_once(ValueTypes::register_used::<Self>);
        Rc::new(Self)
    }
}

impl ValueTypeRegistration for ManagedStorageFallback {
    fn register_value_conversions() {
        ValueTypes::register_reference::<ManagedStorageFallback>();
        ValueTypes::register_cast::<Rc<ManagedStorageFallback>, Rc<dyn IStorageProviderFactory>>(|factory| {
            factory.clone()
        });
    }
}

impl IStorageProviderFactory for ManagedStorageFallback {
    fn create_provider(&self, top_level: &Ref<TopLevel>) -> Rc<dyn IStorageProvider> {
        Rc::new(ManagedStorageProvider::new(Some(top_level), None))
    }
}

ferro_markup_type!(class ManagedStorageFallback {
    this: Rc<ManagedStorageFallback>,
    handles: [ManagedStorageFallback, Rc<ManagedStorageFallback>, Option<Rc<ManagedStorageFallback>>],
    constructors: [() => ManagedStorageFallback::new],
});

#[cfg(test)]
mod tests {
    // Not from upstream.
    use super::*;
    use ferroui_base::controls::ResourceKey;
    use ferroui_base::metadata::from_markup_value;
    use ferroui_base::{BoxedValue, Upcast};
    use ferroui_controls::platform::{managed_storage_fallback, MANAGED_STORAGE_FALLBACK_KEY};
    use ferroui_controls::testing::{TestServices, UnitTestApplication};
    use ferroui_controls::Window;

    #[test]
    fn the_value_a_theme_holds_is_the_factory_of_the_managed_provider() {
        let _scope = UnitTestApplication::start(TestServices::styled_window());
        crate::register_types();
        let held: BoxedValue = Rc::new(ManagedStorageFallback::new());
        assert!(from_markup_value::<Rc<dyn IStorageProviderFactory>>(&Some(held.clone())).is_some());

        let window = Window::new();
        window.resources().add(ResourceKey::from(MANAGED_STORAGE_FALLBACK_KEY), Some(held));
        let top_level: Ref<TopLevel> = window.upcast();
        let provider = managed_storage_fallback(&top_level);
        assert!(provider.can_open() && provider.can_save() && provider.can_pick_folder());
    }

    /// Every theme that ships the chooser holds the factory beside it.
    #[test]
    fn every_theme_with_the_chooser_holds_the_factory() {
        let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("the folder of the crates");
        let mut themes = 0;
        for entry in std::fs::read_dir(source).expect("the crates can be listed").flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let chooser = entry.path().join("Controls/ManagedFileChooser.xaml");
            if !name.starts_with("FerroUI.Themes.") || !chooser.exists() {
                continue;
            }
            themes += 1;
            let text = std::fs::read_to_string(&chooser).expect("the document can be read");
            let element = format!("<internal:ManagedStorageFallback x:Key=\"{MANAGED_STORAGE_FALLBACK_KEY}\"");
            assert!(text.contains(&element), "{name} has no storage fallback beside its chooser");
        }
        // Outside the repository (a packaged crate) there is nothing to read.
        assert!(themes == 0 || themes >= 5, "found {themes} themes");
    }
}
