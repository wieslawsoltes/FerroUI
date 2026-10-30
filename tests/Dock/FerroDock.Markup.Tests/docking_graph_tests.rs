// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Original compiled layout dictionaries and real engine/control graph integration.
mod composed_drag_tests;
mod persisted_layout_tests;
use ferrodock_ferro::controls::{
    DockControl, DocumentControl, DocumentTabStrip, PinnedDockControl, PinnedDockHostPanel,
    ProportionalDockControl, RootDockControl, ToolControl, ToolTabStrip,
};
use ferrodock_model::{collections::ListHandle, core::*};
use ferrodock_model_reactiveui::{
    controls::{
        Document, DocumentDock, ProportionalDock, ProportionalDockSplitter, RootDock, Tool,
        ToolDock,
    },
    Factory,
};
use ferroui_base::{
    threading::{CancellationToken, Dispatcher},
    Ref,
};
use ferroui_controls::{AppBuilder, Application, Grid, GridUnitType, Panel, Window};
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
    name: "original-docking-dictionaries",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
fn run(f: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            move || {
                ferrodock_ferro_themes_fluent::register_types();
                ferroui_markup_xaml::register_types();
                crate::compiled_markup::register();
                f();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
fn drain(window: &Window) {
    for _ in 0..4 {
        Dispatcher::ui_thread().run_jobs(None);
        window.update_layout();
    }
}
struct Scene {
    window: Ref<Window>,
    control: Ref<DockControl>,
    root: Rc<RootDock>,
    documents: Rc<DocumentDock>,
    tools: Rc<ToolDock>,
    factory: Rc<Factory>,
}
impl Scene {
    fn new(pinned: bool) -> Self {
        Self::from_document(pinned, "DockingGraphConsumer.xaml")
    }
    fn from_document(pinned: bool, document: &str) -> Self {
        let value = crate::compiled_xaml::try_load(
            None,
            &format!("ferres://ferrodock_markup_tests/{document}"),
        )
        .unwrap()
        .unwrap();
        let panel = value.downcast_ref::<Ref<Panel>>().unwrap();
        let control = panel.find_descendant_of_type::<DockControl>(false).unwrap();
        let factory = Factory::new();
        let documents = DocumentDock::new();
        documents.set_id("Documents".into());
        let docs = ListHandle::<Option<Rc<dyn IDockable>>>::new();
        for id in ["Doc1", "Doc2"] {
            let doc = Document::new();
            doc.set_id(id.into());
            doc.set_title(id.into());
            docs.add(Some(doc));
        }
        documents.set_visible_dockables(Some(docs.clone()));
        documents.set_active_dockable(docs.get(0));
        let tools = ToolDock::new();
        tools.set_id("Tools".into());
        let items = ListHandle::<Option<Rc<dyn IDockable>>>::new();
        for id in ["Tool1", "Tool2"] {
            let tool = Tool::new();
            tool.set_id(id.into());
            tool.set_title(id.into());
            items.add(Some(tool));
        }
        tools.set_visible_dockables(Some(items.clone()));
        tools.set_active_dockable(items.get(0));
        let proportional = ProportionalDock::new();
        proportional.set_id("Split".into());
        proportional.set_visible_dockables(Some(ListHandle::from_items(vec![
            Some(tools.clone() as Rc<dyn IDockable>),
            Some(ProportionalDockSplitter::new()),
            Some(documents.clone()),
        ])));
        proportional.set_active_dockable(Some(documents.clone()));
        let root = RootDock::new();
        root.set_id("Root".into());
        root.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
            proportional.clone() as Rc<dyn IDockable>,
        )])));
        root.set_active_dockable(Some(proportional));
        if pinned {
            let dock = ToolDock::new();
            dock.set_alignment(Alignment::Right);
            dock.set_is_empty(false);
            let tool = Tool::new();
            tool.set_id("Pinned".into());
            tool.set_pinned_bounds(0.0, 0.0, 210.0, 150.0);
            tool.set_pinned_dock_display_mode_override(Some(PinnedDockDisplayMode::Inline));
            dock.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
                tool.clone() as Rc<dyn IDockable>
            )])));
            dock.set_active_dockable(Some(tool));
            root.set_pinned_dock(Some(dock));
        }
        control.set_factory(Some(factory.clone()));
        control.set_initialize_layout(true);
        control.set_layout(Some(root.clone()));
        let window = Window::new();
        window.set_width(1000.0);
        window.set_height(700.0);
        window.set_content(Some(Rc::new(panel.clone())));
        window.show();
        drain(&window);
        Self {
            window,
            control,
            root,
            documents,
            tools,
            factory,
        }
    }
}
impl Drop for Scene {
    fn drop(&mut self) {
        self.window.close();
    }
}
#[test]
fn original_control_and_factory_realize_root_split_document_and_tool_graph_and_live_selection() {
    run(|| {
        let scene = Scene::new(false);
        assert!(scene
            .control
            .find_descendant_of_type::<RootDockControl>(false)
            .is_some());
        assert!(scene
            .control
            .find_descendant_of_type::<ProportionalDockControl>(false)
            .is_some());
        let documents = scene
            .control
            .find_descendant_of_type::<DocumentControl>(false)
            .expect("original document host");
        let tools = scene
            .control
            .find_descendant_of_type::<ToolControl>(false)
            .expect("original tool host");
        let document_tabs = documents
            .find_descendant_of_type::<DocumentTabStrip>(false)
            .unwrap();
        let tool_tabs = tools
            .find_descendant_of_type::<ToolTabStrip>(false)
            .unwrap();
        assert_eq!(document_tabs.items().count(), 2);
        assert_eq!(tool_tabs.items().count(), 2);
        assert!(documents.has_visible_dockables());
        let second = scene.documents.visible_dockables().unwrap().get(1);
        scene.factory.set_active_dockable(second.clone());
        drain(&scene.window);
        assert!(Rc::ptr_eq(
            &scene.documents.active_dockable().unwrap(),
            (second).as_ref().expect("nonnull identity fixture")
        ));
        assert_eq!(document_tabs.selected_index(), 1);
        assert!(scene.root.factory().is_some());
        assert_eq!(scene.tools.visible_dockables().unwrap().count(), 2);
    });
}
#[test]
fn original_root_pinned_template_uses_actual_host_panel_and_per_tool_inline_override() {
    run(|| {
        let scene = Scene::new(true);
        let host = scene
            .control
            .find_descendant_of_type::<PinnedDockHostPanel>(false)
            .unwrap();
        let pinned = host
            .find_descendant_of_type::<PinnedDockControl>(false)
            .unwrap();
        assert_eq!(pinned.pinned_dock_alignment(), Alignment::Right);
        let grid = pinned
            .find_descendant_of_type_where::<Grid>(false, |g| {
                g.name().as_deref() == Some("PART_PinnedDockGrid")
            })
            .unwrap();
        assert_eq!(grid.column_definitions().count(), 2);
        assert_eq!(
            grid.column_definitions().get(1).width().grid_unit_type(),
            GridUnitType::Pixel
        );
        assert_eq!(grid.column_definitions().get(1).width().value(), 210.0);
    });
}

