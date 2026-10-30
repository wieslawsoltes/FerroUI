// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Complete original tab and pinned dictionaries with their actual native controls.
use ferrodock_ferro::{
    controls::{
        DockableControl, DocumentTabStrip, DocumentTabStripItem, ToolPinItemControl,
        ToolPinnedControl, ToolTabStrip, ToolTabStripItem,
    },
    model_projection,
};
use ferrodock_model::core::{IDockable, IFactory, TrackingMode};
use ferrodock_model_reactiveui::{
    controls::{Document, DocumentDock, RootDock, Tool, ToolDock},
    Factory,
};
use ferroui_base::{
    controls::ResourceKey,
    styling::ControlTheme,
    threading::{CancellationToken, Dispatcher},
    Ref,
};
use ferroui_controls::{AppBuilder, Application, Control, StackPanel, Window};
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
    name: "original-tab-theme-consumer",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
fn run(test: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            move || {
                ferrodock_ferro_themes_fluent::register_types();
                ferroui_markup_xaml::register_types();
                crate::compiled_markup::register();
                test();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
#[test]
fn complete_source_tab_and_pinned_templates_realize_native_containers_and_tracking_panels() {
    run(|| {
        let value = crate::compiled_xaml::try_load(
            None,
            "ferres://ferrodock_markup_tests/TabThemeConsumer.xaml",
        )
        .unwrap()
        .unwrap();
        let panel = value.downcast_ref::<Ref<StackPanel>>().unwrap();
        let documents = panel
            .find_descendant_of_type::<DocumentTabStrip>(false)
            .unwrap();
        let tools = panel
            .find_descendant_of_type::<ToolTabStrip>(false)
            .unwrap();
        let pinned = panel
            .find_descendant_of_type::<ToolPinnedControl>(false)
            .unwrap();
        let factory = Factory::new();
        let document_dock = DocumentDock::new();
        let tool_dock = ToolDock::new();
        let root = RootDock::new();
        let document = Document::new();
        document.set_title("Document source".into());
        document.set_factory(Some(factory.clone()));
        document.set_owner(Some(document_dock.clone()));
        let tool = Tool::new();
        tool.set_title("Tool source".into());
        tool.set_factory(Some(factory.clone()));
        tool.set_owner(Some(tool_dock.clone()));
        documents.set_data_context(Some(model_projection::to_dockable_value(document_dock)));
        tools.set_data_context(Some(model_projection::to_dockable_value(tool_dock)));
        pinned.set_data_context(Some(model_projection::to_dockable_value(root)));
        documents
            .items()
            .add(Some(model_projection::to_dockable_value(document.clone())));
        tools
            .items()
            .add(Some(model_projection::to_dockable_value(tool.clone())));
        tools
            .items()
            .add(Some(model_projection::to_dockable_value(Tool::new())));
        pinned
            .items()
            .add(Some(model_projection::to_dockable_value(tool.clone())));
        for (control, ty) in [
            (
                documents.clone().upcast::<Control>(),
                DocumentTabStrip::TYPE,
            ),
            (tools.clone().upcast(), ToolTabStrip::TYPE),
            (pinned.clone().upcast(), ToolPinnedControl::TYPE),
        ] {
            let theme = ferroui_base::metadata::from_markup_value::<Ref<ControlTheme>>(
                &control.find_resource(&ResourceKey::from(ty)),
            )
            .unwrap();
            control.set_theme(Some(theme));
        }
        let window = Window::new();
        window.set_width(800.0);
        window.set_height(400.0);
        window.set_content(Some(Rc::new(panel.clone())));
        window.show();
        window.update_layout();
        Dispatcher::ui_thread().run_jobs(None);
        window.update_layout();
        let doc_item = documents
            .container_from_index(0)
            .unwrap()
            .cast::<DocumentTabStripItem>()
            .unwrap();
        let tool_item = tools
            .container_from_index(0)
            .unwrap()
            .cast::<ToolTabStripItem>()
            .unwrap();
        let pin_item = pinned
            .container_from_index(0)
            .unwrap()
            .cast::<ToolPinItemControl>()
            .unwrap();
        for (item, ty) in [
            (
                doc_item.clone().upcast::<Control>(),
                DocumentTabStripItem::TYPE,
            ),
            (tool_item.clone().upcast(), ToolTabStripItem::TYPE),
            (pin_item.clone().upcast(), ToolPinItemControl::TYPE),
        ] {
            let theme = ferroui_base::metadata::from_markup_value::<Ref<ControlTheme>>(
                &item.find_resource(&ResourceKey::from(ty)),
            )
            .unwrap();
            item.set_theme(Some(theme));
            item.apply_template();
        }
        window.update_layout();
        let tracking = doc_item
            .find_descendant_of_type::<DockableControl>(false)
            .unwrap();
        assert_eq!(tracking.tracking_mode(), TrackingMode::Tab);
        assert_eq!(
            tool_item
                .find_descendant_of_type::<DockableControl>(false)
                .unwrap()
                .tracking_mode(),
            TrackingMode::Tab
        );
        assert_eq!(
            pin_item
                .find_descendant_of_type::<DockableControl>(false)
                .unwrap()
                .tracking_mode(),
            TrackingMode::Pinned
        );
        let document_key: Rc<dyn IDockable> = document;
        assert!(IFactory::tab_dockable_controls(&*factory)
            .borrow()
            .contains_key(&document_key));
        window.close();
        assert!(IFactory::tab_dockable_controls(&*factory)
            .borrow()
            .is_empty());
    });
}

struct DocumentScene {
    window: Ref<Window>,
    control: Ref<ferrodock_ferro::controls::DocumentControl>,
    dock: Rc<DocumentDock>,
    _factory: Rc<Factory>,
}
impl DocumentScene {
    fn new(
        with_document: bool,
        template: Option<Rc<dyn ferroui_controls::templates::IDataTemplate>>,
    ) -> Self {
        use ferrodock_model::{collections::ListHandle, core::IDock};
        let value = crate::compiled_xaml::try_load(
            None,
            "ferres://ferrodock_markup_tests/TabThemeConsumer.xaml",
        )
        .unwrap()
        .unwrap();
        let panel = value.downcast_ref::<Ref<StackPanel>>().unwrap();
        panel.children().clear();
        let factory = Factory::new();
        let dock = DocumentDock::new();
        dock.set_factory(Some(factory.clone()));
        dock.set_visible_dockables(Some(ListHandle::new()));
        dock.set_empty_content(Some(Rc::new("No documents are open.".to_owned())));
        if with_document {
            let doc = Document::new();
            doc.set_title("Document 1".into());
            doc.set_owner(Some(dock.clone()));
            dock.visible_dockables().unwrap().add(Some(doc.clone()));
            dock.set_active_dockable(Some(doc));
        }
        let control = ferrodock_ferro::controls::DocumentControl::new();
        control.set_empty_content_template(template);
        control.set_data_context(Some(model_projection::to_dockable_value(dock.clone())));
        panel.children().add(control.clone().upcast::<Control>());
        let window = Window::new();
        window.set_width(800.0);
        window.set_height(600.0);
        window.set_content(Some(Rc::new(panel.clone())));
        window.show();
        control.apply_template();
        window.update_layout();
        control.update_layout();
        Self {
            window,
            control,
            dock,
            _factory: factory,
        }
    }
    fn empty(&self) -> Ref<ferroui_controls::ContentControl> {
        self.control
            .find_descendant_of_type_where::<ferroui_controls::ContentControl>(false, |c| {
                c.name().as_deref() == Some("PART_EmptyContentHost")
            })
            .expect("complete original document template")
    }
}
impl Drop for DocumentScene {
    fn drop(&mut self) {
        self.window.close();
    }
}
#[test]
fn original_document_host_shows_empty_content_when_no_documents() {
    run(|| {
        let scene = DocumentScene::new(false, None);
        let empty = scene.empty();
        assert!(empty.is_visible());
        assert_eq!(
            empty
                .content()
                .unwrap()
                .downcast_ref::<String>()
                .map(String::as_str),
            Some("No documents are open.")
        );
        assert!(!scene.control.has_visible_dockables());
    });
}
#[test]
fn original_document_host_hides_empty_content_when_documents_exist() {
    run(|| {
        let scene = DocumentScene::new(true, None);
        assert!(!scene.empty().is_visible());
        assert!(scene.control.has_visible_dockables());
    });
}
#[test]
fn original_document_host_updates_empty_content_on_actual_list_add_remove() {
    run(|| {
        use ferrodock_model::core::IDock;
        let scene = DocumentScene::new(false, None);
        let empty = scene.empty();
        assert!(empty.is_visible());
        let document = Document::new();
        document.set_owner(Some(scene.dock.clone()));
        scene
            .dock
            .visible_dockables()
            .unwrap()
            .add(Some(document.clone()));
        scene.dock.set_active_dockable(Some(document.clone()));
        scene.window.update_layout();
        assert!(!empty.is_visible());
        assert!(scene.control.has_visible_dockables());
        scene
            .dock
            .visible_dockables()
            .unwrap()
            .remove(&Some((document as Rc<dyn IDockable>).clone()));
        scene.dock.set_active_dockable(None);
        scene.window.update_layout();
        assert!(empty.is_visible());
        assert!(!scene.control.has_visible_dockables());
    });
}
#[test]
fn original_document_host_applies_empty_content_template() {
    run(|| {
        use ferroui_controls::{
            templates::{FuncDataTemplate, IDataTemplate},
            TextBlock,
        };
        let template = FuncDataTemplate::for_type::<String>(
            |text, _| {
                let control = TextBlock::new();
                control.set_text(Some(&format!("Template: {text}")));
                Some(control.upcast())
            },
            true,
        );
        let scene = DocumentScene::new(false, Some(template.clone()));
        let empty = scene.empty();
        let template: Rc<dyn IDataTemplate> = template;
        assert!(Rc::ptr_eq(&empty.content_template().unwrap(), &template));
        assert!(empty
            .find_descendant_of_type_where::<TextBlock>(false, |text| text.text().as_deref()
                == Some("Template: No documents are open."))
            .is_some());
    });
}
#[test]
fn original_tool_dock_template_realizes_chrome_grip_buttons_and_content_host_with_model_bindings() {
    run(|| {
        use ferrodock_ferro::controls::{ToolChromeControl, ToolControl, ToolDockControl};
        use ferrodock_model::{collections::ListHandle, core::IDock};
        use ferroui_controls::{Button, TextBlock};
        let value = crate::compiled_xaml::try_load(
            None,
            "ferres://ferrodock_markup_tests/TabThemeConsumer.xaml",
        )
        .unwrap()
        .unwrap();
        let panel = value.downcast_ref::<Ref<StackPanel>>().unwrap();
        panel.children().clear();
        let factory = Factory::new();
        let dock = ToolDock::new();
        dock.set_factory(Some(factory.clone()));
        dock.set_title("Original tool pane".into());
        let tool = Tool::new();
        tool.set_owner(Some(dock.clone()));
        let list = ListHandle::<Option<Rc<dyn IDockable>>>::new();
        list.add(Some(tool.clone()));
        dock.set_visible_dockables(Some(list));
        dock.set_active_dockable(Some(tool));
        let control = ToolDockControl::new();
        control.set_data_context(Some(model_projection::to_dockable_value(dock.clone())));
        panel.children().add(control.clone().upcast::<Control>());
        let window = Window::new();
        window.set_width(800.0);
        window.set_height(600.0);
        window.set_content(Some(Rc::new(panel.clone())));
        window.show();
        window.update_layout();
        Dispatcher::ui_thread().run_jobs(None);
        window.update_layout();
        Dispatcher::ui_thread().run_jobs(None);
        window.update_layout();
        let chrome = control
            .find_descendant_of_type::<ToolChromeControl>(false)
            .expect("original chrome template");
        assert!(chrome
            .find_descendant_of_type_where::<Button>(false, |b| b.name().as_deref()
                == Some("PART_CloseButton"))
            .is_some());
        assert!(chrome
            .find_descendant_of_type_where::<TextBlock>(false, |t| t.name().as_deref()
                == Some("PART_Title"))
            .is_some());
        assert!(chrome
            .find_descendant_of_type::<ToolControl>(false)
            .is_some());
        let grip = chrome
            .find_descendant_of_type_where::<ferroui_controls::Grid>(false, |grid| {
                grid.name().as_deref() == Some("PART_Grid")
            })
            .expect("tool header grip");
        let title = chrome
            .find_descendant_of_type_where::<TextBlock>(false, |text| {
                text.name().as_deref() == Some("PART_Title")
            })
            .unwrap();
        assert!(
            grip.bounds().width > 200.0,
            "grip must use spare header width"
        );
        assert!(
            title.bounds().width < 200.0,
            "title keeps its content width"
        );
        let initial_grip_width = grip.bounds().width;
        window.set_width(1000.0);
        window.update_layout();
        Dispatcher::ui_thread().run_jobs(None);
        window.update_layout();
        assert!((grip.bounds().width - initial_grip_width - 200.0).abs() < 0.001);
        dock.set_is_active(true);
        window.update_layout();
        assert!(chrome.is_active());
        window.close();
    });
}

#[test]
fn original_scroll_template_buttons_bind_source_command_and_native_viewer_and_follow_offset() {
    run(|| {
        use ferroui_base::Vector;
        use ferroui_controls::{primitives::ScrollBarVisibility, Border, Button, ScrollViewer};
        let value = crate::compiled_xaml::try_load(
            None,
            "ferres://ferrodock_markup_tests/TabThemeConsumer.xaml",
        )
        .unwrap()
        .unwrap();
        let panel = value.downcast_ref::<Ref<StackPanel>>().unwrap();
        panel.children().clear();
        let viewer = ScrollViewer::new();
        viewer.set_width(200.0);
        viewer.set_height(80.0);
        viewer.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Visible);
        let content = Border::new();
        content.set_width(1000.0);
        content.set_height(40.0);
        viewer.set_content(Some(Rc::new(content)));
        panel.children().add(viewer.clone().upcast::<Control>());
        let theme = ferroui_base::metadata::from_markup_value::<Ref<ControlTheme>>(
            &viewer.find_resource(&ResourceKey::from("DockTabStripScrollViewerHorizontal")),
        )
        .unwrap();
        viewer.set_theme(Some(theme));
        let window = Window::new();
        window.set_width(400.0);
        window.set_height(300.0);
        window.set_content(Some(Rc::new(panel.clone())));
        window.show();
        window.update_layout();
        let buttons: Vec<_> = viewer
            .get_visual_descendants()
            .filter_map(|c| c.cast::<Button>())
            .filter(|b| b.command().is_some())
            .collect();
        assert_eq!(buttons.len(), 4);
        let buttons: Vec<_> = buttons
            .into_iter()
            .filter(|b| ferroui_controls::Grid::get_row(b) == 1)
            .collect();
        assert_eq!(buttons.len(), 2);
        let right = buttons
            .iter()
            .find(|b| b.is_visible())
            .expect("right source scroll button at zero offset");
        let command = right.command().unwrap();
        let parameter = right.command_parameter();
        assert!(command.can_execute(parameter.as_ref()));
        let before = viewer.offset();
        command.execute(parameter.as_ref());
        assert!(viewer.offset().x > before.x);
        viewer.set_offset(Vector::new(100.0, 0.0));
        window.update_layout();
        assert!(buttons.iter().all(|b| b.is_visible()));
        window.close();
    });
}
