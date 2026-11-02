//! Evaluate a real emitted dictionary through both data backends and the XamlX interpreter.

use ferroui_base::controls::ResourceDictionary;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::media::{Color, SolidColorBrush};
use ferroui_base::Ref;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_markup_xaml::xaml_il::runtime::XamlIlRuntimeHelpers;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;

fn materialize(dictionary: Ref<ResourceDictionary>) -> Vec<(Color, f64)> {
    for key in ["Scale", "Margin", "Radii", "Accent"] {
        assert!(dictionary.contains_key(&key.into()));
        assert!(!dictionary.contains_deferred_key(&key.into()));
    }
    assert_eq!(dictionary.get(&"Scale".into()).unwrap().downcast_ref::<f64>(), Some(&0.8125));
    assert_eq!(dictionary.get(&"Margin".into()).unwrap().downcast_ref::<ferroui_base::Thickness>(), Some(&ferroui_base::Thickness::new(1.,2.,3.,4.)));
    assert_eq!(dictionary.get(&"Radii".into()).unwrap().downcast_ref::<ferroui_base::CornerRadius>(), Some(&ferroui_base::CornerRadius::new(4.,3.,2.,1.)));
    let keys = [
        "Alias2", "Aliás1", "Literal1", "Literal2", "Static1", "Static2", "Dynamic1", "Dynamic2", "Fallback",
    ];
    for key in keys {
        assert!(dictionary.contains_deferred_key(&key.into()), "{key} must be lazy");
    }
    assert!(dictionary.contains_deferred_key(&"ImportedTemplate".into()));
    let template = dictionary.get(&"ImportedTemplate".into()).expect("imported template");
    let template = ferroui_base::metadata::from_markup_value::<std::rc::Rc<dyn ferroui_controls::templates::IControlTemplate>>(&Some(template.clone()))
        .unwrap_or_else(|| panic!("expected IControlTemplate, found {}", template.type_name()));
    let owner = ferroui_controls::ContentControl::new().upcast();
    let built = template.build(&owner).unwrap();
    assert_eq!(built.result().name().as_deref(), Some("ImportedChild"));
    assert!(!built.result().ptr_eq(template.build(&owner).unwrap().result()));
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
    #[cfg(feature = "binary-reload-tests")]
    {
        let syntax = ferroui_markup_xaml_loader::binary_document::compile(crate::documents::TABLE_TEST[0].1).unwrap();
        let value = FerroRuntimeXamlLoader::load_stream(
            Box::new(std::io::Cursor::new(syntax)), Some(&crate::ASSEMBLY), None, None, false,
        ).unwrap();
        let dictionary = ferroui_base::metadata::from_markup_value::<Ref<ResourceDictionary>>(&Some(value)).unwrap();
        assert_eq!(materialize(dictionary), expected);
    }

    for build in [
        crate::compact_rust::build_compact_xaml,
        crate::compact_binary::build_compact_xaml,
    ] {
        let dictionary = build(Some(XamlIlRuntimeHelpers::create_root_service_provider_v3(None))).unwrap();
        assert_eq!(materialize(dictionary), expected);
    }
    let rust = include_str!(concat!(env!("OUT_DIR"), "/xaml/compact_rust.rs"));
    let binary = include_str!(concat!(env!("OUT_DIR"), "/xaml/compact_binary.rs"));
    for helper in ["aliases", "literal_brushes", "static_brushes", "dynamic_brushes", "deferred_factories", "colors", "doubles", "thicknesses", "corner_radii"] {
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

#[test]
fn duplicate_table_key_matches_the_typed_call_and_stops_the_run() {
    use ferroui_base::data::core::ValueType;
    use ferroui_markup_xaml::xaml_il::runtime::compiled as rt;

    static DOC: rt::DocumentInfo = rt::DocumentInfo {
        base_uri: None,
        namespaces: &[],
    };
    let _app = UnitTestApplication::start(TestServices::styled_window());
    crate::register_types();
    let dictionary = ResourceDictionary::new();
    dictionary.add_value("taken", 41_i32);
    let context = rt::populate_context(None, &DOC, rt::to_value(dictionary.clone()));
    context.push_parent(rt::to_value(dictionary.clone()));
    let original = rt::add_deferred_resource(&dictionary, "taken", ValueType::object(), &context, |_| Ok(None), 17, 9)
        .unwrap_err();
    let compact = rt::theme_resources::aliases(
        &dictionary,
        &context,
        &DOC,
        &[
            ("before", "taken", 16, 9, 16, 9),
            ("taken", "before", 17, 9, 17, 9),
            ("after", "before", 18, 9, 18, 9),
        ],
    )
    .unwrap_err();
    assert_eq!(compact.message(), original.message());
    assert!(compact.message().contains("same key"));
    assert!(dictionary.contains_deferred_key(&"before".into()));
    assert!(!dictionary.contains_key(&"after".into()));
    assert_eq!(
        dictionary.get(&"taken".into()).unwrap().downcast_ref::<i32>(),
        Some(&41)
    );
}

#[test]
fn explicit_root_keeps_alias_chain_and_omits_unreferenced_factories() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    crate::register_types();
    let dictionary = crate::compact_slice::build_compact_xaml(Some(
        XamlIlRuntimeHelpers::create_root_service_provider_v3(None),
    )).unwrap();
    assert!(dictionary.get(&"Alias2".into()).is_some());
    assert!(dictionary.contains_key(&"Literal1".into()));
    assert!(!dictionary.contains_key(&"Literal2".into()));
    let generated = include_str!(concat!(env!("OUT_DIR"), "/xaml/compact_slice.rs"));
    assert!(!generated.contains("Dynamic1"));
    assert!(!generated.contains("bind_dynamic_resource"));
}

#[cfg(all(feature = "binary-reload-tests", debug_assertions))]
#[test]
fn generated_manifest_reloads_a_live_resource_through_its_alias() {
    use ferroui_base::styling::{IStyle, Styles};
    use ferroui_hot_reload::{BinaryThemeWatcher, ReloadError};
    use std::{fs, path::Path, rc::Rc, time::{Duration, Instant}};

    let _app = UnitTestApplication::start(TestServices::styled_window());
    crate::register_types();
    let (original, embedded) = crate::compact_binary::FXB_MANIFEST;
    let base = Path::new(original).parent().unwrap();
    let folder = std::env::temp_dir().join(format!("compiled-theme-reload-{}", std::process::id()));
    fs::create_dir_all(&folder).unwrap();
    use ferroui_markup_xaml::xaml_il::runtime::compiled::binary_resources::fingerprint;
    let mut changed = 0;
    let mut generation = embedded.to_owned();
    for entry in fs::read_dir(base).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        let name = name.to_str().unwrap();
        if !name.ends_with(".fxb") || !embedded.contains(&format!("\"{name}\"")) { continue; }
        let mut bytes = fs::read(entry.path()).unwrap();
        if let Some(at) = bytes.windows(4).position(|w| w == 0xff112233u32.to_le_bytes()) {
            let old = format!("{:016x}", fingerprint(&bytes));
            bytes[at..at + 4].copy_from_slice(&0xffabcdefu32.to_le_bytes());
            generation = generation.replace(&old, &format!("{:016x}", fingerprint(&bytes)));
            changed += 1;
        }
        fs::write(folder.join(name), bytes).unwrap();
    }
    assert_eq!(changed, 1);
    let manifest = folder.join("compact_binary.fxb.json");
    fs::write(&manifest, generation).unwrap();
    let build = || {
        let theme = Styles::new();
        let resources = crate::compact_binary::build_compact_xaml(Some(XamlIlRuntimeHelpers::create_root_service_provider_v3(None)))
            .map_err(|e| ReloadError::Failed(e.to_string()))?;
        theme.set_resources(resources);
        Ok::<Rc<dyn IStyle>, ReloadError>(theme.into())
    };
    let owner = Styles::new();
    owner.add(build().unwrap());
    let color = || {
        let alias = owner.try_get_resource(&"Alias2".into(), None).flatten().unwrap();
        ValueTypes::as_object(&*alias).and_then(|value| value.cast::<SolidColorBrush>()).unwrap().color()
    };
    assert_eq!(color(), Color::from_uint32(0xff112233));
    let mut watcher = BinaryThemeWatcher::from_manifest(&owner, 0, (manifest.to_str().unwrap(), embedded), Duration::from_millis(10)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !watcher.poll(build).unwrap() {
        assert!(Instant::now() < deadline, "no compiled file revision delivered");
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(color(), Color::from_uint32(0xffabcdef));
    drop(watcher);
    fs::remove_dir_all(folder).unwrap();
}

#[test]
fn unknown_factory_slot_is_rejected_before_registering_any_key() {
    use ferroui_markup_xaml::xaml_il::runtime::compiled as rt;
    static DOC: rt::DocumentInfo = rt::DocumentInfo { base_uri: None, namespaces: &[] };
    // Two valid FXB records. Only the second import index is absent from the caller.
    let block = rt::binary_resources::Block::new("fixture/imports",
        b"FXB\0\x01\x05\x02\x02\x00\x01A\x00\x01B\x00\x00\x01\x00\x00\x00\x01\x01\x01\x00\x00\x00");
    let _app = UnitTestApplication::start(TestServices::styled_window());
    crate::register_types();
    let dictionary = ResourceDictionary::new();
    let context = rt::populate_context(None, &DOC, rt::to_value(dictionary.clone()));
    let error = rt::binary_resources::deferred_factories(&dictionary, &context, &[|_| Ok(None)], block).unwrap_err();
    assert!(error.message().contains("invalid FXB deferred factory import"));
    assert!(!dictionary.contains_key(&"A".into()));
}
