fn inventory(theme: &Ref<WinUITheme>) -> (usize, usize) {
    use ferroui_base::controls::ResourceDictionary;
    use ferroui_base::styling::Styles;
    use std::collections::BTreeSet;
    ferroui_base::FerroLocator::current_mutable()
        .bind::<dyn ferroui_base::platform::IPlatformRenderInterface>()
        .to_constant(ferroui_headless::testing::headless_platform_render_interface());
    struct ProbeCursor;
    impl ferroui_base::platform::ICursorImpl for ProbeCursor {
        fn dispose(&self) {}
    }
    impl ferroui_base::platform::ICursorFactory for ProbeCursor {
        fn get_cursor(&self, _: ferroui_base::input::StandardCursorType) -> Rc<dyn ferroui_base::platform::ICursorImpl> {
            Rc::new(ProbeCursor)
        }
    }
    ferroui_base::FerroLocator::current_mutable()
        .bind::<dyn ferroui_base::platform::ICursorFactory>()
        .to_constant(Rc::new(ProbeCursor));
    let app = ferroui_controls::Application::new();
    ferroui_controls::Application::bind_current(app.clone());
    app.styles().add(theme.as_style());
    let mut keys = BTreeSet::new();
    fn gather(dict: &ResourceDictionary, keys: &mut BTreeSet<String>) {
        for key in dict.keys() {
            if let Some(name) = key.as_str() { keys.insert(name.to_owned()); }
        }
        for provider in dict.merged_dictionaries().snapshot().iter() {
            if let Some(child) = provider.as_object().and_then(|o|o.downcast_ref::<ResourceDictionary>()) {
                gather(child, keys);
            }
        }
        for (_, provider) in dict.theme_dictionaries_snapshot() {
            if let Some(child) = provider.as_object().and_then(|o|o.downcast_ref::<ResourceDictionary>()) {
                gather(child, keys);
            }
        }
    }
    assert_eq!(theme.count(), 1);
    let child = theme.get(0);
    let controls = child.as_object().and_then(|o| o.downcast_ref::<Styles>()).unwrap();
    let mut types=Vec::new();
    for key in controls.resources().keys() {
        if let ResourceKey::Type(t) = &key {
            let value=theme.try_get_resource(&key, Some(&ThemeVariant::light())).flatten();
            let style=from_markup_value::<Ref<ControlTheme>>(&value).expect("typed control theme");
            types.push(format!("{}={}", t.name(),style.setters().snapshot().len()));
        }
    }
    types.sort();
    gather(&theme.resources(), &mut keys);
    gather(&controls.resources(), &mut keys);
    // Opt-in output permits byte-for-byte comparisons of the type/setter and string-key inventories.
    if let Some(path)=std::env::var_os("FULL_SPIKE_KEYS") {
        std::fs::write(path,types.iter().chain(keys.iter()).cloned().collect::<Vec<_>>().join("\n")).unwrap();
    }
    eprintln!("{} type keys and {} distinct string keys",types.len(), keys.len());
    (keys.len(), types.len())
}

fn verify_full(theme: &Ref<WinUITheme>) {
    use ferroui_themes_winui::DensityStyle;
    let (key_count, type_count) = inventory(theme);
    assert!(key_count > 1000);
    assert!(type_count > 70);
    let min_height=|| {
        let value=theme.try_get_resource(&"ComboBoxMinHeight".into(), Some(&ThemeVariant::light())).flatten();
        from_markup_value::<f64>(&value).unwrap()
    };
    assert_eq!(min_height(),32.);
    theme.set_density_style(DensityStyle::Compact);
    assert_eq!(min_height(),24.);
    theme.set_density_style(DensityStyle::Normal);
}
