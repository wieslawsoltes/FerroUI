use super::*;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::ContentControl;

fn named(session: &DesignerSession, name: &str) -> NodeId {
    session.document().ast().find_name(name).unwrap()
}
fn session(source: &str) -> DesignerSession {
    let session = DesignerSession::new(source, ContentControl::new()).unwrap();
    assert!(
        matches!(session.preview_status(), PreviewStatus::Ready { .. }),
        "{:?}",
        session.preview_status()
    );
    session
}
fn value<'a>(session: &'a DesignerSession, node: NodeId, name: &str) -> Option<&'a str> {
    session.document().ast().node(node).unwrap().attribute(name)
}

#[test]
fn authored_appearance_copies_only_compatible_properties_in_one_undo_transaction() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<StackPanel xmlns='https://github.com/ferroui'>
      <!-- preserve all source around the operation -->
      <Button Name='Source' Content='Do not copy' Background='Red' FontSize='22' Width='91' Opacity='0.7'/>
      <Border Name='Frame' Background='Blue' Width='130'/>
      <TextBlock Name='Label' Text='Keep text' FontSize='12'/>
    </StackPanel>"#;
    let mut session = session(source);
    let from = named(&session, "Source");
    let frame = named(&session, "Frame");
    let label = named(&session, "Label");
    session.select([from]);
    let clipboard = session.copy_properties().unwrap();
    assert_eq!(
        clipboard.property_names(),
        ["Background", "Opacity", "FontSize"]
    );
    assert_eq!(session.document().source(), source);
    session.select([frame, label]);
    session.set_recording(true);
    let preview = session
        .prepare_property_paste(&clipboard, &clipboard.property_names())
        .unwrap();
    assert!(preview
        .changes()
        .iter()
        .any(|change| change.target == frame && change.property == "Background"));
    assert!(preview
        .skipped()
        .iter()
        .any(|issue| issue.target == Some(frame) && issue.property == "FontSize"));
    let revision = session.document().revision();
    session.commit_property_paste(&preview).unwrap();
    assert_eq!(session.document().revision(), revision + 1);
    assert_eq!(value(&session, frame, "Background"), Some("Red"));
    assert_eq!(value(&session, frame, "Width"), Some("130"));
    assert_eq!(value(&session, label, "FontSize"), Some("22"));
    assert_eq!(value(&session, label, "Text"), Some("Keep text"));
    assert!(!session.document().source().contains("Animation"));
    session.undo().unwrap();
    assert_eq!(session.document().source(), source);
    assert!(!session.can_undo());
    session.redo().unwrap();
    assert_eq!(value(&session, frame, "Background"), Some("Red"));
}

#[test]
fn clipboard_is_owned_and_literals_remain_reusable_after_source_edits_and_document_changes() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut source = session(
        r#"<Border xmlns='https://github.com/ferroui' Background='Red' CornerRadius='7'/>"#,
    );
    let root = source.document().ast().root();
    source.select([root]);
    let clipboard = source.copy_properties().unwrap();
    source.set_property(&[root], "Background", "Green").unwrap();
    let mut target = session(r#"<Border xmlns='https://github.com/ferroui' Background='Blue'/>"#);
    let root = target.document().ast().root();
    target.select([root]);
    let preview = target
        .prepare_property_paste(&clipboard, &["Background".into()])
        .unwrap();
    target.commit_property_paste(&preview).unwrap();
    assert_eq!(value(&target, root, "Background"), Some("Red"));
    assert_eq!(value(&target, root, "CornerRadius"), None);
    let no_op = target
        .prepare_property_paste(&clipboard, &["Background".into()])
        .unwrap();
    assert_eq!(no_op.change_count(), 0);
    let revision = target.document().revision();
    target.commit_property_paste(&no_op).unwrap();
    assert_eq!(target.document().revision(), revision);
}

