use crate::dock_settings::Scope;
use crate::{
    DockCommandBarMergingScope, DockFloatingWindowOwnerPolicy as Policy, DockGlobalDockingPreset,
    DockLogger, DockSettings as S, WindowDragDockScope,
};
use ferrodock_model::core::{DockFloatingWindowHostMode as Mode, DockWindowOwnerMode};
use ferrodock_model_reactiveui::controls::RootDock;
use ferroui_base::input::{Key, KeyGesture, KeyModifiers};
use ferroui_base::{PixelPoint, Vector};
use std::sync::{Arc, Mutex};
#[test]
fn original_host_resolution_uses_root_override() {
    let _scope = Scope::new();
    S::set_floating_window_host_mode(Mode::Native);
    S::set_use_managed_windows(false);
    let root = RootDock::new();
    root.set_floating_window_host_mode(Mode::Managed);
    assert_eq!(
        S::resolve_floating_window_host_mode(Some(&*root)),
        Mode::Managed
    );
}
#[test]
fn unnamed_host_modes_retain_original_root_and_global_precedence() {
    let _scope = Scope::new();
    let check = |name: &str, value: bool| {
        assert!(value, "{name}");
        assert!(
            include_str!("../../../tests/Dock/data/numeric-enum-markup-reference.tsv")
                .lines()
                .any(|row| row == format!("{name}\t1")),
            "missing source check {name}"
        );
    };
    let root = RootDock::new();
    root.set_floating_window_host_mode(Mode::from_raw(-17));
    S::set_floating_window_host_mode(Mode::Managed);
    S::set_use_managed_windows(true);
    check(
        "numeric-host-mode-root-override",
        S::resolve_floating_window_host_mode(Some(&*root)).to_raw() == -17
            && !S::is_managed_window_hosting_enabled(Some(&*root)),
    );
    root.set_floating_window_host_mode(Mode::Default);
    S::set_floating_window_host_mode(Mode::from_raw(123));
    check(
        "numeric-host-mode-global-override",
        S::resolve_floating_window_host_mode(Some(&*root)).to_raw() == 123
            && !S::is_managed_window_hosting_enabled(Some(&*root)),
    );
    S::set_floating_window_host_mode(Mode::Default);
    check(
        "numeric-host-mode-original-fallback",
        S::resolve_floating_window_host_mode(Some(&*root)) == Mode::Managed
            && S::is_managed_window_hosting_enabled(Some(&*root)),
    );
}
#[test]
fn original_host_resolution_uses_global_when_set() {
    let _scope = Scope::new();
    S::set_floating_window_host_mode(Mode::Managed);
    S::set_use_managed_windows(false);
    let root = RootDock::new();
    root.set_floating_window_host_mode(Mode::Default);
    assert_eq!(
        S::resolve_floating_window_host_mode(Some(&*root)),
        Mode::Managed
    );
}
#[test]
fn original_host_resolution_falls_back_to_managed_flag() {
    let _scope = Scope::new();
    S::set_floating_window_host_mode(Mode::Default);
    S::set_use_managed_windows(true);
    assert_eq!(S::resolve_floating_window_host_mode(None), Mode::Managed);
}
#[test]
fn original_owner_policy_precedes_owner_flag() {
    let _scope = Scope::new();
    S::set_use_owner_for_floating_windows(false);
    S::set_floating_window_owner_policy(Policy::AlwaysOwned);
    assert!(S::should_use_owner_for_floating_windows());
    S::set_use_owner_for_floating_windows(true);
    S::set_floating_window_owner_policy(Policy::NeverOwned);
    assert!(!S::should_use_owner_for_floating_windows());
    S::set_floating_window_owner_policy(Policy::Default);
    assert!(S::should_use_owner_for_floating_windows());
    S::set_use_owner_for_floating_windows(false);
    assert!(!S::should_use_owner_for_floating_windows());
}
#[test]
fn host_mode_precedence_matrix_and_managed_predicate() {
    let _scope = Scope::new();
    let root = RootDock::new();
    for r in [
        Mode::Default,
        Mode::Native,
        Mode::Managed,
        Mode::from_raw(-17),
        Mode::from_raw(i32::MIN),
        Mode::from_raw(i32::MAX),
    ] {
        root.set_floating_window_host_mode(r);
        for g in [
            Mode::Default,
            Mode::Native,
            Mode::Managed,
            Mode::from_raw(123),
            Mode::from_raw(i32::MIN),
            Mode::from_raw(i32::MAX),
        ] {
            S::set_floating_window_host_mode(g);
            for managed in [false, true] {
                S::set_use_managed_windows(managed);
                let expected = if r != Mode::Default {
                    r
                } else if g != Mode::Default {
                    g
                } else if managed {
                    Mode::Managed
                } else {
                    Mode::Native
                };
                assert_eq!(S::resolve_floating_window_host_mode(Some(&*root)), expected);
                assert_eq!(
                    S::is_managed_window_hosting_enabled(Some(&*root)),
                    expected == Mode::Managed
                );
            }
        }
    }
}
#[test]
fn defaults_match_original_settings_fields_and_properties() {
    let _scope = Scope::new();
    assert_eq!(S::minimum_horizontal_drag_distance(), 4.0);
    assert_eq!(S::minimum_vertical_drag_distance(), 4.0);
    assert!(!S::use_floating_dock_adorner());
    assert!(!S::use_pinned_dock_window());
    assert!(!S::use_managed_windows());
    assert_eq!(S::floating_window_host_mode(), Mode::Default);
    assert!(S::use_owner_for_floating_windows());
    assert_eq!(S::floating_window_owner_policy(), Policy::Default);
    assert_eq!(
        S::default_floating_window_owner_mode(),
        DockWindowOwnerMode::Default
    );
    assert!(!S::enable_window_magnetism());
    assert_eq!(S::window_magnet_distance(), 16.0);
    assert!(S::bring_windows_to_front_on_drag());
    assert_eq!(
        S::document_window_drag_scope(),
        WindowDragDockScope::WindowChrome
    );
    assert!(!S::close_floating_windows_on_main_window_close());
    assert!(!S::show_dockable_preview_on_drag());
    assert_eq!(S::drag_preview_opacity(), 1.0);
    assert!(S::update_items_source_on_unregister());
    assert_eq!(S::global_docking_proportion(), 0.5);
    assert_eq!(
        S::global_docking_preset(),
        DockGlobalDockingPreset::GlobalFirst
    );
    assert!(!S::enable_diagnostics_logging());
    assert!(S::diagnostics_log_handler().is_none());
    assert!(S::selector_enabled());
    assert_eq!(
        S::document_selector_key_gesture(),
        KeyGesture::new(Key::Tab, KeyModifiers::CONTROL)
    );
    assert_eq!(
        S::tool_selector_key_gesture(),
        KeyGesture::new(Key::Tab, KeyModifiers::CONTROL | KeyModifiers::ALT)
    );
    assert!(!S::command_bar_merging_enabled());
    assert_eq!(
        S::command_bar_merging_scope(),
        DockCommandBarMergingScope::ActiveDocument
    );
}
#[test]
fn settings_are_process_wide_and_builder_values_are_visible_to_other_threads() {
    let _scope = Scope::new();
    S::set_minimum_horizontal_drag_distance(19.0);
    std::thread::spawn(|| {
        assert_eq!(S::minimum_horizontal_drag_distance(), 19.0);
        S::set_minimum_vertical_drag_distance(23.0);
    })
    .join()
    .unwrap();
    assert_eq!(S::minimum_vertical_drag_distance(), 23.0);
}
#[test]
fn drag_thresholds_are_strict_or_signed_and_unvalidated() {
    let _scope = Scope::new();
    for (x, y, expected) in [
        (4.0, 4.0, false),
        (-4.0, -4.0, false),
        (4.1, 0.0, true),
        (0.0, -4.1, true),
        (f64::NAN, 0.0, false),
        (f64::INFINITY, 0.0, true),
    ] {
        assert_eq!(S::is_minimum_drag_distance(Vector::new(x, y)), expected);
    }
    assert!(!S::is_minimum_drag_distance_pixels(PixelPoint::new(-4, 4)));
    assert!(S::is_minimum_drag_distance_pixels(PixelPoint::new(5, 0)));
    S::set_minimum_horizontal_drag_distance(-1.0);
    assert!(S::is_minimum_drag_distance(Vector::default()));
    S::set_minimum_horizontal_drag_distance(f64::NAN);
    S::set_minimum_vertical_drag_distance(f64::INFINITY);
    assert!(!S::is_minimum_drag_distance(Vector::new(99.0, 99.0)));
}
#[test]
fn source_pixel_min_value_overflow_and_or_short_circuit_are_preserved() {
    let _scope = Scope::new();
    assert!(
        std::panic::catch_unwind(|| S::is_minimum_drag_distance_pixels(PixelPoint::new(
            i32::MIN,
            0
        )))
        .is_err()
    );
    assert!(
        std::panic::catch_unwind(|| S::is_minimum_drag_distance_pixels(PixelPoint::new(
            0,
            i32::MIN
        )))
        .is_err()
    );
    assert!(S::is_minimum_drag_distance_pixels(PixelPoint::new(
        5,
        i32::MIN
    )));
}
#[test]
fn logger_keeps_exact_format_gate_release_visible_method_and_callback_reentrancy() {
    let _scope = Scope::new();
    let messages = Arc::new(Mutex::new(Vec::new()));
    let out = messages.clone();
    let handler: crate::DiagnosticsLogHandler = Arc::new(move |message| {
        out.lock().unwrap().push(message);
        S::set_selector_enabled(false);
    });
    S::set_diagnostics_log_handler(Some(handler.clone()));
    assert!(Arc::ptr_eq(
        &S::diagnostics_log_handler().unwrap(),
        &handler
    ));
    DockLogger::log("Disabled", "none");
    DockLogger::log_debug("Disabled", "none");
    assert!(messages.lock().unwrap().is_empty());
    S::set_enable_diagnostics_logging(true);
    DockLogger::log("Layout", "one");
    DockLogger::log_debug("Drag", "two");
    assert_eq!(
        *messages.lock().unwrap(),
        vec!["[Dock] Layout: one", "[Dock] Drag: two"]
    );
    assert!(!S::selector_enabled());
    std::thread::spawn(|| DockLogger::log("Worker", "three"))
        .join()
        .unwrap();
    assert_eq!(messages.lock().unwrap().len(), 3);
    S::set_diagnostics_log_handler(None);
    DockLogger::log("Default", "debug transport");
}
#[test]
fn replacing_callback_releases_capture_outside_settings_lock_and_panics_propagate() {
    let _scope = Scope::new();
    struct Reenter;
    impl Drop for Reenter {
        fn drop(&mut self) {
            S::set_selector_enabled(false);
        }
    }
    let captured = Reenter;
    S::set_diagnostics_log_handler(Some(Arc::new(move |_| {
        let _ = &captured;
    })));
    S::set_diagnostics_log_handler(None);
    assert!(!S::selector_enabled());
    S::set_enable_diagnostics_logging(true);
    S::set_diagnostics_log_handler(Some(Arc::new(|_| panic!("source callback failed"))));
    assert!(std::panic::catch_unwind(|| DockLogger::log("Fault", "message")).is_err());
    S::set_diagnostics_log_handler(None);
    S::set_selector_enabled(true);
    assert!(S::selector_enabled());
}
