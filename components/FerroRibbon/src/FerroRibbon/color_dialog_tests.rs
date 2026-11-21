use super::*;
use crate::theming::to_ribbon_color;
use ferroui_base::input::{InputElement, Key};
use ferroui_base::{
    Rect, Size,
    input::{KeyEventArgs, KeyModifiers},
    threading::Dispatcher,
};
use ferroui_controls::{Application, Button, StackPanel, TextBlock, Window};
use ferroui_headless::FerroHeadlessPlatformExtensions;
use futures_util::FutureExt;
use std::{
    pin::Pin,
    task::{Context, Waker},
};
fn native(test: impl FnOnce() + Send + 'static) {
    ferroui_headless::HeadlessUnitTestSession::start_new(|| {
        ferroui_controls::AppBuilder::configure::<Application>()
            .use_headless(ferroui_headless::FerroHeadlessPlatformOptions::default())
    })
    .dispatch(
        move || {
            crate::register_types();
            let theme = ferroui_themes_simple::SimpleTheme::new().as_style();
            Application::current().unwrap().styles().add(theme);
            test();
        },
        ferroui_base::threading::CancellationToken::none(),
    )
    .wait()
    .unwrap();
}
fn window() -> Ref<Window> {
    let w = Window::new();
    w.set_width(700.);
    w.set_height(700.);
    w.show();
    w.measure(Size::new(700., 700.));
    w.arrange(Rect::new(0., 0., 700., 700.));
    Dispatcher::ui_thread().run_jobs(None);
    w
}
fn session() -> Rc<Session> {
    ACTIVE.with_borrow(|s| s.last().unwrap().clone())
}
#[test]
fn source_default_key_dispatch_completion_counts_and_handled_state() {
    native(|| {
        use crate::compat::ContentDialogButton;
        let data: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/content-dialog-visual.json"
        ))
        .unwrap();
        let w = window();
        for case in data["default_button_corpus"].as_array().unwrap() {
            for key_case in case["keys"].as_array().unwrap() {
                let task = crate::dialog_future_test_support::show_color(
                    Some(w.clone().upcast()),
                    Color::from_rgb(11, 22, 33),
                    None,
                );
                let s = session();
                assert_eq!(
                    s.shield.default_button(),
                    ContentDialogButton::Primary,
                    "static source service explicitly selects Primary"
                );
                s.shield.set_default_button(ContentDialogButton::from_raw(
                    case["choice"].as_i64().unwrap() as i32,
                ));
                let native_clicks = Rc::new(Cell::new(0));
                for b in [
                    s.shield.primary_button().unwrap(),
                    s.shield.secondary_button().unwrap(),
                    s.shield.close_button().unwrap(),
                ] {
                    let count = native_clicks.clone();
                    b.click(move |_, _| count.set(count.get() + 1));
                }
                use std::sync::{
                    Arc,
                    atomic::{AtomicUsize, Ordering},
                };
                struct WakeCount(AtomicUsize);
                impl std::task::Wake for WakeCount {
                    fn wake(self: Arc<Self>) {
                        self.0.fetch_add(1, Ordering::SeqCst);
                    }
                }
                let completions = Arc::new(WakeCount(AtomicUsize::new(0)));
                let waker = Waker::from(completions.clone());
                let mut context = Context::from_waker(&waker);
                let mut waiter = task.clone();
                assert!(Pin::new(&mut waiter).poll(&mut context).is_pending());
                let key = match key_case["key"].as_i64().unwrap() {
                    13 => Key::Enter,
                    27 => Key::Escape,
                    32 => Key::Space,
                    _ => unreachable!(),
                };
                let mut e = KeyEventArgs::default();
                e.key = key;
                e.set_routed_event(Some(InputElement::key_down_event()));
                s.content.hue.raise_event(&e);
                let processes = key_case["processes"].as_array().unwrap();
                assert_eq!(
                    e.handled(),
                    key_case["handled"].as_bool().unwrap(),
                    "{case} {key_case}"
                );
                assert!(
                    task.clone().now_or_never().is_none(),
                    "source queues result after Closed"
                );
                assert_eq!(completions.0.load(Ordering::SeqCst), 0);
                Dispatcher::ui_thread().run_jobs(None);
                assert_eq!(
                    task.clone().now_or_never().is_some(),
                    !processes.is_empty(),
                    "{case} {key_case}"
                );
                assert_eq!(
                    completions.0.load(Ordering::SeqCst),
                    processes.len(),
                    "{case} {key_case}"
                );
                assert_eq!(
                    native_clicks.get(),
                    0,
                    "source key path calls Process*, not Button.Click"
                );
                if processes.is_empty() {
                    finish_for_test(&s, false);
                } else {
                    assert_eq!(
                        task.clone().now_or_never(),
                        Some(Ok(if processes[0] == "Primary" {
                            Some(s.content.current())
                        } else {
                            None
                        }))
                    );
                }
                finish_for_test(&s, true);
                assert_eq!(
                    completions.0.load(Ordering::SeqCst),
                    1,
                    "reentrant/repeated finish completes once"
                );
            }
        }
        w.close();
    });
}
fn finish_for_test(session: &Session, primary: bool) {
    if !session.closing.get() {
        session.shield.request_close(if primary {
            crate::compat::ContentDialogButton::Primary
        } else {
            crate::compat::ContentDialogButton::None
        });
    }
    Dispatcher::ui_thread().run_jobs(None);
}
fn hex(c: Color) -> String {
    to_ribbon_color(c).to_hex(true)
}
#[test]
fn source_dialog_null_alpha_hex_queued_rounding_with_standard_shared_observation() {
    native(|| {
        let empty =
            crate::dialog_future_test_support::show_color(None, Color::from_rgb(1, 2, 3), None);
        assert_eq!(empty.clone().now_or_never(), Some(Ok(None)));
        assert!(ACTIVE.with_borrow(|s| s.is_empty()));
        let w = window();
        let task = crate::dialog_future_test_support::show_color(
            Some(w.clone().upcast()),
            Color::from_argb(128, 1, 2, 3),
            None,
        );
        assert!(task.clone().now_or_never().is_none());
        let s = session();
        assert_eq!(hex(s.content.current()), "#FF010203");
        assert_eq!(s.content.hex.text().as_deref(), Some("#010203"));
        assert_eq!(
            hex(s
                .content
                .original
                .background()
                .unwrap()
                .as_solid_color_brush()
                .unwrap()
                .color()),
            "#80010203"
        );
        Dispatcher::ui_thread().run_jobs(None);
        s.content.hex.set_text(Some("#12345678"));
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(hex(s.content.current()), "#12345678");
        s.content.lightness.set_range_value(50.);
        assert_eq!(s.content.hue.value(), 210.);
        Dispatcher::ui_thread().run_jobs(None);
        assert!((s.content.hue.value() - 210.2970297029703).abs() < 1e-10);
        assert_eq!(hex(s.content.current()), "#FF4D7FB2");
        s.shield.primary_button().unwrap().on_click();
        assert!(
            task.clone().now_or_never().is_none(),
            "source result is queued"
        );
        Dispatcher::ui_thread().run_jobs(None);
        let expected = Some(Ok(Some(Color::from_argb(255, 77, 127, 178))));
        assert_eq!(task.clone().now_or_never(), expected);
        assert_eq!(task.clone().now_or_never(), expected);
        assert!(ACTIVE.with_borrow(|s| s.is_empty()));
        w.close();
    });
}
#[test]
fn native_dialog_owner_focus_dismissal_resize_duplicate_escape_and_close_cleanup() {
    native(|| {
        let w = window();
        let original = Button::new();
        w.set_content(Some(Control::boxed(original.clone())));
        w.update_layout();
        assert!(original.focus());
        let task = crate::dialog_future_test_support::show_color(
            Some(w.clone().upcast()),
            Color::from_rgb(255, 0, 0),
            Some(ThemeVariant::dark()),
        );
        let s = session();
        assert!(s.content.hue.is_focused());
        assert!(s.contains_focus());
        assert_eq!(s.shield.bounds().size(), w.client_size());
        assert_eq!(s.shield.actual_theme_variant(), Some(ThemeVariant::dark()));
        let second = crate::dialog_future_test_support::show_color(
            Some(w.clone().upcast()),
            Color::from_rgb(0, 0, 0),
            None,
        );
        assert_eq!(
            second.clone().now_or_never(),
            Some(Err(RibbonColorDialogError::AlreadyOpen))
        );
        assert!(task.clone().now_or_never().is_none());
        original.focus();
        assert!(s.contains_focus());
        w.set_width(900.);
        w.update_layout();
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(s.shield.width(), w.client_size().width);
        let mut e = KeyEventArgs::default();
        e.key = Key::Escape;
        e.key_modifiers = KeyModifiers::NONE;
        e.set_routed_event(Some(InputElement::key_down_event()));
        s.content.hue.raise_event(&e);
        assert!(
            !e.handled(),
            "source OnPopupKeyDown leaves Handled unchanged"
        );
        assert!(task.clone().now_or_never().is_none());
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(task.clone().now_or_never(), Some(Ok(None)));
        assert!(original.is_focused());
        assert_eq!(s.shield.visual_parent(), None);
        assert!(s.scope.borrow().is_none());
        let next = crate::dialog_future_test_support::show_color(
            Some(w.clone().upcast()),
            Color::from_rgb(0, 0, 0),
            None,
        );
        w.close();
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(next.clone().now_or_never(), Some(Ok(None)));
        assert!(ACTIVE.with_borrow(|s| s.is_empty()));
    });
}
fn color(text: &str) -> Color {
    crate::theming::to_color(
        ferroribbon_core::theming::RibbonColor::parse(Some(&text.into())).unwrap(),
    )
}
fn header(c: &Control) -> Option<String> {
    c.parent()
        .unwrap()
        .cast::<StackPanel>()
        .unwrap()
        .children()
        .get(0)
        .cast::<TextBlock>()
        .unwrap()
        .text()
}
fn snapshot(s: &Session) -> serde_json::Value {
    use serde_json::json;
    let c = &s.content;
    let title = s
        .shield
        .get_visual_descendants()
        .find_map(|v| {
            v.cast::<ferroui_controls::ContentControl>()
                .filter(|c| c.name().as_deref() == Some("Title"))
        })
        .unwrap()
        .content()
        .map(|v| v.downcast_ref::<String>().unwrap().clone());
    json!({"Title":title,"PrimaryButtonText":s.shield.primary_button().unwrap().content().unwrap().downcast_ref::<String>().unwrap(),"CloseButtonText":s.shield.close_button().unwrap().content().unwrap().downcast_ref::<String>().unwrap(),"defaultButton":if s.shield.default_button()==crate::compat::ContentDialogButton::Primary{"Primary"}else{"None"},"theme":if s.shield.get_value(ThemeVariant::requested_theme_variant_property())==Some(ThemeVariant::dark()){"Dark"}else{"unexpected"},"hasRoot":s.scope.borrow().as_ref().and_then(|scope|scope.owner()).is_some(),"Spacing":c.root.spacing(),"sliders":([&c.hue,&c.saturation,&c.lightness].into_iter().map(|v|json!({"Header":header(v),"Minimum":v.minimum(),"Maximum":v.maximum(),"Value":v.value(),"ValueBits":format!("{:016X}",v.value().to_bits()),"Width":v.width()})).collect::<Vec<_>>()),"hex":c.hex.text(),"hexWidth":c.hex.width(),"hexHeader":c.hex.parent().unwrap().cast::<StackPanel>().unwrap().children().get(0).cast::<TextBlock>().unwrap().text(),"swatches":([&c.preview,&c.original].into_iter().map(|b|json!({"label":header(b),"color":hex(b.background().unwrap().as_solid_color_brush().unwrap().color()),"width":b.width(),"height":b.height()})).collect::<Vec<_>>())})
}
fn same_json(actual: &serde_json::Value, expected: &serde_json::Value) {
    use serde_json::Value;
    match (actual, expected) {
        (Value::Number(a), Value::Number(b)) => {
            assert_eq!(a.as_f64().unwrap().to_bits(), b.as_f64().unwrap().to_bits())
        }
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len());
            for (a, b) in a.iter().zip(b) {
                same_json(a, b)
            }
        }
        (Value::Object(a), Value::Object(b)) => {
            assert_eq!(a.len(), b.len());
            for (k, v) in b {
                if k == "Value" && b.contains_key("ValueBits") {
                    let bits = u64::from_str_radix(b["ValueBits"].as_str().unwrap(), 16).unwrap();
                    assert_eq!(
                        a[k].as_f64().unwrap().to_bits(),
                        bits,
                        "original .NET slider bits"
                    );
                } else {
                    same_json(&a[k], v)
                }
            }
        }
        _ => assert_eq!(actual, expected),
    }
}
#[test]
fn all_queued_unchanged_dialog_reference_cases_match_native_content_and_results() {
    native(|| {
        let corpus: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/color-dialog.json")).unwrap();
        let w = window();
        let mut count = 0;
        for case in corpus["corpus"]["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["queued"] == true)
        {
            count += 1;
            let name = case["name"].as_str().unwrap();
            let task = crate::dialog_future_test_support::show_color(
                Some(w.clone().upcast()),
                color(case["initial"].as_str().unwrap()),
                Some(ThemeVariant::dark()),
            );
            let s = session();
            same_json(&snapshot(&s), &case["states"][0]);
            Dispatcher::ui_thread().run_jobs(None);
            same_json(&snapshot(&s), &case["states"][1]);
            match name {
                "alpha_initial" | "rgb_initial" | "secondary" => {}
                "slider_hue" => s.content.hue.set_range_value(120.),
                "slider_bounds" => {
                    s.content.hue.set_range_value(999.);
                    s.content.saturation.set_range_value(-1.);
                    s.content.lightness.set_range_value(123.);
                }
                "hex_alpha" => s.content.hex.set_text(Some("#7F12AB34")),
                "hex_short" => s.content.hex.set_text(Some("#abc")),
                "hex_invalid" => s.content.hex.set_text(Some("not a color")),
                "hex_empty" => s.content.hex.set_text(Some("")),
                "hex_then_slider" => {
                    s.content.hex.set_text(Some("#12345678"));
                    Dispatcher::ui_thread().run_jobs(None);
                    s.content.lightness.set_range_value(50.);
                }
                "cancel" => s.content.hex.set_text(Some("#112233")),
                _ => panic!("unknown source operation: {name}"),
            }
            same_json(&snapshot(&s), &case["states"][2]);
            Dispatcher::ui_thread().run_jobs(None);
            same_json(&snapshot(&s), &case["states"][3]);
            finish_for_test(&s, !matches!(name, "cancel" | "secondary"));
            assert_eq!(
                task.clone().now_or_never(),
                Some(Ok(case["result"].as_str().map(color))),
                "{name}"
            );
        }
        assert_eq!(count, 11);
        w.close();
    });
}

fn localization_protocol_cases() -> Vec<serde_json::Value> {
    let corpus: serde_json::Value =
        serde_json::from_str(include_str!("../../tests/fixtures/color-dialog.json")).unwrap();
    corpus["corpus"]["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["name"] == "localized_protocol")
        .cloned()
        .collect()
}
fn localized_units(value: &serde_json::Value) -> Option<ferroribbon_core::RibbonString> {
    value.as_array().map(|v| {
        ferroribbon_core::RibbonString::from_utf16(
            v.iter()
                .map(|u| u.as_u64().unwrap() as u16)
                .collect::<Vec<_>>(),
        )
    })
}
fn install_localization(case: &serde_json::Value) {
    use ferroribbon_core::{localization::RibbonCulture, model::RibbonItemsSource};
    let field = case["field"].as_str().unwrap();
    RibbonStrings::register(
        Some("en-US".into()),
        Some(RibbonItemsSource::from_values(
            [
                ("MoreColors", "More Colors..."),
                ("Ok", "OK"),
                ("Cancel", "Cancel"),
            ]
            .into_iter()
            .map(|(key, fallback)| {
                (
                    Some(key.into()),
                    if key == field {
                        localized_units(&case["input"])
                    } else {
                        Some(fallback.into())
                    },
                )
            })
            .collect(),
        )),
    )
    .unwrap();
    RibbonStrings::set_current(Some(
        RibbonStrings::for_culture(Some(RibbonCulture::new("en-US".into(), "en".into()))).unwrap(),
    ))
    .unwrap();
}
fn native_text_units(value: Option<String>) -> serde_json::Value {
    value.map_or(serde_json::Value::Null, |s| {
        serde_json::json!(s.encode_utf16().collect::<Vec<_>>())
    })
}
#[test]
fn localized_null_title_faults_task_without_overlay_or_focus_changes() {
    if crate::test_process::isolated_case(concat!(
        module_path!(),
        "::localized_null_title_faults_task_without_overlay_or_focus_changes"
    )) {
        return;
    }
    native(|| {
        let previous = RibbonStrings::current();
        let cases = localization_protocol_cases();
        let case = &cases[0];
        assert_eq!(case["error"], "NullReferenceException");
        install_localization(case);
        let w = window();
        let button = Button::new();
        w.set_content(Some(Control::boxed(button.clone())));
        w.update_layout();
        assert!(button.focus());
        let task = crate::dialog_future_test_support::show_color(
            Some(w.clone().upcast()),
            Color::from_rgb(1, 2, 3),
            None,
        );
        let result = task.clone().now_or_never();
        // Clean up the old faulty behavior before asserting the captured result.
        if task.clone().now_or_never().is_none() {
            finish_for_test(&session(), false);
        }
        assert_eq!(
            result,
            Some(Err(RibbonColorDialogError::NullLocalizedTitle))
        );
        assert!(button.is_focused());
        assert!(ACTIVE.with_borrow(|s| s.is_empty()));
        assert_eq!(
            crate::dialog_future_test_support::show_color(None, Color::from_rgb(1, 2, 3), None)
                .clone()
                .now_or_never(),
            Some(Ok(None))
        );
        RibbonStrings::set_current(Some(previous)).unwrap();
        w.close();
    });
}
#[test]
fn localized_nullable_button_labels_and_unicode_titles_match_source() {
    if crate::test_process::isolated_case(concat!(
        module_path!(),
        "::localized_nullable_button_labels_and_unicode_titles_match_source"
    )) {
        return;
    }
    native(|| {
        let previous = RibbonStrings::current();
        let w = window();
        let mut checked = 0;
        for case in localization_protocol_cases().iter().filter(|c| {
            !c["faulted"].as_bool().unwrap()
                && localized_units(&c["input"]).is_none_or(|s| s.to_utf8().is_ok())
        }) {
            checked += 1;
            install_localization(case);
            let task = crate::dialog_future_test_support::show_color(
                Some(w.clone().upcast()),
                Color::from_rgb(1, 2, 3),
                None,
            );
            assert!(task.clone().now_or_never().is_none());
            let s = session();
            let title = s
                .shield
                .get_visual_descendants()
                .find_map(|v| {
                    v.cast::<ferroui_controls::ContentControl>()
                        .filter(|c| c.name().as_deref() == Some("Title"))
                })
                .unwrap()
                .content()
                .map(|v| v.downcast_ref::<String>().unwrap().clone());
            let ok = s
                .shield
                .primary_button()
                .unwrap()
                .content()
                .map(|v| v.downcast_ref::<String>().unwrap().clone());
            let cancel = s
                .shield
                .close_button()
                .unwrap()
                .content()
                .map(|v| v.downcast_ref::<String>().unwrap().clone());
            finish_for_test(&s, false);
            assert_eq!(native_text_units(title), case["title"], "{case}");
            assert_eq!(native_text_units(ok), case["ok"], "{case}");
            assert_eq!(native_text_units(cancel), case["cancel"], "{case}");
            assert_eq!(task.clone().now_or_never(), Some(Ok(None)));
        }
        assert_eq!(checked, 9);
        RibbonStrings::set_current(Some(previous)).unwrap();
        w.close();
    });
}
#[test]
fn localized_unpaired_utf16_has_explicit_framework_failure_without_panic_or_overlay() {
    if crate::test_process::isolated_case(concat!(
        module_path!(),
        "::localized_unpaired_utf16_has_explicit_framework_failure_without_panic_or_overlay"
    )) {
        return;
    }
    native(|| {
        let previous = RibbonStrings::current();
        let w = window();
        let mut checked = 0;
        for case in localization_protocol_cases()
            .iter()
            .filter(|c| localized_units(&c["input"]).is_some_and(|s| s.to_utf8().is_err()))
        {
            checked += 1;
            assert_eq!(
                case["error"],
                serde_json::Value::Null,
                "source accepts unpaired UTF16"
            );
            assert_eq!(case["showCalled"], true);
            install_localization(case);
            assert_eq!(
                crate::dialog_future_test_support::show_color(None, Color::from_rgb(1, 2, 3), None)
                    .clone()
                    .now_or_never(),
                Some(Ok(None))
            );
            let actual = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                crate::dialog_future_test_support::show_color(
                    Some(w.clone().upcast()),
                    Color::from_rgb(1, 2, 3),
                    None,
                )
            }));
            assert!(
                actual.is_ok(),
                "framework limitation must be an explicit task failure, not panic"
            );
            let task = actual.unwrap();
            let (key, property) = match case["field"].as_str().unwrap() {
                "MoreColors" => ("title", "ContentDialog.Title"),
                "Ok" => ("ok", "ContentDialog.PrimaryButtonText"),
                "Cancel" => ("cancel", "ContentDialog.CloseButtonText"),
                _ => unreachable!(),
            };
            assert_eq!(
                task.clone().now_or_never(),
                Some(Err(RibbonColorDialogError::UnsupportedUtf16 {
                    property,
                    units: localized_units(&case[key]).unwrap().as_utf16().to_vec(),
                }))
            );
            assert!(ACTIVE.with_borrow(|s| s.is_empty()));
        }
        assert_eq!(checked, 3);
        RibbonStrings::set_current(Some(previous)).unwrap();
        w.close();
    });
}
fn install_dialog_table(values: Vec<(&str, Option<ferroribbon_core::RibbonString>)>) {
    use ferroribbon_core::{localization::RibbonCulture, model::RibbonItemsSource};
    RibbonStrings::register(
        Some("en-US".into()),
        Some(RibbonItemsSource::from_values(
            values
                .into_iter()
                .map(|(k, v)| (Some(k.into()), v))
                .collect(),
        )),
    )
    .unwrap();
    RibbonStrings::set_current(Some(
        RibbonStrings::for_culture(Some(RibbonCulture::new("en-US".into(), "en".into()))).unwrap(),
    ))
    .unwrap();
}
fn visual_protocol() -> Vec<serde_json::Value> {
    serde_json::from_str::<serde_json::Value>(include_str!(
        "../../tests/fixtures/content-dialog-visual.json"
    ))
    .unwrap()["corpus"]
        .as_array()
        .unwrap()
        .clone()
}
#[test]
fn original_content_dialog_visibility_states_preserve_own_and_parent_visibility() {
    if crate::test_process::isolated_case(concat!(
        module_path!(),
        "::original_content_dialog_visibility_states_preserve_own_and_parent_visibility"
    )) {
        return;
    }
    native(|| {
        let w = window();
        let previous = RibbonStrings::current();
        let mut count = 0;
        for case in visual_protocol()
            .iter()
            .filter(|c| c["name"] == "visual" && !c["title"].is_null())
        {
            count += 1;
            let text = |key: &str| case[key].as_str().map(Into::into);
            install_dialog_table(vec![
                ("MoreColors", text("title")),
                ("Ok", text("primary")),
                ("Cancel", text("close")),
            ]);
            let task = crate::dialog_future_test_support::show_color(
                Some(w.clone().upcast()),
                Color::from_rgb(1, 2, 3),
                None,
            );
            assert!(task.clone().now_or_never().is_none());
            let s = session();
            let actual = serde_json::json!({"CommandSpace":s.shield.primary_button().unwrap().parent().unwrap().cast::<Control>().unwrap().is_visible(),"PrimaryButton":s.shield.primary_button().unwrap().is_visible(),"CloseButton":s.shield.close_button().unwrap().is_visible()});
            let title = s
                .shield
                .get_visual_descendants()
                .find_map(|v| {
                    v.cast::<ferroui_controls::ContentControl>()
                        .filter(|c| c.name().as_deref() == Some("Title"))
                })
                .unwrap();
            let title_visible = title.is_visible();
            finish_for_test(&s, false);
            for key in ["CommandSpace", "PrimaryButton", "CloseButton"] {
                assert_eq!(actual[key], case["visibility"][key], "{case}");
            }
            assert_eq!(title_visible, case["titleVisibility"] == "Visible");
        }
        assert_eq!(count, 18);
        RibbonStrings::set_current(Some(previous)).unwrap();
        w.close();
    });
}
#[test]
fn original_content_dialog_focus_fallback_uses_content_default_then_command_space() {
    if crate::test_process::isolated_case(concat!(
        module_path!(),
        "::original_content_dialog_focus_fallback_uses_content_default_then_command_space"
    )) {
        return;
    }
    native(|| {
        let w = window();
        let previous = RibbonStrings::current();
        let outside = Button::new();
        w.set_content(Some(Control::boxed(outside.clone())));
        w.update_layout();
        let mut count = 0;
        for case in visual_protocol().iter().filter(|c| c["name"] == "focus") {
            count += 1;
            install_dialog_table(vec![
                ("MoreColors", Some("More Colors...".into())),
                (
                    "Ok",
                    case["primary"].as_bool().unwrap().then(|| "OK".into()),
                ),
                (
                    "Cancel",
                    case["close"].as_bool().unwrap().then(|| "Cancel".into()),
                ),
            ]);
            assert!(outside.focus());
            let task = crate::dialog_future_test_support::show_color(
                Some(w.clone().upcast()),
                Color::from_rgb(1, 2, 3),
                None,
            );
            let s = session();
            match case["content"].as_str() {
                Some("hue") => {}
                Some("saturation") => s.content.hue.set_is_enabled(false),
                None => s.content.root.set_is_enabled(false),
                _ => unreachable!(),
            }
            outside.focus();
            let actual = w.focus_manager().get_focused_element().map(|e| {
                if e == s.content.hue.clone().upcast::<InputElement>() {
                    "hue"
                } else if e == s.content.saturation.clone().upcast::<InputElement>() {
                    "saturation"
                } else if e
                    == s.shield
                        .primary_button()
                        .unwrap()
                        .clone()
                        .upcast::<InputElement>()
                {
                    "primary"
                } else if e
                    == s.shield
                        .close_button()
                        .unwrap()
                        .clone()
                        .upcast::<InputElement>()
                {
                    "close"
                } else if e == outside.clone().upcast::<InputElement>() {
                    "outside"
                } else {
                    "unexpected"
                }
            });
            finish_for_test(&s, false);
            assert_eq!(actual, case["focused"].as_str(), "{case}");
            assert_eq!(task.clone().now_or_never(), Some(Ok(None)));
        }
        assert_eq!(count, 5);
        RibbonStrings::set_current(Some(previous)).unwrap();
        w.close();
    });
}

