use super::*;
use crate::editors::PropertyValueEdit;
use ferroui_controls::testing::{TestServices, UnitTestApplication};

#[test]
fn inspector_object_resource_and_literal_edits_share_one_undo_and_reject_stale_rows() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<StackPanel xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><StackPanel.Resources><SolidColorBrush x:Key="Accent" Color="Purple" /></StackPanel.Resources><Border Name="Card"><Border.Background><LinearGradientBrush><!-- keep --><GradientStop Color="Red" Offset="0"/><GradientStop Color="Blue" Offset="1"/></LinearGradientBrush></Border.Background></Border><Border Name="Other"/></StackPanel>"#;
    let workspace = DesignerWorkspace::new(source).unwrap();
    let card = workspace
        .session
        .borrow()
        .document()
        .ast()
        .nodes_in_source_order()
        .find(|node| node.xaml_name() == Some("Card"))
        .unwrap()
        .id;
    workspace.session.borrow_mut().select([card]);
    workspace.rebuild_properties();
    let get = || {
        workspace
            .inspectors
            .property_source
            .items()
            .to_vec()
            .into_iter()
            .find(|row| row.name == "Background")
            .unwrap()
    };
    let row = get();
    let metadata = row.metadata.as_ref().unwrap();
    let context = row.context.as_ref().unwrap();
    let graph = metadata
        .object_source
        .clone()
        .unwrap()
        .replace("Color=\"Red\"", "Color=\"Green\"");
    metadata
        .apply(context, PropertyValueEdit::Object(graph))
        .unwrap();
    assert!(
        workspace
            .session
            .borrow()
            .document()
            .source()
            .contains("Green")
    );
    assert!(
        workspace
            .session
            .borrow()
            .document()
            .source()
            .contains("<!-- keep -->")
    );
    workspace.session.borrow_mut().undo().unwrap();
    assert_eq!(workspace.session.borrow().document().source(), source);
    assert!(
        metadata
            .apply(context, PropertyValueEdit::Literal("Orange".into()))
            .is_err(),
        "Undo changes revision, so an old popup must be rejected"
    );
    workspace.rebuild_properties();
    let row = get();
    let metadata = row.metadata.as_ref().unwrap();
    let resource = metadata
        .resources
        .iter()
        .find(|resource| resource.key == "Accent")
        .unwrap()
        .node;
    metadata
        .apply(
            row.context.as_ref().unwrap(),
            PropertyValueEdit::Resource {
                node: resource,
                dynamic: true,
            },
        )
        .unwrap();
    assert!(
        workspace
            .session
            .borrow()
            .document()
            .ast()
            .node(card)
            .unwrap()
            .attribute("Background")
            .unwrap()
            .contains("DynamicResource")
    );
    workspace.session.borrow_mut().undo().unwrap();
    assert_eq!(workspace.session.borrow().document().source(), source);
    workspace.rebuild_properties();
    let row = get();
    row.metadata
        .as_ref()
        .unwrap()
        .apply(
            row.context.as_ref().unwrap(),
            PropertyValueEdit::Literal("Orange".into()),
        )
        .unwrap();
    assert_eq!(
        workspace
            .session
            .borrow()
            .document()
            .ast()
            .node(card)
            .unwrap()
            .attribute("Background"),
        Some("Orange")
    );
    workspace.session.borrow_mut().undo().unwrap();
    assert_eq!(workspace.session.borrow().document().source(), source);
}

