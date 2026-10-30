//! Independent consumers bind the original Model.Controls assembly, without shadow models.
use crate::markup_extension_tests::run;
use ferrodock_model::core::*;
use ferroui_base::{threading::Dispatcher, BoxedValue, Ref};
use ferroui_controls::{Button, CheckBox, ContentControl, Control, StackPanel, TextBlock, TextBox};
use std::rc::Rc;
fn text(p: &Ref<StackPanel>, i: usize) -> Option<String> {
    p.children().get(i).cast::<TextBlock>().unwrap().text()
}
fn flush() {
    Dispatcher::ui_thread().run_jobs(None);
}
fn invoke(p: &Ref<StackPanel>, i: usize) {
    let b = p.children().get(i).cast::<Button>().unwrap();
    b.command().unwrap().execute(b.command_parameter().as_ref());
    flush();
}
fn assert_root(p: &Ref<StackPanel>) {
    let f = ferrodock_model_reactiveui::Factory::new();
    let root = f.create_root_dock();
    root.set_title("Original root".into());
    let windows = f.create_list::<Option<Rc<dyn IDockWindow>>>(Vec::new());
    let pins = f.create_list::<Option<Rc<dyn IDockable>>>(Vec::new());
    root.set_windows(Some(windows.clone()));
    root.set_left_pinned_dockables(Some(pins.clone()));
    p.set_data_context(Some(Rc::new(root.clone())));
    flush();
    assert_eq!(text(p, 0).as_deref(), Some("Original root"));
    assert_eq!(text(p, 1).as_deref(), Some("0"));
    assert_eq!(text(p, 2).as_deref(), Some("0"));
    windows.add(Some(f.create_dock_window()));
    pins.add(Some(f.create_tool()));
    root.set_title("Updated root".into());
    flush();
    assert_eq!(text(p, 0).as_deref(), Some("Updated root"));
    assert_eq!(text(p, 1).as_deref(), Some("1"));
    assert_eq!(text(p, 2).as_deref(), Some("1"));
    p.children()
        .get(3)
        .cast::<CheckBox>()
        .unwrap()
        .set_is_checked(Some(false));
    flush();
    assert!(!root.is_focusable_root());
    windows.clear();
    pins.clear();
    flush();
    assert_eq!(text(p, 1).as_deref(), Some("0"));
    assert_eq!(text(p, 2).as_deref(), Some("0"));
    p.set_data_context(None);
}
fn assert_document(p: &Ref<StackPanel>) {
    let f = ferrodock_model_reactiveui::Factory::new();
    let dock = f.create_document_dock();
    dock.set_factory(Some(f.clone()));
    dock.set_visible_dockables(Some(f.create_list(Vec::new())));
    dock.set_title("Document dock".into());
    let doc: Rc<dyn IDockable> = f.create_document();
    dock.set_default_dockable(Some(doc.clone()));
    let content = TextBlock::new().upcast::<Control>();
    dock.set_empty_content(Some(Rc::new(content.clone())));
    p.set_data_context(Some(Rc::new(dock.clone())));
    flush();
    assert_eq!(
        p.children()
            .get(0)
            .cast::<TextBox>()
            .unwrap()
            .text()
            .as_deref(),
        Some("Document dock")
    );
    assert_eq!(
        Control::from_boxed(
            p.children()
                .get(1)
                .cast::<ContentControl>()
                .unwrap()
                .content()
                .as_ref()
                .unwrap()
        ),
        Some(content)
    );
    p.children()
        .get(0)
        .cast::<TextBox>()
        .unwrap()
        .set_text(Some("Edited dock"));
    p.children()
        .get(2)
        .cast::<CheckBox>()
        .unwrap()
        .set_is_checked(Some(true));
    flush();
    assert_eq!(dock.title(), "Edited dock");
    assert!(dock.can_create_document());
    invoke(p, 3);
    assert!(Rc::ptr_eq(
        (dock.visible_dockables().unwrap().get(0))
            .as_ref()
            .expect("nonnull identity fixture"),
        &doc
    ));
    assert!(Rc::ptr_eq(&dock.active_dockable().unwrap(), &doc));
    let owner: Rc<dyn IDockable> = dock.clone();
    assert!(Rc::ptr_eq(&doc.owner().unwrap(), &owner));
    dock.set_empty_content(None);
    flush();
    assert!(p
        .children()
        .get(1)
        .cast::<ContentControl>()
        .unwrap()
        .content()
        .is_none());
    p.set_data_context(None);
}
fn assert_tool(p: &Ref<StackPanel>) {
    let f = ferrodock_model_reactiveui::Factory::new();
    let dock = f.create_tool_dock();
    dock.set_factory(Some(f.clone()));
    dock.set_visible_dockables(Some(f.create_list(Vec::new())));
    let tool: Rc<dyn IDockable> = f.create_tool();
    tool.set_title("Tool".into());
    dock.set_default_dockable(Some(tool.clone()));
    p.set_data_context(Some(Rc::new(dock.clone())));
    flush();
    p.children()
        .get(1)
        .cast::<CheckBox>()
        .unwrap()
        .set_is_checked(Some(false));
    flush();
    assert!(!dock.auto_hide());
    invoke(p, 2);
    assert!(Rc::ptr_eq(&dock.active_dockable().unwrap(), &tool));
    assert_eq!(text(p, 0).as_deref(), Some("Tool"));
    tool.set_title("Updated tool".into());
    flush();
    assert_eq!(text(p, 0).as_deref(), Some("Updated tool"));
    let owner: Rc<dyn IDockable> = dock.clone();
    assert!(Rc::ptr_eq(&tool.owner().unwrap(), &owner));
    p.set_data_context(None);
}
fn assert_mdi(p: &Ref<StackPanel>) {
    let f = ferrodock_model_reactiveui::Factory::new();
    let doc = f.create_document().as_mdi_document().unwrap();
    doc.set_title("MDI".into());
    doc.set_mdi_z_index(3);
    p.set_data_context(Some(Rc::new(doc.clone())));
    flush();
    assert_eq!(text(p, 1).as_deref(), Some("3"));
    p.children()
        .get(0)
        .cast::<TextBox>()
        .unwrap()
        .set_text(Some("Edited MDI"));
    flush();
    assert_eq!(doc.title(), "Edited MDI");
    doc.set_mdi_z_index(9);
    doc.set_mdi_state(MdiWindowState::Maximized);
    flush();
    assert_eq!(text(p, 1).as_deref(), Some("9"));
    assert_eq!(text(p, 2).as_deref(), Some("Maximized"));
    p.set_data_context(None);
}
fn assert_grid(p: &Ref<StackPanel>) {
    let f = ferrodock_model_reactiveui::Factory::new();
    let dock = f.create_grid_dock();
    dock.set_column_definitions(Some("*,2*".into()));
    dock.set_row_definitions(Some("Auto,*".into()));
    p.set_data_context(Some(Rc::new(dock.clone())));
    flush();
    assert_eq!(
        p.children()
            .get(0)
            .cast::<TextBox>()
            .unwrap()
            .text()
            .as_deref(),
        Some("*,2*")
    );
    p.children()
        .get(1)
        .cast::<TextBox>()
        .unwrap()
        .set_text(Some("1*,3*"));
    flush();
    assert_eq!(dock.row_definitions().as_deref(), Some("1*,3*"));
    dock.set_column_definitions(None);
    flush();
    assert!(p
        .children()
        .get(0)
        .cast::<TextBox>()
        .unwrap()
        .text()
        .is_none());
    p.set_data_context(None);
}
fn assert_stack(p: &Ref<StackPanel>) {
    let f = ferrodock_model_reactiveui::Factory::new();
    let dock = f.create_stack_dock();
    dock.set_spacing(12.0);
    dock.set_orientation(Orientation::Horizontal);
    p.set_data_context(Some(Rc::new(dock.clone())));
    flush();
    assert_eq!(text(p, 0).as_deref(), Some("12"));
    assert_eq!(text(p, 1).as_deref(), Some("Horizontal"));
    dock.set_spacing(24.0);
    dock.set_orientation(Orientation::Vertical);
    flush();
    assert_eq!(text(p, 0).as_deref(), Some("24"));
    assert_eq!(text(p, 1).as_deref(), Some("Vertical"));
    p.set_data_context(None);
}
fn load(name: &str) -> Ref<StackPanel> {
    let value = crate::compiled_xaml::try_load(
        None,
        &format!("ferres://ferrodock_markup_tests/CoreControls{name}Bindings.xaml"),
    )
    .unwrap()
    .unwrap();
    value.downcast_ref::<Ref<StackPanel>>().unwrap().clone()
}
macro_rules! compiled_case {
    ($name:ident,$asset:literal,$check:ident) => {
        #[test]
        fn $name() {
            run(|| $check(&load($asset)));
        }
    };
}
compiled_case!(
    compiled_core_root_retains_windows_and_pinned_lists,
    "Root",
    assert_root
);
compiled_case!(
    compiled_core_document_method_uses_original_engine_graph,
    "Document",
    assert_document
);
compiled_case!(
    compiled_core_tool_method_activates_same_original_tool,
    "Tool",
    assert_tool
);
compiled_case!(
    compiled_core_mdi_observes_inherited_document_and_original_state,
    "Mdi",
    assert_mdi
);
compiled_case!(
    compiled_core_grid_two_way_nullable_definitions,
    "Grid",
    assert_grid
);
compiled_case!(
    compiled_core_stack_observes_original_spacing_and_orientation,
    "Stack",
    assert_stack
);
#[test]
fn runtime_compiler_and_interpreter_use_original_control_contract_owners_and_behavior() {
    run(|| {
        let mut config = ferroui_markup_xaml::RuntimeXamlLoaderConfiguration::default();
        config.local_assembly = Some(&crate::locator_factory::ASSEMBLY);
        config.use_compiled_bindings_by_default = true;
        let documents = [
            ("Root.xaml", include_str!("CoreControlsRootBindings.xaml")),
            (
                "Document.xaml",
                include_str!("CoreControlsDocumentBindings.xaml"),
            ),
            ("Tool.xaml", include_str!("CoreControlsToolBindings.xaml")),
            ("Mdi.xaml", include_str!("CoreControlsMdiBindings.xaml")),
            ("Grid.xaml", include_str!("CoreControlsGridBindings.xaml")),
            ("Stack.xaml", include_str!("CoreControlsStackBindings.xaml")),
        ];
        let emitted = ferroui_markup_xaml_loader::rust_emitter::compile_documents(
            &documents,
            Some("ferres://ferrodock_markup_tests/"),
            &config,
            &[],
        );
        assert_eq!(emitted.len(), documents.len());
        for doc in emitted {
            assert!(doc.source.is_ok(), "{:?}", doc.source);
        }
        let checks: [fn(&Ref<StackPanel>); 6] = [
            assert_root,
            assert_document,
            assert_tool,
            assert_mdi,
            assert_grid,
            assert_stack,
        ];
        for ((_, source), check) in documents.iter().zip(checks) {
            let value: BoxedValue =
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(
                    ferroui_markup_xaml::RuntimeXamlLoaderDocument::new(source),
                    Some(config.clone()),
                )
                .unwrap();
            check(value.downcast_ref::<Ref<StackPanel>>().unwrap());
        }
    });
}
