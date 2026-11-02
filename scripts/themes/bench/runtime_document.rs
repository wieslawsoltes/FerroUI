//! Fresh-process probe of runtime XML or FXD loading and the first deferred resource.
use ferroui_base::{
    controls::{ResourceDictionary, ResourceKey},
    media::{Colors, SolidColorBrush, ThemeShadow},
    metadata::from_markup_value,
    Ref,
};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::io::{Cursor, Write};
use std::time::Instant;

fn main() {
    let bytes: &'static [u8] = include_bytes!(env!("FERROUI_BENCH_DOCUMENT"));
    let start = Instant::now();
    let root = FerroRuntimeXamlLoader::load_stream(Box::new(Cursor::new(bytes)), None, None, None, false)
        .expect("load the embedded theme dictionary");
    let dictionary = from_markup_value::<Ref<ResourceDictionary>>(&Some(root)).expect("resource dictionary");
    let loaded = start.elapsed();
    let resource = env!("FERROUI_BENCH_RESOURCE");
    let key = ResourceKey::from(resource);
    assert!(dictionary.contains_deferred_key(&key));
    let value = dictionary.get(&key);
    match resource {
        "WinUIThemeShadow" => assert!(from_markup_value::<Ref<ThemeShadow>>(&value).is_some()),
        "SystemControlTransparentBrush" => {
            let brush = from_markup_value::<Ref<SolidColorBrush>>(&value).expect("SolidColorBrush");
            assert_eq!(brush.color(), Colors::TRANSPARENT);
        }
        _ => panic!("unrecognized benchmark resource"),
    }
    println!("{} {}", loaded.as_nanos(), start.elapsed().as_nanos());
    std::io::stdout().flush().expect("flush ready marker");
}
