//! Native classes and original model operations across the application build boundary.
use ferrodock_ferro::{controls::*, model_projection};
use ferrodock_model::core::{IDockable, SplitViewDisplayMode};
use ferrodock_model_reactiveui::{
    controls::{Document, GridDock, SplitViewDock},
    Factory,
};
use ferroui_base::{
    threading::{CancellationToken, Dispatcher},
    Ref,
};
use ferroui_controls::{
    AppBuilder, Application, Control, Grid, ItemsControl, Panel, SplitView, Window,
};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use std::rc::Rc;
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "ferrodock-container-consumer",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
#[test]
fn compiled_consumer_loads_original_container_types_and_connects_original_grid_split_and_content_models(
) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            || {
                ferrodock_ferro::register_types();
                ferroui_markup_xaml::register_types();
                crate::compiled_markup::register();
                let value = crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferrodock_markup_tests/ContainerControlsConsumer.xaml",
                )
                .unwrap()
                .unwrap();
                let panel = value.downcast_ref::<Ref<Panel>>().unwrap();
                assert_eq!(panel.children().count(), 11);
                let names = [
                    "DocumentDockControl",
                    "DockDockControl",
                    "StackDockControl",
                    "WrapDockControl",
                    "UniformGridDockControl",
                    "ProportionalDockControl",
                    "ToolDockControl",
                    "DocumentContentControl",
                    "ToolContentControl",
                    "SplitViewDockControl",
                    "GridDockControl",
                ];
                for (index, name) in names.iter().enumerate() {
                    assert_eq!(panel.children().get(index).get_type().name(), *name);
                }
                let factory = Factory::new();
                let document = Document::new();
                document.set_factory(Some(factory.clone()));
                let content = panel
                    .children()
                    .get(7)
                    .cast::<DocumentContentControl>()
                    .unwrap();
                content
                    .set_data_context(Some(model_projection::to_dockable_value(document.clone())));
                let split = panel
                    .children()
                    .get(9)
                    .cast::<SplitViewDockControl>()
                    .unwrap();
                let split_model = SplitViewDock::new();
                split_model.set_display_mode(SplitViewDisplayMode::Overlay);
                split.set_data_context(Some(model_projection::to_dockable_value(
                    split_model.clone(),
                )));
                let grid = panel.children().get(10).cast::<GridDockControl>().unwrap();
                let grid_model = GridDock::new();
                grid_model.set_column_definitions(Some(String::from("Auto,2*")));
                grid_model.set_row_definitions(Some(String::from("30,*")));
                grid.set_data_context(Some(model_projection::to_dockable_value(grid_model)));
                let window = Window::new();
                window.set_width(640.0);
                window.set_height(480.0);
                window.set_content(Some(Control::boxed(panel.clone())));
                window.show();
                Dispatcher::ui_thread().run_jobs(None);
                window.update_layout();
                let native_split = split.visual_children().get(0).cast::<SplitView>().unwrap();
                native_split.set_is_pane_open(true);
                assert!(split_model.is_pane_open());
                native_split.set_is_pane_open(false);
                assert!(native_split.is_pane_open());
                assert!(split_model.is_pane_open());
                split_model.set_use_light_dismiss_overlay_mode(true);
                native_split.set_is_pane_open(false);
                assert!(!split_model.is_pane_open());
                let items = grid
                    .visual_children()
                    .get(0)
                    .cast::<ItemsControl>()
                    .unwrap();
                let native_grid = items.items_panel_root().unwrap().cast::<Grid>().unwrap();
                assert_eq!(native_grid.column_definitions().to_string(), "Auto,2*");
                assert_eq!(native_grid.row_definitions().to_string(), "30,1*");
                assert!(factory
                    .document_controls()
                    .borrow()
                    .contains_key(&(document.clone() as Rc<dyn IDockable>)));
                window.close();
                assert!(!factory
                    .document_controls()
                    .borrow()
                    .contains_key(&(document as Rc<dyn IDockable>)));
            },
            CancellationToken::none(),
        )
        .wait()
        .expect("native container consumer completed");
}
