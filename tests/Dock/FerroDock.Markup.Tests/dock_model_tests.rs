// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Compiled inherited/own dock bindings and the original engine command through native UI.
use ferrodock_model::{collections::ListHandle, controls::IDocumentDock, core::*};
use ferrodock_model_reactiveui::controls::{Document, DocumentDock};
use ferroui_base::{
    controls::ResourceKey,
    threading::{CancellationToken, Dispatcher},
    Ref,
};
use ferroui_controls::{
    automation::{peers::ControlAutomationPeer, provider::IInvokeProvider},
    AppBuilder, Application, Button, CheckBox, Control, StackPanel, TextBlock, TextBox, Window,
};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use std::rc::Rc;
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>()
        .use_headless(FerroHeadlessPlatformOptions::default())
        .after_setup(|_| {
            Application::current()
                .unwrap()
                .styles()
                .add(ferroui_themes_fluent::FluentTheme::new().as_style())
        })
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "dock-model-consumer",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
fn text(panel: &StackPanel, name: &str) -> Ref<TextBlock> {
    panel
        .find_descendant_of_type_where::<TextBlock>(false, |t| t.name().as_deref() == Some(name))
        .unwrap()
}
#[test]
fn compiled_dock_inheritance_live_count_two_way_fields_policy_resources_and_original_commands() {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            || {
                ferrodock_ferro::register_types();
                ferroui_markup_xaml::register_types();
                crate::compiled_markup::register();
                let value = crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferrodock_markup_tests/DockModelConsumer.xaml",
                )
                .unwrap()
                .unwrap();
                let panel = value.downcast_ref::<Ref<StackPanel>>().unwrap();
                let policy = ferroui_base::metadata::from_markup_value::<
                    Rc<dyn DockCapabilityPolicyImpl>,
                >(&panel.find_resource(&ResourceKey::from("Policy")))
                .unwrap();
                assert_eq!(policy.can_close(), Some(false));
                assert_eq!(policy.can_drop(), Some(true));
                let overrides =
                    ferroui_base::metadata::from_markup_value::<Rc<DockCapabilityOverrides>>(
                        &panel.find_resource(&ResourceKey::from("Overrides")),
                    )
                    .unwrap();
                assert_eq!(overrides.can_drag(), Some(false));
                assert!(overrides.has_any_override());
                assert_eq!(
                    panel
                        .find_resource(&ResourceKey::from("UnsetAlignment"))
                        .unwrap()
                        .downcast_ref::<Alignment>(),
                    Some(&Alignment::Unset)
                );
                let dock = DocumentDock::new();
                dock.set_title("Source dock".into());
                dock.set_layout_mode(DocumentLayoutMode::Mdi);
                dock.set_visible_bounds(0.0, 0.0, 800.0, 600.0);
                let first = Document::new();
                first.set_title("First".into());
                first.set_mdi_bounds(DockRect::new(200.0, 300.0, 200.0, 160.0));
                let list: ListHandle<Option<Rc<dyn IDockable>>> =
                    ListHandle::from_items(vec![Some(first.clone())]);
                dock.set_visible_dockables(Some(list.clone()));
                dock.set_active_dockable(Some(first.clone()));
                let model: Rc<dyn IDocumentDock> = dock.clone();
                panel.set_data_context(Some(Rc::new(model)));
                let window = Window::new();
                window.set_width(600.0);
                window.set_height(500.0);
                window.set_content(Some(Control::boxed(panel.clone())));
                window.show();
                Dispatcher::ui_thread().run_jobs(None);
                window.update_layout();
                assert_eq!(text(panel, "Title").text().as_deref(), Some("Source dock"));
                assert_eq!(text(panel, "Count").text().as_deref(), Some("1"));
                assert_eq!(text(panel, "First").text().as_deref(), Some("First"));
                assert_eq!(text(panel, "Active").text().as_deref(), Some("First"));
                let second = Document::new();
                second.set_title("Second".into());
                list.add(Some(second.clone()));
                assert_eq!(text(panel, "Count").text().as_deref(), Some("2"));
                list.set(0, Some(second.clone()));
                // Characterize the published runtime gap rather than altering IList's contract:
                // compiled paths on IList use the native INPC accessor, whose indexer name
                // is Item; System ObservableCollection emits the original Item[].
                assert_eq!(text(panel, "First").text().as_deref(), Some("First"));
                first.set_title("Old item still subscribed".into());
                assert_eq!(
                    text(panel, "First").text().as_deref(),
                    Some("Old item still subscribed")
                );
                second.set_title("Live second".into());
                dock.set_active_dockable(Some(second));
                assert_eq!(text(panel, "Active").text().as_deref(), Some("Live second"));
                let editor = panel.find_descendant_of_type::<TextBox>(false).unwrap();
                editor.set_text(Some("Edited empty"));
                assert_eq!(
                    ferrodock_controls_recycling::from_recycling_value(
                        &dock.empty_content().unwrap()
                    )
                    .downcast_ref::<String>()
                    .map(String::as_str),
                    Some("Edited empty")
                );
                editor.set_text(None);
                assert!(dock.empty_content().is_none());
                dock.set_empty_content(Some(Rc::new("Source empty".to_owned())));
                assert_eq!(editor.text().as_deref(), Some("Source empty"));
                let create = panel.find_descendant_of_type::<CheckBox>(false).unwrap();
                create.set_is_checked(Some(true));
                assert!(dock.can_create_document());
                dock.set_can_create_document(false);
                assert_eq!(create.is_checked(), Some(false));
                let replacement: ListHandle<Option<Rc<dyn IDockable>>> =
                    ListHandle::from_items(vec![Some(first.clone())]);
                dock.set_visible_dockables(Some(replacement));
                assert_eq!(text(panel, "Count").text().as_deref(), Some("1"));
                list.clear();
                assert_eq!(text(panel, "Count").text().as_deref(), Some("1"));
                first.set_title("Replacement path is live".into());
                assert_eq!(
                    text(panel, "First").text().as_deref(),
                    Some("Replacement path is live")
                );
                let cascade = panel
                    .find_descendant_of_type_where::<Button>(false, |button| {
                        button.name().as_deref() == Some("Cascade")
                    })
                    .unwrap();
                ControlAutomationPeer::create_peer_for_element(&cascade)
                    .get_provider::<dyn IInvokeProvider>()
                    .unwrap()
                    .invoke()
                    .unwrap();
                assert_ne!(
                    first.mdi_bounds(),
                    DockRect::new(200.0, 300.0, 200.0, 160.0)
                );
                assert_eq!(first.mdi_bounds().x, 0.0);
                assert_eq!(first.mdi_bounds().y, 0.0);
                window.close();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
