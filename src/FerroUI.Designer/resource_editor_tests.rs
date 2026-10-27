use super::*;
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_controls::testing::{TestServices, UnitTestApplication};

fn click(editor: &ResourceDefinitionEditor, name: &str) {
    editor
        .control()
        .get_control::<Button>(name)
        .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
}
fn model() -> ResourceDefinitionModel {
    ResourceDefinitionModel {
        node: 1,
        title: "Accent".into(),
        type_name: "SolidColorBrush".into(),
        kind: ComponentKind::Resource,
        key: Some("Accent".into()),
        scope: "Document".into(),
        source: "<SolidColorBrush Color=\"Red\" />".into(),
        writable: true,
        can_preview: false,
        preview: Default::default(),
        apply_property: "Background".into(),
        properties: Vec::new(),
        references: Vec::new(),
        primitive_value: None,
        setter_properties: Vec::new(),
    }
}

#[test]
fn definition_drafts_remain_local_until_an_explicit_semantic_commit() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let editor = ResourceDefinitionEditor::new();
    let commands = Rc::new(RefCell::new(Vec::new()));
    let captured = commands.clone();
    editor.on_command(move |command| {
        captured.borrow_mut().push(command);
        Ok(())
    });
    editor.configure(model());
    editor
        .control()
        .get_control::<TextBox>("DefinitionResourceKey")
        .set_text(Some("BrandAccent"));
    editor
        .control()
        .get_control::<TextBox>("DefinitionSourceDraft")
        .set_text(Some("<SolidColorBrush Color=\"Blue\" />"));
    assert!(commands.borrow().is_empty());
    click(&editor, "DefinitionRename");
    assert!(
        matches!(&commands.borrow()[0],DefinitionEditorCommand::Rename(key)if key=="BrandAccent")
    );
    click(&editor, "DefinitionApplySource");
    assert!(
        matches!(&commands.borrow()[1],DefinitionEditorCommand::ReplaceSource(source)if source.contains("Blue"))
    );
}

#[test]
fn definition_preview_rejects_invalid_sizes_and_reports_host_errors_inline() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let editor = ResourceDefinitionEditor::new();
    let mut model = model();
    model.kind = ComponentKind::ControlTemplate;
    model.can_preview = true;
    editor.configure(model);
    let calls = Rc::new(RefCell::new(0));
    let captured = calls.clone();
    editor.on_command(move |_| {
        *captured.borrow_mut() += 1;
        Err("Target type is not registered".into())
    });
    editor
        .control()
        .get_control::<TextBox>("DefinitionPreviewWidth")
        .set_text(Some("NaN"));
    click(&editor, "DefinitionEditCanvas");
    assert_eq!(*calls.borrow(), 0);
    assert!(editor.error.is_visible());
    editor
        .control()
        .get_control::<TextBox>("DefinitionPreviewWidth")
        .set_text(Some("360"));
    click(&editor, "DefinitionEditCanvas");
    assert_eq!(*calls.borrow(), 1);
    assert_eq!(
        editor.error.text().as_deref(),
        Some("Target type is not registered")
    );
}
