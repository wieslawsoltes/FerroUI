// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Downstream compiled API boundary; full original MDI dictionaries remain separate work.
use ferrodock_ferro::{
    controls::{ManagedDockWindowDocument, ManagedWindowDock, MdiDocumentControl},
    mdi::{ClassicMdiLayoutManager, IMdiLayoutManager},
    model_projection,
};
use ferrodock_model::{collections::ListHandle, controls::IToolDock, core::*};
use ferrodock_model_reactiveui::{
    controls::{Document, DocumentDock, RootDock, ToolDock},
    core::DockWindow,
};
use ferroui_base::{
    threading::{CancellationToken, Dispatcher},
    BoxedValue, Ref,
};
use ferroui_controls::{
    AppBuilder, Application, Border, ContentControl, Control, StackPanel, TextBlock, Window,
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
    name: "mdi-host-consumer",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
fn run(f: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            move || {
                ferrodock_ferro::register_types();
                ferroui_markup_xaml::register_types();
                crate::compiled_markup::register();
                f();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
fn load(name: &str) -> Ref<StackPanel> {
    let value = crate::compiled_xaml::try_load(
        None,
        &format!("ferres://ferrodock_markup_tests/{name}.xaml"),
    )
    .unwrap()
    .unwrap();
    value.downcast_ref::<Ref<StackPanel>>().unwrap().clone()
}
#[test]
fn compiled_host_api_keeps_readonly_state_live_and_exposes_original_templates_and_manager() {
    run(|| {
        let panel = load("MdiHostConsumer");
        let dock = ManagedWindowDock::new();
        panel.set_data_context(Some(model_projection::to_dockable_value(dock.clone())));
        let window = Window::new();
        window.set_content(Some(Control::boxed(panel.clone())));
        window.show();
        Dispatcher::ui_thread().run_jobs(None);
        window.update_layout();
        let host = panel
            .find_descendant_of_type::<MdiDocumentControl>(false)
            .unwrap();
        let empty = panel
            .find_descendant_of_type_where::<Border>(false, |c| {
                c.name().as_deref() == Some("EmptyHost")
            })
            .unwrap();
        let manager: Rc<dyn IMdiLayoutManager> = ClassicMdiLayoutManager::instance();
        assert!(Rc::ptr_eq(&host.layout_manager().unwrap(), &manager));
        assert!(!host.has_visible_documents());
        assert!(empty.is_visible());
        assert!(
            host.icon_template().is_some()
                && host.header_template().is_some()
                && host.modified_template().is_some()
                && host.close_template().is_some()
                && host.close_button_theme().is_some()
        );
        let built = host
            .empty_content_template()
            .unwrap()
            .build(&None)
            .unwrap()
            .cast::<TextBlock>()
            .unwrap();
        assert_eq!(built.text().as_deref(), Some("Compiled empty host"));
        let document = Document::new();
        dock.visible_dockables().unwrap().add(Some(document));
        assert!(host.has_visible_documents());
        assert!(!empty.is_visible());
        dock.set_is_active(true);
        assert!(host.is_active());
        assert!(host.pseudo_classes().contains(":active"));
        dock.set_visible_dockables(None);
        assert!(!host.has_visible_documents());
        assert!(empty.is_visible());
        panel.set_data_context(None);
        let second = Document::new();
        dock.set_visible_dockables(Some(ListHandle::from_items(vec![Some(second)])));
        let context: Rc<dyn IDock> = dock.clone();
        panel.set_data_context(Some(Rc::new(context)));
        assert!(host.has_visible_documents());
        dock.visible_dockables().unwrap().clear();
        assert!(!host.has_visible_documents());
        window.close();
    });
}
#[test]
fn compiled_converter_fields_keep_managed_type_tests_inversion_and_original_tool_identity() {
    run(|| {
        let panel = load("ManagedConvertersConsumer");
        let root = RootDock::new();
        let tool = ToolDock::new();
        root.set_active_dockable(Some(tool.clone()));
        let window_model = DockWindow::new();
        window_model.set_layout(Some(root.clone()));
        let document = ManagedDockWindowDocument::new(window_model);
        let tool_header = panel
            .find_descendant_of_type_where::<Border>(false, |c| {
                c.name().as_deref() == Some("ToolHeader")
            })
            .unwrap();
        let document_header = panel
            .find_descendant_of_type_where::<Border>(false, |c| {
                c.name().as_deref() == Some("DocumentHeader")
            })
            .unwrap();
        let content = panel
            .find_descendant_of_type::<ContentControl>(false)
            .unwrap();
        for context in [
            model_projection::to_dockable_value(document.clone()),
            model_projection::to_mdi_document_value(document.clone()),
            model_projection::to_managed_dockable_value(document.clone()),
        ] {
            panel.set_data_context(Some(context));
            assert!(tool_header.is_visible());
            assert!(!document_header.is_visible());
            let resolved = content.content().unwrap();
            assert!(Rc::ptr_eq(
                resolved.downcast_ref::<Rc<dyn IToolDock>>().unwrap(),
                &(tool.clone() as Rc<dyn IToolDock>)
            ));
            panel.set_data_context(None);
        }
        root.set_active_dockable(Some(DocumentDock::new()));
        panel.set_data_context(Some(Rc::new(document.clone()) as BoxedValue));
        assert!(!tool_header.is_visible());
        assert!(document_header.is_visible());
        assert!(content.content().is_none());
        panel.set_data_context(None);
        document.dispose();
    });
}
