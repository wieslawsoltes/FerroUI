use super::*;
use crate::mvvm::{RibbonElementFactory, RibbonModelBindings};
use ferroribbon_core::model::*;
use ferroui_base::{
    Point,
    controls::ResourceDictionary,
    input::{InputElement, Pointer, PointerEventArgs, PointerPointProperties, PointerType},
    metadata::from_markup_value,
};
use serde_json::{Value, json};
fn case(name: &str) -> Value {
    let mut value =
        serde_json::from_str::<Value>(include_str!("../../tests/fixtures/color-palette.json"))
            .unwrap()["corpus"]["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["name"] == name)
            .unwrap()
            .clone();
    // ColorNames is lookup data. The Rust snapshot deliberately has HashMap
    // ordering; compare its entries as a map while preserving every UI array.
    fn names_as_maps(value: &mut Value) {
        match value {
            Value::Object(fields) => {
                if let Some(Value::Array(names)) = fields.get("names") {
                    let mut map = serde_json::Map::new();
                    for name in names {
                        assert!(
                            map.insert(
                                name["color"].as_str().unwrap().into(),
                                name["name"].clone()
                            )
                            .is_none()
                        );
                    }
                    fields.insert("names".into(), Value::Object(map));
                }
                for value in fields.values_mut() {
                    names_as_maps(value);
                }
            }
            Value::Array(values) => values.iter_mut().for_each(names_as_maps),
            _ => (),
        }
    }
    names_as_maps(&mut value);
    value
}
fn custom(i: u8) -> Color {
    Color::from_argb(128, i, 2, 3)
}
fn hex(c: Option<Color>) -> Option<String> {
    c.map(|c| core_color(c).to_hex(false))
}
fn state(p: &RibbonColorPalette) -> Value {
    json!({"selected":hex(p.selected_color()),"recent":p.recent_colors.iter().map(|c|hex(Some(c))).collect::<Vec<_>>(),"names":p.color_names().snapshot().into_iter().map(|(c,n)|(hex(Some(c)).unwrap(),json!(n))).collect::<serde_json::Map<_,_>>(),"rows":p.stack.children().to_vec().iter().map(|c|if let Some(t)=c.cast::<TextBlock>(){format!("Header:{}",t.text().unwrap_or_default())}else if let Some(b)=c.cast::<Button>(){let panel=Control::from_boxed(&b.content().unwrap()).unwrap().cast::<StackPanel>().unwrap();let t=panel.children().to_vec().iter().find_map(|c|c.cast::<TextBlock>()).unwrap();format!("Command:{}",t.text().unwrap_or_default())}else if let Some(s)=c.cast::<StackPanel>(){format!("Stack:{}:{}",s.children().count(),s.spacing())}else{panic!("unexpected palette row")}).collect::<Vec<_>>(),"swatches":p.swatches.iter().map(|b|{let b=b.unwrap();json!({"color":hex(Some(*b.tag().unwrap().downcast_ref::<Color>().unwrap())),"name":AutomationProperties::get_name(&b),"selected":b.pseudo_classes().contains(":ribbon-selected"),"width":b.width().to_string(),"height":b.height().to_string()})}).collect::<Vec<_>>()})
}
fn watch(p: &RibbonColorPalette) -> Rc<RefCell<Vec<String>>> {
    let t = Rc::new(RefCell::new(vec![]));
    let trace = t.clone();
    p.add_color_preview(Some(MarkupDelegate::new(move |a| {
        let c = from_markup_value::<Option<Color>>(&a[1]).unwrap();
        trace.borrow_mut().push(format!(
            "preview:{}",
            hex(c).unwrap_or_else(|| "<null>".into())
        ));
        None
    })));
    let trace = t.clone();
    p.add_color_selected(Some(MarkupDelegate::new(move |a| {
        let c = from_markup_value::<Option<Color>>(&a[1]).unwrap();
        trace.borrow_mut().push(format!(
            "selected:{}",
            hex(c).unwrap_or_else(|| "<null>".into())
        ));
        None
    })));
    let trace = t.clone();
    let weak = p.to_ref().downgrade();
    p.recent_colors.collection_changed().add(Rc::new(move |e| {
        let p = weak.upgrade().unwrap();
        trace.borrow_mut().push(format!(
            "recent:{:?}:{}:{}:{}",
            e.action,
            e.new_starting_index,
            e.old_starting_index,
            p.recent_colors.count()
        ));
    }));
    t
}
fn swatch(p: &RibbonColorPalette, i: usize) -> Ref<Button> {
    p.swatches.get(i).unwrap()
}
fn pointer(b: &Button, entered: bool) {
    let e = PointerEventArgs::new(
        Some(if entered {
            InputElement::pointer_entered_event()
        } else {
            InputElement::pointer_exited_event()
        }),
        b.to_ref().upcast::<Control>(),
        Pointer::new(1, PointerType::Mouse, true),
        None,
        Point::default(),
        0,
        PointerPointProperties::default(),
        KeyModifiers::NONE,
    );
    b.raise_event(&e);
}
#[test]
fn original_palette_defaults_flags_names_and_rebuild_identity() {
    crate::group_tests::run(|| {
        let p = RibbonColorPalette::new();
        let c = case("defaults");
        assert_eq!(state(&p), c["state"]);
        assert_eq!(p.show_automatic(), c["ShowAutomatic"]);
        assert_eq!(p.show_no_color(), c["ShowNoColor"]);
        assert_eq!(p.show_more_colors(), c["ShowMoreColors"]);
        assert_eq!(p.show_theme_colors(), c["ShowThemeColors"]);
        assert_eq!(json!(hex(Some(p.automatic_color()))), c["automatic"]);
        assert!(p.color_names() == p.color_names());
        p.set_show_automatic(false);
        p.set_show_theme_colors(false);
        p.set_show_more_colors(false);
        p.set_show_no_color(true);
        assert_eq!(state(&p), case("flags")["state"]);
        let p = RibbonColorPalette::new();
        p.theme_colors().add(custom(1));
        p.standard_colors().add(custom(2));
        let names = p.color_names();
        let old = swatch(&p, 0);
        p.set_color_name(custom(1), Some("Custom".into()));
        let c = case("names");
        assert_eq!(state(&p), c["named"]);
        assert!(!old.ptr_eq(&swatch(&p, 0)));
        let old = swatch(&p, 0);
        p.set_color_name(custom(1), Some("Custom".into()));
        assert!(old.ptr_eq(&swatch(&p, 0)));
        p.set_color_name(custom(9), None);
        assert!(old.ptr_eq(&swatch(&p, 0)));
        p.set_color_name(custom(1), Some(String::new()));
        assert_eq!(state(&p), c["cleared"]);
        assert!(names == p.color_names());
    });
}
#[test]
fn original_recent_limit_repeat_and_builtin_instead_of_displayed_membership() {
    crate::group_tests::run(|| {
        let p = RibbonColorPalette::new();
        p.set_show_theme_colors(false);
        let t = watch(&p);
        for i in 0..13 {
            p.select(Some(custom(i)));
        }
        p.select(Some(custom(5)));
        p.select(None);
        p.select(Some(native_color(
            Palette::office_theme_colors()[2].as_ref().unwrap().color,
        )));
        let c = case("recent");
        assert_eq!(state(&p), c["state"]);
        assert_eq!(json!(*t.borrow()), c["trace"]);
        let p = RibbonColorPalette::new();
        p.theme_colors().add(custom(1));
        p.standard_colors().add(custom(2));
        let t = watch(&p);
        p.select(Some(custom(1)));
        p.select(Some(custom(2)));
        let c = case("custom_membership");
        assert_eq!(state(&p), c["state"]);
        assert_eq!(json!(*t.borrow()), c["trace"]);
    });
}
#[test]
fn original_preview_order_old_swatch_callbacks_and_reentrant_selected_snapshot() {
    crate::group_tests::run(|| {
        let p = RibbonColorPalette::new();
        let t = watch(&p);
        let b = swatch(&p, 0);
        pointer(&b, true);
        pointer(&b, true);
        pointer(&b, false);
        pointer(&b, false);
        pointer(&b, true);
        p.select(Some(custom(1)));
        p.end_preview();
        let c = case("preview_select");
        assert_eq!(state(&p), c["state"]);
        assert_eq!(json!(*t.borrow()), c["trace"]);
        let p = RibbonColorPalette::new();
        let t = watch(&p);
        let old = swatch(&p, 0);
        p.theme_colors().add(custom(1));
        pointer(&old, true);
        old.on_click();
        let e = ferroui_base::interactivity::RoutedEventArgs::with_event(Control::unloaded_event());
        p.raise_event(&e);
        let c = case("retained_button");
        assert_eq!(state(&p), c["state"]);
        assert_eq!(json!(*t.borrow()), c["trace"]);
        let p = RibbonColorPalette::new();
        let t = watch(&p);
        let weak = p.downgrade();
        p.add_color_selected(Some(MarkupDelegate::new(move |a| {
            let c = from_markup_value::<Option<Color>>(&a[1]).unwrap();
            if c == Some(custom(1)) {
                weak.upgrade().unwrap().select(Some(custom(2)));
            }
            None
        })));
        let trace = t.clone();
        p.add_color_selected(Some(MarkupDelegate::new(move |a| {
            let c = from_markup_value::<Option<Color>>(&a[1]).unwrap();
            trace.borrow_mut().push(format!(
                "later:{}",
                hex(c).unwrap_or_else(|| "<null>".into())
            ));
            None
        })));
        p.select(Some(custom(1)));
        let c = case("reentrant_selected");
        assert_eq!(state(&p), c["state"]);
        assert_eq!(json!(*t.borrow()), c["trace"]);
    });
}
#[test]
fn original_recent_reentrancy_throws_before_recursive_edit() {
    crate::group_tests::run(|| {
        let p = RibbonColorPalette::new();
        let t = watch(&p);
        let weak = p.downgrade();
        // Bounded even on a defective SDK: the original reentrancy failure must
        // be an assertion/error, not unbounded recursion or a stack abort.
        let attempted = Rc::new(Cell::new(false));
        p.recent_colors.collection_changed().add(Rc::new(move |_| {
            if !attempted.replace(true) {
                weak.upgrade().unwrap().recent_colors().add(custom(9));
            }
        }));
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| p.select(Some(custom(1)))))
                .is_err()
        );
        let c = case("reentrant_collection");
        assert_eq!(c["error"], "InvalidOperationException");
        assert_eq!(state(&p), c["state"]);
        assert_eq!(json!(*t.borrow()), c["trace"]);
    });
}
fn theme(p: &RibbonColorPalette) {
    let value =
        crate::compiled_shared::try_load(None, "ferres://FerroRibbon/Themes/ColorPalette.xaml")
            .unwrap()
            .unwrap();
    let dictionary = ValueTypes::as_object(&*value)
        .unwrap()
        .cast::<ResourceDictionary>()
        .unwrap();
    let theme = dictionary
        .try_get_resource(
            &<RibbonColorPalette as ferroui_base::StaticType>::TYPE.into(),
            None,
        )
        .flatten()
        .unwrap();
    p.set_resources(dictionary);
    p.set_theme(
        ValueTypes::as_object(&*theme)
            .unwrap()
            .cast::<ControlTheme>()
            .unwrap(),
    );
    p.apply_styling();
    p.apply_template();
}
#[test]
fn original_template_reuses_root_and_buttons_and_more_uses_empty_args_identity() {
    crate::group_tests::run(|| {
        let p = RibbonColorPalette::new();
        let first = swatch(&p, 0);
        let calls = Rc::new(Cell::new(0));
        let out = calls.clone();
        p.add_more_colors_requested(Some(MarkupDelegate::new(move |a| {
            let value = from_markup_value::<EventArgs>(&a[1]).unwrap();
            assert_eq!(value, EventArgs::EMPTY);
            assert!(Rc::ptr_eq(
                a[1].as_ref().unwrap(),
                &crate::compat::empty_event_args()
            ));
            out.set(out.get() + 1);
            None
        })));
        let more = p
            .stack
            .children()
            .get(p.stack.children().count() - 1)
            .cast::<Button>()
            .unwrap();
        more.on_click();
        more.on_click();
        theme(&p);
        let old = p.root.visual_parent().unwrap().cast::<Border>().unwrap();
        theme(&p);
        let c = case("template_more");
        assert_eq!(calls.get(), c["calls"]);
        assert!(old.child().is_none());
        assert!(
            p.root
                .visual_parent()
                .unwrap()
                .cast::<Border>()
                .unwrap()
                .child()
                .unwrap()
                .ptr_eq(&p.root)
        );
        assert!(first.ptr_eq(&swatch(&p, 0)));
    });
}
#[test]
fn original_factory_names_replace_recent_two_way_limit_and_scope_disposal() {
    crate::group_tests::run(|| {
        let m = RibbonColorPickerModel::new(Some("colors".into()), Some("Colors".into()));
        m.theme_colors().clear();
        m.theme_colors().add(Some(RibbonColorSwatch::new(
            core_color(custom(1)),
            Some("Named".into()),
        )));
        m.standard_colors().clear();
        m.standard_colors().add(Some(RibbonColorSwatch::new(
            core_color(custom(2)),
            Some("Standard".into()),
        )));
        for i in 0..13 {
            m.recent_colors().add(core_color(custom(i)));
        }
        let p = RibbonColorPalette::new();
        let bindings = RibbonModelBindings::new();
        RibbonElementFactory::new().bind_palette(Some(&p), Some(m.clone()), Some(&bindings));
        let c = case("factory");
        assert_eq!(state(&p), c["initial"]);
        let before = swatch(&p, 0);
        m.theme_colors().set(
            0,
            Some(RibbonColorSwatch::new(
                core_color(custom(1)),
                Some("Rename".into()),
            )),
        );
        assert_eq!(state(&p), c["renamed"]);
        assert!(!before.ptr_eq(&swatch(&p, 0)));
        p.select(Some(custom(99)));
        assert_eq!(state(&p), c["selected"]);
        assert_eq!(
            json!(
                m.recent_colors()
                    .iter()
                    .map(|v| v.to_hex(false))
                    .collect::<Vec<_>>()
            ),
            c["model"]
        );
        bindings.dispose();
        m.theme_colors().clear();
        p.select(Some(custom(100)));
        assert_eq!(state(&p), c["disposed"]);
        assert_eq!(
            json!(
                m.recent_colors()
                    .iter()
                    .map(|v| v.to_hex(false))
                    .collect::<Vec<_>>()
            ),
            c["disposedModel"]
        );
    });
}

