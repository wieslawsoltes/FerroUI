use crate::platform_settings;
use crate::{
    tests::run,
    theming::{RibbonThemeResources, to_ribbon_color},
};
use ferroribbon_core::theming::{
    RibbonChromeStyle, RibbonColor, RibbonThemePalette, RibbonThemeStyle,
};
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::media::{Color, SolidColorBrush};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{BoxedValue, CornerRadius, Ref, Thickness};
use ferroui_controls::Application;
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

fn corpus() -> Value {
    serde_json::from_str(include_str!("../../tests/fixtures/theme-resources.json")).unwrap()
}
fn resources() -> Ref<RibbonThemeResources> {
    // Supplied shared-resource contract only. No source Shared.xaml is pretended loaded.
    RibbonThemeResources::with_shared_resources(ResourceDictionary::new())
}
fn hex(b: &SolidColorBrush) -> String {
    to_ribbon_color(b.color()).to_hex(true)
}
fn dict(r: &RibbonThemeResources, name: &str) -> Ref<ResourceDictionary> {
    let variant = match name {
        "Light" => ThemeVariant::light(),
        "Dark" => ThemeVariant::dark(),
        "Default" => ThemeVariant::default(),
        _ => RibbonThemeResources::high_contrast_variant(),
    };
    r.theme_dictionary(&variant)
        .unwrap()
        .as_object()
        .unwrap()
        .to_ref()
        .cast()
        .unwrap()
}
fn shape(v: &BoxedValue) -> [f64; 4] {
    if let Some(c) = v.downcast_ref::<CornerRadius>() {
        [c.top_left, c.top_right, c.bottom_right, c.bottom_left]
    } else {
        let t = v.downcast_ref::<Thickness>().unwrap();
        [t.left, t.top, t.right, t.bottom]
    }
}
fn compare(r: &RibbonThemeResources, expected: &Value) {
    assert_eq!(
        r.palette().name.as_ref().map(|s| s.to_utf8().unwrap()),
        expected["palette"].as_str().map(str::to_owned)
    );
    assert_eq!(
        r.chrome_style().raw() as i64,
        expected["chrome"].as_i64().unwrap()
    );
    assert_eq!(r.style().raw() as i64, expected["style"].as_i64().unwrap());
    for row in expected["colors"].as_array().unwrap() {
        let d = dict(r, row["theme"].as_str().unwrap());
        for (&key, value) in RibbonThemeResources::brush_keys()
            .iter()
            .zip(row["values"].as_array().unwrap())
        {
            let b = d.get(&key.into()).unwrap();
            let b = b.downcast_ref::<Ref<SolidColorBrush>>().unwrap();
            assert_eq!(hex(b), value.as_str().unwrap(), "{} {key}", row["theme"]);
        }
    }
    for theme in ["Light", "Default", "Dark", "HighContrast"] {
        let d = dict(r, theme);
        for (&key, value) in RibbonThemeResources::shape_keys()
            .iter()
            .zip(expected["shapes"].as_array().unwrap())
        {
            let actual = shape(&d.get(&key.into()).unwrap());
            for (a, e) in actual.iter().zip(value.as_array().unwrap()) {
                assert_eq!(a.to_bits(), e.as_f64().unwrap().to_bits(), "{theme}/{key}");
            }
        }
    }
}
#[test]
fn whole_source_theme_resource_palette_chrome_surface_matrix() {
    run(|| {
        let c = corpus();
        compare(&resources(), &c["initial"]);
        assert_eq!(json!(RibbonThemeResources::brush_keys()), c["brushKeys"]);
        assert_eq!(json!(RibbonThemeResources::shape_keys()), c["shapeKeys"]);
        assert_eq!(
            RibbonThemeResources::high_contrast_variant().inherit_variant(),
            Some(ThemeVariant::dark())
        );
        for row in c["cases"].as_array().unwrap() {
            let r = resources();
            let v = &row["result"];
            let p = RibbonThemePalette::new(
                Some(v["palette"].as_str().unwrap().into()),
                RibbonColor::parse(Some(&row["accent"].as_str().unwrap().into())).unwrap(),
                RibbonColor::parse(Some(&row["accentDark"].as_str().unwrap().into())).unwrap(),
            );
            r.apply(
                Some(p),
                RibbonChromeStyle::from_raw(v["chrome"].as_i64().unwrap() as i32),
                RibbonThemeStyle::from_raw(v["style"].as_i64().unwrap() as i32),
            )
            .unwrap();
            compare(&r, v);
        }
    });
}
#[test]
fn source_resource_alias_override_null_palette_and_shape_replacement_rules() {
    run(|| {
        let c = corpus();
        let shared = ResourceDictionary::new();
        let removed_shared = ResourceDictionary::new();
        let removed_weak = removed_shared.downgrade();
        let owner = RibbonThemeResources::with_shared_resources(removed_shared.clone());
        drop(removed_shared);
        owner.merged_dictionaries().clear();
        assert_eq!(
            removed_weak.upgrade().is_some(),
            c["removedSharedAlive"].as_bool().unwrap()
        );
        shared.set("TestSharedMarker", Some(Rc::new(42i32)));
        let r = RibbonThemeResources::with_shared_resources(shared.clone());
        assert!(
            r.merged_dictionaries()
                .get(0)
                .as_object()
                .unwrap()
                .to_ref()
                .ptr_eq(&shared)
        );
        let light = dict(&r, "Light");
        let fallback = dict(&r, "Default");
        let dark = dict(&r, "Dark");
        assert_ne!(light, fallback);
        for &key in RibbonThemeResources::brush_keys() {
            let a = light.get(&key.into()).unwrap();
            let b = fallback.get(&key.into()).unwrap();
            let d = dark.get(&key.into()).unwrap();
            assert!(
                a.downcast_ref::<Ref<SolidColorBrush>>()
                    .unwrap()
                    .ptr_eq(b.downcast_ref::<Ref<SolidColorBrush>>().unwrap())
            );
            assert!(
                !a.downcast_ref::<Ref<SolidColorBrush>>()
                    .unwrap()
                    .ptr_eq(d.downcast_ref::<Ref<SolidColorBrush>>().unwrap())
            );
        }
        let first = r.get_brush("RibbonAccentBrush", None).unwrap();
        r.set_color("RibbonAccentBrush", Color::from_argb(255, 1, 2, 3), None);
        assert_eq!(hex(&first), c["names"]["custom"]);
        r.apply_palette_and_chrome(
            Some(RibbonThemePalette::excel()),
            RibbonChromeStyle::Colorful,
        )
        .unwrap();
        assert_eq!(hex(&first), c["names"]["applied"]);
        assert!(first.ptr_eq(&r.get_brush("RibbonAccentBrush", None).unwrap()));
        assert!(first.ptr_eq(&r.get_brush("RibbonAccentBrush", Some("dark")).unwrap()));
        assert!(r.get_brush("unknown", None).is_none());
        r.set_color("unknown", Color::from_argb(255, 10, 20, 30), None);
        let before = r.palette();
        let e = r
            .apply(None, RibbonChromeStyle::Neutral, RibbonThemeStyle::Cad)
            .unwrap_err();
        assert_eq!(e.parameter, c["nullPalette"]["error"]);
        assert!(Rc::ptr_eq(&before, &r.palette()));
        assert_eq!(
            r.chrome_style().raw() as i64,
            c["nullPalette"]["chrome"].as_i64().unwrap()
        );
        assert_eq!(
            r.style().raw() as i64,
            c["nullPalette"]["style"].as_i64().unwrap()
        );
        let r = resources();
        let d = dict(&r, "Light");
        d.set(
            "RibbonPopupCornerRadius",
            Some(Rc::new(CornerRadius::uniform(77.))),
        );
        r.apply_palette_and_chrome(
            Some(RibbonThemePalette::excel()),
            RibbonChromeStyle::Neutral,
        )
        .unwrap();
        assert_eq!(
            shape(&d.get(&"RibbonPopupCornerRadius".into()).unwrap()),
            [77.; 4]
        );
        r.apply(
            Some(RibbonThemePalette::word()),
            RibbonChromeStyle::Neutral,
            RibbonThemeStyle::Cad,
        )
        .unwrap();
        assert_eq!(
            shape(&d.get(&"RibbonPopupCornerRadius".into()).unwrap()),
            [2.; 4]
        );
        // Public dictionary edits don't change the source's private brush maps.
        d.set("RibbonAccentBrush", Some(Rc::new(7i32)));
        assert!(r.get_brush("RibbonAccentBrush", None).is_some());
    });
}
#[test]
fn source_reentrant_palette_assignment_preserves_write_order_and_stale_outer_inputs() {
    run(|| {
        let c = corpus();
        let r = resources();
        let writes = Rc::new(RefCell::new(Vec::new()));
        let entered = Rc::new(Cell::new(false));
        let state = Rc::new(RefCell::new(None));
        for theme in ["Light", "Dark", "HighContrast"] {
            for &key in RibbonThemeResources::brush_keys() {
                let b = r.get_brush(key, Some(theme)).unwrap();
                let weak = r.downgrade();
                let writes = writes.clone();
                let entered = entered.clone();
                let state = state.clone();
                b.add_property_changed_handler(Rc::new(move |e| {
            if e.property()!=SolidColorBrush::color_property().as_property() || !e.is_effective_value_change() {return;}
            let b=e.sender().downcast_ref::<SolidColorBrush>().unwrap();writes.borrow_mut().push(json!({"theme":theme,"key":key,"color":hex(b)}));
            if theme=="Light"&&key=="RibbonChromeBackgroundBrush"&&!entered.replace(true) {
                let r=weak.upgrade().unwrap();state.replace(Some(json!({"palette":r.palette().name.as_ref().unwrap().to_utf8().unwrap(),"chrome":r.chrome_style().raw(),"style":r.style().raw()})));
                r.apply(Some(RibbonThemePalette::word()),RibbonChromeStyle::Neutral,RibbonThemeStyle::Office).unwrap();
            }
        }));
            }
        }
        r.apply(
            Some(RibbonThemePalette::excel()),
            RibbonChromeStyle::Colorful,
            RibbonThemeStyle::Cad,
        )
        .unwrap();
        assert_eq!(
            state.borrow().as_ref().unwrap(),
            &c["reentry"]["stateAtReentry"]
        );
        assert_eq!(json!(*writes.borrow()), c["reentry"]["writes"]);
        compare(&r, &c["reentry"]["result"]);
    });
}
#[test]
fn source_system_resource_fallback_precedence_alpha_zero_and_wrong_type() {
    run(|| {
        let app = Application::current().unwrap();
        let d = app.resources();
        for (key, v) in [
            ("SystemColorWindowColor", Color::from_argb(255, 1, 3, 5)),
            (
                "SystemColorWindowTextColor",
                Color::from_argb(255, 250, 248, 246),
            ),
            (
                "SystemColorHighlightColor",
                Color::from_argb(255, 200, 100, 0),
            ),
            ("SystemColorGrayTextColor", Color::from_argb(0, 11, 22, 33)),
        ] {
            d.set(key, Some(Rc::new(v)));
        }
        d.set(
            "SystemColorHotlightColor",
            Some(Rc::new("wrong type".to_owned())),
        );
        compare(&resources(), &corpus()["resourcesFallback"]);
        for key in [
            "SystemColorWindowColor",
            "SystemColorWindowTextColor",
            "SystemColorHighlightColor",
            "SystemColorGrayTextColor",
            "SystemColorHotlightColor",
        ] {
            d.remove(&key.into());
        }
    });
}
#[test]
fn native_contrast_subscription_updates_existing_brush_once_and_releases_on_drop() {
    run(|| {
        use ferroui_base::{
            FerroLocator,
            platform::{ColorContrastPreference, IPlatformSettings, PlatformColorValues},
        };
        let scope = FerroLocator::enter_scope();
        let settings = platform_settings::Settings::new();
        FerroLocator::current_mutable()
            .bind::<dyn IPlatformSettings>()
            .to_constant(settings.clone());
        let r = resources();
        let weak = r.downgrade();
        let brush = r
            .get_brush("RibbonWindowBackgroundBrush", Some("HighContrast"))
            .unwrap();
        for _ in 0..3 {
            r.apply_palette_and_chrome(
                Some(RibbonThemePalette::excel()),
                RibbonChromeStyle::Neutral,
            )
            .unwrap();
        }
        assert_eq!(settings.counts.adds.get(), 1);
        let app = Application::current().unwrap();
        app.resources().set(
            "SystemColorWindowColor",
            Some(Rc::new(Color::from_argb(255, 1, 2, 3))),
        );
        settings.set_colors(
            PlatformColorValues::new().with_contrast_preference(ColorContrastPreference::High),
        );
        assert_eq!(brush.color(), Color::from_argb(255, 1, 2, 3));
        drop(r);
        assert!(weak.upgrade().is_none());
        assert_eq!(settings.counts.removes.get(), 1);
        app.resources().set(
            "SystemColorWindowColor",
            Some(Rc::new(Color::from_argb(255, 9, 8, 7))),
        );
        settings.set_colors(PlatformColorValues::new());
        assert_eq!(brush.color(), Color::from_argb(255, 1, 2, 3));
        app.resources().remove(&"SystemColorWindowColor".into());
        scope.dispose();
    });
}