#[test]
fn property_object_transfer_closes_lexical_namespaces_and_replaces_existing_graphs() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut source = session(
        r#"<Border xmlns='https://github.com/ferroui' xmlns:paint='using:FerroUI.Media'>
      <Border.Background><paint:LinearGradientBrush StartPoint='0%,0%' EndPoint='100%,100%'>
        <!-- two deliberate stops -->
        <paint:GradientStop Color='Red' Offset='0'/><paint:GradientStop Color='Blue' Offset='1'/>
      </paint:LinearGradientBrush></Border.Background>
    </Border>"#,
    );
    source.select([source.document().ast().root()]);
    let clipboard = source.copy_properties().unwrap();
    assert_eq!(clipboard.entries().len(), 1, "{:?}", clipboard.skipped());
    assert_eq!(clipboard.entries()[0].kind, CopiedPropertyKind::Object);
    let original = r#"<Border xmlns='https://github.com/ferroui' xmlns:paint='urn:foreign'>
      <Border.Background><SolidColorBrush Color='Green'/></Border.Background>
    </Border>"#;
    let mut target = session(original);
    target.select([target.document().ast().root()]);
    let plan = target
        .prepare_property_paste(&clipboard, &clipboard.property_names())
        .unwrap();
    target.commit_property_paste(&plan).unwrap();
    assert!(
        matches!(target.preview_status(), PreviewStatus::Ready { .. }),
        "{:?}",
        target.preview_status()
    );
    assert!(target
        .document()
        .source()
        .contains("xmlns:paint=\"using:FerroUI.Media\""));
    assert!(target
        .document()
        .source()
        .contains("<!-- two deliberate stops -->"));
    assert!(!target.document().source().contains("SolidColorBrush"));
    target.undo().unwrap();
    assert_eq!(target.document().source(), original);
}

#[test]
fn resource_transfer_requires_same_captured_document_and_same_lookup_identity() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let original = r#"<StackPanel xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
      <StackPanel.Resources><SolidColorBrush x:Key='Accent' Color='Red'/></StackPanel.Resources>
      <Border Name='From' Background='{StaticResource Accent}'/>
      <Border Name='To'/>
      <Border Name='Shadowed'><Border.Resources><SolidColorBrush x:Key='Accent' Color='Blue'/></Border.Resources></Border>
    </StackPanel>"#;
    let mut session = session(original);
    let from = named(&session, "From");
    let to = named(&session, "To");
    let shadowed = named(&session, "Shadowed");
    session.select([from]);
    let clipboard = session.copy_properties().unwrap();
    assert_eq!(clipboard.entries()[0].kind, CopiedPropertyKind::Resource);
    session.select([to, shadowed]);
    let preview = session
        .prepare_property_paste(&clipboard, &clipboard.property_names())
        .unwrap();
    assert_eq!(preview.change_count(), 1);
    assert!(preview.skipped().iter().any(
        |issue| issue.target == Some(shadowed) && issue.reason.contains("resolves differently")
    ));
    session.commit_property_paste(&preview).unwrap();
    assert_eq!(
        value(&session, to, "Background"),
        Some("{StaticResource 'Accent'}")
    );
    assert_eq!(value(&session, shadowed, "Background"), None);
    session.select([shadowed]);
    let stale_clipboard = session
        .prepare_property_paste(&clipboard, &clipboard.property_names())
        .unwrap();
    assert_eq!(stale_clipboard.change_count(), 0);
    assert!(stale_clipboard.skipped()[0]
        .reason
        .contains("unchanged source document"));
    session.undo().unwrap();
    assert_eq!(session.document().source(), original);
    let mut other = super::tests::session(original);
    other.select([named(&other, "To")]);
    let cross_document = other
        .prepare_property_paste(&clipboard, &clipboard.property_names())
        .unwrap();
    assert_eq!(cross_document.change_count(), 0);
    assert!(cross_document.skipped()[0]
        .reason
        .contains("unchanged source document"));
}

