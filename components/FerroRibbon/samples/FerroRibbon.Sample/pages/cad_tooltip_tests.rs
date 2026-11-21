use super::{
    CadPage,
    cad_action_tests::{mount, run},
};
use ferroribbon::controls::{
    RibbonButton, RibbonComboBox, RibbonGroup, RibbonScreenTip, RibbonScreenTipService, RibbonTab,
    RibbonToggleButton,
};
use ferroui_base::{
    BoxedValue, Point, Ref,
    animation::SignedDuration,
    data::core::ValueTypes,
    metadata::{MarkupType, from_markup_value, into_markup_value},
    styling::ThemeVariant,
    threading::Dispatcher,
};
use ferroui_controls::{Control, ToolTip, Window};
use ferroui_headless::HeadlessWindowExtensions;
use serde_json::{Value, json};
use std::{cell::Cell, rc::Rc};

fn units(value: Option<String>) -> Value {
    value
        .map(|s| json!(s.encode_utf16().collect::<Vec<_>>()))
        .unwrap_or(Value::Null)
}
fn native_text(value: &Value) -> Option<String> {
    value.as_array().map(|a| {
        String::from_utf16(
            &a.iter()
                .map(|n| n.as_u64().unwrap() as u16)
                .collect::<Vec<_>>(),
        )
        .unwrap()
    })
}
fn snapshot(content: Option<BoxedValue>) -> Value {
    let Some(content) = content else {
        return Value::Null;
    };
    if let Some(text) = content.downcast_ref::<String>() {
        return json!({"kind":"text","text":units(Some(text.clone()))});
    }
    if let Some(tip) = ValueTypes::as_object(&*content).and_then(|o| o.cast::<RibbonScreenTip>()) {
        return json!({"kind":"screen","title":units(tip.title()),"shortcut":units(tip.shortcut()),"description":units(tip.description()),"help":units(tip.help_text()),"disabled":units(tip.disabled_reason()),"extended":units(tip.extended_description())});
    }
    json!({"kind":"other"})
}
fn screen(mode: &str) -> Option<BoxedValue> {
    match mode {
        "description" => Some(Rc::new("Body".to_owned())),
        "model" => {
            let tip = ferroribbon_core::model::RibbonScreenTip::new();
            tip.set_title(Some("".into()));
            tip.set_description(Some("Body".into()));
            tip.set_shortcut(Some("".into()));
            tip.set_help_text(Some("F1".into()));
            tip.set_disabled_reason(Some("Disabled".into()));
            tip.set_extended_description(Some("More".into()));
            Some(Rc::new(tip))
        }
        "control-tip" => {
            let tip = RibbonScreenTip::new();
            tip.set_title(Some("".into()));
            tip.set_description(Some("Body".into()));
            tip.set_shortcut(Some("".into()));
            tip.set_help_text(Some("F1".into()));
            tip.set_disabled_reason(Some("Disabled".into()));
            tip.set_extended_description(Some("More".into()));
            Some(Control::boxed(tip))
        }
        "none" => None,
        _ => panic!("unknown source mode"),
    }
}
struct Inputs {
    page: Ref<CadPage>,
    window: Ref<Window>,
    button: Ref<RibbonButton>,
    control: Ref<RibbonComboBox>,
    other: Ref<RibbonToggleButton>,
}
fn inputs() -> Inputs {
    let page = CadPage::new();
    let group = RibbonGroup::new();
    group.set_header(Some("ScreenTips".into()));
    let button = RibbonButton::new();
    button.set_id(Some("tip-button".into()));
    let control = RibbonComboBox::new();
    control.set_id(Some("tip-control".into()));
    let other = RibbonToggleButton::new();
    other.set_id(Some("tip-other".into()));
    for item in [
        button.clone().upcast::<Control>(),
        control.clone().upcast(),
        other.clone().upcast(),
    ] {
        group.items().add(Some(item));
    }
    let tab = RibbonTab::new();
    tab.set_id(Some("tip-tests".into()));
    tab.set_header(Some("ScreenTips".into()));
    tab.groups().add(Some(group));
    page.cad_ribbon().tabs().add(Some(tab));
    assert!(page.cad_ribbon().select_tab_id("tip-tests"));
    let window = mount(&page);
    Inputs {
        page,
        window,
        button,
        control,
        other,
    }
}
fn popup_tip(page: &CadPage) -> Ref<ToolTip> {
    page.tip_popup
        .borrow()
        .as_ref()
        .unwrap()
        .child()
        .unwrap()
        .cast()
        .unwrap()
}