#[test]
fn property_callbacks_reject_selection_scope_and_read_only_changes_without_source_edits() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<UserControl xmlns="https://github.com/ferroui"><StackPanel><Border Name="Card" Width="100"/><Border Name="Other"/></StackPanel></UserControl>"#;
    let workspace = DesignerWorkspace::new(source).unwrap();
    let card = workspace
        .session
        .borrow()
        .document()
        .ast()
        .find_name("Card")
        .unwrap();
    let other = workspace
        .session
        .borrow()
        .document()
        .ast()
        .find_name("Other")
        .unwrap();
    let root = workspace.session.borrow().document().ast().root();
    let row = || {
        workspace.rebuild_properties();
        workspace
            .inspectors
            .property_source
            .items()
            .to_vec()
            .into_iter()
            .find(|row| row.name == "Width")
            .unwrap()
    };
    workspace.session.borrow_mut().select([card]);
    let old = row();
    workspace.session.borrow_mut().select([other]);
    assert!(
        old.metadata
            .as_ref()
            .unwrap()
            .apply(
                old.context.as_ref().unwrap(),
                PropertyValueEdit::Literal("200".into())
            )
            .is_err()
    );
    workspace.session.borrow_mut().select([card]);
    let old = row();
    workspace.session.borrow_mut().set_read_only(true);
    assert!(
        old.metadata
            .as_ref()
            .unwrap()
            .apply(
                old.context.as_ref().unwrap(),
                PropertyValueEdit::Literal("200".into())
            )
            .is_err()
    );
    workspace.session.borrow_mut().set_read_only(false);
    let old = row();
    workspace
        .session
        .borrow_mut()
        .enter_definition_scope(root)
        .unwrap();
    workspace.session.borrow_mut().select([card]);
    assert!(
        old.metadata
            .as_ref()
            .unwrap()
            .apply(
                old.context.as_ref().unwrap(),
                PropertyValueEdit::Literal("200".into())
            )
            .is_err()
    );
    assert_eq!(workspace.session.borrow().document().source(), source);
    assert!(!workspace.session.borrow().can_undo());
}

#[test]
fn inspector_canonical_attached_property_retains_alias_and_unknown_attached_graph_survives_reset() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<Grid xmlns="https://github.com/ferroui" xmlns:ui="https://github.com/ferroui" RowDefinitions="*,*"><Border Name="Card" ui:Grid.Row="1" /></Grid>"#;
    let workspace = DesignerWorkspace::new(source).unwrap();
    let card = workspace
        .session
        .borrow()
        .document()
        .ast()
        .find_name("Card")
        .unwrap();
    workspace.session.borrow_mut().select([card]);
    workspace.rebuild_properties();
    let rows = workspace.inspectors.property_source.items().to_vec();
    assert!(!rows.iter().any(|row| row.name == "ui:Grid.Row"));
    let row = rows.iter().find(|row| row.name == "Grid.Row").unwrap();
    assert_eq!(row.context.as_ref().unwrap().value.as_deref(), Some("1"));
    row.metadata
        .as_ref()
        .unwrap()
        .apply(
            row.context.as_ref().unwrap(),
            PropertyValueEdit::Literal("0".into()),
        )
        .unwrap();
    assert_eq!(
        workspace
            .session
            .borrow()
            .document()
            .ast()
            .node(card)
            .unwrap()
            .attribute("ui:Grid.Row"),
        Some("0")
    );
    assert!(
        workspace
            .session
            .borrow()
            .document()
            .ast()
            .node(card)
            .unwrap()
            .attribute("Grid.Row")
            .is_none()
    );
    workspace.session.borrow_mut().undo().unwrap();
    assert_eq!(workspace.session.borrow().document().source(), source);

    // A source-only application attached value is independent of native Background.
    let source = r#"<Border xmlns="https://github.com/ferroui" xmlns:a="urn:application"><Border.Background><SolidColorBrush Color="Red"/></Border.Background><a:Holder.Background><a:CustomBrush Token="Keep"/></a:Holder.Background></Border>"#;
    let mut session =
        DesignerSession::new(source, ferroui_controls::ContentControl::new()).unwrap();
    let root = session.document().ast().root();
    apply_property_value(
        &mut session,
        &[root],
        "Background",
        PropertyValueEdit::Reset,
    )
    .unwrap();
    assert!(session.document().source().contains("a:Holder.Background"));
    assert!(session.document().source().contains("Token=\"Keep\""));
    assert!(!session.document().source().contains("<Border.Background>"));
    session.undo().unwrap();
    assert_eq!(session.document().source(), source);
}
