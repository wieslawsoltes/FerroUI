use super::*;
use crate::controls::{Ribbon, RibbonGroup, RibbonItemLayout, RibbonTab};
use crate::primitives::RibbonMetrics;
use ferroribbon_core::{RibbonItemSize, commands::RibbonRelayCommand};
use ferroui_base::{
    Point, Rect, Size, StaticType,
    controls::ResourceDictionary,
    data::core::ValueTypes,
    input::{
        FocusChangedEventArgs, KeyModifiers, Pointer, PointerEventArgs, PointerPointProperties,
        PointerType,
    },
    styling::ControlTheme,
    threading::Dispatcher,
};
use ferroui_controls::{TextBox, Window};
use serde_json::{Value, json};
fn corpus() -> Value {
    serde_json::from_str(include_str!("../../tests/fixtures/text-input.json")).unwrap()
}
fn case(name: &str) -> Value {
    corpus()["corpus"]["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == name)
        .unwrap()
        .clone()
}
fn theme(c: &RibbonTextBox) {
    let value =
        crate::compiled_shared::try_load(None, "ferres://FerroRibbon/Themes/TextInput.xaml")
            .unwrap()
            .unwrap();
    let d = ValueTypes::as_object(&*value)
        .unwrap()
        .cast::<ResourceDictionary>()
        .unwrap();
    d.add_merged_dictionary(
        crate::theming::RibbonThemeResources::new().upcast::<ResourceDictionary>(),
    );
    let t = d
        .try_get_resource(&<RibbonTextBox as StaticType>::TYPE.into(), None)
        .flatten()
        .unwrap();
    let t = ValueTypes::as_object(&*t)
        .unwrap()
        .cast::<ControlTheme>()
        .unwrap();
    c.set_resources(d);
    c.set_theme(t);
}
fn mount(c: &RibbonTextBox) -> (Ref<Ribbon>, Ref<Window>) {
    theme(c);
    let r = Ribbon::new();
    r.set_is_application_button_visible(false);
    r.set_is_display_options_button_visible(false);
    let tab = RibbonTab::new();
    let g = RibbonGroup::new();
    g.items().add(Some(c.to_ref().upcast()));
    tab.groups().add(Some(g));
    r.tabs().add(Some(tab));
    let w = crate::ribbon_tests::mount(&r);
    settle();
    (r, w)
}
fn settle() {
    Dispatcher::ui_thread().run_jobs(None);
}
fn part(c: &RibbonTextBox) -> Ref<TextBox> {
    c.text_box.borrow().clone().unwrap()
}
fn key(t: &TextBox, k: Key) -> bool {
    let mut e = KeyEventArgs::new();
    e.key = k;
    e.set_routed_event(Some(InputElement::key_down_event()));
    t.raise_event(&e);
    e.handled()
}
fn state(c: &RibbonTextBox) -> String {
    [
        (":input-disabled", "InputDisabled"),
        (":input-focused", "InputFocused"),
        (":input-pointerover", "InputPointerOver"),
        (":input-normal", "InputNormal"),
    ]
    .iter()
    .find(|(s, _)| c.classes().contains(s))
    .unwrap()
    .1
    .to_string()
}
fn pointer(c: &RibbonTextBox, entered: bool) {
    let event = if entered {
        InputElement::pointer_entered_event()
    } else {
        InputElement::pointer_exited_event()
    };
    let e = PointerEventArgs::new(
        Some(event),
        c.to_ref().upcast::<ferroui_base::FerroObject>(),
        Pointer::new(12, PointerType::Mouse, true),
        None,
        Point::new(0., 0.),
        0,
        PointerPointProperties::NONE,
        KeyModifiers::NONE,
    );
    if entered {
        c.on_pointer_entered(&e)
    } else {
        c.on_pointer_exited(&e)
    }
}
fn owner_trace(r: &Ribbon, trace: &Rc<RefCell<Vec<String>>>) {
    let log = trace.clone();
    r.add_item_invoked(Some(MarkupDelegate::new(move |args| {
        let e = args[1]
            .as_ref()
            .unwrap()
            .downcast_ref::<Rc<crate::controls::RibbonItemInvokedEventArgs>>()
            .unwrap();
        let text = e
            .parameter
            .as_ref()
            .map(|p| p.downcast_ref::<String>().unwrap().clone())
            .unwrap_or("<null>".into());
        log.borrow_mut().push(format!("invoke:{text}"));
        None
    })));
}
fn command(c: &RibbonTextBox, trace: &Rc<RefCell<Vec<String>>>) {
    let log = trace.clone();
    c.set_command(Some(crate::commands::adapt_core_command(
        RibbonRelayCommand::new_with_parameter(
            move |p| {
                log.borrow_mut().push(format!(
                    "execute:{}",
                    p.and_then(|p| p.downcast_ref::<String>())
                        .map(String::as_str)
                        .unwrap_or("<null>")
                ));
            },
            None,
        ),
    )));
}
#[test]
fn source_input_layout_and_visibility_enum_match_all_frozen_combinations() {
    crate::group_tests::run(|| {
        for v in corpus()["corpus"]["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v["name"] == "layout")
        {
            let c = RibbonTextBox::new();
            c.set_label(v["label"].as_str().map(Into::into));
            if v["icon"] == true {
                c.set_icon(Some(Rc::new("I".to_string())));
            }
            let mut layout = RibbonItemLayout::new(
                RibbonItemSize::from_raw(v["size"].as_i64().unwrap() as i32),
                Some(RibbonMetrics::comfortable()),
            );
            layout.is_simplified = v["simplified"].as_bool().unwrap();
            layout.show_label = v["show"].as_bool();
            c.apply_layout(layout);
            assert_eq!(c.input_height(), v["height"].as_f64().unwrap());
            assert_eq!(
                c.label_visibility().raw(),
                v["labelVisibility"].as_i64().unwrap() as i32
            );
            assert_eq!(
                c.icon_visibility().raw(),
                v["iconVisibility"].as_i64().unwrap() as i32
            );
        }
    });
}
#[test]
fn pinned_native_single_line_coercion_and_asynchronous_host_update_use_actual_editor() {
    crate::group_tests::run(|| {
        for case in corpus()["corpus"]["singleLine"].as_array().unwrap() {
            let text = case["text"].as_str().unwrap();
            let expected = case["expected"].as_str().unwrap();
            let c = RibbonTextBox::new();
            let (_, w) = mount(&c);
            let t = part(&c);
            assert!(t.is::<crate::compat::RibbonInputTextBox>());
            t.set_text(Some(text));
            assert_eq!(t.text().as_deref(), Some(expected));
            // Source and native both queue TextChanged; a direct edit does not
            // synchronously push the host property through the TextChanged handler.
            assert_eq!(c.text().as_deref(), Some(""));
            settle();
            assert_eq!(c.text().as_deref(), Some(expected));
            t.set_accepts_return(true);
            t.set_text(Some(text));
            assert_eq!(t.text().as_deref(), Some(text));
            t.set_accepts_return(false);
            assert_eq!(t.text().as_deref(), Some(expected));
            w.close();
        }
        let t = crate::compat::RibbonInputTextBox::new();
        assert_eq!(t.text().as_deref(), Some(""));
        t.set_text(Some("before"));
        t.set_text(None);
        assert_eq!(t.text().as_deref(), Some(""));
    });
}
#[test]
fn source_input_focus_wins_over_pointer_and_disabled_wins_over_both() {
    crate::group_tests::run(|| {
        let c = RibbonTextBox::new();
        theme(&c);
        c.apply_styling();
        c.apply_template();
        let mut states = vec![state(&c)];
        pointer(&c, true);
        states.push(state(&c));
        c.on_got_focus(&FocusChangedEventArgs::new(InputElement::got_focus_event()));
        states.push(state(&c));
        pointer(&c, false);
        states.push(state(&c));
        c.set_is_enabled(false);
        states.push(state(&c));
        pointer(&c, true);
        states.push(state(&c));
        c.set_is_enabled(true);
        states.push(state(&c));
        c.on_lost_focus(&FocusChangedEventArgs::new(InputElement::lost_focus_event()));
        states.push(state(&c));
        pointer(&c, false);
        states.push(state(&c));
        let v = case("input_states");
        assert_eq!(json!(states), v["states"]);
        assert_eq!(c.is_tab_stop(), v["IsTabStop"].as_bool().unwrap());
        assert_eq!(
            c.can_add_to_quick_access(),
            v["CanAddToQuickAccess"].as_bool().unwrap()
        );
        assert_eq!(
            c.uses_command_can_execute(),
            v["UsesCanExecute"].as_bool().unwrap()
        );
    });
}
#[test]
fn source_text_template_submit_reentry_and_old_part_detachment() {
    crate::group_tests::run(|| {
        let c = RibbonTextBox::new();
        c.set_label(Some("Search".into()));
        c.set_text(Some("before".into()));
        let (r, w) = mount(&c);
        let old = part(&c);
        old.set_text(Some("typed"));
        settle();
        let typed = c.text();
        let trace = Rc::new(RefCell::new(vec![]));
        let t = trace.clone();
        let weak = c.downgrade();
        c.add_submitted(Some(MarkupDelegate::new(move |args| {
            let text = args[1]
                .as_ref()
                .and_then(|v| v.downcast_ref::<String>())
                .unwrap();
            t.borrow_mut().push(format!("submitted:{text}"));
            weak.upgrade().unwrap().set_text(Some("changed".into()));
            None
        })));
        command(&c, &trace);
        owner_trace(&r, &trace);
        let handled = key(&old, Key::Enter);
        let escape = key(&old, Key::Escape);
        // Replacing the source-derived theme forces a fresh actual native template part.
        let original_theme = c.theme();
        c.set_theme(None);
        theme(&c);
        w.measure(Size::new(1400., 400.));
        w.arrange(Rect::new(0., 0., 1400., 400.));
        settle();
        let current = part(&c);
        assert!(!old.ptr_eq(&current));
        old.set_text(Some("stale"));
        settle();
        key(&old, Key::Enter);
        c.on_key_tip_core();
        c.invoke_core();
        let v = case("text_lifecycle");
        assert_eq!(json!(typed), v["typed"]);
        assert_eq!(json!(c.text()), v["text"]);
        assert_eq!(json!(current.text()), v["inner"]);
        assert_eq!(json!(trace.borrow().clone()), v["trace"]);
        assert_eq!(handled, v["handled"].as_bool().unwrap());
        assert_eq!(escape, v["escape"].as_bool().unwrap());
        assert_eq!(
            ferroui_controls::automation::AutomationProperties::get_name(&current),
            v["automationName"].as_str().map(Into::into)
        );
        assert!(current.is_focused());
        drop(original_theme);
        w.close();
    });
}
#[test]
fn source_linked_text_updates_both_directions_and_submits_original_command_once() {
    crate::group_tests::run(|| {
        let c = RibbonTextBox::new();
        c.set_text(Some("source".into()));
        let (r, w) = mount(&c);
        let trace = Rc::new(RefCell::new(vec![]));
        command(&c, &trace);
        owner_trace(&r, &trace);
        let log = trace.clone();
        c.add_submitted(Some(MarkupDelegate::new(move |args| {
            let text = args[1].as_ref().unwrap().downcast_ref::<String>().unwrap();
            log.borrow_mut().push(format!("submitted:{text}"));
            None
        })));
        let copy = c
            .create_linked_copy_core()
            .unwrap()
            .cast::<RibbonTextBox>()
            .unwrap();
        theme(&copy);
        let popup = Window::new();
        popup.set_content(Some(Control::boxed(copy.clone())));
        popup.show();
        popup.measure(Size::new(400., 100.));
        popup.arrange(Rect::new(0., 0., 400., 100.));
        settle();
        let p = part(&copy);
        p.set_text(Some("copy"));
        settle();
        let after_copy = c.text();
        c.set_text(Some("back".into()));
        settle();
        assert!(key(&p, Key::Enter));
        let v = case("linked_submit");
        assert_eq!(json!(after_copy), v["afterCopy"]);
        assert_eq!(json!(c.text()), v["source"]);
        assert_eq!(json!(copy.text()), v["copy"]);
        assert_eq!(json!(trace.borrow().clone()), v["trace"]);
        assert_eq!(v["copyTrace"], json!([]));
        popup.close();
        w.close();
    });
}
#[test]
fn source_submission_snapshots_handlers_and_copy_captures_forwarded_text() {
    crate::group_tests::run(|| {
        let c = RibbonTextBox::new();
        c.set_text(Some("initial".into()));
        let (r, w) = mount(&c);
        let trace = Rc::new(RefCell::new(vec![]));
        command(&c, &trace);
        owner_trace(&r, &trace);
        let log = trace.clone();
        let third = MarkupDelegate::new(move |args| {
            log.borrow_mut().push(format!(
                "third:{}",
                args[1].as_ref().unwrap().downcast_ref::<String>().unwrap()
            ));
            None
        });
        let log = trace.clone();
        let second = MarkupDelegate::new(move |args| {
            log.borrow_mut().push(format!(
                "second:{}",
                args[1].as_ref().unwrap().downcast_ref::<String>().unwrap()
            ));
            None
        });
        let weak = c.downgrade();
        let log = trace.clone();
        let remove = second.clone();
        c.add_submitted(Some(MarkupDelegate::new(move |args| {
            log.borrow_mut().push(format!(
                "first:{}",
                args[1].as_ref().unwrap().downcast_ref::<String>().unwrap()
            ));
            let c = weak.upgrade().unwrap();
            c.remove_submitted(Some(remove.clone()));
            c.add_submitted(Some(third.clone()));
            c.set_text(Some("changed".into()));
            None
        })));
        c.add_submitted(Some(second));
        assert!(key(&part(&c), Key::Enter));
        assert!(key(&part(&c), Key::Enter));
        assert_eq!(
            json!(trace.borrow().clone()),
            case("event_snapshot")["trace"]
        );
        w.close();

        let c = RibbonTextBox::new();
        c.set_text(Some("initial".into()));
        let (r, w) = mount(&c);
        let trace = Rc::new(RefCell::new(vec![]));
        command(&c, &trace);
        owner_trace(&r, &trace);
        let weak = c.downgrade();
        let log = trace.clone();
        c.add_submitted(Some(MarkupDelegate::new(move |args| {
            log.borrow_mut().push(format!(
                "submitted:{}",
                args[1].as_ref().unwrap().downcast_ref::<String>().unwrap()
            ));
            weak.upgrade().unwrap().set_text(Some("changed".into()));
            None
        })));
        let copy = c
            .create_linked_copy_core()
            .unwrap()
            .cast::<RibbonTextBox>()
            .unwrap();
        let copy_trace = Rc::new(RefCell::new(vec![]));
        command(&copy, &copy_trace);
        theme(&copy);
        copy.apply_styling();
        copy.apply_template();
        assert!(key(&part(&copy), Key::Enter));
        let v = case("copy_reentrant_submit");
        assert_eq!(json!(c.text()), v["source"]);
        assert_eq!(json!(copy.text()), v["copy"]);
        assert_eq!(json!(trace.borrow().clone()), v["trace"]);
        assert_eq!(json!(copy_trace.borrow().clone()), v["copyTrace"]);
        // Unlink removes property subscriptions. It does not remove the source's
        // explicit Submitted callback installed on the copy by the source class.
        RibbonItemHelper::unlink(&copy);
        copy.set_text(Some("detached".into()));
        assert_eq!(c.text().as_deref(), Some("changed"));
        trace.borrow_mut().clear();
        assert!(key(&part(&copy), Key::Enter));
        assert_eq!(
            *trace.borrow(),
            vec!["submitted:detached", "execute:detached", "invoke:detached"]
        );
        w.close();
    });
}
#[test]
fn source_missing_part_detaches_handlers_and_custom_metrics_preserve_nonfinite_values() {
    crate::group_tests::run(|| {
        let c = RibbonTextBox::new();
        c.set_text(Some("before".into()));
        let (_, w) = mount(&c);
        let old = part(&c);
        c.set_template(Some(ferroui_controls::templates::FuncControlTemplate::new(
            |_, _| ferroui_controls::Border::new().upcast(),
        )));
        c.apply_template();
        assert!(c.text_box.borrow().is_none());
        old.set_text(Some("stale"));
        settle();
        let v = case("missing_part");
        assert_eq!(json!(c.text()), v["text"]);
        assert_eq!(key(&old, Key::Enter), v["handled"].as_bool().unwrap());
        assert_eq!(c.invoke_core(), v["invoked"].as_bool().unwrap());
        assert!(Rc::ptr_eq(
            &c.on_key_tip_core(),
            &RibbonKeyTipResult::close()
        ));
        w.close();
        for v in corpus()["corpus"]["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v["name"] == "metrics")
        {
            let parse = |s: &str| match s {
                "NaN" => f64::NAN,
                "Infinity" => f64::INFINITY,
                "-Infinity" => f64::NEG_INFINITY,
                _ => s.parse::<f64>().unwrap(),
            };
            let value = parse(v["value"].as_str().unwrap());
            let expected = parse(v["height"].as_str().unwrap());
            let c = RibbonTextBox::new();
            c.set_label(Some("café 😀".into()));
            let metrics = RibbonMetrics::comfortable().with_values(|m| {
                m.row_height = value;
                m.simplified_item_height = value;
            });
            let mut layout = RibbonItemLayout::new(RibbonItemSize::Medium, Some(metrics));
            layout.is_simplified = v["simplified"].as_bool().unwrap();
            c.apply_layout(layout);
            assert!(
                c.input_height() == expected || (c.input_height().is_nan() && expected.is_nan())
            );
            assert_eq!(json!(c.label_visibility().raw()), v["labelVisibility"]);
        }
    });
}
#[test]
fn actual_input_template_visibility_disabled_opacity_and_focus_border_follow_source_states() {
    crate::group_tests::run(|| {
        use ferroui_controls::{Border, TextBlock};
        let c = RibbonTextBox::new();
        c.set_label(Some("Find".into()));
        let (r, w) = mount(&c);
        let label = c
            .get_visual_descendants()
            .find_map(|p| {
                p.cast::<TextBlock>()
                    .filter(|p| p.name().as_deref() == Some("PART_Label"))
            })
            .unwrap();
        let border = c
            .get_visual_descendants()
            .find_map(|p| {
                p.cast::<Border>()
                    .filter(|p| p.name().as_deref() == Some("PART_InputBorder"))
            })
            .unwrap();
        assert!(label.is_visible());
        assert_eq!(border.opacity(), 1.);
        let group = r.find_location(&c).1.unwrap();
        group.set_is_enabled(false);
        assert!(!c.is_effectively_enabled());
        assert_eq!(state(&c), "InputDisabled");
        assert_eq!(border.opacity(), 0.6);
        c.set_is_enabled(true);
        assert_eq!(border.opacity(), 0.6);
        group.set_is_enabled(true);
        assert_eq!(state(&c), "InputNormal");
        assert_eq!(border.opacity(), 1.);
        let normal = border.border_brush().unwrap();
        pointer(&c, true);
        let hover = border.border_brush().unwrap();
        assert!(!Rc::ptr_eq(&normal, &hover));
        c.on_got_focus(&FocusChangedEventArgs::new(InputElement::got_focus_event()));
        let focus = border.border_brush().unwrap();
        assert!(!Rc::ptr_eq(&hover, &focus));
        pointer(&c, false);
        assert!(Rc::ptr_eq(&focus, &border.border_brush().unwrap()));
        c.set_is_enabled(false);
        assert_eq!(border.opacity(), 0.6);
        c.set_is_enabled(true);
        assert_eq!(border.opacity(), 1.);
        c.apply_layout(RibbonItemLayout::new(
            RibbonItemSize::Small,
            Some(RibbonMetrics::comfortable()),
        ));
        assert!(!label.is_visible());
        // Name is set once on template application; subsequent labels do not silently retitle it.
        c.set_label(Some("changed".into()));
        assert_eq!(
            ferroui_controls::automation::AutomationProperties::get_name(&part(&c)).as_deref(),
            Some("Find")
        );
        let other = RibbonTextBox::new();
        assert!(c.create_overflow_menu_items_core() == other.create_overflow_menu_items_core());
        assert_eq!(c.create_overflow_menu_items_core().iter().count(), 0);
        w.close();
    });
}
#[test]
fn source_input_number_helper_matches_all_culture_fallback_and_out_value_bits() {
    crate::group_tests::run(|| {
        use ferroui_base::utilities::{CultureInfo, NumberFormatInfo};
        struct Restore(CultureInfo);
        impl Drop for Restore {
            fn drop(&mut self) {
                CultureInfo::set_current_formatting(self.0.clone());
            }
        }
        let _restore = Restore(CultureInfo::current_formatting());
        let corpus: Value =
            serde_json::from_str(include_str!("../../tests/fixtures/input-number.json")).unwrap();
        let mut count = 0;
        for format in corpus["corpus"]["formats"].as_array().unwrap() {
            let get = |name: &str| format[name].as_str().unwrap();
            let info = NumberFormatInfo::new()
                .with_positive_sign(get("positive"))
                .with_negative_sign(get("negative"))
                .with_number_decimal_separator(get("dec"))
                .with_nan_symbol(get("nan"))
                .with_positive_infinity_symbol(get("inf"))
                .with_negative_infinity_symbol(get("ninf"));
            info.set_number_negative_pattern(format["pattern"].as_i64().unwrap() as i32);
            CultureInfo::set_current_formatting(
                CultureInfo::invariant_culture().with_number_format(info),
            );
            for case in format["cases"].as_array().unwrap() {
                let mut out = f64::NEG_INFINITY;
                let success = RibbonInputBase::try_parse_number(case["text"].as_str(), &mut out);
                assert_eq!(
                    success,
                    case["ok"].as_bool().unwrap(),
                    "format {} text {:?}",
                    get("name"),
                    case["text"]
                );
                assert_eq!(
                    out.to_bits(),
                    u64::from_str_radix(case["bits"].as_str().unwrap(), 16).unwrap(),
                    "format {} text {:?}",
                    get("name"),
                    case["text"]
                );
                count += 1;
            }
        }
        assert_eq!(count, 11805);
    });
}