#[test]
fn original_switch_inline_pinned_dockables_rebuilds_grid_after_alignment_change() {
    run(|| {
        let scene = Scene::new(true);
        let old = scene.root.pinned_dock().unwrap();
        old.set_alignment(Alignment::Left);
        drain(&scene.window);
        let control = scene
            .control
            .find_descendant_of_type::<PinnedDockControl>(false)
            .unwrap();
        let grid = control
            .find_descendant_of_type_where::<Grid>(false, |g| {
                g.name().as_deref() == Some("PART_PinnedDockGrid")
            })
            .unwrap();
        assert_eq!(grid.column_definitions().count(), 2);
        old.visible_dockables().unwrap().clear();
        old.set_is_empty(true);
        scene.root.set_pinned_dock(None);
        let right = ToolDock::new();
        right.set_alignment(Alignment::Right);
        right.set_is_empty(true);
        right.set_visible_dockables(Some(ListHandle::new()));
        scene.root.set_pinned_dock(Some(right.clone()));
        let tool = Tool::new();
        tool.set_pinned_dock_display_mode_override(Some(PinnedDockDisplayMode::Inline));
        right.visible_dockables().unwrap().add(Some(tool.clone()));
        right.set_active_dockable(Some(tool.clone()));
        tool.set_pinned_bounds(0.0, 0.0, 200.0, 150.0);
        right.set_is_empty(false);
        drain(&scene.window);
        assert_eq!(control.pinned_dock_alignment(), Alignment::Right);
        assert_eq!(grid.column_definitions().count(), 2);
        assert_eq!(grid.column_definitions().get(1).width().value(), 200.0);
    });
}

#[test]
fn full_source_theme_realizes_cached_documents_and_keeps_both_views_across_selection() {
    run(|| {
        use ferrodock_ferro::controls::DockableControl;
        use ferroui_controls::ItemsControl;
        let scene = Scene::from_document(true, "FullDockingGraphConsumer.xaml");
        let documents = scene
            .control
            .find_descendant_of_type::<DocumentControl>(false)
            .unwrap();
        let host = documents
            .find_descendant_of_type_where::<ItemsControl>(false, |control| {
                control.name().as_deref() == Some("PART_CachedContentHost")
            })
            .unwrap();
        assert_eq!(host.items().count(), 2);
        let first = host.container_from_index(0).unwrap();
        let second = host.container_from_index(1).unwrap();
        let first_view = first
            .find_descendant_of_type::<DockableControl>(false)
            .unwrap();
        let second_view = second
            .find_descendant_of_type::<DockableControl>(false)
            .unwrap();
        assert!(first_view.is_visible());
        assert!(!second_view.is_visible());
        let next = scene.documents.visible_dockables().unwrap().get(1);
        scene.factory.set_active_dockable(next);
        drain(&scene.window);
        assert!(!first_view.is_visible());
        assert!(second_view.is_visible());
        assert!(host.container_from_index(0).unwrap().ptr_eq(&first));
        assert!(host.container_from_index(1).unwrap().ptr_eq(&second));
        scene
            .factory
            .set_active_dockable(scene.documents.visible_dockables().unwrap().get(0));
        drain(&scene.window);
        assert!(first_view.is_visible());
        assert!(!second_view.is_visible());
        assert!(scene
            .control
            .find_descendant_of_type::<PinnedDockControl>(false)
            .is_some());
    });
}

#[path = "simple_theme_graph_tests.rs"]
mod simple_theme_graph_tests;

#[path = "browser_theme_graph_tests.rs"]
mod browser_theme_graph_tests;
