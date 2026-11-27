use super::*;
use crate::controls::RibbonObjectItemsExt;
use crate::controls::{Ribbon, RibbonFontComboBox, RibbonFontSizeComboBox, RibbonGroup, RibbonTab};
use ferroribbon_core::collections::RibbonFixedItems;
use ferroui_base::{
    controls::ResourceDictionary,
    input::{Key, KeyEventArgs},
    styling::ControlTheme,
    threading::Dispatcher,
};
use serde_json::{Value, json};
fn case(name: &str) -> Value {
    serde_json::from_str::<Value>(include_str!("../../tests/fixtures/combo.json")).unwrap()["corpus"]["cases"].as_array().unwrap().iter().find(|v|v["name"]==name).unwrap().clone()
}
fn text(value: Option<BoxedValue>) -> Option<String> {
    value.map(|v| ValueTypes::to_display_string(Some(&v)))
}
fn theme(c: &RibbonComboBox) {
    let v = crate::compiled_shared::try_load(None, "ferres://FerroRibbon/Themes/Combo.xaml")
        .unwrap()
        .unwrap();
    let d = ValueTypes::as_object(&*v)
        .unwrap()
        .cast::<ResourceDictionary>()
        .unwrap();
    d.add_merged_dictionary(
        crate::theming::RibbonThemeResources::new().upcast::<ResourceDictionary>(),
    );
    let typ = if c.is::<RibbonFontComboBox>() {
        RibbonFontComboBox::TYPE
    } else if c.is::<RibbonFontSizeComboBox>() {
        RibbonFontSizeComboBox::TYPE
    } else {
        RibbonComboBox::TYPE
    };
    let v = d.try_get_resource(&typ.into(), None).flatten().unwrap();
    let t = ValueTypes::as_object(&*v)
        .unwrap()
        .cast::<ControlTheme>()
        .unwrap();
    c.set_resources(d);
    c.set_theme(t);
}
fn mount(c: &RibbonComboBox) -> (Ref<Ribbon>, Ref<ferroui_controls::Window>) {
    theme(c);
    let r = Ribbon::new();
    r.set_is_application_button_visible(false);
    r.set_is_display_options_button_visible(false);
    let t = RibbonTab::new();
    let g = RibbonGroup::new();
    g.items().add(Some(c.to_ref().upcast()));
    t.groups().add(Some(g));
    r.tabs().add(Some(t));
    let w = crate::ribbon_tests::mount(&r);
    pump();
    (r, w)
}
fn pump() {
    Dispatcher::ui_thread().run_jobs(None);
}
#[test]
fn disabled_group_keeps_combo_box_popup_closed_and_applies_disabled_input_style() {
    crate::group_tests::run(|| {
        let combo = RibbonComboBox::new();
        let (ribbon, window) = mount(&combo);
        let group = ribbon.find_location(&combo).1.unwrap();
        let border = combo
            .get_visual_descendants()
            .find_map(|p| {
                p.cast::<Border>()
                    .filter(|p| p.name().as_deref() == Some("PART_InputBorder"))
            })
            .unwrap();
        let check = |enabled, opacity| {
            assert_eq!(combo.is_effectively_enabled(), enabled);
            combo.open_drop_down();
            assert_eq!(combo.is_drop_down_open(), enabled);
            assert_eq!(border.opacity(), opacity);
            combo.close_drop_down();
        };
        check(true, 1.);
        group.set_is_enabled(false);
        check(false, 0.6);
        combo.set_is_enabled(true);
        check(false, 0.6);
        group.set_is_enabled(true);
        check(true, 1.);
        combo.set_is_enabled(false);
        check(false, 0.6);
        window.close();
    });
}