#[test]
fn bindings_and_context_dependent_object_values_are_reported_without_evaluating_them() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut session = session(
        r#"<Border xmlns='https://github.com/ferroui' Background='{Binding Accent}' Opacity='0.5'>
      <Border.BorderBrush><SolidColorBrush Color='{Binding Color}'/></Border.BorderBrush>
    </Border>"#,
    );
    session.select([session.document().ast().root()]);
    let clipboard = session.copy_properties().unwrap();
    assert_eq!(clipboard.property_names(), ["Opacity"]);
    assert_eq!(clipboard.skipped().len(), 2);
    assert!(clipboard
        .skipped()
        .iter()
        .all(|issue| issue.reason.contains("context")));
    assert!(!session.can_undo());
}

#[test]
fn property_aliases_keep_destination_spelling_and_inherited_owner_identity() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let original = r#"<StackPanel xmlns='https://github.com/ferroui' xmlns:u='using:FerroUI.Controls'>
      <Border Name='From' u:Border.Background='Red'/>
      <Border Name='To' u:Border.Background='Blue'/>
    </StackPanel>"#;
    let mut session = session(original);
    let from = named(&session, "From");
    let to = named(&session, "To");
    session.select([from]);
    let clipboard = session.copy_properties().unwrap();
    assert_eq!(clipboard.property_names(), ["Background"]);
    session.select([to]);
    let preview = session
        .prepare_property_paste(&clipboard, &clipboard.property_names())
        .unwrap();
    session.commit_property_paste(&preview).unwrap();
    assert_eq!(value(&session, to, "u:Border.Background"), Some("Red"));
    assert_eq!(value(&session, to, "Background"), None);
    session.undo().unwrap();
    assert_eq!(session.document().source(), original);
}

#[test]
fn foreign_qualified_members_are_not_native_property_forms() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let _native = ferroui_controls::Border::new();
    let document = Document::new(r#"<Border xmlns='https://github.com/ferroui' xmlns:foreign='urn:foreign' Background='Red' foreign:Border.Background='keep'><foreign:Border.Background><foreign:Brush/></foreign:Border.Background></Border>"#).unwrap();
    let root = document.ast().root();
    let forms = authored_forms(
        &document,
        root,
        ferroui_controls::Border::TYPE,
        ferroui_controls::Border::background_property().as_property(),
    )
    .unwrap();
    assert_eq!(forms.attribute.as_deref(), Some("Background"));
    assert!(forms.object.is_none());
}

#[test]
fn object_replacement_retains_namespace_declarations_on_the_destination_property_wrapper() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut source = session(
        r#"<Border xmlns='https://github.com/ferroui'><Border.Background><SolidColorBrush Color='Red'/></Border.Background></Border>"#,
    );
    source.select([source.document().ast().root()]);
    let clipboard = source.copy_properties().unwrap();
    let original = r#"<Border xmlns='https://github.com/ferroui'><p:Border.Background xmlns:p='using:FerroUI.Controls'><SolidColorBrush Color='Blue'/></p:Border.Background></Border>"#;
    let mut target = session(original);
    target.select([target.document().ast().root()]);
    let preview = target
        .prepare_property_paste(&clipboard, &clipboard.property_names())
        .unwrap();
    target.commit_property_paste(&preview).unwrap();
    assert!(target
        .document()
        .source()
        .contains("<p:Border.Background xmlns:p='using:FerroUI.Controls'>"));
    assert!(target.document().source().contains("Color='Red'"));
    assert!(
        matches!(target.preview_status(), PreviewStatus::Ready { .. }),
        "{:?}",
        target.preview_status()
    );
    target.undo().unwrap();
    assert_eq!(target.document().source(), original);
}

