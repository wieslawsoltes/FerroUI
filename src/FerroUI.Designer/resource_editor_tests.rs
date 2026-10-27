use super::*;
use ferroui_base::ObjectType;
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_controls::testing::{TestServices, UnitTestApplication};

fn named<T: ObjectType>(editor: &ResourceDefinitionEditor, name: &str) -> Ref<T> {
    editor
        .control()
        .get_logical_descendants()
        .find(|control| control.name().as_deref() == Some(name))
        .and_then(|control| control.cast::<T>())
        .unwrap_or_else(|| panic!("Missing native definition editor control {name}"))
}
fn click(editor: &ResourceDefinitionEditor, name: &str) {
    named::<Button>(editor, name).raise_event(&RoutedEventArgs::with_event(Button::click_event()));
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
        definition_properties: Vec::new(),
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
    named::<TextBox>(&editor, "DefinitionResourceKey").set_text(Some("BrandAccent"));
    named::<TextBox>(&editor, "DefinitionSourceDraft")
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
    named::<TextBox>(&editor, "DefinitionPreviewWidth").set_text(Some("NaN"));
    click(&editor, "DefinitionEditCanvas");
    assert_eq!(*calls.borrow(), 0);
    assert!(editor.error.is_visible());
    named::<TextBox>(&editor, "DefinitionPreviewWidth").set_text(Some("360"));
    click(&editor, "DefinitionEditCanvas");
    assert_eq!(*calls.borrow(), 1);
    assert_eq!(
        editor.error.text().as_deref(),
        Some("Target type is not registered")
    );
}

#[test]
fn retired_definition_controls_cannot_apply_to_the_next_definition() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let editor = ResourceDefinitionEditor::new();
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    editor.on_command(move |_| {
        observed.set(observed.get() + 1);
        Ok(())
    });
    editor.configure(model());
    let previous_apply = named::<Button>(&editor, "DefinitionApplySource");
    let previous_source = named::<TextBox>(&editor, "DefinitionSourceDraft");
    previous_source.set_text(Some("<SolidColorBrush Color=\"Blue\" />"));
    editor.configure(model());
    previous_apply.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(
        calls.get(),
        0,
        "A retained retired row cannot author the current definition"
    );
    click(&editor, "DefinitionApplySource");
    assert_eq!(calls.get(), 1);
}

#[test]
fn navigation_does_not_overwrite_the_replacement_editors_status() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let editor = ResourceDefinitionEditor::new();
    editor.configure(model());
    let weak = Rc::downgrade(&editor);
    editor.on_command(move |_| {
        let editor = weak.upgrade().unwrap();
        editor.configure(model());
        editor.show_error(Some("Current definition status"));
        Err("The retired definition failed".into())
    });
    click(&editor, "DefinitionShowSource");
    assert_eq!(
        editor.error.text().as_deref(),
        Some("Current definition status")
    );
}

#[test]
fn writable_definition_retains_editor_factory_read_only_state() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let editor = ResourceDefinitionEditor::new();
    let field = TextBox::new();
    field.set_is_enabled(false);
    let mut model = model();
    model.properties.push(DefinitionPropertyField {
        name: "ComputedValue".into(),
        editor: field.clone().upcast::<Control>(),
        description: None,
    });
    editor.configure(model);
    assert!(
        !field.is_enabled(),
        "Writable source does not make a computed property writable"
    );
}