#[test]
fn original_tooltip_helper_and_service_cases_match_actual_native_content() {
    run(|| {
        let input = inputs();
        let data: Value =
            serde_json::from_str(include_str!("../../../tests/fixtures/cad-tip.json")).unwrap();
        let mut count = 0;
        for case in data["cases"].as_array().unwrap() {
            let kind = case["kind"].as_str().unwrap();
            if !["button", "control", "other", "missing"].contains(&kind) {
                continue;
            }
            let label = native_text(&case["label"]);
            let mode = case["mode"].as_str().unwrap();
            let id = match kind {
                "button" => {
                    input.button.set_label(label);
                    input.button.set_screen_tip(screen(mode));
                    input.button.set_shortcut(Some("Ctrl+L".into()));
                    "tip-button"
                }
                "control" => {
                    input.control.set_label(label);
                    input.control.set_screen_tip(screen(mode));
                    input.control.set_shortcut(Some("Ctrl+L".into()));
                    "tip-control"
                }
                "other" => {
                    input.other.set_label(label);
                    input.other.set_screen_tip(screen(mode));
                    input.other.set_shortcut(Some("Ctrl+L".into()));
                    "tip-other"
                }
                _ => "tip-missing",
            };
            let old = input.page.tip_popup.borrow().clone();
            let result = input.page.show_tool_tip(Some(id)).unwrap();
            assert_eq!(result, case["result"].as_bool().unwrap(), "{case}");
            if result {
                let popup = input.page.tip_popup.borrow().clone().unwrap();
                assert!(popup.is_open());
                assert!(popup.is_light_dismiss_enabled());
                if let Some(old) = old {
                    assert!(!old.is_open());
                    assert!(!old.ptr_eq(&popup));
                }
                let tip = popup_tip(&input.page);
                assert_eq!(snapshot(tip.content()), case["content"], "{case}");
                assert_eq!(
                    tip.actual_theme_variant(),
                    input.page.actual_theme_variant()
                );
            } else if let Some(old) = old {
                assert!(input.page.tip_popup.borrow().as_ref().unwrap().ptr_eq(&old));
                assert!(old.is_open());
            }
            count += 1;
        }
        assert_eq!(count, 80);
        input.window.close();
    });
}

#[test]
fn actual_tip_popup_offsets_theme_snapshot_replacement_order_and_light_dismiss() {
    run(|| {
        let input = inputs();
        input.button.set_label(Some("Line".into()));
        input
            .button
            .set_screen_tip(Some(Rc::new("Body".to_owned())));
        Dispatcher::ui_thread().run_jobs(None);
        input.window.update_layout();
        assert!(input.page.show_tool_tip(Some("tip-button")).unwrap());
        let old = input.page.tip_popup.borrow().clone().unwrap();
        let origin = input
            .button
            .translate_point(
                Point::new(0., input.button.bounds().height + 4.),
                &input.window,
            )
            .unwrap();
        assert_eq!(old.horizontal_offset(), origin.x);
        assert_eq!(old.vertical_offset(), origin.y);
        let old_tip = popup_tip(&input.page);
        let theme = old_tip.actual_theme_variant();
        input.page.set_light_theme(true);
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(old_tip.actual_theme_variant(), theme);
        let closed = Rc::new(Cell::new(0));
        let calls = closed.clone();
        let page = input.page.downgrade();
        let previous = old.downgrade();
        old.closed(move || {
            calls.set(calls.get() + 1);
            assert!(
                page.upgrade()
                    .unwrap()
                    .tip_popup
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .ptr_eq(&previous.upgrade().unwrap())
            );
        })
        .detach();
        assert!(!input.page.show_tool_tip(Some("MISSING")).unwrap());
        assert_eq!(closed.get(), 0);
        assert!(old.is_open());
        assert!(input.page.show_tool_tip(Some("tip-button")).unwrap());
        assert_eq!(closed.get(), 1);
        let current = input.page.tip_popup.borrow().clone().unwrap();
        assert!(!old.ptr_eq(&current));
        assert_eq!(
            popup_tip(&input.page).actual_theme_variant(),
            Some(ThemeVariant::light())
        );
        use ferroui_base::input::{MouseButton, RawInputModifiers};
        let point = Point::new(1350., 600.);
        input.window.mouse_move(point, RawInputModifiers::NONE);
        input
            .window
            .mouse_down(point, MouseButton::Left, RawInputModifiers::NONE);
        input
            .window
            .mouse_up(point, MouseButton::Left, RawInputModifiers::NONE);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(!current.is_open());
        assert!(
            input
                .page
                .tip_popup
                .borrow()
                .as_ref()
                .unwrap()
                .ptr_eq(&current)
        );
        input.window.close();
    });
}