#[test]
fn window_projection_keeps_authored_owner_identity_when_pasting_object_properties() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut source = session(
        r#"<Window xmlns='https://github.com/ferroui'><Window.Background><SolidColorBrush Color='Red'/></Window.Background></Window>"#,
    );
    source.select([source.document().ast().root()]);
    let clipboard = source.copy_properties().unwrap();
    assert_eq!(clipboard.property_names(), ["Background"]);
    let original = r#"<Window xmlns='https://github.com/ferroui' Title='Keep title'><Window.Background><SolidColorBrush Color='Blue'/></Window.Background></Window>"#;
    let mut target = session(original);
    target.select([target.document().ast().root()]);
    let preview = target
        .prepare_property_paste(&clipboard, &clipboard.property_names())
        .unwrap();
    target.commit_property_paste(&preview).unwrap();
    assert!(target.document().source().contains("<Window.Background>"));
    assert!(target.document().source().contains("Color='Red'"));
    assert!(target.document().source().contains("Title='Keep title'"));
    assert!(!target.document().source().contains("<Border"));
    assert!(
        matches!(target.preview_status(), PreviewStatus::Ready { .. }),
        "{:?}",
        target.preview_status()
    );
    target.undo().unwrap();
    assert_eq!(target.document().source(), original);
}

#[test]
fn nested_foreign_property_elements_and_context_directives_are_rejected_explicitly() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let _brush = ferroui_base::media::SolidColorBrush::new();
    for fragment in [
        "<SolidColorBrush><app:Brush.Tag><SolidColorBrush Color='Blue'/></app:Brush.Tag></SolidColorBrush>",
        "<SolidColorBrush><SolidColorBrush.Color app:Context='value'>Red</SolidColorBrush.Color></SolidColorBrush>",
        "<SolidColorBrush Name='Shared' Color='Blue'/>",
    ] {
        let source = format!("<Border xmlns='https://github.com/ferroui' xmlns:app='urn:foreign'><Border.Background>{fragment}</Border.Background></Border>");
        let document = Document::new(source).unwrap();
        let property = document.ast().element_children(document.ast().root())[0];
        assert!(self_contained_object(&document, property).is_err());
    }
}

#[test]
fn reviewed_paste_rejects_stale_selection_source_scope_layers_and_viewer_roles() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<StackPanel xmlns='https://github.com/ferroui'><Border Name='From' Background='Red'/><Border Name='To'/></StackPanel>"#;
    let mut session = session(source);
    let from = named(&session, "From");
    let to = named(&session, "To");
    session.select([from]);
    session.set_read_only(true);
    let clipboard = session.copy_properties().unwrap();
    session.select([to]);
    let preview = session
        .prepare_property_paste(&clipboard, &clipboard.property_names())
        .unwrap();
    assert!(session.commit_property_paste(&preview).is_err());
    session.set_read_only(false);
    session.select([from]);
    assert!(!preview.is_current(&session));
    assert!(session.commit_property_paste(&preview).is_err());
    session.select([to]);
    session.set_layers_locked(&[to], true).unwrap();
    assert!(session
        .prepare_property_paste(&clipboard, &clipboard.property_names())
        .is_err());
    session.set_layers_locked(&[to], false).unwrap();
    assert!(
        !preview.is_current(&session),
        "Lock/unlock round trips retire drafts"
    );
    let preview = session
        .prepare_property_paste(&clipboard, &clipboard.property_names())
        .unwrap();
    session.definition_scope_epoch += 1;
    assert!(!preview.is_current(&session));
    let preview = session
        .prepare_property_paste(&clipboard, &clipboard.property_names())
        .unwrap();
    session.set_property(&[to], "CornerRadius", "3").unwrap();
    assert!(!preview.is_current(&session));
    assert!(session.commit_property_paste(&preview).is_err());
    assert_eq!(value(&session, to, "Background"), None);
}

#[test]
fn explicit_property_selection_rejects_unknown_or_duplicate_names_without_changes() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut session = session(r#"<Border xmlns='https://github.com/ferroui' Background='Red'/>"#);
    session.select([session.document().ast().root()]);
    let clipboard = session.copy_properties().unwrap();
    assert!(session
        .prepare_property_paste(&clipboard, &["Width".into()])
        .is_err());
    assert!(session
        .prepare_property_paste(&clipboard, &["Background".into(), "Background".into()])
        .is_err());
    let empty = session.prepare_property_paste(&clipboard, &[]).unwrap();
    session.commit_property_paste(&empty).unwrap();
    assert!(!session.can_undo());
}
