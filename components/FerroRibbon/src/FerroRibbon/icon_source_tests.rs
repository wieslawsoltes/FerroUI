use crate::{
    compat::{FontIcon, FontIconSource, IconSource, ImageIcon, ImageIconSource, PathIconSource},
    controls::RibbonItemHelper,
    primitives::RibbonIconPresenter,
};
use ferroui_base::{
    BoxedValue, Ref, instantiate,
    media::{
        DrawingImage, FontFamily, FontStyle, FontWeight, Geometry, IBrush, IImage, SolidColorBrush,
    },
};
use ferroui_controls::{Application, Control, Image, PathIcon, Viewbox};
use serde_json::Value;
use std::rc::Rc;

fn brush(value: u32) -> Rc<dyn IBrush> {
    SolidColorBrush::from_uint32(value).into()
}
fn snapshot(
    root: Option<Ref<Control>>,
    expected: &Value,
    data: &Ref<Geometry>,
    other_data: &Ref<Geometry>,
    image: &Rc<dyn IImage>,
    other_image: &Rc<dyn IImage>,
) {
    let box_ = root.as_ref().and_then(|r| r.cast::<Viewbox>());
    let element = box_
        .as_ref()
        .and_then(|b| b.child())
        .or_else(|| root.clone());
    let font = element.as_ref().and_then(|e| e.cast::<FontIcon>());
    let path = element.as_ref().and_then(|e| e.cast::<PathIcon>());
    let plain = element.as_ref().and_then(|e| e.cast::<Image>());
    let menu_image = element.as_ref().and_then(|e| e.cast::<ImageIcon>());
    let kind = if font.is_some() {
        Some("FontIcon")
    } else if path.is_some() {
        Some("PathIcon")
    } else if plain.is_some() {
        Some("Image")
    } else if menu_image.is_some() {
        Some("ImageIcon")
    } else {
        assert!(element.is_none());
        None
    };
    assert_eq!(kind, expected["kind"].as_str(), "{expected}");
    assert_eq!(
        if box_.is_some() {
            Some("Viewbox")
        } else {
            kind
        },
        expected["root"].as_str(),
        "{expected}"
    );
    for (key, value) in [
        ("width", element.as_ref().map(|e| e.width())),
        ("height", element.as_ref().map(|e| e.height())),
        ("boxWidth", box_.as_ref().map(|b| b.width())),
        ("boxHeight", box_.as_ref().map(|b| b.height())),
        ("fontSize", font.as_ref().map(|f| f.font_size())),
    ] {
        assert_eq!(
            value.filter(|v| !v.is_nan()).map(f64::to_bits),
            expected[key].as_f64().map(f64::to_bits),
            "{key}: {expected}"
        );
    }
    let glyph = font
        .as_ref()
        .and_then(|f| f.glyph())
        .map(|s| s.encode_utf16().map(u64::from).collect::<Vec<_>>());
    assert_eq!(
        glyph,
        expected["glyph"]
            .as_array()
            .map(|a| a.iter().map(|n| n.as_u64().unwrap()).collect::<Vec<_>>()),
        "{expected}"
    );
    if let Some(font) = &font {
        let family = expected["family"].as_str().unwrap();
        assert_eq!(
            font.font_family(),
            if family == "Symbols" {
                FontIcon::new().font_family()
            } else {
                FontFamily::new(family)
            }
        );
        assert_eq!(
            font.font_weight().0 as i64,
            expected["weight"].as_i64().unwrap()
        );
        assert_eq!(
            font.font_style().to_string(),
            expected["style"].as_str().unwrap()
        );
        assert!(!font.mirrored_when_right_to_left());
    }
    let icon_element = element
        .as_ref()
        .and_then(|e| e.cast::<ferroui_controls::IconElement>());
    assert_eq!(
        icon_element.as_ref().is_some_and(|e| e
            .get_local_value(ferroui_controls::primitives::TemplatedControl::foreground_property())
            .is_some()),
        expected["hasLocalForeground"].as_bool().unwrap()
    );
    let color = element
        .as_ref()
        .and_then(|e| e.cast::<ferroui_controls::IconElement>())
        .and_then(|e| e.foreground())
        .and_then(|b| {
            b.as_solid_color_brush()
                .map(|b| format!("#{:08X}", b.color().to_uint32()))
        });
    if let Some(color_expected) = expected["color"].as_str() {
        assert_eq!(color.as_deref(), Some(color_expected), "{expected}");
    } else if let Some(icon) = icon_element {
        // The descriptor has a null default; actual source and native defaults
        // differ. Preserve native theming and compare the no-assignment policy.
        let default = if font.is_some() {
            FontIcon::new().upcast::<ferroui_controls::IconElement>()
        } else if path.is_some() {
            PathIcon::new().upcast()
        } else {
            ImageIcon::new().upcast()
        };
        assert_eq!(icon.foreground(), default.foreground());
    } else {
        assert!(color.is_none());
    }
    let geometry = path.and_then(|p| p.data());
    let bitmap = plain
        .as_ref()
        .and_then(|i| i.source())
        .or_else(|| menu_image.and_then(|i| i.source()));
    let payload = if let Some(g) = geometry {
        if g.ptr_eq(data) {
            1
        } else {
            assert!(g.ptr_eq(other_data));
            2
        }
    } else if let Some(i) = bitmap {
        if Rc::ptr_eq(&i, image) {
            1
        } else {
            assert!(Rc::ptr_eq(&i, other_image));
            2
        }
    } else {
        0
    };
    assert_eq!(payload, expected["payload"].as_u64().unwrap());
    assert_eq!(
        plain.map(|p| format!("{:?}", p.stretch())).as_deref(),
        expected["stretch"].as_str()
    );
}

