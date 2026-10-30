use crate::dock_settings::Scope;
use crate::{
    AppBuilderExtensions, DockCommandBarMergingScope as ScopeMode,
    DockFloatingWindowOwnerPolicy as Policy, DockSettings as S, DockSettingsOptions as O,
};
use ferrodock_model::core::{DockFloatingWindowHostMode as Mode, DockWindowOwnerMode};
use ferroui_base::input::{Key, KeyGesture, KeyModifiers};
use ferroui_controls::AppBuilder;
use std::cell::Cell;
use std::rc::Rc;
fn builder() -> AppBuilder {
    crate::test_app_builder::TestAppBuilder::build_ferro_app()
}
fn assert_same_builder(original: &AppBuilder, result: &AppBuilder) {
    let count = Rc::new(Cell::new(0));
    let observer = count.clone();
    result.after_setup(move |_| observer.set(observer.get() + 1));
    (original.after_setup_callback())(original);
    assert_eq!(count.get(), 1);
}
#[test]
fn original_with_settings_applies_only_supplied_options() {
    let _scope = Scope::new();
    let b = builder();
    let o = O::new();
    o.set_minimum_horizontal_drag_distance(Some(10.0));
    o.set_minimum_vertical_drag_distance(Some(12.0));
    o.set_use_floating_dock_adorner(Some(true));
    o.set_use_pinned_dock_window(Some(true));
    o.set_use_owner_for_floating_windows(Some(false));
    o.set_floating_window_host_mode(Some(Mode::Managed));
    o.set_floating_window_owner_policy(Some(Policy::NeverOwned));
    o.set_default_floating_window_owner_mode(Some(DockWindowOwnerMode::RootWindow));
    o.set_close_floating_windows_on_main_window_close(Some(true));
    o.set_update_items_source_on_unregister(Some(false));
    let result = b.with_dock_settings(Some(&o));
    assert_same_builder(&b, &result);
    assert_eq!(S::minimum_horizontal_drag_distance(), 10.0);
    assert_eq!(S::minimum_vertical_drag_distance(), 12.0);
    assert!(S::use_floating_dock_adorner());
    assert!(S::use_pinned_dock_window());
    assert!(!S::use_owner_for_floating_windows());
    assert_eq!(S::floating_window_host_mode(), Mode::Managed);
    assert_eq!(S::floating_window_owner_policy(), Policy::NeverOwned);
    assert_eq!(
        S::default_floating_window_owner_mode(),
        DockWindowOwnerMode::RootWindow
    );
    assert!(S::close_floating_windows_on_main_window_close());
    assert!(!S::update_items_source_on_unregister());
}
#[test]
fn original_null_options_returns_same_builder_without_changes() {
    let _scope = Scope::new();
    let b = builder();
    let result = b.with_dock_settings(None);
    assert_same_builder(&b, &result);
    assert_eq!(S::minimum_horizontal_drag_distance(), 4.0);
    assert_eq!(S::minimum_vertical_drag_distance(), 4.0);
}
#[test]
fn original_fluent_extension_chain_sets_flags_and_keeps_builder() {
    let _scope = Scope::new();
    let b = builder();
    let result = b
        .use_floating_dock_adorner(true)
        .use_pinned_dock_window(true)
        .use_owner_for_floating_windows(false)
        .use_floating_window_host_mode(Mode::Native)
        .use_floating_window_owner_policy(Policy::AlwaysOwned)
        .use_default_floating_window_owner_mode(DockWindowOwnerMode::ParentWindow)
        .close_floating_windows_on_main_window_close(true)
        .update_items_source_on_unregister(false);
    assert_same_builder(&b, &result);
    assert!(S::use_floating_dock_adorner());
    assert!(S::use_pinned_dock_window());
    assert!(!S::use_owner_for_floating_windows());
    assert_eq!(S::floating_window_host_mode(), Mode::Native);
    assert_eq!(S::floating_window_owner_policy(), Policy::AlwaysOwned);
    assert_eq!(
        S::default_floating_window_owner_mode(),
        DockWindowOwnerMode::ParentWindow
    );
    assert!(S::close_floating_windows_on_main_window_close());
    assert!(!S::update_items_source_on_unregister());
}
#[test]
fn remaining_builder_extensions_and_options_keep_source_validation_behavior() {
    let _scope = Scope::new();
    let b = builder();
    b.use_managed_windows(true)
        .enable_window_magnetism(true)
        .set_window_magnet_distance(-11.0)
        .bring_windows_to_front_on_drag(false)
        .show_dockable_preview_on_drag(true)
        .set_drag_preview_opacity(2.0);
    assert!(S::use_managed_windows());
    assert!(S::enable_window_magnetism());
    assert_eq!(S::window_magnet_distance(), -11.0);
    assert!(!S::bring_windows_to_front_on_drag());
    assert!(S::show_dockable_preview_on_drag());
    assert_eq!(S::drag_preview_opacity(), 2.0);
    let o = O::new();
    o.set_use_managed_windows(Some(false));
    o.set_enable_window_magnetism(Some(false));
    o.set_window_magnet_distance(Some(17.0));
    o.set_bring_windows_to_front_on_drag(Some(true));
    o.set_show_dockable_preview_on_drag(Some(false));
    o.set_drag_preview_opacity(Some(-0.5));
    o.set_selector_enabled(Some(false));
    o.set_document_selector_key_gesture(Some(KeyGesture::new(Key::F3, KeyModifiers::SHIFT)));
    o.set_tool_selector_key_gesture(Some(KeyGesture::new(Key::F4, KeyModifiers::ALT)));
    o.set_command_bar_merging_enabled(Some(true));
    o.set_command_bar_merging_scope(Some(ScopeMode::ActiveDockable));
    b.with_dock_settings(Some(&o));
    assert!(!S::use_managed_windows());
    assert!(!S::enable_window_magnetism());
    assert_eq!(S::window_magnet_distance(), 17.0);
    assert!(S::bring_windows_to_front_on_drag());
    assert!(!S::show_dockable_preview_on_drag());
    assert_eq!(S::drag_preview_opacity(), -0.5);
    assert!(!S::selector_enabled());
    assert_eq!(
        S::document_selector_key_gesture(),
        KeyGesture::new(Key::F3, KeyModifiers::SHIFT)
    );
    assert_eq!(
        S::tool_selector_key_gesture(),
        KeyGesture::new(Key::F4, KeyModifiers::ALT)
    );
    assert!(S::command_bar_merging_enabled());
    assert_eq!(S::command_bar_merging_scope(), ScopeMode::ActiveDockable);
    o.set_selector_enabled(None);
    S::set_selector_enabled(true);
    b.with_dock_settings(Some(&o));
    assert!(S::selector_enabled());
    S::set_global_docking_proportion(-2.0);
    S::set_global_docking_preset(crate::DockGlobalDockingPreset::LocalFirst);
    S::set_document_window_drag_scope(crate::WindowDragDockScope::FullWindow);
    assert_eq!(S::global_docking_proportion(), -2.0);
    assert_eq!(
        S::global_docking_preset(),
        crate::DockGlobalDockingPreset::LocalFirst
    );
    assert_eq!(
        S::document_window_drag_scope(),
        crate::WindowDragDockScope::FullWindow
    );
}
#[test]
fn every_new_option_is_unset_and_empty_options_preserve_nondefault_globals() {
    let _scope = Scope::new();
    let o = O::new();
    assert!(o.minimum_horizontal_drag_distance().is_none());
    assert!(o.minimum_vertical_drag_distance().is_none());
    assert!(o.use_floating_dock_adorner().is_none());
    assert!(o.use_pinned_dock_window().is_none());
    assert!(o.use_managed_windows().is_none());
    assert!(o.floating_window_host_mode().is_none());
    assert!(o.use_owner_for_floating_windows().is_none());
    assert!(o.floating_window_owner_policy().is_none());
    assert!(o.default_floating_window_owner_mode().is_none());
    assert!(o.enable_window_magnetism().is_none());
    assert!(o.window_magnet_distance().is_none());
    assert!(o.bring_windows_to_front_on_drag().is_none());
    assert!(o.close_floating_windows_on_main_window_close().is_none());
    assert!(o.show_dockable_preview_on_drag().is_none());
    assert!(o.drag_preview_opacity().is_none());
    assert!(o.update_items_source_on_unregister().is_none());
    assert!(o.selector_enabled().is_none());
    assert!(o.document_selector_key_gesture().is_none());
    assert!(o.tool_selector_key_gesture().is_none());
    assert!(o.command_bar_merging_enabled().is_none());
    assert!(o.command_bar_merging_scope().is_none());
    S::set_minimum_horizontal_drag_distance(21.0);
    S::set_use_managed_windows(true);
    builder().with_dock_settings(Some(&o));
    assert_eq!(S::minimum_horizontal_drag_distance(), 21.0);
    assert!(S::use_managed_windows());
}
