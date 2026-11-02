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
    #[cfg(feature = "operand-updates")]
    if image.starts_with(b"FXU\x01") {
        return ferroui_themes_winui::load_theme_update(&image).expect("load compiled operands");
    }
    ferroui_themes_winui::register_types();
    let mut config = RuntimeXamlLoaderConfiguration::new();
    config.local_assembly = Some(&ferroui_themes_winui::ASSEMBLY);
    let uri = Uri::absolute(WinUITheme::DOCUMENT_URI).unwrap();
    let root = ferroui_themes_winui::compiled_archive_root();
    #[cfg(feature = "legacy-archive")]
    let native = root.try_load(&image, &uri, &config, None);
    #[cfg(not(feature = "legacy-archive"))]
    let native = FerroRuntimeXamlLoader::try_load_compiled_archive(&root, &image, &uri, &config, None);
    if let Some(value) = native.expect("load compiled archive") {
        return from_markup_value(&Some(value)).expect("compiled WinUI theme class");
    }
    let indexed = if is_solid(&image) {
        expand_solid(&image).unwrap()
    } else {
        image
    };
    let archive = ThemePack::open(&indexed).unwrap();
    let value =
        FerroRuntimeXamlLoader::load_packed_document(&archive, uri, None, None, config).unwrap();
    from_markup_value(&Some(value)).expect("WinUI theme class")
}

fn main() {
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
    // A separate verification launch can prove that the external revision took effect.
    #[cfg(feature = "retain-archive")]
    {
        use ferroui_base::styling::ThemeVariant;
        use ferroui_themes_winui::DensityStyle;
        for (variable, density) in [
            ("FERROUI_BENCH_EXPECT_NORMAL_FONT_SIZE", DensityStyle::Normal),
            ("FERROUI_BENCH_EXPECT_COMPACT_FONT_SIZE", DensityStyle::Compact),
        ] {
            if let Ok(expected) = std::env::var(variable) {
                let expected: f64 = expected.parse().expect("expected numeric font size");
                theme.set_density_style(density);
                let value = theme.try_get_resource(
                    &"ControlContentThemeFontSize".into(), Some(&ThemeVariant::dark()),
                ).flatten();
                assert_eq!(from_markup_value::<f64>(&value), Some(expected), "{variable}");
            }
        }
    }
}
