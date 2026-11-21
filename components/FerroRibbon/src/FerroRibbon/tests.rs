use crate::{
    primitives::{RibbonUniformGridPanel, ribbon_hidden_focus},
    register_types,
};
use ferroui_base::data::BindingPriority;
use ferroui_base::diagnostics::FerroObjectDiagnosticExtensions;
use ferroui_base::input::KeyboardNavigation;
use ferroui_base::threading::CancellationToken;
use ferroui_base::{Rect, Size};
use ferroui_controls::{AppBuilder, Application, Button, Panel};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const TEST_ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "FerroRibbon.Headless.Tests",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
pub(crate) fn run(test: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&TEST_ASSEMBLY))
        .dispatch(
            move || {
                register_types();
                test();
            },
            CancellationToken::none(),
        )
        .wait()
        .expect("headless case");
}
fn layout(panel: &RibbonUniformGridPanel) {
    panel.invalidate_measure();
    panel.measure(Size::INFINITY);
    panel.arrange(Rect::from_size(panel.desired_size()));
}
#[test]
fn unchanged_source_layout_body_corpus() {
    run(|| {
        let corpus: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/uniform-grid.json")).unwrap();
        for case in corpus["cases"].as_array().unwrap() {
            let panel = RibbonUniformGridPanel::new();
            panel.set_use_layout_rounding(false);
            panel.set_columns(case["columns"].as_i64().unwrap() as i32);
            panel.set_item_width(12.5);
            panel.set_item_height(9.25);
            panel.set_spacing(1.5);
            panel.set_first_row(case["first"].as_i64().unwrap() as i32);
            panel.set_max_rows(case["max"].as_i64().unwrap() as i32);
            for i in 0..case["count"].as_u64().unwrap() {
                let child = Button::new();
                child.set_use_layout_rounding(false);
                child.set_is_visible(i % 4 != 3);
                panel.children().add(child);
            }
            layout(&panel);
            assert_eq!(panel.row_count() as i64, case["rows"].as_i64().unwrap());
            assert_eq!(
                vec![panel.desired_size().width, panel.desired_size().height],
                case["size"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|n| n.as_f64().unwrap())
                    .collect::<Vec<_>>(),
                "{case}"
            );
            for (child, expected) in panel
                .children()
                .snapshot()
                .iter()
                .zip(case["children"].as_array().unwrap())
            {
                let b = child.bounds();
                assert_eq!(
                    vec![b.x, b.y, b.width, b.height],
                    expected["bounds"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|n| n.as_f64().unwrap())
                        .collect::<Vec<_>>(),
                    "{case}"
                );
                assert_eq!(
                    KeyboardNavigation::get_is_tab_stop(child),
                    expected["tab"].as_bool().unwrap()
                );
            }
        }
    });
}
#[test]
fn registered_properties_invalidate_and_plain_properties_preserve_source_behavior() {
    run(|| {
        let panel = RibbonUniformGridPanel::new();
        layout(&panel);
        assert!(panel.is_measure_valid());
        panel.set_columns(2);
        assert!(!panel.is_measure_valid());
        layout(&panel);
        panel.set_item_width(20.);
        assert!(!panel.is_measure_valid());
        layout(&panel);
        panel.set_item_height(18.);
        assert!(!panel.is_measure_valid());
        layout(&panel);
        panel.set_spacing(3.);
        assert!(!panel.is_measure_valid());
        layout(&panel);
        panel.set_first_row(1);
        panel.set_max_rows(2);
        assert!(panel.is_measure_valid());
        // Source auto-properties do not invalidate; gallery owners must request layout.
        layout(&panel);
        assert_eq!(panel.desired_size(), Size::new(43., 18.));
    });
}
#[test]
fn focus_restores_local_vs_style_and_matches_source_trace() {
    run(|| {
        let child = Button::new();
        let property = KeyboardNavigation::is_tab_stop_property();
        let mut trace = Vec::new();
        let capture = |trace: &mut Vec<String>| {
            let tab = if KeyboardNavigation::get_is_tab_stop(&child) {
                "True"
            } else {
                "False"
            };
            let unset = if child.get_diagnostic(property.as_property()).priority()
                != BindingPriority::LocalValue
            {
                "True"
            } else {
                "False"
            };
            trace.push(format!("{tab}:{unset}"));
        };
        capture(&mut trace);
        ribbon_hidden_focus::set(&child.clone().upcast(), true);
        capture(&mut trace);
        ribbon_hidden_focus::set(&child.clone().upcast(), true);
        capture(&mut trace);
        ribbon_hidden_focus::set(&child.clone().upcast(), false);
        capture(&mut trace);
        KeyboardNavigation::set_is_tab_stop(&child, false);
        ribbon_hidden_focus::set(&child.clone().upcast(), true);
        capture(&mut trace);
        ribbon_hidden_focus::set(&child.clone().upcast(), false);
        capture(&mut trace);
        let corpus: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/uniform-grid.json")).unwrap();
        assert_eq!(serde_json::json!(trace), corpus["focus"]);
        child.clear_value(property);
        let _style = child.set_value_with_priority(property, false, BindingPriority::Style);
        ribbon_hidden_focus::set(&child.clone().upcast(), true);
        ribbon_hidden_focus::set(&child.clone().upcast(), false);
        assert!(!child.get_value(property));
        assert_eq!(
            child.get_diagnostic(property.as_property()).priority(),
            BindingPriority::Style
        );
    });
}
#[test]
fn panel_descendants_are_visited_and_hidden_controls_are_not_retained() {
    run(|| {
        let panel = Panel::new();
        let child = Button::new();
        let weak = child.downgrade();
        panel.children().add(child.clone());
        ribbon_hidden_focus::set(&panel.clone().upcast(), true);
        assert!(!KeyboardNavigation::get_is_tab_stop(&child));
        ribbon_hidden_focus::set(&panel.clone().upcast(), false);
        assert!(KeyboardNavigation::get_is_tab_stop(&child));
        ribbon_hidden_focus::set(&panel.clone().upcast(), true);
        panel.children().clear();
        drop(child);
        assert!(weak.upgrade().is_none());
    });
}

