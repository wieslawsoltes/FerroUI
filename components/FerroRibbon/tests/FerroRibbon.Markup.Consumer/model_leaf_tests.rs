//! Native leaf metadata against unchanged C# and actual independent compiled/runtime XAML.
use ferroribbon::metadata::{
    model_icon::{RibbonIconLayerMetadata, RibbonIconMetadata},
    model_screen_tip::RibbonScreenTipMetadata,
};
use ferroribbon_core::{
    RibbonString,
    model::{RibbonIcon, RibbonIconKind, RibbonScreenTip},
};
use ferroui_base::{
    BoxedValue,
    data::core::{ValueType, ValueTypes},
    metadata::{MarkupType, MarkupTyped},
    threading::Dispatcher,
};
use ferroui_system::component_model::INotifyPropertyChanged;
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc};
fn corpus() -> Value {
    serde_json::from_str(include_str!("../fixtures/model-leaf.json")).unwrap()
}
fn property(m: &'static MarkupType, name: &str) -> &'static ferroui_base::metadata::MarkupProperty {
    m.properties.iter().find(|p| p.name == name).unwrap()
}
fn set(m: &'static MarkupType, value: &BoxedValue, name: &str, input: Option<BoxedValue>) {
    (property(m, name).set.unwrap())(&[Some(value.clone()), input]).unwrap();
}
fn get(m: &'static MarkupType, value: &BoxedValue, name: &str) -> Option<BoxedValue> {
    (property(m, name).get.unwrap())(&[Some(value.clone())]).unwrap()
}
fn text(v: &str) -> Option<BoxedValue> {
    Some(Rc::new(v.to_owned()))
}
fn cast<T: ferroui_base::PropertyValue>(v: &BoxedValue) -> T {
    ValueTypes::try_cast(v, ValueType::of::<T>())
        .unwrap()
        .downcast_ref::<T>()
        .unwrap()
        .clone()
}
#[test]
fn source_leaf_constructor_property_shapes_and_explicit_init_gap() {
    crate::tests::run(|| {
        for m in [
            RibbonIconMetadata::MARKUP,
            RibbonIconLayerMetadata::MARKUP,
            RibbonScreenTipMetadata::MARKUP,
        ] {
            let source = corpus();
            let class = source["corpus"]["classes"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["name"] == m.name)
                .unwrap();
            assert_eq!(
                m.constructors
                    .iter()
                    .map(|c| c.parameters.len())
                    .collect::<Vec<_>>(),
                class["arities"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|n| n.as_u64().unwrap() as usize)
                    .collect::<Vec<_>>()
            );
            let mut actual = m.properties.iter().map(|p| p.name).collect::<Vec<_>>();
            actual.sort_unstable();
            assert_eq!(
                actual,
                class["properties"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|p| p["name"].as_str().unwrap())
                    .collect::<Vec<_>>()
            );
            for p in class["properties"].as_array().unwrap() {
                let descriptor = property(m, p["name"].as_str().unwrap());
                assert!(descriptor.get.is_some());
                if p["initOnly"] == true {
                    assert_eq!(m.name, "RibbonIcon");
                    assert!(
                        descriptor.set.is_none(),
                        "missing init semantics must not become mutable setters"
                    );
                } else {
                    assert_eq!(descriptor.set.is_some(), p["canWrite"].as_bool().unwrap());
                }
            }
        }
        assert_eq!(
            RibbonScreenTipMetadata::MARKUP.base_type().unwrap().name,
            "ObservableObject"
        );
        for e in corpus()["corpus"]["enumNames"].as_array().unwrap() {
            assert_eq!(
                RibbonIconKind::from_raw(e["value"].as_i64().unwrap() as i32).to_string(),
                e["text"]
            );
        }
    });
}
#[test]
fn source_screentip_notifications_reentrancy_structural_image_equality_and_detach() {
    crate::tests::run(|| {
        let m = RibbonScreenTipMetadata::MARKUP;
        let args = [text("Title"), text("Description"), text("Ctrl+T")];
        let value = (m.constructors[3].invoke)(&args).unwrap().unwrap();
        let tip = cast::<Rc<RibbonScreenTip>>(&value);
        let first = RibbonIcon::text_with_foreground(Some("📐".into()), Some("#112233".into()));
        let equal = RibbonIcon::text_with_foreground(Some("📐".into()), Some("#112233".into()));
        let changed = first.with_foreground(Some("#445566".into()));
        let trace = Rc::new(RefCell::new(Vec::new()));
        let weak = Rc::downgrade(&tip);
        let output = trace.clone();
        let handler = tip.property_changed().add(Rc::new(move |name| {
            let tip = weak.upgrade().unwrap();
            output.borrow_mut().push(format!(
                "{name}:{}:{}",
                tip.title()
                    .map_or_else(|| "<null>".into(), |s| s.to_utf8().unwrap()),
                tip.description()
                    .map_or_else(|| "<null>".into(), |s| s.to_utf8().unwrap())
            ));
            if name == "Title" && tip.title() == Some("Nested".into()) {
                tip.set_description(Some("Inside".into()));
            }
        }));
        set(m, &value, "Title", text("Title"));
        set(m, &value, "Title", text("Nested"));
        set(m, &value, "Description", text("Inside"));
        for (name, input) in [
            ("Shortcut", "Alt+P"),
            ("HelpText", "Help"),
            ("DisabledReason", "Disabled"),
        ] {
            set(m, &value, name, text(input));
        }
        set(m, &value, "Image", Some(Rc::new(first.clone())));
        set(m, &value, "Image", Some(Rc::new(equal.clone())));
        assert_eq!(
            Rc::ptr_eq(&tip.image().unwrap(), &first),
            corpus()["corpus"]["retainedEqualImage"]
        );
        set(m, &value, "Image", Some(Rc::new(changed)));
        set(m, &value, "ExtendedDescription", text("Extended"));
        set(m, &value, "ExtendedImage", Some(Rc::new(first.clone())));
        set(m, &value, "ExtendedImage", Some(Rc::new(equal)));
        assert_eq!(
            Rc::ptr_eq(&tip.extended_image().unwrap(), &first),
            corpus()["corpus"]["retainedEqualExtendedImage"]
        );
        set(m, &value, "Image", None);
        set(m, &value, "ExtendedImage", None);
        tip.property_changed().remove(handler);
        set(m, &value, "Title", text("Detached"));
        assert_eq!(json!(*trace.borrow()), corpus()["corpus"]["trace"]);
        for p in [
            "Title",
            "Description",
            "Shortcut",
            "HelpText",
            "DisabledReason",
            "ExtendedDescription",
        ] {
            assert_eq!(
                json!(cast::<Option<String>>(&get(m, &value, p).unwrap())),
                corpus()["corpus"]["final"][p]
            );
        }
    });
}
#[test]
fn native_leaf_optional_constructor_defaults_and_invalid_utf16_getters() {
    crate::tests::run(|| {
        let m = RibbonScreenTipMetadata::MARKUP;
        let args = [text("T"), text("D"), text("S")];
        for ctor in m.constructors {
            let n = ctor.parameters.len();
            let v = (ctor.invoke)(&args[..n]).unwrap().unwrap();
            let tip = cast::<Rc<RibbonScreenTip>>(&v);
            assert_eq!(tip.title(), (n > 0).then(|| "T".into()));
            assert_eq!(tip.description(), (n > 1).then(|| "D".into()));
            assert_eq!(tip.shortcut(), (n > 2).then(|| "S".into()));
            for p in ["HelpText", "DisabledReason", "ExtendedDescription"] {
                assert!(get(m, &v, p).is_none());
            }
            assert!(tip.image().is_none());
            assert!(tip.extended_image().is_none());
        }
        let args = [
            Some(Rc::new(RibbonIconKind::from_raw(-7)) as BoxedValue),
            None,
            text("font"),
            text("color"),
            Some(Rc::new(-0.) as BoxedValue),
        ];
        for ctor in RibbonIconMetadata::MARKUP.constructors {
            let n = ctor.parameters.len();
            let v = (ctor.invoke)(&args[..n]).unwrap().unwrap();
            let icon = cast::<Rc<RibbonIcon>>(&v);
            assert_eq!(icon.kind.raw(), -7);
            assert!(icon.value.is_none());
            assert_eq!(icon.font_family, (n > 2).then(|| "font".into()));
            assert_eq!(icon.foreground, (n > 3).then(|| "color".into()));
            assert_eq!(
                icon.view_box_size.to_bits(),
                if n > 4 {
                    (-0.0f64).to_bits()
                } else {
                    24.0f64.to_bits()
                }
            );
        }
        let bad = RibbonString::from_utf16(vec![0xd800]);
        let tip = RibbonScreenTip::with_title(Some(bad.clone()));
        let value = Rc::new(tip.clone()) as BoxedValue;
        assert!(tip.title().unwrap().to_utf8().is_err());
        assert!((property(m, "Title").get.unwrap())(&[Some(value)]).is_err());
        let icon = RibbonIcon::text(Some(bad));
        let value = Rc::new(icon) as BoxedValue;
        assert!(
            (property(RibbonIconMetadata::MARKUP, "Value").get.unwrap())(&[Some(value)]).is_err()
        );
    });
}
#[test]
fn independent_model_leaf_xaml_bindings_factory_replacement_and_resource_identity() {
    crate::tests::run(|| {
        use ferroribbon::{
            controls::RibbonScreenTip as TipControl,
            controls::{Ribbon, RibbonButton},
        };
        use ferroribbon_core::model::{RibbonButtonModelImpl, RibbonNodeModelExt};
        use ferroui_controls::{StackPanel, TextBlock, TextBox, ToolTip, Window};
        let _theme = crate::tests::NativeThemeScope::new();
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("ModelLeaves.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/ModelLeaves.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let host = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let resource = |key: &str| {
                host.resources()
                    .try_get_resource(&key.into(), None)
                    .flatten()
                    .unwrap()
            };
            let tip = cast::<Rc<RibbonScreenTip>>(&resource("Tip"));
            let icon = cast::<Rc<RibbonIcon>>(&resource("Illustration"));
            let model = cast::<Rc<dyn RibbonButtonModelImpl>>(&resource("Command"));
            assert!(Rc::ptr_eq(&tip.image().unwrap(), &icon));
            assert!(Rc::ptr_eq(&tip.extended_image().unwrap(), &icon));
            assert!(Rc::ptr_eq(&model.icon().unwrap(), &icon));
            assert!(Rc::ptr_eq(&model.screen_tip().unwrap(), &tip));
            assert_eq!(icon.kind, RibbonIconKind::Text);
            assert_eq!(icon.value, Some("📐".into()));
            assert_eq!(icon.view_box_size, 32.);
            let ribbon = host.children().get(0).cast::<Ribbon>().unwrap();
            let title = host.children().get(1).cast::<TextBlock>().unwrap();
            let image = host.children().get(2).cast::<TextBlock>().unwrap();
            let editor = host.children().get(3).cast::<TextBox>().unwrap();
            let window = Window::new();
            window.set_width(900.);
            window.set_height(350.);
            window.set_content(Some(Rc::new(host.clone()) as BoxedValue));
            window.show();
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(title.text().as_deref(), Some("Measure é 📐"));
            assert_eq!(image.text().as_deref(), Some("📐"));
            let button = ribbon
                .find_item(Some("measure".into()))
                .unwrap()
                .cast::<RibbonButton>()
                .unwrap();
            assert!(Rc::ptr_eq(
                &cast::<Rc<RibbonScreenTip>>(&button.screen_tip().unwrap()),
                &tip
            ));
            let native_tip = || {
                let wrapper = ValueTypes::as_object(&*ToolTip::get_tip(&button).unwrap())
                    .unwrap()
                    .cast::<ToolTip>()
                    .unwrap();
                ValueTypes::as_object(&*wrapper.content().unwrap())
                    .unwrap()
                    .cast::<TipControl>()
                    .unwrap()
            };
            let tip_control = native_tip();
            assert_eq!(tip_control.title(), Some("Measure é 📐".into()));
            tip.set_title(Some("Changed".into()));
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(title.text().as_deref(), Some("Changed"));
            assert_eq!(editor.text().as_deref(), Some("Changed"));
            let updated = native_tip();
            assert_eq!(updated.title(), Some("Changed".into()));
            editor.set_text(Some("From editor"));
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(tip.title(), Some("From editor".into()));
            let replacement = RibbonScreenTip::with_title(Some("Replacement".into()));
            model.set_screen_tip(Some(replacement.clone()));
            Dispatcher::ui_thread().run_jobs(None);
            let current = native_tip();
            assert_eq!(current.title(), Some("Replacement".into()));
            tip.set_title(Some("Detached old".into()));
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(native_tip().title(), Some("Replacement".into()));
            replacement.set_title(Some("New changed".into()));
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(native_tip().title(), Some("New changed".into()));
            ribbon.set_model(None);
            replacement.set_title(Some("Unbound".into()));
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(native_tip().title(), Some("New changed".into()));
            window.close();
        }
    });
}

