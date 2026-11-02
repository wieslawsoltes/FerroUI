//! Fresh-process probe of first WinUI theme construction and deferred Button resource lookup.
use ferroui_base::{
    controls::ResourceKey, metadata::from_markup_value, styling::ControlTheme, Ref,
};
use ferroui_controls::Button;
use ferroui_themes_winui::WinUITheme;
use std::{io::Write, time::Instant};

#[cfg(feature = "retain-archive")]
fn load_archive(path: std::ffi::OsString) -> ferroui_base::Ref<WinUITheme> {
    use ferroui_base::{metadata::from_markup_value, utilities::Uri};
    use ferroui_markup_xaml::RuntimeXamlLoaderConfiguration;
    use ferroui_markup_xaml_loader::{
        binary_document::pack::{expand_solid, is_solid, ThemePack},
        FerroRuntimeXamlLoader,
    };

    let image = std::fs::read(path).expect("read replacement archive");
    let indexed = if is_solid(&image) {
        expand_solid(&image).unwrap()
    } else {
        image
    };
    let archive = ThemePack::open(&indexed).unwrap();
    ferroui_themes_winui::register_types();
    let mut config = RuntimeXamlLoaderConfiguration::new();
    config.local_assembly = Some(&ferroui_themes_winui::ASSEMBLY);
    let uri = Uri::absolute(WinUITheme::DOCUMENT_URI).unwrap();
    let value =
        FerroRuntimeXamlLoader::load_packed_document(&archive, uri, None, None, config).unwrap();
    from_markup_value(&Some(value)).expect("WinUI theme class")
}

fn main() {
    // An editor can fetch the archive later. Keep the exact shipping bytes in this
    // executable without charging an archive decode to the typed startup path.
    #[cfg(feature = "retain-archive")]
    assert!(!std::hint::black_box(ferroui_themes_winui::theme_archive()).is_empty());

    let start = Instant::now();
    #[cfg(feature = "retain-archive")]
    let theme = std::env::args_os()
        .nth(1)
        .map_or_else(WinUITheme::new, load_archive);
    #[cfg(not(feature = "retain-archive"))]
    let theme = WinUITheme::new();
    let created = start.elapsed();
    let button = theme
        .try_get_resource(&ResourceKey::Type(Button::TYPE), None)
        .flatten();
    let button = from_markup_value::<Ref<ControlTheme>>(&button)
        .expect("WinUI must resolve the Button control theme");
    assert_eq!(button.target_type(), Some(Button::TYPE));
    println!("{} {}", created.as_nanos(), start.elapsed().as_nanos());
    std::io::stdout().flush().expect("flush ready marker");
}