#[test]
fn standard_color_future_drop_releases_waker_keeps_dialog_and_releases_session_after_closed() {
    native(|| {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};
        struct Signal(Arc<AtomicUsize>);
        impl std::task::Wake for Signal {
            fn wake(self: Arc<Self>) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }
        let w = window();
        let mut future: RibbonColorDialogFuture =
            RibbonColorDialog::show_async(Some(w.clone().upcast()), Color::from_rgb(1, 2, 3), None);
        let s = session();
        let weak_session = Rc::downgrade(&s);
        let wakes = Arc::new(AtomicUsize::new(0));
        let signal = Arc::new(Signal(wakes.clone()));
        let weak_signal = Arc::downgrade(&signal);
        let waker = Waker::from(signal.clone());
        assert!(
            future
                .as_mut()
                .poll(&mut Context::from_waker(&waker))
                .is_pending()
        );
        drop(waker);
        drop(signal);
        assert!(weak_signal.upgrade().is_some());
        drop(future);
        assert!(weak_signal.upgrade().is_none());
        assert!(
            s.shield.is_dialog_showing(),
            "an abandoned observation does not dismiss its visible dialog"
        );
        let closed = Rc::new(Cell::new(0));
        let count = closed.clone();
        let token = s.shield.dialog_closed(move |_| count.set(count.get() + 1));
        finish_for_test(&s, true);
        assert_eq!(closed.get(), 1);
        assert_eq!(
            wakes.load(Ordering::SeqCst),
            0,
            "dropped observation is never resumed"
        );
        token.dispose();
        drop(s);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(weak_session.upgrade().is_none());
        assert!(ACTIVE.with_borrow(Vec::is_empty));
        w.close();
    });
}

