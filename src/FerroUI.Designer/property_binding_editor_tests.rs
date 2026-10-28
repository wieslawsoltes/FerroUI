use super::*;
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_controls::testing::{TestServices, UnitTestApplication};

fn binding_editor(
    source: Option<&str>,
    extension: &str,
    names: &[String],
) -> Rc<PropertyBindingEditor> {
    let relative = extension.rsplit_once(':').map_or_else(
        || "RelativeSource".to_owned(),
        |(prefix, _)| format!("{prefix}:RelativeSource"),
    );
    PropertyBindingEditor::with_relative_source_extension(source, extension, Some(&relative), names)
}

fn named<T: ferroui_base::ObjectType>(editor: &PropertyBindingEditor, name: &str) -> Ref<T> {
    editor
        .control()
        .get_logical_descendants()
        .find(|control| control.name().as_deref() == Some(name))
        .and_then(|control| control.cast::<T>())
        .unwrap_or_else(|| panic!("Missing binding field {name}"))
}
fn click(editor: &PropertyBindingEditor, name: &str) {
    named::<Button>(editor, name).raise_event(&RoutedEventArgs::with_event(Button::click_event()));
}

#[test]
fn native_binding_editor_applies_fields_as_one_local_value_and_preserves_untouched_source() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = "{f:Binding Name, Converter = {f:StaticResource C}, Extra={app:Token 'keep,raw'}}";
    let editor = binding_editor(Some(source), "f:Binding", &["Owner".into()]);
    assert_eq!(editor.value().unwrap(), source);
    named::<ComboBox>(&editor, "PropertyBindingSource").set_selected_index(2);
    named::<ComboBox>(&editor, "PropertyBindingElementChoice").set_selected_index(1);
    named::<TextBox>(&editor, "PropertyBindingPath").set_text(Some("Text"));
    named::<ComboBox>(&editor, "PropertyBindingMode").set_selected_index(2);
    named::<ComboBox>(&editor, "PropertyBindingUpdateTrigger").set_selected_index(2);
    named::<TextBox>(&editor, "PropertyBindingDelay").set_text(Some("120"));
    let value = editor.value().unwrap();
    assert_eq!(
        named::<TextBox>(&editor, "PropertyExpression")
            .text()
            .as_deref(),
        Some(source),
        "Reading the staged value never mutates even the local expression field"
    );
    let draft = PropertyBindingDraft::new(&value, "f:Binding").unwrap();
    assert_eq!(draft.literal("Path"), Some("Text"));
    assert_eq!(draft.literal("Mode"), Some("TwoWay"));
    assert_eq!(draft.literal("UpdateSourceTrigger"), Some("LostFocus"));
    assert_eq!(draft.literal("Delay"), Some("120"));
    assert_eq!(
        draft.source_specification(),
        Some(PropertyBindingSource::ElementName("Owner".into()))
    );
    assert!(value.contains("Converter = {f:StaticResource C}, Extra={app:Token 'keep,raw'}"));
}

#[test]
fn native_binding_editor_keeps_advanced_unset_empty_and_expression_modes_distinct() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let editor = binding_editor(Some("{Binding Title, FallbackValue=Old}"), "Binding", &[]);
    named::<TextBox>(&editor, "PropertyBindingFallbackValue").set_text(Some(""));
    named::<ComboBox>(&editor, "PropertyBindingTargetNullValueMode").set_selected_index(2);
    named::<TextBox>(&editor, "PropertyBindingTargetNullValue").set_text(Some("{x:Null}"));
    named::<ComboBox>(&editor, "PropertyBindingStringFormatMode").set_selected_index(1);
    named::<TextBox>(&editor, "PropertyBindingStringFormat").set_text(Some("Total: {0}, units"));
    let draft = PropertyBindingDraft::new(&editor.value().unwrap(), "Binding").unwrap();
    assert_eq!(draft.literal("FallbackValue"), Some(""));
    assert_eq!(draft.literal("StringFormat"), Some("Total: {0}, units"));
    assert!(
        matches!(draft.argument("TargetNullValue"), Some(MarkupValue::Extension(extension)) if extension.name == "x:Null")
    );
    named::<ComboBox>(&editor, "PropertyBindingFallbackValueMode").set_selected_index(0);
    assert!(
        PropertyBindingDraft::new(&editor.value().unwrap(), "Binding")
            .unwrap()
            .argument("FallbackValue")
            .is_none()
    );
}

#[test]
fn simultaneous_raw_expression_and_field_changes_require_explicit_reload() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let editor = binding_editor(Some("{Binding Original}"), "Binding", &[]);
    named::<TextBox>(&editor, "PropertyBindingPath").set_text(Some("FieldDraft"));
    named::<TextBox>(&editor, "PropertyExpression").set_text(Some(
        "{Binding SourceDraft, Converter={StaticResource NewConverter}}",
    ));
    assert!(editor.value().unwrap_err().contains("Read expression"));
    click(&editor, "PropertyBindingBuild");
    assert_eq!(
        named::<TextBox>(&editor, "PropertyExpression")
            .text()
            .as_deref(),
        Some("{Binding SourceDraft, Converter={StaticResource NewConverter}}")
    );
    click(&editor, "PropertyBindingRead");
    assert_eq!(
        named::<TextBox>(&editor, "PropertyBindingPath")
            .text()
            .as_deref(),
        Some("SourceDraft")
    );
    assert_eq!(
        editor.value().unwrap(),
        "{Binding SourceDraft, Converter={StaticResource NewConverter}}"
    );
}