#[test]
fn bound_tab_stop_remains_explicitly_unsupported_without_mutating_binding_or_hidden_state() {
    run(|| {
        use ferroui_base::reactive::Observable;
        let property = KeyboardNavigation::is_tab_stop_property();
        for pending in [false, true] {
            let child = Button::new();
            let source = if pending {
                Observable::never()
            } else {
                Observable::single_value(true)
            };
            let binding = child.bind(property, source, BindingPriority::LocalValue);
            let before = child.get_local_value(property);
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                ribbon_hidden_focus::set(&child.clone().upcast(), true)
            }));
            assert!(result.is_err());
            assert!(child.has_local_value_binding(property.as_property()));
            assert_eq!(child.get_local_value(property), before);
            binding.dispose();
            // Rejection happened before installing a hidden entry.
            ribbon_hidden_focus::set(&child.clone().upcast(), true);
            assert!(!child.get_value(property));
            ribbon_hidden_focus::set(&child.clone().upcast(), false);
            assert_eq!(child.get_local_value(property), None);
        }
    });
}

#[test]
fn animated_tab_stop_restores_local_presence_without_overriding_animation() {
    run(|| {
        let property = KeyboardNavigation::is_tab_stop_property();
        for local in [None, Some(false), Some(true)] {
            for styled in [false, true] {
                for animated in [false, true] {
                    let child = Button::new();
                    let style = child
                        .set_value_with_priority(property, styled, BindingPriority::Style)
                        .unwrap();
                    if let Some(value) = local {
                        child.set_value(property, value);
                    }
                    let animation = child
                        .set_value_with_priority(property, animated, BindingPriority::Animation)
                        .unwrap();
                    for _ in 0..3 {
                        ribbon_hidden_focus::set(&child.clone().upcast(), true);
                        ribbon_hidden_focus::set(&child.clone().upcast(), true);
                        assert_eq!(child.get_local_value(property), Some(false));
                        // Source local writes leave the higher-priority animation effective.
                        assert_eq!(child.get_value(property), animated);
                        ribbon_hidden_focus::set(&child.clone().upcast(), false);
                        assert_eq!(child.get_local_value(property), local);
                        assert_eq!(child.get_value(property), animated);
                    }
                    animation.dispose();
                    assert_eq!(child.get_value(property), local.unwrap_or(styled));
                    assert_eq!(child.get_local_value(property), local);
                    style.dispose();
                }
            }
        }
    });
}
