// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Actual host, tab and pinned control metadata across the library build boundary.
use ferrodock_ferro::controls::{
    DockControl, DocumentTabStrip, DocumentTabStripItem, HostWindow, ToolChromeControl,
    ToolPinItemControl, ToolPinnedControl, ToolTabStrip, ToolTabStripItem,
};
use ferroui_base::{layout::Orientation, threading::CancellationToken, Ref};
use ferroui_controls::{AppBuilder, Application, Panel};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "host-control-consumer",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
#[test]
fn downstream_compiler_constructs_actual_host_tab_and_pinned_controls_and_source_properties() {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            || {
                ferrodock_ferro::register_types();
                ferroui_markup_xaml::register_types();
                crate::compiled_markup::register();
                let value = crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferrodock_markup_tests/HostControlConsumer.xaml",
                )
                .unwrap()
                .unwrap();
                let panel = value.downcast_ref::<Ref<Panel>>().unwrap();
                let dock = panel.find_descendant_of_type::<DockControl>(false).unwrap();
                assert!(!dock.initialize_layout());
                assert!(!dock.initialize_factory());
                assert!(!dock.enable_managed_window_layer());
                assert!(!dock.is_docking_enabled());
                let documents = panel
                    .find_descendant_of_type::<DocumentTabStrip>(false)
                    .unwrap();
                assert_eq!(documents.orientation(), Orientation::Vertical);
                assert!(documents.can_create_item());
                assert!(!documents.enable_window_drag());
                let document = panel
                    .find_descendant_of_type::<DocumentTabStripItem>(false)
                    .unwrap();
                assert!(document.is_active());
                assert_eq!(document.width(), 24.0);
                let tools = panel
                    .find_descendant_of_type::<ToolTabStrip>(false)
                    .unwrap();
                assert!(tools.can_create_item());
                assert_eq!(
                    tools.mouse_wheel_scroll_orientation(),
                    Orientation::Vertical
                );
                assert!(panel
                    .find_descendant_of_type::<ToolTabStripItem>(false)
                    .unwrap()
                    .is_selected());
                let chrome = panel
                    .find_descendant_of_type::<ToolChromeControl>(false)
                    .unwrap();
                assert_eq!(chrome.title().as_deref(), Some("Source chrome"));
                assert!(chrome.is_pinned());
                assert!(chrome.is_maximized());
                assert_eq!(
                    panel
                        .find_descendant_of_type::<ToolPinItemControl>(false)
                        .unwrap()
                        .orientation(),
                    Orientation::Vertical
                );
                assert_eq!(
                    panel
                        .find_descendant_of_type::<ToolPinnedControl>(false)
                        .unwrap()
                        .orientation(),
                    Orientation::Vertical
                );
                let value = crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferrodock_markup_tests/HostWindowConsumer.xaml",
                )
                .unwrap()
                .unwrap();
                let host = value.downcast_ref::<Ref<HostWindow>>().unwrap();
                assert_eq!(host.title().as_deref(), Some("Source host"));
                assert_eq!(host.width(), 640.0);
                assert_eq!(host.height(), 480.0);
                host.close();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
