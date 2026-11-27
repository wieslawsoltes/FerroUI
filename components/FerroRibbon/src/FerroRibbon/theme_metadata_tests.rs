use crate::{
    metadata::theme_types::{
        RibbonColorMetadata, RibbonThemePaletteMetadata, RibbonThemePalettePresetsMetadata,
    },
    tests::run,
    theming::*,
};
use ferroui_base::{
    Ref,
    data::core::ValueType,
    media::Color,
    metadata::{MarkupType, MarkupTyped, MarkupValue, from_markup_value, into_markup_value},
};
use std::{any::TypeId, rc::Rc};
fn get(meta: &MarkupType, name: &str, instance: MarkupValue) -> MarkupValue {
    (meta.find_property(name).unwrap().get.unwrap())(&[instance]).unwrap()
}
fn call(meta: &MarkupType, name: &str, args: &[MarkupValue]) -> MarkupValue {
    (meta
        .find_methods(name)
        .find(|m| m.parameters.len() + usize::from(!m.is_static) == args.len())
        .unwrap()
        .invoke)(args)
    .unwrap()
}
fn source() -> serde_json::Value {
    serde_json::from_str(include_str!("../../tests/fixtures/theme-metadata.json")).unwrap()
}
#[test]
fn native_color_metadata_uses_actual_core_value_and_nullable_owner() {
    run(|| {
        let m = RibbonColorMetadata::MARKUP;
        assert_eq!(m.full_name(), "FerroRibbon.Theming.RibbonColor");
        assert_eq!(
            m.nullable.unwrap()(),
            ValueType::of::<Option<RibbonColor>>()
        );
        assert!(std::ptr::eq(
            MarkupType::find_by_nullable_handle(TypeId::of::<Option<RibbonColor>>()).unwrap(),
            m
        ));
        let c = RibbonColor::new(17, 1, 2, 3);
        let value = into_markup_value(c);
        assert_eq!(
            from_markup_value::<u8>(&get(m, "A", value.clone())),
            Some(17)
        );
        assert_eq!(
            from_markup_value::<String>(&call(m, "ToHex", std::slice::from_ref(&value))),
            Some("#11010203".into())
        );
        let rgba = call(m, "Parse", &[into_markup_value(Some("#abc".to_string()))]);
        assert_eq!(
            from_markup_value::<RibbonColor>(&rgba),
            Some(RibbonColor::from_rgb(170, 187, 204))
        );
        let method = m.find_methods("Parse").next().unwrap();
        assert!((method.invoke)(&[None]).is_err());
        assert_eq!(
            from_markup_value::<RibbonColor>(&call(
                m,
                "FromHsl",
                &[
                    into_markup_value(210.),
                    into_markup_value(0.5),
                    into_markup_value(0.4)
                ]
            )),
            Some(RibbonColor::from_hsl(210., 0.5, 0.4))
        );
        for property in source()["surfaces"][0]["properties"].as_array().unwrap() {
            let name = property["name"].as_str().unwrap();
            assert!(
                m.find_property(name).is_some() || m.find_static_property(name).is_some(),
                "unmapped source getter {name}"
            );
        }
        // Init-only setters/record synthesis/TryParse out ABI are not claimed by
        // these getter/algorithm descriptors; the runtime surface fixture exposes them.
        assert!(m.find_property("A").unwrap().set.is_none());
    });
}
#[test]
fn native_palette_metadata_preserves_constructor_names_presets_and_list_identity() {
    run(|| {
        let m = RibbonThemePaletteMetadata::MARKUP;
        for name in [
            "Word",
            "Excel",
            "PowerPoint",
            "Outlook",
            "OneNote",
            "Access",
            "Visio",
            "Project",
            "Publisher",
            "Teams",
            "Graphite",
            "Cad",
        ] {
            let result = (m.find_static_property(name).unwrap().get.unwrap())(&[]).unwrap();
            let p = from_markup_value::<Rc<RibbonThemePalette>>(&result).unwrap();
            let index = RibbonThemePalette::presets()
                .iter()
                .position(|v| Rc::ptr_eq(v, &p))
                .unwrap();
            assert!(Rc::ptr_eq(&p, &RibbonThemePalette::presets()[index]));
        }
        for row in source()["specialNames"].as_array().unwrap().iter().take(2) {
            let name = row.as_array().map(|r| {
                String::from_utf16(
                    &r.iter()
                        .map(|v| v.as_u64().unwrap() as u16)
                        .collect::<Vec<_>>(),
                )
                .unwrap()
            });
            let value = (m.constructors[0].invoke)(&[
                into_markup_value(name.clone()),
                into_markup_value(RibbonColor::BLACK),
                into_markup_value(RibbonColor::WHITE),
            ])
            .unwrap();
            assert_eq!(
                from_markup_value::<Option<String>>(&get(m, "Name", value.clone())).unwrap(),
                name
            );
            assert_eq!(
                from_markup_value::<RibbonColor>(&call(
                    m,
                    "GetAccent",
                    &[value, into_markup_value(true)]
                )),
                Some(RibbonColor::WHITE)
            );
        }
        let invalid = RibbonThemePalette::new(
            Some(ferroribbon_core::RibbonString::from_utf16(vec![0xd800])),
            RibbonColor::BLACK,
            RibbonColor::WHITE,
        );
        assert!(
            (m.find_property("Name").unwrap().get.unwrap())(&[into_markup_value(invalid)]).is_err()
        );
        let list = (m.find_static_property("Presets").unwrap().get.unwrap())(&[]).unwrap();
        let actual = from_markup_value::<Rc<[Rc<RibbonThemePalette>]>>(&list).unwrap();
        assert!(Rc::ptr_eq(&actual, &RibbonThemePalette::presets()));
        let list_m = RibbonThemePalettePresetsMetadata::MARKUP;
        assert_eq!(
            from_markup_value::<i32>(&get(list_m, "Count", list.clone())),
            Some(source()["preset"]["count"].as_i64().unwrap() as i32)
        );
        let indexer = list_m.find_indexer(1).unwrap();
        assert!(indexer.set.is_none());
        let second = (indexer.get.unwrap())(&[list.clone(), into_markup_value(1_i32)]).unwrap();
        assert!(Rc::ptr_eq(
            &from_markup_value::<Rc<RibbonThemePalette>>(&second).unwrap(),
            &RibbonThemePalette::excel()
        ));
        for (i, key) in [(-1, "negative"), (12, "pastEnd")] {
            let err = RibbonThemePalettePresetsMetadata::item(&actual, i).unwrap_err();
            assert_eq!(err, source()["preset"][key].as_str().unwrap());
            assert!((indexer.get.unwrap())(&[list.clone(), into_markup_value(i)]).is_err());
        }
    });
}
#[test]
fn native_theme_static_properties_overloads_and_errors_use_the_same_service() {
    run(|| {
        RibbonTheme::reset_test_cache();
        let app = ferroui_controls::Application::current().unwrap();
        let previous = app.resources();
        app.set_resources(ferroui_base::controls::ResourceDictionary::new());
        let m = RibbonTheme::MARKUP;
        assert!(m.type_info.is_some());
        let resources = (m.find_static_property("Resources").unwrap().get.unwrap())(&[]).unwrap();
        assert_eq!(
            from_markup_value::<Ref<RibbonThemeResources>>(&resources),
            Some(RibbonTheme::resources())
        );
        call(
            m,
            "Apply",
            &[
                into_markup_value(Some(RibbonThemePalette::cad())),
                into_markup_value(RibbonChromeStyle::Neutral),
                into_markup_value(RibbonThemeStyle::Cad),
            ],
        );
        assert_eq!(RibbonTheme::style(), RibbonThemeStyle::Cad);
        call(
            m,
            "Apply",
            &[
                into_markup_value(Some(RibbonThemePalette::excel())),
                into_markup_value(RibbonChromeStyle::Colorful),
            ],
        );
        assert!(Rc::ptr_eq(
            &RibbonTheme::palette(),
            &RibbonThemePalette::excel()
        ));
        assert_eq!(RibbonTheme::style(), RibbonThemeStyle::Cad);
        let palette = (m.find_static_property("Palette").unwrap().get.unwrap())(&[]).unwrap();
        assert!(Rc::ptr_eq(
            &from_markup_value::<Rc<RibbonThemePalette>>(&palette).unwrap(),
            &RibbonTheme::palette()
        ));
        let rm = RibbonThemeResources::TYPE.markup().unwrap();
        let instance = into_markup_value(RibbonTheme::resources());
        let brush = call(
            rm,
            "GetBrush",
            &[
                instance.clone(),
                into_markup_value(Some("RibbonAccentBrush".to_owned())),
            ],
        );
        let brush = from_markup_value::<Option<Ref<ferroui_base::media::SolidColorBrush>>>(&brush)
            .unwrap()
            .unwrap();
        assert_eq!(
            brush,
            RibbonTheme::resources()
                .get_brush("RibbonAccentBrush", None)
                .unwrap()
        );
        call(
            rm,
            "SetColor",
            &[
                instance.clone(),
                into_markup_value(Some("RibbonAccentBrush".to_owned())),
                into_markup_value(Color::from_argb(17, 3, 5, 7)),
            ],
        );
        assert_eq!(brush.color(), Color::from_argb(17, 3, 5, 7));
        assert_eq!(
            from_markup_value::<RibbonThemeStyle>(&get(rm, "Style", instance.clone())),
            Some(RibbonThemeStyle::Cad)
        );
        assert!(
            (rm.find_methods("GetBrush")
                .find(|m| m.parameters.len() == 1)
                .unwrap()
                .invoke)(&[instance, None])
            .is_err()
        );
        let color = RibbonColor::new(17, 9, 19, 29);
        let native = call(m, "ToColor", &[into_markup_value(color)]);
        assert_eq!(from_markup_value::<Color>(&native), Some(to_color(color)));
        assert_eq!(
            from_markup_value::<RibbonColor>(&call(m, "ToRibbonColor", &[native])),
            Some(color)
        );
        assert!((m.find_methods("ApplyPalette").next().unwrap().invoke)(&[None]).is_err());
        let corner_error = RibbonTheme::markup_get_corner_radius(None, None, 99.).unwrap_err();
        assert_eq!(
            serde_json::json!({"type":"ArgumentNullException","param":corner_error.parameter}),
            source()["nullKeys"]["nullCorner"]
        );
        let brush_error =
            RibbonTheme::markup_set_brush_color(None, Color::from_argb(255, 1, 2, 3)).unwrap_err();
        assert_eq!(
            serde_json::json!({"type":"ArgumentNullException","param":brush_error.parameter}),
            source()["nullKeys"]["nullBrush"]
        );
        let r = RibbonTheme::resources();
        r.remove_theme_dictionary(&ferroui_base::styling::ThemeVariant::light());
        r.remove_theme_dictionary(&ferroui_base::styling::ThemeVariant::dark());
        assert_eq!(
            RibbonTheme::markup_get_corner_radius(None, None, 99.)
                .unwrap()
                .top_left,
            source()["nullKeys"]["missingCorner"].as_f64().unwrap()
        );
        app.set_resources(previous);
        RibbonTheme::reset_test_cache();
    });
}
#[test]
fn theming_enum_descriptors_preserve_unknown_numeric_values_and_nullable_handles() {
    run(|| {
        use crate::metadata::enums::{RibbonChromeStyleMetadata, RibbonThemeStyleMetadata};
        let chrome = RibbonChromeStyleMetadata::MARKUP;
        let style = RibbonThemeStyleMetadata::MARKUP;
        for m in [chrome, style] {
            assert_eq!(m.namespace(), "FerroRibbon.Theming");
            assert!(m.find_enum_member("missing", false).is_none());
            for v in [-37_i64, 0, 1, 99, i64::from(i32::MAX)] {
                let value = m.enum_from_value.unwrap()(v).unwrap();
                assert_eq!(m.enum_to_value.unwrap()(&*value), Some(v));
            }
            assert!(m.enum_from_value.unwrap()(i64::from(i32::MAX) + 1).is_none());
        }
        assert_eq!(
            chrome.nullable.unwrap()(),
            ValueType::of::<Option<RibbonChromeStyle>>()
        );
        assert_eq!(
            style.nullable.unwrap()(),
            ValueType::of::<Option<RibbonThemeStyle>>()
        );
        assert_eq!(
            from_markup_value::<RibbonChromeStyle>(
                &chrome
                    .find_enum_member("Neutral", false)
                    .unwrap()
                    .get(chrome)
            ),
            Some(RibbonChromeStyle::Neutral)
        );
        assert_eq!(
            from_markup_value::<RibbonThemeStyle>(
                &style.find_enum_member("Cad", false).unwrap().get(style)
            ),
            Some(RibbonThemeStyle::Cad)
        );
    });
}