#[test]
fn an_abandoned_color_producer_is_a_typed_error_instead_of_a_pending_or_empty_result() {
    native(|| {
        let w = window();
        let mut future =
            RibbonColorDialog::show_async(Some(w.clone().upcast()), Color::from_rgb(1, 2, 3), None);
        let s = session();
        // Explicitly simulate producer loss after native cleanup. No visible
        // dialog or callback remains responsible for sending a color.
        s.scope.take().unwrap().dismiss();
        ACTIVE.with_borrow_mut(Vec::clear);
        drop(s);
        assert_eq!(
            future
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop())),
            std::task::Poll::Ready(Err(RibbonColorDialogError::CompletionDropped))
        );
        w.close();
    });
}

#[test]
fn source_title_failure_precedes_late_open_failure_and_null_root_stays_first() {
    if crate::test_process::isolated_case(concat!(
        module_path!(),
        "::source_title_failure_precedes_late_open_failure_and_null_root_stays_first"
    )) {
        return;
    }
    native(|| {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/dialog-future-order.json"
        ))
        .unwrap();
        assert_eq!(fixture[2]["error"], "NullReferenceException");
        assert_eq!(fixture[2]["showCalled"], false);
        let previous = RibbonStrings::current();
        let w = window();
        let first = crate::dialog_future_test_support::show_color(
            Some(w.clone().upcast()),
            Color::from_rgb(1, 2, 3),
            None,
        );
        let active = session();
        install_dialog_table(vec![
            ("MoreColors", None),
            ("Ok", Some("OK".into())),
            ("Cancel", Some("Cancel".into())),
        ]);
        for root in [Some(w.clone().upcast()), None] {
            let mut future =
                RibbonColorDialog::show_async(root.clone(), Color::from_rgb(1, 2, 3), None);
            assert_eq!(
                future
                    .as_mut()
                    .poll(&mut Context::from_waker(Waker::noop())),
                std::task::Poll::Ready(if root.is_none() {
                    Ok(None)
                } else {
                    Err(RibbonColorDialogError::NullLocalizedTitle)
                })
            );
        }
        assert_eq!(ACTIVE.with_borrow(Vec::len), 1);
        finish_for_test(&active, false);
        assert_eq!(first.now_or_never(), Some(Ok(None)));
        RibbonStrings::set_current(Some(previous)).unwrap();
        w.close();
    });
}
