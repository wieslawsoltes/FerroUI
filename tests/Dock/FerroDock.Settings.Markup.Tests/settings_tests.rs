//! Dependency-exported settings metadata, defaults and compiled TwoWay attachment.
use ferrodock_model::core::{DockFloatingWindowHostMode, DockOperation, DockWindowOwnerMode};
use ferrodock_settings::{
    DockCommandBarMergingScope, DockFloatingWindowOwnerPolicy, DockProperties as P,
    DockSettings as S, DockSettingsOptions,
};
use ferroui_base::controls::ResourceKey;
use ferroui_base::input::{Key, KeyGesture, KeyModifiers};
use ferroui_base::{threading::CancellationToken, Ref};
use ferroui_controls::{AppBuilder, Application, Border, Grid};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use std::rc::Rc;
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "ferrodock-settings-external",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
#[test]
fn downstream_compiler_uses_original_docking_state_flags_for_literals_and_static_members() {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            || {
                use ferrodock_model::core::DockingWindowState as State;
                ferrodock_settings::register_types();
                ferroui_markup_xaml::register_types();
                crate::compiled_markup::register();
                let value = crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferrodock_settings_markup_tests/DockingStateConsumer.xaml",
                )
                .unwrap()
                .unwrap();
                let grid = value.downcast_ref::<Ref<Grid>>().unwrap();
                assert_eq!(
                    grid.tag().unwrap().downcast_ref::<State>(),
                    Some(&State::DOCUMENT)
                );
                for (key, bits) in [
                    ("None", 0),
                    ("DockedFloating", 9),
                    ("PinnedDocument", 6),
                    ("All", 31),
                    ("Numeric", 9),
                ] {
                    let value = grid
                        .resources()
                        .try_get_resource(&ResourceKey::from(key), None)
                        .flatten()
                        .unwrap();
                    assert_eq!(value.downcast_ref::<State>().unwrap().bits(), bits, "{key}");
                }
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}

#[test]
fn compiled_settings_properties_options_enums_and_default_two_way_bindings_are_native() {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            || {
                ferrodock_settings::register_types();
                ferroui_markup_xaml::register_types();
                crate::compiled_markup::register();
                let value = crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferrodock_settings_markup_tests/SettingsConsumer.xaml",
                )
                .unwrap()
                .unwrap();
                let grid = value.downcast_ref::<Ref<Grid>>().unwrap();
                let dictionary = grid.resources();
                let get = |key: &str| {
                    dictionary
                        .try_get_resource(&ResourceKey::from(key), None)
                        .flatten()
                        .unwrap()
                };
                let options = get("Options");
                let options = options.downcast_ref::<Rc<DockSettingsOptions>>().unwrap();
                assert_eq!(options.minimum_horizontal_drag_distance(), Some(13.0));
                assert_eq!(
                    options.floating_window_host_mode(),
                    Some(DockFloatingWindowHostMode::Managed)
                );
                assert_eq!(
                    options.floating_window_owner_policy(),
                    Some(DockFloatingWindowOwnerPolicy::NeverOwned)
                );
                assert_eq!(
                    options.default_floating_window_owner_mode(),
                    Some(DockWindowOwnerMode::RootWindow)
                );
                assert_eq!(options.selector_enabled(), Some(false));
                assert_eq!(
                    options.document_selector_key_gesture(),
                    Some(KeyGesture::new(
                        Key::F9,
                        KeyModifiers::CONTROL | KeyModifiers::SHIFT
                    ))
                );
                assert_eq!(
                    options.command_bar_merging_scope(),
                    Some(DockCommandBarMergingScope::ActiveDockable)
                );
                assert_eq!(
                    get("Operation").downcast_ref::<DockOperation>(),
                    Some(&DockOperation::OuterBottom)
                );
                assert_eq!(
                    get("Scope").downcast_ref::<DockCommandBarMergingScope>(),
                    Some(&DockCommandBarMergingScope::ActiveDocument)
                );
                assert!(get("PropertyOwner").downcast_ref::<Ref<P>>().is_some());
                assert_eq!(
                    grid.tag().unwrap().downcast_ref::<f64>(),
                    Some(&S::minimum_horizontal_drag_distance())
                );
                let target = grid.children().get(0).cast::<Border>().unwrap();
                let source = get("Model").downcast_ref::<Ref<Border>>().unwrap().clone();
                assert!(P::get_is_drag_area(&target));
                P::set_is_drag_area(&target, false);
                assert!(!source.is_visible());
                source.set_is_visible(true);
                assert!(P::get_is_drag_area(&target));
                assert!(!P::get_is_drag_enabled(&target));
                assert!(!P::get_is_drop_enabled(&target));
                assert_eq!(P::get_dock_group(&target).as_deref(), Some("Workspace"));
                assert_eq!(
                    P::get_indicator_dock_operation(&target),
                    DockOperation::OuterLeft
                );
                assert!(P::get_dock_adorner_host(&target).is_some());
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