fn key(p: &TextBox, k: Key) -> bool {
    let mut e = KeyEventArgs::new();
    e.key = k;
    e.set_routed_event(Some(InputElement::key_down_event()));
    p.raise_event(&e);
    e.handled()
}
fn command(c: &RibbonComboBox, log: &Rc<RefCell<Vec<String>>>) {
    let out = log.clone();
    c.set_command(Some(crate::commands::adapt_core_command(
        ferroribbon_core::commands::RibbonRelayCommand::new_with_parameter(
            move |p| {
                out.borrow_mut().push(format!(
                    "execute:{}",
                    p.map(|p| p.downcast_ref::<String>().unwrap().clone())
                        .unwrap_or_default()
                ))
            },
            None,
        ),
    )));
}
fn owner(r: &Ribbon, log: &Rc<RefCell<Vec<String>>>) {
    let out = log.clone();
    r.add_item_invoked(Some(MarkupDelegate::new(move |args| {
        let e = args[1]
            .as_ref()
            .unwrap()
            .downcast_ref::<Rc<super::super::RibbonItemInvokedEventArgs>>()
            .unwrap();
        out.borrow_mut().push(format!(
            "invoke:{}",
            text(e.parameter.clone()).unwrap_or_default()
        ));
        None
    })));
}
fn string(s: &str) -> Option<BoxedValue> {
    Some(Rc::new(s.to_string()))
}
#[test]
fn source_combo_items_snapshots_strings_override_and_empty_identity() {
    crate::group_tests::run(|| {
        let c = RibbonComboBox::new();
        let empty = c.effective_items();
        assert!(Rc::ptr_eq(&empty, &RibbonComboBox::new().effective_items()));
        c.items().add_native(string("one"));
        c.items().add_native(string("two"));
        let a = c.effective_items();
        c.items().add_native(string("three"));
        let b = c.effective_items();
        c.set_items_source(string("not enumerable"));
        let strsource = c.effective_items();
        c.set_items_source(Some(Rc::new(ferroui_controls::ItemsSource::from_strs([
            "override",
        ]))));
        let v = case("items");
        let strings = |a: RibbonObjectArray| a.iter().map(text).collect::<Vec<_>>();
        assert_eq!(json!(strings(a.clone())), v["a"]);
        assert_eq!(json!(strings(b.clone())), v["b"]);
        assert!(!Rc::ptr_eq(&a, &b));
        assert_eq!(json!(strings(strsource)), v["stringSource"]);
        assert_eq!(json!(strings(c.effective_items())), v["source"]);
    });
}
#[test]
fn source_combo_commit_event_reentry_and_command_owner_order() {
    crate::group_tests::run(|| {
        let c = RibbonComboBox::new();
        c.set_text(Some("initial".into()));
        let (r, w) = mount(&c);
        let log = Rc::new(RefCell::new(vec![]));
        command(&c, &log);
        owner(&r, &log);
        let out = log.clone();
        let weak = c.downgrade();
        c.add_selection_changed(Some(MarkupDelegate::new(move |args| {
            let e = args[1]
                .as_ref()
                .unwrap()
                .downcast_ref::<Rc<dyn IRoutedEventArgs>>()
                .unwrap();
            let e = e.downcast_ref::<SelectionChangedEventArgs>().unwrap();
            let join = |v: &[Option<BoxedValue>]| {
                v.iter()
                    .map(|v| text(v.clone()).unwrap_or_default())
                    .collect::<Vec<_>>()
                    .join(",")
            };
            out.borrow_mut().push(format!(
                "selection:{}>{}:{}",
                join(e.removed_items()),
                join(e.added_items()),
                weak.upgrade().unwrap().text().unwrap_or_default()
            ));
            None
        })));
        let out = log.clone();
        let weak = c.downgrade();
        c.add_committed(Some(MarkupDelegate::new(move |args| {
            let e = args[1]
                .as_ref()
                .unwrap()
                .downcast_ref::<Rc<RibbonComboBoxCommittedEventArgs>>()
                .unwrap();
            out.borrow_mut().push(format!(
                "committed:{}:{}",
                text(e.item()).unwrap_or_default(),
                e.text().unwrap_or_default()
            ));
            weak.upgrade().unwrap().set_text(Some("handler".into()));
            None
        })));
        c.commit(string("A"));
        c.set_selected_item(None);
        c.commit_text(Some("free".into()));
        let v = case("commits");
        assert_eq!(json!(c.text()), v["Text"]);
        assert_eq!(json!(text(c.selected_item())), v["SelectedItem"]);
        assert_eq!(json!(*log.borrow()), v["trace"]);
        w.close();
    });
}
#[test]
fn source_combo_null_first_match_stops_before_later_items_and_matches_culture() {
    crate::group_tests::run(|| {
        let c = RibbonComboBox::new();
        let (r, w) = mount(&c);
        c.items().add(None);
        c.items().add_native(string("ALPHA"));
        c.items().add_native(string("alpha"));
        let log = Rc::new(RefCell::new(vec![]));
        command(&c, &log);
        owner(&r, &log);
        c.commit_text(Some("alpha".into()));
        let matched = text(c.selected_item());
        c.commit_text(Some("".into()));
        let v = case("matching");
        assert_eq!(json!(matched), v["match"]);
        assert_eq!(json!(c.text()), v["Text"]);
        assert_eq!(json!(text(c.selected_item())), v["SelectedItem"]);
        assert_eq!(json!(*log.borrow()), v["trace"]);
        w.close();
    });
}
#[test]
fn source_combo_keyboard_popup_queued_open_and_template_detachment() {
    crate::group_tests::run(|| {
        let c = RibbonComboBox::new();
        c.set_is_editable(true);
        c.set_text(Some("old".into()));
        c.items().add_native(string("first"));
        c.items().add_native(string("second"));
        let (r, w) = mount(&c);
        let p = c.text_box.borrow().clone().unwrap();
        let log = Rc::new(RefCell::new(vec![]));
        owner(&r, &log);
        let mut keys = vec![];
        for k in [Key::Down, Key::Down, Key::Up] {
            keys.push(json!({"key":format!("{k:?}"),"Handled":key(&p,k),"SelectedItem":text(c.selected_item()),"Text":c.text()}));
        }
        p.set_text(Some("typed"));
        assert_eq!(c.text().as_deref(), Some("first"));
        key(&p, Key::Enter);
        p.set_text(Some("discard"));
        let escape = key(&p, Key::Escape);
        let v = case("keyboard");
        assert_eq!(json!(keys), v["keys"]);
        assert_eq!(json!(c.text()), v["text"]);
        assert_eq!(json!(p.text()), v["inner"]);
        assert_eq!(escape, v["escape"].as_bool().unwrap());
        assert_eq!(json!(*log.borrow()), v["trace"]);
        let b = c.drop_down_button.borrow().clone().unwrap();
        b.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
        assert!(!c.is_drop_down_open());
        pump();
        assert!(c.is_drop_down_open());
        assert_eq!(c.list.borrow().as_ref().unwrap().children().count(), 2);
        assert!(key(&p, Key::Down));
        assert_eq!(c.highlight.get(), 0);
        assert!(key(&p, Key::Escape));
        assert!(!c.is_drop_down_open());
        let old = p.clone();
        c.set_template(Some(ferroui_controls::templates::FuncControlTemplate::new(
            |_, _| Border::new().upcast(),
        )));
        c.apply_template();
        old.set_text(Some("stale"));
        key(&old, Key::Enter);
        b.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
        pump();
        assert!(!c.is_drop_down_open());
        assert_eq!(c.text().as_deref(), Some("typed"));
        w.close();
    });
}
#[test]
fn source_combo_linked_selection_text_items_identity_and_original_command_once() {
    crate::group_tests::run(|| {
        let c = RibbonComboBox::new();
        c.set_is_editable(true);
        c.items().add_native(string("one"));
        c.items().add_native(string("two"));
        let (r, w) = mount(&c);
        let log = Rc::new(RefCell::new(vec![]));
        command(&c, &log);
        owner(&r, &log);
        let copy = c
            .create_linked_copy_core()
            .unwrap()
            .cast::<RibbonComboBox>()
            .unwrap();
        assert!(
            copy.items_source()
                .as_ref()
                .unwrap()
                .downcast_ref::<RibbonObjectItems>()
                .unwrap()
                == &c.items()
        );
        copy.commit(string("two"));
        c.items().add_native(string("three"));
        let v = case("copy");
        assert_eq!(json!(text(c.selected_item())), v["SelectedItem"]);
        assert_eq!(json!(c.text()), v["Text"]);
        assert_eq!(json!(text(copy.selected_item())), v["copySelected"]);
        assert_eq!(json!(copy.text()), v["copyText"]);
        assert_eq!(
            copy.effective_items().len(),
            v["count"].as_u64().unwrap() as usize
        );
        assert_eq!(json!(*log.borrow()), v["trace"]);
        assert_eq!(copy.max_drop_down_height(), v["max"].as_f64().unwrap());
        RibbonItemHelper::unlink(&copy);
        assert!(copy.items_source().is_none());
        assert!(copy.item_text_selector().is_none());
        w.close();
    });
}
#[test]
fn source_font_pick_lists_localized_defaults_and_copy_types() {
    crate::group_tests::run(|| {
        let f = RibbonFontComboBox::new();
        let s = RibbonFontSizeComboBox::new();
        let v = case("fonts");
        assert_eq!(
            json!(
                f.items()
                    .iter()
                    .map(|v| text(crate::metadata::model_objects::to_native(v).unwrap()))
                    .collect::<Vec<_>>()
            ),
            v["fonts"]
        );
        assert_eq!(
            s.items()
                .iter()
                .map(|v| *v.unwrap().downcast_ref::<f64>().unwrap())
                .collect::<Vec<_>>(),
            v["sizes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_f64().unwrap())
                .collect::<Vec<_>>()
        );
        assert_eq!(f.input_width(), v["InputWidth"].as_f64().unwrap());
        assert_eq!(s.input_width(), v["sizeWidth"].as_f64().unwrap());
        assert_eq!(json!(f.label()), v["label"]);
        assert_eq!(json!(s.label()), v["sizeLabel"]);
        assert!(f.is_editable() && f.is_font_preview() && s.is_editable());
        assert!(
            f.create_linked_copy_core()
                .unwrap()
                .is::<RibbonFontComboBox>()
        );
        assert!(
            s.create_linked_copy_core()
                .unwrap()
                .is::<RibbonFontSizeComboBox>()
        );
    });
}

#[test]
fn source_combo_item_text_null_node_fallback_selector_and_lost_focus() {
    crate::group_tests::run(|| {
        use ferroribbon_core::model::RibbonNodeModelExt;
        let c = RibbonComboBox::new();
        let root: Value =
            serde_json::from_str(include_str!("../../tests/fixtures/combo.json")).unwrap();
        let cases = root["corpus"]["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["name"] == "item_text")
            .collect::<Vec<_>>();
        let mut values = vec![
            None,
            string(""),
            string("Alpha"),
            Some(Rc::new(12_i32) as BoxedValue),
            Some(Rc::new(10.5_f64) as BoxedValue),
            Some(Rc::new(true) as BoxedValue),
        ];
        for label in [None, Some(""), None] {
            let n = ferroribbon_core::model::RibbonButtonModel::new(Some("seed".into()), None);
            n.set_id(if values.len() == 8 {
                None
            } else {
                Some("id".into())
            });
            n.set_label(label.map(Into::into));
            values.push(Some(Rc::new(n)));
        }
        for (i, value) in values.into_iter().enumerate() {
            assert_eq!(json!(c.get_item_text(value)), cases[i]["text"]);
        }
        // Source GalleryItem branch waits for that actual public native class; not a stand-in.
        c.set_item_text_selector(Some(MarkupDelegate::new(|_| {
            Some(Rc::new("selected".to_string()))
        })));
        assert_eq!(c.get_item_text(None).as_deref(), Some("selected"));
        c.set_item_text_selector(Some(MarkupDelegate::new(|_| None)));
        assert_eq!(c.get_item_text(None).as_deref(), Some(""));
        c.set_is_editable(true);
        c.set_text(Some("original".into()));
        let (_, w) = mount(&c);
        let p = c.text_box.borrow().clone().unwrap();
        p.set_text(Some("uncommitted"));
        assert_eq!(json!(c.text()), case("lost_focus")["before"]);
        p.raise_event(&ferroui_base::input::FocusChangedEventArgs::new(
            InputElement::lost_focus_event(),
        ));
        assert_eq!(json!(c.text()), case("lost_focus")["Text"]);
        w.close();
    });
}
#[test]
fn source_combo_display_member_template_identity_and_unlink_configuration() {
    crate::group_tests::run(|| {
        struct Item {
            name: RefCell<Option<String>>,
        }
        impl PartialEq for Item {
            fn eq(&self, o: &Self) -> bool {
                std::ptr::eq(self, o)
            }
        }
        impl std::fmt::Display for Item {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("fallback")
            }
        }
        ferroui_base::data::model::ModelTypes::register::<Rc<Item>>(|b| {
            b.read_only::<ferroui_base::data::core::Value<Option<String>>>("Name", |v| {
                v.name.borrow().clone()
            })
        });
        ValueTypes::register_display::<Rc<Item>>();
        let item = Rc::new(Item {
            name: RefCell::new(Some("member".into())),
        });
        let c = RibbonComboBox::new();
        let before = c.get_item_text(Some(Rc::new(item.clone())));
        c.set_display_member_path(Some("Name".into()));
        let member = c.get_item_text(Some(Rc::new(item.clone())));
        item.name.replace(None);
        let empty = c.get_item_text(Some(Rc::new(item.clone())));
        c.set_display_member_path(Some("Missing".into()));
        let missing = c.get_item_text(Some(Rc::new(item.clone())));
        let expected = case("display_member");
        assert_eq!(
            json!({"before":before,"member":member,"empty":empty,"missing":missing}),
            json!({"before":expected["before"],"member":expected["member"],"empty":expected["empty"],"missing":expected["missing"]})
        );
        let weak = Rc::downgrade(&item);
        drop(item);
        assert!(
            weak.upgrade().is_none(),
            "cleared one-time binding must not retain its source"
        );
        c.set_display_member_path(None);
        c.set_selected_item(string("selected"));
        let template = ferroui_controls::templates::FuncDataTemplate::new(
            |_| true,
            |_, _| Some(TextBlock::new().upcast()),
            false,
        );
        c.set_selection_box_template(Some(template));
        let (_, w) = mount(&c);
        let p = c.selection_box.borrow().clone().unwrap();
        assert!(p.is_visible());
        assert_eq!(text(p.content()).as_deref(), Some("selected"));
        assert_eq!(c.text_box.borrow().as_ref().unwrap().opacity(), 0.);
        c.set_is_editable(true);
        assert!(!p.is_visible());
        assert!(p.content().is_none());
        assert_eq!(c.text_box.borrow().as_ref().unwrap().opacity(), 1.);
        w.close();
        let c = RibbonComboBox::new();
        c.set_max_drop_down_height(777.);
        c.items().add_native(string("source"));
        c.set_item_text_selector(Some(MarkupDelegate::new(|_| {
            Some(Rc::new("first".to_string()))
        })));
        let copy = c
            .create_linked_copy_core()
            .unwrap()
            .cast::<RibbonComboBox>()
            .unwrap();
        c.set_item_text_selector(Some(MarkupDelegate::new(|_| {
            Some(Rc::new("second".to_string()))
        })));
        let before = copy.get_item_text(string("x"));
        RibbonItemHelper::unlink(&copy);
        let v = case("copy_configuration");
        assert_eq!(json!(before), v["before"]);
        assert_eq!(copy.max_drop_down_height(), v["height"].as_f64().unwrap());
        assert_eq!(
            copy.items_source().is_none(),
            v["sourceNull"].as_bool().unwrap()
        );
        assert_eq!(
            copy.item_text_selector().is_none(),
            v["selectorNull"].as_bool().unwrap()
        );
        assert_eq!(
            copy.effective_items().len(),
            v["count"].as_u64().unwrap() as usize
        );
    });
}
#[test]
fn upstream_runtime_font_size_commit_and_public_static_array_source() {
    crate::group_tests::run(|| {
        // FeatureTests.InputTests.Combo_box_commit_text_selects_matching_item.
        let size = RibbonFontSizeComboBox::new();
        let (_, w) = mount(&size);
        let result = Rc::new(RefCell::new(None));
        let out = result.clone();
        size.add_committed(Some(MarkupDelegate::new(move |args| {
            out.replace(Some(
                args[1]
                    .as_ref()
                    .unwrap()
                    .downcast_ref::<Rc<RibbonComboBoxCommittedEventArgs>>()
                    .unwrap()
                    .clone(),
            ));
            None
        })));
        size.commit_text(Some("14".into()));
        assert_eq!(
            size.selected_item().unwrap().downcast_ref::<f64>(),
            Some(&14.)
        );
        size.commit_text(Some("13.5".into()));
        assert!(size.selected_item().is_none());
        assert_eq!(
            result.borrow().as_ref().unwrap().text().as_deref(),
            Some("13.5")
        );
        w.close();
        let c = RibbonComboBox::new();
        let sizes = RibbonFixedItems::new(vec![10.5, 13.5]);
        c.set_items_source(Some(Rc::new(sizes.clone())));
        assert_eq!(c.effective_items().len(), 2);
        sizes.set(1, 14.);
        assert_eq!(
            c.effective_items().get(1).unwrap().downcast_ref::<f64>(),
            Some(&14.)
        );
        let fonts = RibbonFixedItems::new(vec!["Arial".to_string()]);
        c.set_items_source(Some(Rc::new(fonts.clone())));
        fonts.set(0, "Consolas".into());
        assert_eq!(
            text(c.effective_items().get(0)).as_deref(),
            Some("Consolas")
        );
    });
}

#[test]
fn source_font_defaults_retain_read_only_shared_identity() {
    crate::group_tests::run(|| {
        let v = case("default_list_identity");
        assert!(v["fontsReadOnly"].as_bool().unwrap() && v["sizesReadOnly"].as_bool().unwrap());
        assert_eq!(
            RibbonFontComboBox::default_fonts() == RibbonFontComboBox::default_fonts(),
            v["sameFonts"].as_bool().unwrap()
        );
        assert_eq!(
            RibbonFontSizeComboBox::default_sizes() == RibbonFontSizeComboBox::default_sizes(),
            v["sameSizes"].as_bool().unwrap()
        );
        let c = RibbonComboBox::new();
        c.set_items_source(Some(Rc::new(RibbonFontSizeComboBox::default_sizes())));
        assert_eq!(c.effective_items().len(), 17);
    });
}

#[test]
fn source_direct_resource_priority_selects_one_theme_dictionary() {
    crate::group_tests::run(|| {
        let key: ResourceKey = "RibbonMenuItemButtonStyle".into();
        let root = ResourceDictionary::new();
        let first = ResourceDictionary::new();
        let last = ResourceDictionary::new();
        let dark = ResourceDictionary::new();
        let default = ResourceDictionary::new();
        first.set(key.clone(), string("first"));
        last.set(key.clone(), string("last"));
        dark.set(key.clone(), string("dark"));
        default.set(key.clone(), string("default"));
        root.add_merged_dictionary(first);
        root.add_merged_dictionary(last.clone());
        root.theme_dictionaries()
            .add(ThemeVariant::dark(), dark.clone().into());
        root.theme_dictionaries()
            .add(ThemeVariant::default(), default.into());
        let lookup = || source_dictionary_resource(&root, &key, Some(&ThemeVariant::dark()));
        // Pinned Uno ResourceDictionary.cs: own, reverse merged, active theme.
        // The source selects a dictionary before looking up the requested key:
        // a miss in an existing Dark dictionary must not fall back to Default.
        assert_eq!(text(lookup().unwrap()).as_deref(), Some("last"));
        root.set(key.clone(), string("own"));
        assert_eq!(text(lookup().unwrap()).as_deref(), Some("own"));
        root.set(key.clone(), None);
        assert!(lookup().unwrap().is_none());
        root.remove(&key);
        root.merged_dictionaries().clear();
        assert_eq!(text(lookup().unwrap()).as_deref(), Some("dark"));
        dark.remove(&key);
        assert!(lookup().is_none());
        root.theme_dictionaries().remove(&ThemeVariant::dark());
        assert_eq!(text(lookup().unwrap()).as_deref(), Some("default"));
    });
}

#[test]
fn source_popup_uses_application_style_and_fresh_unselected_brushes() {
    crate::group_tests::run(|| {
        let app = ferroui_controls::Application::current().unwrap();
        let key: ResourceKey = "RibbonMenuItemButtonStyle".into();
        let old = app.resources().try_get_value(&key);
        let style = ControlTheme::for_type::<Button>();
        app.resources()
            .set(key.clone(), Some(Rc::new(style.clone())));
        let c = RibbonComboBox::new();
        c.items().add_native(string("one"));
        c.items().add_native(string("two"));
        let (_, w) = mount(&c);
        c.open_drop_down();
        pump();
        let list = c.list.borrow().clone().unwrap();
        let a = list.children().get(0).cast::<Button>().unwrap();
        let b = list.children().get(1).cast::<Button>().unwrap();
        assert_eq!(a.theme(), Some(style));
        c.highlight(-1);
        let brush_a = a.background().unwrap();
        let brush_b = b.background().unwrap();
        assert!(!std::ptr::eq(
            brush_a.as_object().unwrap(),
            brush_b.as_object().unwrap()
        ));
        c.highlight(-1);
        assert!(!std::ptr::eq(
            brush_a.as_object().unwrap(),
            a.background().unwrap().as_object().unwrap()
        ));
        assert_eq!(
            a.background()
                .unwrap()
                .as_object()
                .unwrap()
                .downcast_ref::<SolidColorBrush>()
                .unwrap()
                .color(),
            Color::from_argb(0, 255, 255, 255)
        );
        c.close_drop_down();
        w.close();
        match old {
            Some(value) => app.resources().set(key, value),
            None => {
                app.resources().remove(&key);
            }
        }
    });
}