fn source_system_palette(values: &Value) -> ferroui_base::platform::SystemColors {
    let color = |index: usize| {
        values[index]
            .as_str()
            .map(|text| Color::parse(text).unwrap())
            .unwrap_or(Color::from_argb(0, 0, 0, 0))
    };
    ferroui_base::platform::SystemColors::new(color(0), color(1))
        .with_highlight(color(2))
        .with_highlight_text(color(3))
        .with_button_face(color(4))
        .with_gray_text(color(5))
        .with_hotlight(color(6))
}

struct SystemResourceScope {
    dictionary: Ref<ResourceDictionary>,
    previous: Vec<(&'static str, Option<BoxedValue>)>,
}
impl SystemResourceScope {
    fn new(enabled: bool) -> Self {
        let dictionary = Application::current().unwrap().resources();
        let values: [(&str, BoxedValue); 5] = [
            (
                "SystemColorWindowColor",
                Rc::new(Color::from_argb(255, 1, 3, 5)),
            ),
            (
                "SystemColorWindowTextColor",
                Rc::new(Color::from_argb(255, 250, 248, 246)),
            ),
            (
                "SystemColorHighlightColor",
                Rc::new(Color::from_argb(255, 200, 100, 0)),
            ),
            (
                "SystemColorGrayTextColor",
                Rc::new(Color::from_argb(0, 11, 22, 33)),
            ),
            ("SystemColorHotlightColor", Rc::new("wrong type".to_owned())),
        ];
        let mut previous = Vec::new();
        for (key, value) in values {
            previous.push((key, dictionary.get(&key.into())));
            if enabled {
                dictionary.set(key, Some(value));
            } else {
                dictionary.remove(&key.into());
            }
        }
        Self {
            dictionary,
            previous,
        }
    }
}
impl Drop for SystemResourceScope {
    fn drop(&mut self) {
        for (key, value) in &self.previous {
            if let Some(value) = value {
                self.dictionary.set(*key, Some(value.clone()));
            } else {
                self.dictionary.remove(&(*key).into());
            }
        }
    }
}
fn compare_system_palette(owner: &RibbonThemeResources, expected: &Value) {
    let actual: Vec<_> = RibbonThemeResources::brush_keys()
        .iter()
        .map(|key| hex(&owner.get_brush(key, Some("HighContrast")).unwrap()))
        .collect();
    assert_eq!(json!(actual), *expected);
}

#[test]
fn source_system_palette_all_roles_and_fallbacks_match_unchanged_reference() {
    run(|| {
        use ferroui_base::{
            FerroLocator,
            platform::{ColorContrastPreference, IPlatformSettings, PlatformColorValues},
        };
        let fixture: Value =
            serde_json::from_str(include_str!("../../tests/fixtures/system-colors.json")).unwrap();
        assert_eq!(
            json!(RibbonThemeResources::brush_keys()),
            fixture["brushKeys"]
        );
        let scope = FerroLocator::enter_scope();
        let settings = platform_settings::Settings::new();
        FerroLocator::current_mutable()
            .bind::<dyn IPlatformSettings>()
            .to_constant(settings.clone());
        for case in fixture["cases"].as_array().unwrap() {
            let _resources = SystemResourceScope::new(case["resources"].as_bool().unwrap());
            settings.set_colors(
                PlatformColorValues::new()
                    .with_contrast_preference(if case["active"].as_bool().unwrap() {
                        ColorContrastPreference::High
                    } else {
                        ColorContrastPreference::NoPreference
                    })
                    .with_system_colors(Some(source_system_palette(&case["colors"]))),
            );
            compare_system_palette(&resources(), &case["expected"]);
        }
        scope.dispose();
    });
}

#[test]
fn source_system_palette_changes_update_existing_brush_and_release_subscription() {
    run(|| {
        use ferroui_base::{
            FerroLocator,
            platform::{ColorContrastPreference, IPlatformSettings, PlatformColorValues},
        };
        let fixture: Value =
            serde_json::from_str(include_str!("../../tests/fixtures/system-colors.json")).unwrap();
        let event = &fixture["events"];
        let _resources = SystemResourceScope::new(true);
        let scope = FerroLocator::enter_scope();
        let settings = platform_settings::Settings::new();
        let initial = PlatformColorValues::new()
            .with_contrast_preference(ColorContrastPreference::High)
            .with_system_colors(Some(source_system_palette(&event["original"])));
        settings.set_colors(initial);
        FerroLocator::current_mutable()
            .bind::<dyn IPlatformSettings>()
            .to_constant(settings.clone());
        let owner = resources();
        let weak = owner.downgrade();
        let brush = owner
            .get_brush("RibbonWindowBackgroundBrush", Some("HighContrast"))
            .unwrap();
        compare_system_palette(&owner, &event["before"]);
        assert_eq!(settings.counts.adds.get(), 1);
        let changed = initial.with_system_colors(Some(source_system_palette(&event["changed"])));
        settings.set_colors(changed);
        compare_system_palette(&owner, &event["after"]);
        settings.set_colors(changed);
        compare_system_palette(&owner, &event["after"]);
        settings
            .set_colors(changed.with_contrast_preference(ColorContrastPreference::NoPreference));
        compare_system_palette(&owner, &event["inactive"]);
        assert!(event["sameBrush"].as_bool().unwrap());
        assert!(
            brush.ptr_eq(
                &owner
                    .get_brush("RibbonWindowBackgroundBrush", Some("HighContrast"))
                    .unwrap()
            )
        );
        drop(owner);
        assert!(weak.upgrade().is_none());
        assert_eq!(settings.counts.removes.get(), 1);
        let retained = brush.color();
        settings.set_colors(initial);
        assert_eq!(brush.color(), retained);
        scope.dispose();
    });
}