#[test]
fn palette_names_object_and_view_keep_identity_through_core_payload_bridge() {
    crate::group_tests::run(|| {
        let palette = RibbonColorPalette::new();
        let names = palette.color_names();
        let left: BoxedValue = Rc::new(names.clone());
        let right: BoxedValue = Rc::new(palette.color_names());
        let left_object = ValueTypes::reference_object(&left).expect("declared reference view");
        let right_object = ValueTypes::reference_object(&right).expect("same actual dictionary");
        assert!(Rc::ptr_eq(&left_object, &right_object));
        let actual: Rc<dyn std::any::Any> = names.clone();
        let payload = crate::metadata::model_objects::from_native(Some(left)).unwrap();
        assert!(
            Rc::ptr_eq(&payload.payload(), &actual),
            "retain actual allocation, no payload proxy"
        );
        let native = crate::metadata::model_objects::to_native(Some(payload))
            .unwrap()
            .unwrap();
        let view = ValueTypes::try_cast(
            &native,
            ferroui_base::data::core::ValueType::of::<Rc<RibbonColorNames>>(),
        )
        .unwrap();
        let view = view.downcast_ref::<Rc<RibbonColorNames>>().unwrap();
        assert!(Rc::ptr_eq(view, &names));
        palette.set_color_name(custom(2), Some("Through palette".into()));
        assert_eq!(view.get(&custom(2)).as_deref(), Some("Through palette"));
        assert_eq!(
            palette.color_names().get(&custom(2)),
            Some("Through palette".into())
        );
        let weak = palette.downgrade();
        drop(palette);
        assert!(weak.upgrade().is_none());
        assert_eq!(
            names.get(&custom(2)),
            Some("Through palette".into()),
            "dictionary outlives its former control"
        );
    });
}

