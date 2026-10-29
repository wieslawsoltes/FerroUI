//! Numeric enum literals through the shared model and runtime XAML compilers.
use crate::markup_extension_tests::run;
use ferrodock_model::core::*;
use ferroui_base::{metadata::from_markup_value, BoxedValue};
use std::rc::Rc;
fn load(name: &str) -> BoxedValue {
    crate::compiled_xaml::try_load(
        None,
        &format!("ferres://ferrodock_markup_tests/{name}.xaml"),
    )
    .unwrap()
    .unwrap()
}
fn check(name: &str, value: bool) {
    assert!(value, "{name}");
    assert!(
        include_str!("../data/numeric-enum-markup-reference.tsv")
            .lines()
            .any(|row| row == format!("{name}\t1")),
        "missing original check {name}"
    );
}
#[test]
fn model_compiled_numeric_enum_literals_retain_original_values_nullable_and_parent_identity() {
    run(|| {
        let options =
            from_markup_value::<Rc<DockWindowOptions>>(&Some(load("CoreNumericWindowOptionsRoot")))
                .unwrap();
        check(
            "numeric-enum-options-owner-and-flags",
            options.owner_mode().to_raw() == -19
                && options.is_modal()
                && options.show_in_taskbar() == Some(false),
        );
        let parent = options.parent_window().unwrap();
        check(
            "numeric-enum-parent-boundaries",
            parent.owner_mode().to_raw() == i32::MIN
                && parent.window_state().to_raw() == i32::MAX
                && parent.topmost()
                && parent.id() == "numeric-parent",
        );
        let window =
            from_markup_value::<Rc<dyn IDockWindow>>(&Some(load("CoreNumericWindowRoot"))).unwrap();
        check(
            "numeric-enum-window-values",
            window.owner_mode().to_raw() == 123 && window.window_state().to_raw() == -17,
        );
        options.apply_to(Some(window.as_ref()));
        check(
            "numeric-enum-original-apply-parent",
            Rc::ptr_eq(&window.parent_window().unwrap(), &parent)
                && window.owner_mode().to_raw() == -19
                && window.is_modal(),
        );
        let settings = from_markup_value::<Rc<ferrodock_settings::DockSettingsOptions>>(&Some(
            load("CoreNumericSettingsOptionsRoot"),
        ))
        .unwrap();
        check(
            "numeric-enum-nullable-option",
            settings.default_floating_window_owner_mode()
                == Some(DockWindowOwnerMode::from_raw(-19)),
        );
        let positive = from_markup_value::<Rc<DockWindowOptions>>(&Some(load(
            "CoreNumericPositiveOverflowRoot",
        )))
        .unwrap();
        let negative = from_markup_value::<Rc<DockWindowOptions>>(&Some(load(
            "CoreNumericNegativeOverflowRoot",
        )))
        .unwrap();
        check(
            "numeric-enum-positive-overflow-narrowing",
            positive.owner_mode().to_raw() == i32::MIN,
        );
        check(
            "numeric-enum-negative-overflow-narrowing",
            negative.owner_mode().to_raw() == i32::MAX,
        );
        let root = from_markup_value::<Rc<dyn ferrodock_model::controls::IRootDock>>(&Some(load(
            "CoreNumericDockEnumsRoot",
        )))
        .unwrap();
        check(
            "numeric-dock-enum-original-model-values",
            root.dock().to_raw() == -19
                && root.pinned_dock_display_mode().to_raw() == 123
                && root.pinned_dock_display_mode_override().map(|v| v.to_raw()) == Some(i32::MIN)
                && root.floating_window_host_mode().to_raw() == -17,
        );
        let settings = from_markup_value::<Rc<ferrodock_settings::DockSettingsOptions>>(&Some(
            load("CoreNumericHostSettingsRoot"),
        ))
        .unwrap();
        check(
            "numeric-dock-enum-nullable-host-option",
            settings.floating_window_host_mode().map(|v| v.to_raw()) == Some(-19),
        );
    });
}
#[test]
fn runtime_emitter_uses_explicit_factory_with_source_narrowing_and_no_guessed_fallback() {
    run(|| {
        let config = ferroui_markup_xaml::RuntimeXamlLoaderConfiguration::default();
        let documents = [
            (
                "Options.xaml",
                include_str!("CoreNumericWindowOptionsRoot.xaml"),
            ),
            ("Window.xaml", include_str!("CoreNumericWindowRoot.xaml")),
            (
                "Settings.xaml",
                include_str!("CoreNumericSettingsOptionsRoot.xaml"),
            ),
            (
                "DockEnums.xaml",
                include_str!("CoreNumericDockEnumsRoot.xaml"),
            ),
            (
                "HostSettings.xaml",
                include_str!("CoreNumericHostSettingsRoot.xaml"),
            ),
        ];
        let result = ferroui_markup_xaml_loader::rust_emitter::compile_documents(
            &documents,
            Some("ferres://ferrodock_markup_tests/"),
            &config,
            &[],
        );
        for output in &result {
            assert!(output.source.is_ok(), "{:?}", output.source);
        }
        let source = result[0].source.as_ref().unwrap();
        assert!(
            source.contains("DockWindowOwnerModeMetadata::__markup_enum_i32(-19_i32)"),
            "{source}"
        );
        assert!(
            source.contains("DockWindowOwnerModeMetadata::__markup_enum_i32(-2147483648_i32)"),
            "{source}"
        );
        assert!(
            source.contains("DockWindowStateMetadata::__markup_enum_i32(2147483647_i32)"),
            "{source}"
        );
        let source = result[2].source.as_ref().unwrap();
        assert!(source.contains("__markup_enum_i32(-19_i32)"), "{source}");
        let source = result[3].source.as_ref().unwrap();
        for expected in [
            "DockModeMetadata::__markup_enum_i32(-19_i32)",
            "PinnedDockDisplayModeMetadata::__markup_enum_i32(123_i32)",
            "PinnedDockDisplayModeMetadata::__markup_enum_i32(-2147483648_i32)",
            "DockFloatingWindowHostModeMetadata::__markup_enum_i32(-17_i32)",
        ] {
            assert!(source.contains(expected), "{source}");
        }
        for (value, expected) in [("2147483648", i32::MIN), ("-2147483649", i32::MAX)] {
            let document=format!("<core:DockWindowOptions xmlns:core=\"clr-namespace:FerroDock.Model.Core;assembly=FerroDock.Model\" OwnerMode=\"{value}\" />");
            let output = ferroui_markup_xaml_loader::rust_emitter::compile_documents(
                &[("Outside.xaml", &document)],
                None,
                &config,
                &[],
            );
            let source = output[0].source.as_ref().unwrap();
            assert!(
                source.contains(&format!("__markup_enum_i32({expected}_i32)")),
                "{value}: {source}"
            );
        }
        let document="<model:Document xmlns:model=\"clr-namespace:FerroDock.Model.ReactiveUI.Controls;assembly=FerroDock.Model.ReactiveUI\" MdiState=\"123\" />";
        let output = ferroui_markup_xaml_loader::rust_emitter::compile_documents(
            &[("Closed.xaml", document)],
            None,
            &config,
            &[],
        );
        assert!(
            output[0].source.is_err(),
            "closed enum has no declared factory"
        );
    });
}