#[test]
fn invalid_fields_cannot_replace_the_expression_and_unknown_current_source_is_retained() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = "{Binding Value, RelativeSource={app:RelativeSource Context=keep}}";
    let editor = binding_editor(Some(source), "Binding", &[]);
    assert_eq!(
        named::<ComboBox>(&editor, "PropertyBindingSource").selected_index(),
        0
    );
    named::<TextBox>(&editor, "PropertyBindingPath").set_text(Some("Other"));
    assert!(
        editor
            .value()
            .unwrap()
            .contains("RelativeSource={app:RelativeSource Context=keep}")
    );
    named::<ComboBox>(&editor, "PropertyBindingSource").set_selected_index(5);
    named::<TextBox>(&editor, "PropertyBindingAncestorType").set_text(Some("Grid"));
    named::<TextBox>(&editor, "PropertyBindingAncestorLevel").set_text(Some("0"));
    assert!(editor.value().is_err());
    named::<TextBox>(&editor, "PropertyBindingAncestorLevel").set_text(Some("1"));
    named::<TextBox>(&editor, "PropertyBindingDelay").set_text(Some("NaN"));
    assert!(editor.value().is_err());
    assert_eq!(
        named::<TextBox>(&editor, "PropertyExpression")
            .text()
            .as_deref(),
        Some(source)
    );
}

#[test]
fn binding_namespace_metadata_resolves_suffixes_and_relative_source_independently() {
    use crate::component_session::{
        property_binding_extension, property_relative_source_extension,
    };
    let document = ferroui_designer_core::Document::new(r#"<ui:Grid xmlns="using:App" xmlns:ui="https://github.com/ferroui" xmlns:b="using:FerroUI.Data" xmlns:m="using:FerroUI.Markup.Xaml.MarkupExtensions"/>"#).unwrap();
    let root = document.ast().root();
    assert_eq!(
        property_binding_extension(&document, root, Some("{ui:BindingExtension Value}")),
        "ui:BindingExtension"
    );
    assert_eq!(
        property_binding_extension(
            &document,
            root,
            Some("{m:ReflectionBindingExtension Value}")
        ),
        "m:ReflectionBindingExtension"
    );
    assert_eq!(
        property_binding_extension(&document, root, Some("{Binding Value}")),
        "ui:Binding"
    );
    let relative = property_relative_source_extension(&document, root).unwrap();
    assert_ne!(
        relative, "b:RelativeSource",
        "The data object is not the RelativeSource markup extension"
    );
    let mut draft = PropertyBindingDraft::new("{b:ReflectionBinding Value}", "b:ReflectionBinding")
        .unwrap()
        .with_relative_source_extension(Some(&relative));
    draft.set_source(PropertyBindingSource::SelfValue).unwrap();
    assert!(draft.source().contains(&format!("{{{relative} ")));
    let mut unavailable =
        PropertyBindingDraft::new("{b:ReflectionBinding Value}", "b:ReflectionBinding")
            .unwrap()
            .with_relative_source_extension(None);
    assert!(
        unavailable
            .set_source(PropertyBindingSource::SelfValue)
            .is_err()
    );
    assert_eq!(unavailable.source(), "{b:ReflectionBinding Value}");
    let custom = ferroui_designer_core::Document::new("<Grid xmlns='using:App'/>").unwrap();
    assert!(property_binding_extension(&custom, custom.ast().root(), None).is_empty());
    assert!(property_relative_source_extension(&custom, custom.ast().root()).is_none());
}

#[test]
fn structured_sources_evaluate_in_native_preview_and_undo_as_one_source_change() {
    use crate::{DesignerSession, PreviewStatus};
    use ferroui_base::threading::Dispatcher;
    use ferroui_controls::{ContentControl, Window};
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let original = r#"<ui:StackPanel xmlns="using:Application" xmlns:ui="https://github.com/ferroui" Name="Container" Tag="ancestor value"><ui:TextBlock Name="Target" Tag="self value" Text="original"/><ui:TextBox Name="Input" Text="element value"/></ui:StackPanel>"#;
    let host = ContentControl::new();
    let mut session = DesignerSession::new(original, host.clone()).unwrap();
    let window = Window::new();
    window.set_content(Some(Control::boxed(host)));
    window.show();
    let target = session.document().ast().find_name("Target").unwrap();
    for (source, path, expected) in [
        (PropertyBindingSource::SelfValue, "Tag", "self value"),
        (
            PropertyBindingSource::Ancestor {
                type_name: "ui:StackPanel".into(),
                level: 1,
                logical_tree: false,
            },
            "Tag",
            "ancestor value",
        ),
        (
            PropertyBindingSource::ElementName("Input".into()),
            "Text",
            "element value",
        ),
    ] {
        let mut draft = PropertyBindingDraft::empty("ui:Binding")
            .unwrap()
            .with_relative_source_extension(Some("ui:RelativeSource"));
        draft.set_path(path);
        draft.set_source(source).unwrap();
        let revision = session.document().revision();
        assert_eq!(session.document().source(), original);
        session
            .set_property(&[target], "Text", &draft.source())
            .unwrap();
        assert_eq!(session.document().revision(), revision + 1);
        assert!(
            matches!(session.preview_status(), PreviewStatus::Ready { .. }),
            "{:?}",
            session.preview_status()
        );
        window.update_layout();
        Dispatcher::ui_thread().run_jobs(None);
        let text = session
            .preview
            .visual(target)
            .unwrap()
            .cast::<TextBlock>()
            .unwrap();
        assert_eq!(text.text().as_deref(), Some(expected), "{}", draft.source());
        session.undo().unwrap();
        assert_eq!(session.document().source(), original);
    }
    window.close();
}