#[test]
fn read_only_names_are_live_independent_snapshots_and_release_with_the_last_rc() {
    crate::group_tests::run(|| {
        let palette = RibbonColorPalette::new();
        let names = palette.color_names();
        let other = RibbonColorPalette::new();
        assert!(
            names != other.color_names(),
            "different empty palettes have different names"
        );
        assert!(Rc::ptr_eq(&names, &palette.color_names()));
        assert!(names.is_empty());
        assert!(!names.contains_key(&custom(1)));
        assert_eq!(names.get(&custom(1)), None);
        palette.set_color_name(custom(1), Some("أزرق e\u{301} 🎨".into()));
        palette.set_color_name(custom(2), Some(" ".into()));
        let mut snapshot: std::collections::HashMap<Color, String> = names.snapshot();
        assert_eq!(snapshot.len(), 2);
        assert_eq!(snapshot.remove(&custom(2)).as_deref(), Some(" "));
        assert_eq!(
            names.len(),
            2,
            "owned snapshot mutation cannot change the palette"
        );
        palette.set_color_name(custom(1), Some("Updated".into()));
        palette.set_color_name(custom(2), None);
        assert!(names.contains_key(&custom(1)));
        assert_eq!(names.get(&custom(1)).as_deref(), Some("Updated"));
        assert_eq!(
            snapshot.get(&custom(1)).map(String::as_str),
            Some("أزرق e\u{301} 🎨")
        );
        let weak_names = Rc::downgrade(&names);
        let weak_palette = palette.downgrade();
        drop(palette);
        assert!(
            weak_palette.upgrade().is_none(),
            "live names do not retain the control"
        );
        assert_eq!(names.get(&custom(1)).as_deref(), Some("Updated"));
        drop(names);
        assert!(
            weak_names.upgrade().is_none(),
            "no registry or name-store cycle retains the map"
        );
    });
}