#[test]
fn original_presenter_and_menu_icon_source_snapshots_keep_different_copy_sets() {
    crate::tests::run(|| {
        let fixture: Value =
            serde_json::from_str(include_str!("../../tests/fixtures/icon-sources.json")).unwrap();
        let resources = Application::current().unwrap().resources();
        let key = "SymbolThemeFontFamily".into();
        let previous = resources.try_get_value(&key);
        for case in fixture["corpus"]["cases"].as_array().unwrap() {
            resources.remove(&key);
            match case["resource"].as_str().unwrap() {
                "custom" => resources.set(
                    key.clone(),
                    Some(Rc::new(FontFamily::new("Resource Symbols"))),
                ),
                "wrong" => resources.set(key.clone(), Some(Rc::new("wrong".to_owned()))),
                "absent" => (),
                v => panic!("{v}"),
            };
            let data = Geometry::parse("M0,0 L10,0 L0,10 Z").unwrap();
            let other_data = Geometry::parse("M0,0 L20,0 L0,20 Z").unwrap();
            let image: Rc<dyn IImage> = DrawingImage::new().into();
            let other_image: Rc<dyn IImage> = DrawingImage::new().into();
            let kind = case["kind"].as_str().unwrap();
            let source: Ref<IconSource> = match kind {
                "font" => {
                    let f = FontIconSource::new();
                    f.set_glyph(case["glyph"].as_array().map(|a| {
                        String::from_utf16(
                            &a.iter()
                                .map(|n| n.as_u64().unwrap() as u16)
                                .collect::<Vec<_>>(),
                        )
                        .unwrap()
                    }));
                    f.set_font_family(case["family"].as_str().map(FontFamily::new));
                    f.set_font_weight(FontWeight::Bold);
                    f.set_font_style(FontStyle::Italic);
                    f.set_font_size(99.);
                    f.set_mirrored_when_right_to_left(true);
                    f.set_is_text_scale_factor_enabled(false);
                    f.upcast()
                }
                "path" => {
                    let p = PathIconSource::new();
                    p.set_data(case["payload"].as_bool().unwrap().then(|| data.clone()));
                    p.upcast()
                }
                "image" => {
                    let i = ImageIconSource::new();
                    i.set_image_source(case["payload"].as_bool().unwrap().then(|| image.clone()));
                    i.upcast()
                }
                "unknown" => instantiate(IconSource::construct()),
                v => panic!("{v}"),
            };
            source.set_foreground(
                case["foreground"]
                    .as_bool()
                    .unwrap()
                    .then(|| brush(0xff0000ff)),
            );
            let value: BoxedValue = Rc::new(source.clone());
            let p = RibbonIconPresenter::new();
            p.set_foreground(Some(brush(0xffff0000)));
            p.set_icon_size(23.5);
            p.set_icon(Some(value.clone()));
            let menu = RibbonItemHelper::create_menu_icon(Some(value.clone()))
                .unwrap()
                .map(|i| i.upcast());
            let check =
                |root, expected| snapshot(root, expected, &data, &other_data, &image, &other_image);
            check(
                (p.children().count() > 0).then(|| p.children().get(0)),
                &case["first"],
            );
            check(menu.clone(), &case["menuFirst"]);
            if let Some(font) = source.cast::<FontIconSource>() {
                font.set_glyph(Some("Later".into()));
                font.set_font_weight(FontWeight(900));
            }
            if let Some(path) = source.cast::<PathIconSource>() {
                path.set_data(Some(other_data.clone()));
            }
            if let Some(image_source) = source.cast::<ImageIconSource>() {
                image_source.set_image_source(Some(other_image.clone()));
            }
            source.set_foreground(None);
            check(
                (p.children().count() > 0).then(|| p.children().get(0)),
                &case["unchanged"],
            );
            check(menu.clone(), &case["menuUnchanged"]);
            p.set_icon_size(32.);
            check(
                (p.children().count() > 0).then(|| p.children().get(0)),
                &case["rebuilt"],
            );
            check(
                RibbonItemHelper::create_menu_icon(Some(value))
                    .unwrap()
                    .map(|i| i.upcast()),
                &case["menuRebuilt"],
            );
        }
        resources.remove(&key);
        if let Some(previous) = previous {
            resources.set(key, previous);
        }
    });
}

#[test]
fn shared_icon_sources_keep_independent_snapshots_and_release_native_payloads() {
    crate::tests::run(|| {
        let source = PathIconSource::new();
        let weak_source = source.downgrade();
        let geometry = Geometry::parse("M0,0 L10,0 L0,10 Z").unwrap();
        let weak_geometry = geometry.downgrade();
        source.set_data(Some(geometry.clone()));
        let a = RibbonIconPresenter::new();
        let b = RibbonIconPresenter::new();
        a.set_icon(Some(Rc::new(source.clone())));
        b.set_icon(Some(Rc::new(source.clone())));
        let menu = RibbonItemHelper::create_menu_icon(Some(Rc::new(source.clone())))
            .unwrap()
            .unwrap();
        assert!(!a.children().get(0).ptr_eq(&b.children().get(0)));
        assert!(
            menu.cast::<PathIcon>()
                .unwrap()
                .data()
                .unwrap()
                .ptr_eq(&geometry)
        );
        drop(source);
        drop(geometry);
        a.set_icon(None);
        assert!(weak_source.upgrade().is_some());
        b.set_icon(None);
        assert!(weak_source.upgrade().is_none());
        assert!(weak_geometry.upgrade().is_some());
        drop(menu);
        drop(a);
        drop(b);
        assert!(weak_geometry.upgrade().is_none());
    });
}