#[test]
fn native_theme_event_metadata_has_both_nullable_public_accessors_without_an_instance() {
    run(|| {
        use ferroui_base::{BoxedValue, metadata::MarkupDelegate, utilities::EventArgs};
        use std::{cell::Cell, rc::Rc};
        RibbonTheme::reset_test_changed();
        let meta = RibbonTheme::MARKUP;
        assert!(RibbonTheme::TYPE.default_constructor().is_none());
        let event = meta.find_event("Changed").unwrap();
        assert!(event.is_static && event.handler_nullable && event.remove.is_some());
        assert_eq!(
            (event.arguments[0])(),
            ValueType::of::<Option<BoxedValue>>()
        );
        assert_eq!((event.arguments[1])(), ValueType::of::<EventArgs>());
        let source: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/theme-events.json")).unwrap();
        for accessor in source["accessors"].as_array().unwrap() {
            let name = accessor["name"].as_str().unwrap();
            let methods = meta.find_methods(name).collect::<Vec<_>>();
            assert_eq!(methods.len(), 1);
            let method = methods[0];
            assert_eq!(method.is_static, accessor["isStatic"].as_bool().unwrap());
            assert!(method.return_type.is_none());
            assert_eq!(accessor["returns"], "Void");
            // Physical native nullable delegate ABI; nominal EventHandler identity
            // remains a separate framework limitation, never relabeled as equal.
            assert_eq!(
                (method.parameters[0])(),
                ValueType::of::<Option<MarkupDelegate>>()
            );
            assert_eq!((method.invoke)(&[None]), Ok(None));
        }
        let count = Rc::new(Cell::new(0));
        let called = count.clone();
        let handler = MarkupDelegate::new(move |args| {
            assert!(args[0].is_none());
            assert_eq!(
                from_markup_value::<EventArgs>(&args[1]),
                Some(EventArgs::EMPTY)
            );
            called.set(called.get() + 1);
            None
        });
        (event.add)(&[into_markup_value(handler.clone())]).unwrap();
        RibbonTheme::apply_chrome_style(RibbonChromeStyle::Neutral).unwrap();
        assert_eq!(count.get(), 1);
        (meta.find_methods("remove_Changed").next().unwrap().invoke)(&[into_markup_value(handler)])
            .unwrap();
        RibbonTheme::apply_chrome_style(RibbonChromeStyle::Neutral).unwrap();
        assert_eq!(count.get(), 1);
        RibbonTheme::reset_test_changed();
    });
}
