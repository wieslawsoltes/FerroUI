//! Compare repeated construction from a portable FXP archive and a prepared program.
use ferroui_base::{controls::ResourceKey, metadata::from_markup_value, utilities::Uri, Ref};
use ferroui_controls::Button;
use ferroui_markup_xaml::RuntimeXamlLoaderConfiguration;
use ferroui_markup_xaml_loader::{binary_document::pack, FerroRuntimeXamlLoader};
use ferroui_themes_winui::{theme_archive, WinUITheme, ASSEMBLY};
use std::time::Instant;

fn main() {
    let prepared = match std::env::args().nth(1).as_deref() {
        Some("prepare") => true,
        Some("load") => false,
        _ => panic!("Specify load or prepare"),
    };
    let bytes = theme_archive();
    let start = Instant::now();
    ferroui_themes_winui::register_types();
    let indexed;
    let bytes = if pack::is_solid(bytes) {
        indexed = pack::expand_solid(bytes).unwrap();
        indexed.as_slice()
    } else {
        bytes
    };
    let archive = pack::ThemePack::open(bytes).unwrap();
    let uri = Uri::absolute(WinUITheme::DOCUMENT_URI).unwrap();
    let mut configuration = RuntimeXamlLoaderConfiguration::new();
    configuration.local_assembly = Some(&ASSEMBLY);
    let plan = prepared.then(|| {
        FerroRuntimeXamlLoader::prepare_packed_document(&archive, uri.clone(), configuration.clone()).unwrap()
    });
    println!("setup {}", start.elapsed().as_nanos());
    for index in 0..6 {
        let start = Instant::now();
        let value = match &plan {
            Some(plan) => plan.build(None).unwrap(),
            None => FerroRuntimeXamlLoader::load_packed_document(
                &archive, uri.clone(), None, None, configuration.clone(),
            ).unwrap(),
        };
        let theme = from_markup_value::<Ref<WinUITheme>>(&Some(value)).unwrap();
        assert_eq!(theme.count(), 1);
        assert!(theme.try_get_resource(&ResourceKey::Type(Button::TYPE), None).flatten().is_some());
        println!("build {index} {}", start.elapsed().as_nanos());
    }
}