#[test]
fn source_icon_factory_optional_arities_and_layer_formatting_through_native_metadata() {
    crate::tests::run(|| {
        let mut actual = Vec::new();
        for name in ["Glyph", "Path", "Stroke", "Image", "Text"] {
            let arguments: Vec<Option<BoxedValue>> = match name {
                "Glyph" => vec![text("x"), text("font"), text("#112233")],
                "Path" => vec![text("M0,0 L4,4"), Some(Rc::new(32.)), text("#112233")],
                "Stroke" => vec![
                    text("M0,0 L4,4"),
                    Some(Rc::new(2.5)),
                    Some(Rc::new(32.)),
                    text("#112233"),
                ],
                "Image" => vec![text("https://example.test/icon.png")],
                _ => vec![text("📐"), text("#112233")],
            };
            for method in RibbonIconMetadata::MARKUP
                .methods
                .iter()
                .filter(|m| m.name == name)
            {
                let arity = method.parameters.len();
                let value = (method.invoke)(&arguments[..arity]).unwrap().unwrap();
                let icon = cast::<Rc<RibbonIcon>>(&value);
                actual.push(json!({"name":name,"arity":arity,"kind":icon.kind.raw(),"Value":icon.value.as_ref().map(|s|s.to_utf8().unwrap()),"FontFamily":icon.font_family.as_ref().map(|s|s.to_utf8().unwrap()),"Foreground":icon.foreground.as_ref().map(|s|s.to_utf8().unwrap()),"viewBoxBits":format!("{:016x}",icon.view_box_size.to_bits())}));
            }
        }
        assert_eq!(json!(actual), corpus()["corpus"]["factoryCalls"]);
        let original = RibbonIcon::text(Some("Original".into()));
        let original_value = Rc::new(original.clone()) as BoxedValue;
        let with_color = RibbonIconMetadata::MARKUP
            .methods
            .iter()
            .find(|m| m.name == "WithForeground")
            .unwrap();
        let changed_value = (with_color.invoke)(&[Some(original_value), text("#445566")])
            .unwrap()
            .unwrap();
        let changed = cast::<Rc<RibbonIcon>>(&changed_value);
        assert!(!Rc::ptr_eq(&changed, &original));
        assert_eq!(changed.value, original.value);
        assert_eq!(changed.foreground, Some("#445566".into()));
        assert!(original.foreground.is_none());
        let args = [
            text("M0,0 L4,4"),
            Some(Rc::new(2.5) as BoxedValue),
            text("#112233"),
            Some(Rc::new(0.25) as BoxedValue),
        ];
        for constructor in RibbonIconLayerMetadata::MARKUP.constructors {
            let arity = constructor.parameters.len();
            let value = (constructor.invoke)(&args[..arity]).unwrap().unwrap();
            let layer = cast::<Rc<ferroribbon_core::model::RibbonIconLayer>>(&value);
            assert_eq!(layer.data(), Some("M0,0 L4,4".into()));
            assert_eq!(layer.stroke_thickness(), if arity > 1 { 2.5 } else { 0. });
            assert_eq!(layer.color(), (arity > 2).then(|| "#112233".into()));
            assert_eq!(layer.opacity(), if arity > 3 { 0.25 } else { 1. });
            let method = RibbonIconLayerMetadata::MARKUP
                .methods
                .iter()
                .find(|m| m.name == "ToString")
                .unwrap();
            let result = (method.invoke)(&[Some(value)]).unwrap().unwrap();
            assert_eq!(
                cast::<String>(&result),
                layer.to_text().unwrap().to_utf8().unwrap()
            );
        }
        let original: Value = serde_json::from_str(include_str!("../fixtures/icons.json")).unwrap();
        let format = RibbonScreenTipMetadata::MARKUP
            .methods
            .iter()
            .find(|m| m.name == "FormatTitle")
            .unwrap();
        let mut valid = 0;
        let mut invalid = 0;
        for case in original["titles"].as_array().unwrap() {
            let decode = |v: &Value| -> Result<Option<String>, std::string::FromUtf16Error> {
                v.as_array()
                    .map(|a| {
                        String::from_utf16(
                            &a.iter()
                                .map(|n| n.as_u64().unwrap() as u16)
                                .collect::<Vec<_>>(),
                        )
                    })
                    .transpose()
            };
            match (decode(&case["title"]), decode(&case["shortcut"])) {
                (Ok(title), Ok(shortcut)) => {
                    let result = (format.invoke)(&[
                        title.map(|s| Rc::new(s) as BoxedValue),
                        shortcut.map(|s| Rc::new(s) as BoxedValue),
                    ])
                    .unwrap()
                    .unwrap();
                    assert_eq!(
                        json!(cast::<String>(&result).encode_utf16().collect::<Vec<_>>()),
                        case["result"]
                    );
                    valid += 1;
                }
                _ => {
                    invalid += 1;
                } // Native String cannot express these source inputs; Core corpus still tests them.
            }
        }
        assert!(valid > 0);
        assert!(invalid > 0);
    });
}
