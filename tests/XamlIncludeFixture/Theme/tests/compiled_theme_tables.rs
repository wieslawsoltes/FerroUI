//! Evaluate a real emitted dictionary through both data backends and the XamlX interpreter.

use ferroui_base::controls::ResourceDictionary;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::media::{Color, SolidColorBrush};
use ferroui_base::Ref;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_markup_xaml::xaml_il::runtime::XamlIlRuntimeHelpers;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;

fn materialize(dictionary: Ref<ResourceDictionary>) -> Vec<(Color, f64)> {
    let keys = [
        "Alias2", "Aliás1", "Literal1", "Literal2", "Static1", "Static2", "Dynamic1", "Dynamic2", "Fallback",
    ];
    for key in keys {
        assert!(dictionary.contains_deferred_key(&key.into()), "{key} must be lazy");
    }
    let brush = |key: &str| -> Ref<SolidColorBrush> {
        let value = dictionary.get(&key.into()).expect(key);
        ValueTypes::as_object(&*value)
            .and_then(|object| object.cast::<SolidColorBrush>())
            .expect(key)
    };
    let alias = brush("Alias2");
    assert!(alias.ptr_eq(&brush("Literal1")));
    assert!(alias.ptr_eq(&brush("Alias2")));
    assert!(!dictionary.contains_deferred_key(&"Alias2".into()));
    keys.iter()
        .map(|key| {
            let brush = brush(key);
            (brush.color(), brush.opacity())
        })
        .collect()
}

#[test]
fn compiled_theme_tables_match_the_interpreter() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    crate::register_types();
    FerroRuntimeXamlLoader::register();
    let expected = materialize(
        FerroRuntimeXamlLoader::parse_as(crate::documents::TABLE_TEST[0].1, Some(&crate::ASSEMBLY)).unwrap(),
    );
    assert_eq!(expected[4].0, Color::from_uint32(0xff102030));
    assert_eq!(expected[8].1, 0.5);

    for build in [
        crate::compact_rust::build_compact_xaml,
        crate::compact_binary::build_compact_xaml,
    ] {
        let dictionary = build(Some(XamlIlRuntimeHelpers::create_root_service_provider_v3(None))).unwrap();
        assert_eq!(materialize(dictionary), expected);
    }
    let rust = include_str!(concat!(env!("OUT_DIR"), "/xaml/compact_rust.rs"));
    let binary = include_str!(concat!(env!("OUT_DIR"), "/xaml/compact_binary.rs"));
    for helper in ["aliases", "literal_brushes", "static_brushes", "dynamic_brushes"] {
        assert!(
            rust.contains(&format!("rt::theme_resources::{helper}(")),
            "{helper} was not lowered"
        );
        assert!(
            binary.contains(&format!("rt::binary_resources::{helper}(")),
            "{helper} was not serialized"
        );
    }
}
