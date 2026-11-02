//! The entry points exported in `compiled_xaml.xamlmeta`, backed by one immutable
//! FXP archive. They preserve the compiled theme's URI surface and parent services.
use crate::{WinUITheme, ASSEMBLY};
use ferroui_base::{
    controls::ResourceDictionary,
    metadata::{from_markup_value, into_markup_value, IServiceProvider},
    styling::Styles,
    utilities::Uri,
    BoxedValue, Ref,
};
use ferroui_markup_xaml::{
    xaml_il::runtime::{compiled::uri_equals, XamlIlRuntimeHelpers},
    FerroXamlLoader, RuntimeXamlLoaderConfiguration, XamlLoadError,
};
use ferroui_markup_xaml_loader::{binary_document::pack::ThemePack, FerroRuntimeXamlLoader};
use std::{rc::Rc, sync::OnceLock};

type Parent = Option<Rc<dyn IServiceProvider>>;

fn load(path: &str, root: Option<BoxedValue>, parent: Parent) -> Result<BoxedValue, XamlLoadError> {
    static ARCHIVE: OnceLock<Result<ThemePack<'static>, String>> = OnceLock::new();
    let archive = ARCHIVE.get_or_init(|| {
        ThemePack::open(include_bytes!(concat!(env!("OUT_DIR"), "/winui.fxp")))
            .map_err(|error| error.to_string())
    });
    let archive = archive
        .as_ref()
        .map_err(|message| XamlLoadError::with_message(message.clone()))?;
    let uri = Uri::absolute(&format!("ferres://{}/{path}", ASSEMBLY.name))
        .map_err(|error| XamlLoadError::with_message(error.to_string()))?;
    let mut configuration = RuntimeXamlLoaderConfiguration::new();
    configuration.local_assembly = Some(&ASSEMBLY);
    FerroRuntimeXamlLoader::load_packed_document(archive, uri, root, parent, configuration)
}

/// Class constructor and external compiled markup entry point.
pub fn populate(parent: Parent, root: &Ref<WinUITheme>) -> Result<(), XamlLoadError> {
    FerroXamlLoader::track_instance(WinUITheme::TYPE, root);
    if let Some(populate) = FerroXamlLoader::populate_override(WinUITheme::TYPE) {
        return populate(parent.as_ref(), &(Rc::new(root.clone()) as BoxedValue));
    }
    populate_winuitheme_xaml(parent, root)
}

pub fn populate_winuitheme_xaml(
    parent: Parent,
    root: &Ref<WinUITheme>,
) -> Result<(), XamlLoadError> {
    load("WinUITheme.xaml", Some(Rc::new(root.clone())), parent).map(|_| ())
}

fn build<T: Clone + 'static>(path: &str, parent: Parent) -> Result<T, XamlLoadError> {
    let value = load(path, None, parent)?;
    from_markup_value(&Some(value))
        .ok_or_else(|| XamlLoadError::with_message(format!("Unexpected root type in {path}")))
}

pub fn build_densitystyles_compact_xaml(
    parent: Parent,
) -> Result<Ref<ResourceDictionary>, XamlLoadError> {
    build("DensityStyles/Compact.xaml", parent)
}

pub fn build_controls_winuicontrols_xaml(parent: Parent) -> Result<Ref<Styles>, XamlLoadError> {
    build("Controls/WinUIControls.xaml", parent)
}

pub fn try_load(
    parent: Option<&Rc<dyn IServiceProvider>>,
    uri: &str,
) -> Result<Option<BoxedValue>, XamlLoadError> {
    if uri_equals(uri, "ferres://ferroui.themes.winui/WinUITheme.xaml", "") {
        let parent = XamlIlRuntimeHelpers::create_root_service_provider_v3(parent.cloned());
        return Ok(into_markup_value(WinUITheme::with_service_provider(Some(
            parent,
        ))));
    }
    Ok(None)
}