#[test]
fn tooltip_helper_metadata_progressive_content_and_page_disposal_use_native_services() {
    run(|| {
        let input = inputs();
        input.button.set_label(Some("Line".into()));
        input.button.set_screen_tip(screen("model"));
        input.button.set_is_enabled(false);
        let delay = RibbonScreenTipService::extended_delay();
        RibbonScreenTipService::set_extended_delay(SignedDuration::ZERO);
        let method = MarkupType::find_by_type_info(CadPage::TYPE)
            .unwrap()
            .find_methods("ShowToolTip")
            .next()
            .unwrap();
        let result = (method.invoke)(&[
            into_markup_value(input.page.clone()),
            into_markup_value(Some("tip-button".to_owned())),
        ])
        .unwrap();
        assert_eq!(from_markup_value::<bool>(&result), Some(true));
        Dispatcher::ui_thread().run_jobs(None);
        let popup = input.page.tip_popup.borrow().clone().unwrap();
        let tip = popup_tip(&input.page);
        let content = ValueTypes::as_object(&*tip.content().unwrap())
            .unwrap()
            .cast::<RibbonScreenTip>()
            .unwrap();
        assert!(content.is_extended());
        assert_eq!(content.disabled_reason(), None);
        RibbonScreenTipService::set_extended_delay(delay);
        let page = input.page.downgrade();
        let model = Rc::downgrade(&input.page.view_model());
        input.window.set_content(None);
        input.window.close();
        drop(input);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(page.upgrade().is_none());
        assert!(model.upgrade().is_none());
        assert!(!popup.is_open());
    });
}

#[test]
fn helper_hidden_item_identity_and_text_errors_do_not_close_the_previous_popup() {
    run(|| {
        let unattached = CadPage::new();
        assert_eq!(
            unattached.show_tool_tip(Some("line")),
            Err(super::CadToolTipError::NotAttached)
        );
        let input = inputs();
        let hidden = input
            .page
            .cad_ribbon()
            .find_item(Some("multiplePoints".into()))
            .unwrap();
        assert!(!hidden.is_attached_to_visual_tree());
        assert!(input.page.show_tool_tip(Some("multiplePoints")).unwrap());
        let popup = input.page.captured_tip_popup().unwrap();
        assert_eq!(popup.horizontal_offset(), 0.);
        assert_eq!(popup.vertical_offset(), hidden.bounds().height + 4.);
        let model = Rc::new(ferroribbon_core::model::RibbonScreenTip::new());
        model.set_title(Some("Valid".into()));
        input.button.set_label(Some("Line".into()));
        input.button.set_screen_tip(Some(model.clone()));
        model.set_title(Some(ferroribbon_core::RibbonString::from_utf16(vec![
            0xd800,
        ])));
        assert_eq!(
            input.page.show_tool_tip(Some("tip-button")),
            Err(super::CadToolTipError::Text("Title"))
        );
        assert!(popup.is_open());
        assert!(input.page.captured_tip_popup().unwrap().ptr_eq(&popup));
        let method = MarkupType::find_by_type_info(CadPage::TYPE)
            .unwrap()
            .find_methods("ShowToolTip")
            .next()
            .unwrap();
        assert!(
            (method.invoke)(&[
                into_markup_value(input.page.clone()),
                into_markup_value(Some("tip-button".to_owned()))
            ])
            .is_err()
        );
        input.window.close();
    });
}

#[test]
fn helper_closed_callback_can_reenter_without_holding_a_page_state_borrow() {
    run(|| {
        let input = inputs();
        input.button.set_label(Some("Line".into()));
        assert!(input.page.show_tool_tip(Some("tip-button")).unwrap());
        let old = input.page.captured_tip_popup().unwrap();
        let page = input.page.downgrade();
        let inner = Rc::new(std::cell::RefCell::new(None));
        let result = inner.clone();
        old.closed(move || {
            let page = page.upgrade().unwrap();
            assert!(page.show_tool_tip(Some("tip-button")).unwrap());
            result.replace(page.captured_tip_popup());
        })
        .detach();
        assert!(input.page.show_tool_tip(Some("tip-button")).unwrap());
        let current = input.page.captured_tip_popup().unwrap();
        let inner = inner.borrow().clone().unwrap();
        assert!(!old.is_open());
        assert!(!current.ptr_eq(&inner));
        assert!(current.is_open());
        inner.set_is_open(false);
        input.window.close();
    });
}
