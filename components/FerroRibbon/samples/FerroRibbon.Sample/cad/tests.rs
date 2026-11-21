use super::{CadArt, CadIcon, CadIcons};
use ferroribbon_core::{RibbonString, model::RibbonIcon};
use ferroui_base::metadata::MarkupTyped;
use serde_json::Value;
use std::rc::Rc;
fn source() -> Value {
    serde_json::from_str(include_str!("../../../tests/fixtures/cad-icons.json")).unwrap()
}
fn text(value: &Value) -> Option<String> {
    if value.is_null() {
        None
    } else {
        String::from_utf16(
            &value
                .as_array()
                .unwrap()
                .iter()
                .map(|u| u.as_u64().unwrap() as u16)
                .collect::<Vec<_>>(),
        )
        .ok()
    }
}
fn icon(icon: &RibbonIcon, expected: &Value) {
    assert_eq!(icon.kind.raw(), expected["kind"].as_i64().unwrap() as i32);
    assert_eq!(
        icon.value.as_ref().map(|v| v.to_utf8().unwrap()),
        text(&expected["value"])
    );
    assert_eq!(
        icon.font_family.as_ref().map(|v| v.to_utf8().unwrap()),
        text(&expected["font"])
    );
    assert_eq!(
        icon.foreground.as_ref().map(|v| v.to_utf8().unwrap()),
        text(&expected["foreground"])
    );
    assert_eq!(
        icon.view_box_size.to_bits(),
        expected["viewbox"].as_f64().unwrap().to_bits()
    );
}
#[test]
fn all216_original_asset_strings_are_identical_through_native_static_metadata() {
    let source = source();
    assert_eq!(CadIcons::MARKUP.static_properties.len(), 180);
    assert_eq!(CadArt::MARKUP.static_properties.len(), 36);
    for row in source["properties"].as_array().unwrap() {
        let meta = if row["owner"] == "CadIcons" {
            CadIcons::MARKUP
        } else {
            CadArt::MARKUP
        };
        let property = meta
            .find_static_property(row["name"].as_str().unwrap())
            .unwrap();
        let value = (property.get.unwrap())(&[]).unwrap().unwrap();
        assert_eq!(
            value.downcast_ref::<String>().unwrap(),
            &text(&row["value"]).unwrap(),
            "{row}"
        );
    }
}
#[test]
fn source_lookup_cache_identity_fallback_sizes_and_icon_wrapping() {
    assert!(String::from_utf16(&[0xd800]).is_err());
    let source = source();
    let mut checked = 0;
    let mut unsupported = 0;
    for row in source["lookups"].as_array().unwrap() {
        let Some(name) = text(&row["name"]) else {
            unsupported += 1;
            continue;
        };
        let value = CadIcon::get(&name);
        let again = CadIcon::get(&name);
        let extension = CadIcon::new();
        extension.set_name(name);
        let provided = extension.provide_value();
        if row["icon"]["isNull"] == true {
            assert!(value.is_none() && again.is_none());
            assert_eq!(provided.downcast_ref::<String>().unwrap(), "?");
        } else {
            let value = value.unwrap();
            icon(&value, &row["icon"]);
            assert!(Rc::ptr_eq(&value, &again.unwrap()));
            assert!(Rc::ptr_eq(
                &value,
                provided.downcast_ref::<Rc<RibbonIcon>>().unwrap()
            ));
        }
        checked += 1;
    }
    assert_eq!((checked, unsupported), (226, 2)); // Native UTF-8 name API: null/lone-surrogate source inputs are separate documented differences.
    for row in source["sizes"].as_array().unwrap() {
        assert_eq!(
            CadArt::size_of(&text(&row["name"]).unwrap()),
            row["size"].as_f64().unwrap()
        );
    }
    for row in source["wraps"].as_array().unwrap() {
        let value = text(&row["input"]).map(RibbonString::from);
        let a = CadIcons::icon(value.clone());
        let b = CadIcons::icon(value);
        icon(&a, &row["icon"]);
        assert!(!Rc::ptr_eq(&a, &b));
    }
}
#[test]
fn native_extension_constructor_name_setter_and_provide_value_use_same_cache() {
    let value = (CadIcon::MARKUP.constructors[0].invoke)(&[])
        .unwrap()
        .unwrap();
    let extension = value.downcast_ref::<Rc<CadIcon>>().unwrap();
    assert_eq!(extension.name(), "");
    let name = CadIcon::MARKUP.find_property("Name").unwrap();
    (name.set.unwrap())(&[Some(value.clone()), Some(Rc::new("LineHelp".to_owned()))]).unwrap();
    let result = (CadIcon::MARKUP
        .find_methods("ProvideValue")
        .next()
        .unwrap()
        .invoke)(&[Some(value.clone())])
    .unwrap()
    .unwrap();
    assert!(Rc::ptr_eq(
        result.downcast_ref::<Rc<RibbonIcon>>().unwrap(),
        &CadIcon::get("LineHelp").unwrap()
    ));
    assert_eq!(
        extension
            .provide_value()
            .downcast_ref::<Rc<RibbonIcon>>()
            .unwrap()
            .view_box_size,
        160.
    );
}

#[test]
fn original718_layers_match_core_and_are_accepted_by_native_geometry_and_presenter() {
    use ferroribbon_core::model::RibbonIconLayer;
    use ferroui_base::{media::Geometry, threading::CancellationToken};
    use ferroui_controls::{AppBuilder, Application, Canvas, Viewbox};
    use ferroui_headless::{
        FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, HeadlessUnitTestSession,
    };
    fn build() -> AppBuilder {
        AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
    }
    let session = HeadlessUnitTestSession::start_new(build);
    let result = session
        .dispatch(
            || {
                ferroribbon::register_types();
                let source = source();
                let mut total = 0;
                for row in source["properties"].as_array().unwrap() {
                    let value = RibbonString::from(text(&row["value"]).unwrap());
                    let layers = RibbonIconLayer::parse(Some(&value));
                    let expected = row["layers"].as_array().unwrap();
                    assert_eq!(layers.count() as usize, expected.len(), "{row}");
                    for (layer, expected) in layers.iter().zip(expected) {
                        let layer = layer.unwrap();
                        assert_eq!(
                            layer.data().as_ref().map(|s| s.to_utf8().unwrap()),
                            text(&expected["data"])
                        );
                        assert_eq!(
                            layer.color().as_ref().map(|s| s.to_utf8().unwrap()),
                            text(&expected["color"])
                        );
                        assert_eq!(
                            layer.stroke_thickness().to_bits(),
                            expected["strokeBits"].as_u64().unwrap()
                        );
                        assert_eq!(
                            layer.opacity().to_bits(),
                            expected["opacityBits"].as_u64().unwrap()
                        );
                        Geometry::parse(&layer.data().unwrap().to_utf8().unwrap()).unwrap();
                        total += 1;
                    }
                    let icon = ferroribbon::primitives::RibbonIconPresenter::create_path(
                        Some(&value),
                        row["viewbox"].as_f64().unwrap(),
                        24.,
                        None,
                    );
                    let canvas = icon
                        .cast::<Viewbox>()
                        .unwrap()
                        .child()
                        .unwrap()
                        .cast::<Canvas>()
                        .unwrap();
                    assert_eq!(
                        canvas.children().count(),
                        expected.len(),
                        "Every original asset layer must survive native presenter parsing"
                    );
                }
                assert_eq!(total, 718);
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}
