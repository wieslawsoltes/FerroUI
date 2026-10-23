//! XAML in a component ([`load_xaml`], [`xaml`], [`use_xaml`]; feature
//! `xaml`). The types of this crate for markup are registered by
//! [`register_types`](crate::register_types).
//!
//! ```xml
//! <UserControl xmlns="https://github.com/ferroui"
//!              xmlns:view="using:FerroUI.View">
//!   <view:ComponentHost Component="Counter" />
//! </UserControl>
//! ```

#[cfg(feature = "xaml")]
mod loading {
    use crate::element::Element;
    use crate::hooks::use_hook;
    use ferroui_base::utilities::Uri;
    use ferroui_base::Ref;
    use ferroui_controls::Control;
    use ferroui_markup_xaml::{FerroXamlLoader, XamlLoadException};
    use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;

    fn prepare() {
        ferroui_markup_xaml::register_types();
        crate::register_types();
        FerroRuntimeXamlLoader::register();
    }

    fn control(loaded: ferroui_base::BoxedValue) -> Result<Ref<Control>, XamlLoadException> {
        let type_name = loaded.type_name();
        Control::from_boxed(&loaded)
            .ok_or_else(|| XamlLoadException::with_message(format!("the root of the XAML is a {type_name}, not a control")))
    }

    /// Loads XAML text whose root is a control, with the run-time loader.
    /// Its named controls are found with `find_control` / `get_control` of
    /// the root.
    pub fn load_xaml(text: &str) -> Result<Ref<Control>, XamlLoadException> {
        prepare();
        control(FerroRuntimeXamlLoader::parse(text, None)?)
    }

    /// Loads the XAML document at `uri` (`ferres://<assembly>/<path>`): its
    /// compiled markup when the assembly has it, the run-time loader
    /// otherwise.
    pub fn load_xaml_uri(uri: &str) -> Result<Ref<Control>, XamlLoadException> {
        prepare();
        let uri = Uri::absolute(uri).map_err(|error| XamlLoadException::with_message(format!("{uri}: {error:?}")))?;
        control(FerroXamlLoader::try_load_with_service_provider(None, &uri, None)?)
    }

    /// An element of XAML text: the control it loads (see [`load_xaml`]).
    /// In a component that runs again, use [`use_xaml`], which loads once.
    ///
    /// # Panics
    /// Panics with the message of the loader when the XAML does not load.
    pub fn xaml(text: &str) -> Element {
        Element::Existing(load_xaml(text).unwrap_or_else(|error| panic!("XAML did not load: {error}")))
    }

    /// The control of XAML text, loaded on the first run of the component
    /// and kept: `Element::Existing(use_xaml(..))` keeps its control on
    /// every run. A text that starts with `ferres://` is a URI.
    ///
    /// # Panics
    /// Panics with the message of the loader when the XAML does not load.
    pub fn use_xaml(text: &str) -> Ref<Control> {
        use_hook(|| {
            let loaded = if text.starts_with("ferres://") { load_xaml_uri(text) } else { load_xaml(text) };
            loaded.unwrap_or_else(|error| panic!("XAML did not load: {error}"))
        })
    }
}

#[cfg(feature = "xaml")]
pub use loading::{load_xaml, load_xaml_uri, use_xaml, xaml};
