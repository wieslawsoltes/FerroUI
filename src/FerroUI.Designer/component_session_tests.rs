use crate::DesignerSession;
use ferroui_controls::ContentControl;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_designer_tools::ComponentAction;

const SOURCE: &str = r#"<Canvas xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><Canvas.Resources><SolidColorBrush x:Key="Accent" Color="Red"/></Canvas.Resources><Button Name="Target" Background="{StaticResource Accent}"/><Button Name="Other"/></Canvas>"#;

#[test]
fn property_expression_helpers_retain_the_authored_framework_alias() {
    let document = ferroui_designer_core::Document::new(r#"<Canvas xmlns="https://github.com/ferroui" xmlns:ui="https://github.com/ferroui" xmlns:app="using:Example"/>"#).unwrap();
    let node = document.ast().root();
    assert_eq!(
        super::property_binding_extension(&document, node, Some("{ui:Binding Path=Value}")),
        "ui:Binding"
    );
    assert_eq!(
        super::property_binding_extension(&document, node, Some("{ui:DynamicResource Accent}")),
        "ui:Binding"
    );
    assert_eq!(
        super::property_binding_extension(&document, node, Some("{app:Binding Path=Value}")),
        "Binding"
    );
}

#[test]
fn native_resource_picker_and_apply_follow_the_target_theme_and_retire_stale_previews() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<Border xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" RequestedThemeVariant="Dark"><Border.Resources><ResourceDictionary><ResourceDictionary.ThemeDictionaries><ResourceDictionary x:Key="Dark"><SolidColorBrush x:Key="Accent" Color="Red"/></ResourceDictionary><ResourceDictionary x:Key="Light"><SolidColorBrush x:Key="Accent" Color="Blue"/></ResourceDictionary></ResourceDictionary.ThemeDictionaries></ResourceDictionary></Border.Resources></Border>"#;
    let mut session = DesignerSession::new(source, ContentControl::new()).unwrap();
    let target = session.document().ast().root();
    assert!(
        matches!(session.preview_status(), crate::PreviewStatus::Ready { .. }),
        "{:?}",
        session.preview_status()
    );
    let context = session.component_resource_context(target).unwrap();
    assert_eq!(context.theme.as_deref(), Some("Dark"));
    let choices = session.visible_component_resources(target).unwrap();
    let dark = choices
        .iter()
        .find(|entry| entry.key.as_deref() == Some("Accent"))
        .unwrap()
        .node;
    assert_eq!(
        session
            .document()
            .ast()
            .node(dark)
            .unwrap()
            .attribute("Color"),
        Some("Red")
    );
    let action = ComponentAction::ApplyResource {
        resource: dark,
        targets: vec![target],
        property: "Background".into(),
        dynamic: true,
    };
    let preview = session.prepare_component_edit(&action).unwrap();
    let native = session
        .preview
        .visual(target)
        .unwrap()
        .cast::<ferroui_base::StyledElement>()
        .unwrap();
    native.set_value(
        ferroui_base::styling::ThemeVariant::requested_theme_variant_property(),
        Some(ferroui_base::styling::ThemeVariant::light()),
    );
    assert!(!preview.is_current(&session));
    assert!(session.commit_component_edit(&preview).is_err());
    assert_eq!(session.document().source(), source);
    native.set_value(
        ferroui_base::styling::ThemeVariant::requested_theme_variant_property(),
        Some(ferroui_base::styling::ThemeVariant::dark()),
    );
    session.edit_component(action).unwrap();
    assert_eq!(
        session
            .document()
            .ast()
            .node(target)
            .unwrap()
            .attribute("Background"),
        Some("{DynamicResource 'Accent'}")
    );
    session.undo().unwrap();
    assert_eq!(session.document().source(), source);
}

#[test]
fn edit_copy_keeps_other_instances_linked_and_undo_restores_resource_and_reference_together() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<StackPanel xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><StackPanel.Resources><ControlTheme x:Key="Action" TargetType="Button"><Setter Property="Padding" Value="12"/></ControlTheme></StackPanel.Resources><Button Name="First" Theme="{StaticResource Action}"/><Button Name="Second" Theme="{StaticResource Action}"/></StackPanel>"#;
    let mut session = DesignerSession::new(source, ContentControl::new()).unwrap();
    let first = session.document().ast().find_name("First").unwrap();
    let second = session.document().ast().find_name("Second").unwrap();
    let original =
        ferroui_designer_tools::resolve_component_resource(session.document(), first, "Action")
            .unwrap();
    let revision = session.document().revision();
    let copy = session
        .copy_definition_for_instance(original, first, "Theme", "PersonalAction")
        .unwrap();
    assert_ne!(copy, original);
    assert_eq!(session.document().revision(), revision + 1);
    assert_eq!(
        session
            .document()
            .ast()
            .node(first)
            .unwrap()
            .attribute("Theme"),
        Some("{StaticResource 'PersonalAction'}")
    );
    assert_eq!(
        session
            .document()
            .ast()
            .node(second)
            .unwrap()
            .attribute("Theme"),
        Some("{StaticResource Action}")
    );
    session.undo().unwrap();
    assert_eq!(session.document().source(), source);
    assert!(!session.can_undo());
    session.set_read_only(true);
    assert!(
        session
            .copy_definition_for_instance(original, first, "Theme", "ReadOnlyCopy")
            .is_err()
    );
    assert_eq!(session.document().source(), source);
}

#[test]
fn component_session_resource_rename_and_references_share_one_history_transaction() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut session = DesignerSession::new(SOURCE, ContentControl::new()).unwrap();
    let resource = session
        .component_catalog()
        .unwrap()
        .entries
        .into_iter()
        .find(|entry| entry.key.as_deref() == Some("Accent"))
        .unwrap()
        .node;
    let target = session.document().ast().find_name("Target").unwrap();
    session.select([target]);
    let revision = session.document().revision();
    let edit = session
        .prepare_component_edit(&ComponentAction::RenameResource {
            resource,
            key: "Primary".into(),
        })
        .unwrap();
    assert!(edit.is_current(&session));
    assert_eq!(session.document().source(), SOURCE);
    session.commit_component_edit(&edit).unwrap();
    assert_eq!(session.document().revision(), revision + 1);
    assert_eq!(
        session
            .document()
            .ast()
            .node(resource)
            .unwrap()
            .attribute("x:Key"),
        Some("Primary")
    );
    assert_eq!(
        session.component_resource_references(resource).unwrap()[0].node,
        target
    );
    assert!(!edit.is_current(&session));
    session.undo().unwrap();
    assert_eq!(session.document().source(), SOURCE);
    assert!(!session.can_undo());
}

#[test]
fn prepared_component_edits_decline_selection_document_read_only_and_invalid_source_changes() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    for change in 0..4 {
        let mut session = DesignerSession::new(SOURCE, ContentControl::new()).unwrap();
        let target = session.document().ast().find_name("Target").unwrap();
        let other = session.document().ast().find_name("Other").unwrap();
        session.select([target]);
        let edit = session
            .prepare_component_edit(&ComponentAction::SetProperty {
                node: target,
                property: "Width".into(),
                value: Some("140".into()),
            })
            .unwrap();
        match change {
            0 => {
                session.select([other]);
            }
            1 => {
                session
                    .open_document(SOURCE.into(), session.document_uri().to_owned())
                    .unwrap();
            }
            2 => {
                session.set_read_only(true);
            }
            _ => {
                session.update_source("<Canvas");
            }
        }
        let before = session.document().source().to_owned();
        assert!(!edit.is_current(&session));
        assert!(session.commit_component_edit(&edit).is_err());
        assert_eq!(session.document().source(), before);
    }
}

#[test]
fn component_source_repair_does_not_require_a_successful_runtime_preview() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut session = DesignerSession::new(SOURCE, ContentControl::new()).unwrap();
    assert!(
        session
            .update_source("<MissingControl xmlns='https://github.com/ferroui'/>")
            .valid
    );
    assert!(!session.can_manipulate());
    let root = session.document().ast().root();
    session
        .edit_component(ComponentAction::CreateResource {
            owner: root,
            key: "Accent".into(),
            source: "<SolidColorBrush Color='Red'/>".into(),
        })
        .unwrap();
    assert!(
        session
            .component_catalog()
            .unwrap()
            .entries
            .iter()
            .any(|entry| entry.key.as_deref() == Some("Accent"))
    );
}
