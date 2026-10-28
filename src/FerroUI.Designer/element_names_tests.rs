use super::*;
use ferroui_controls::ContentControl;
use ferroui_controls::testing::{TestServices, UnitTestApplication};

fn named(document: &Document, name: &str) -> NodeId {
    document.ast().find_name(name).unwrap()
}

#[test]
fn rename_keeps_aliased_directive_quotes_references_and_template_namescopes() {
    let source = r##"<Grid xmlns='https://github.com/ferroui' xmlns:q='http://schemas.microsoft.com/winfx/2006/xaml'>
  <!-- Keep Author literally here -->
  <Button q:Name='Author' Content='Author'/>
  <TextBlock Text='{Binding ElementName=Author, Path=Content}' Tag='{q:Reference Author}'/>
  <Border Tag='{Binding #Author.Width}'/>
  <Grid.Resources><ControlTemplate q:Key='T'><Grid><Button Name='Author'/><TextBlock Text='{Binding #Author.Content}'/></Grid></ControlTemplate></Grid.Resources>
</Grid>"##;
    let mut document = Document::new(source).unwrap();
    let id = document
        .ast()
        .nodes()
        .find(|node| node.attribute("q:Name") == Some("Author"))
        .unwrap()
        .id;
    let edits = plan_element_names(&document, &[id], Some("Editor")).unwrap();
    document.apply("Rename", edits).unwrap();
    let changed = document.source();
    assert!(changed.contains("q:Name='Editor'"));
    assert!(changed.contains("Content='Author'"));
    assert!(changed.contains("<!-- Keep Author literally here -->"));
    assert!(changed.contains("ElementName=Editor"));
    assert!(changed.contains("{q:Reference Editor}"));
    assert!(changed.contains("{Binding #Editor.Width}"));
    assert!(
        changed.contains("<Button Name='Author'/><TextBlock Text='{Binding #Author.Content}'/>")
    );
    assert_eq!(document.ast().node(id).unwrap().xaml_name(), Some("Editor"));
    document.undo().unwrap();
    assert_eq!(document.source(), source);
}

#[test]
fn rename_rejects_duplicate_invalid_captured_and_ambiguous_names_atomically() {
    let source = r#"<Grid xmlns="https://github.com/ferroui"><Button Name="First"/><Button Name="Second"/><TextBlock Text="{Binding #Unresolved.Content}"/></Grid>"#;
    let document = Document::new(source).unwrap();
    let first = named(&document, "First");
    let second = named(&document, "Second");
    for name in [
        "Second",
        "Unresolved",
        "",
        "two words",
        "{Binding Name}",
        "1Bad",
    ] {
        assert!(
            plan_element_names(&document, &[first], Some(name)).is_err(),
            "{name}"
        );
        assert_eq!(document.source(), source);
    }
    assert!(plan_element_names(&document, &[first, second], Some("Together")).is_err());
    assert!(
        plan_element_names(&document, &[first], Some("First"))
            .unwrap()
            .is_empty()
    );
    let duplicate =
        Document::new(r#"<Grid><Button Name="Same"/><Button Name="Same"/></Grid>"#).unwrap();
    let id = duplicate
        .ast()
        .nodes()
        .find(|node| node.xaml_name() == Some("Same"))
        .unwrap()
        .id;
    assert!(plan_element_names(&duplicate, &[id], Some("New")).is_err());
}

#[test]
fn name_assignment_and_reset_retain_directive_aliases_and_reject_dangling_references() {
    let mut document = Document::new(r#"<Grid xmlns="https://github.com/ferroui" xmlns:n="http://schemas.microsoft.com/winfx/2006/xaml"><Button n:Name=""/><Button Name="Referenced"/><Label Target="{Binding ElementName=Referenced}"/></Grid>"#).unwrap();
    let first = document
        .ast()
        .nodes()
        .find(|node| node.attribute("n:Name") == Some(""))
        .unwrap()
        .id;
    let referenced = named(&document, "Referenced");
    assert!(plan_element_names(&document, &[referenced], None).is_err());
    document
        .apply(
            "Name",
            plan_element_names(&document, &[first], Some("Assigned")).unwrap(),
        )
        .unwrap();
    assert_eq!(
        document.ast().node(first).unwrap().attribute("n:Name"),
        Some("Assigned")
    );
    assert_eq!(document.ast().node(first).unwrap().attribute("Name"), None);
    document
        .apply(
            "Reset",
            plan_element_names(&document, &[first], None).unwrap(),
        )
        .unwrap();
    assert_eq!(document.ast().node(first).unwrap().xaml_name(), None);
    assert_eq!(
        document.ast().node(first).unwrap().attribute("n:Name"),
        None
    );
}

#[test]
fn name_changes_share_native_undo_roles_and_ignore_animation_recording() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<Grid xmlns="https://github.com/ferroui"><Button Name="Action" Content="Save"/><TextBlock Text="{Binding #Action.Content}"/></Grid>"#;
    let mut session = DesignerSession::new(source, ContentControl::new()).unwrap();
    let id = named(session.document(), "Action");
    session.set_property(&[id], "Name", "Primary").unwrap();
    assert!(session.document().source().contains("#Primary.Content"));
    session.undo().unwrap();
    assert_eq!(session.document().source(), source);
    session.set_read_only(true);
    assert!(session.rename_element(id, "Blocked").is_err());
    session.set_read_only(false);
    session.animation.recording = true;
    session.rename_element(id, "RecordedName").unwrap();
    assert!(
        session
            .document()
            .source()
            .contains("#RecordedName.Content")
    );
    assert!(!session.document().source().contains("KeyFrame"));
    assert!(session.reset_property(&[id], "Name").is_err());
    session.undo().unwrap();
    assert_eq!(session.document().source(), source);
}

#[test]
fn rename_updates_visual_state_metadata_and_removal_protects_it() {
    use ferroui_designer_animation::{StateSetter, VisualState, VisualStateGroup};
    let mut document =
        Document::new(r#"<Grid xmlns="https://github.com/ferroui"><Button Name="Target"/></Grid>"#)
            .unwrap();
    let root = document.ast().root();
    let id = named(&document, "Target");
    let group = VisualStateGroup {
        owner: root,
        name: "Common".into(),
        states: vec![VisualState {
            name: "Highlighted".into(),
            setters: vec![StateSetter {
                target: "Target".into(),
                property: "Opacity".into(),
                value: "0.5".into(),
            }],
        }],
        transitions: Vec::new(),
    };
    document
        .apply("State", group.plan(&document).unwrap())
        .unwrap();
    let original = document.source().to_owned();
    assert!(plan_element_names(&document, &[id], None).is_err());
    document
        .apply(
            "Rename",
            plan_element_names(&document, &[id], Some("Current")).unwrap(),
        )
        .unwrap();
    let groups = VisualStateGroup::read(&document);
    let group = groups.iter().find(|group| group.name == "Common").unwrap();
    assert_eq!(group.states[0].setters[0].target, "Current");
    document.undo().unwrap();
    assert_eq!(document.source(), original);
}

#[test]
fn names_in_separate_scopes_can_be_edited_together_without_cross_references() {
    let source = r##"<Grid xmlns="https://github.com/ferroui"><Button Name="Outer"/><TextBlock Text="{Binding #Outer.Content}"/><Grid.Resources><ControlTemplate><Grid><Button Name="Inner"/><TextBlock Text="{Binding #Inner.Content}"/></Grid></ControlTemplate></Grid.Resources></Grid>"##;
    let mut document = Document::new(source).unwrap();
    let targets = [named(&document, "Outer"), named(&document, "Inner")];
    document
        .apply(
            "Rename",
            plan_element_names(&document, &targets, Some("SharedSpelling")).unwrap(),
        )
        .unwrap();
    assert_eq!(
        document.source().matches("Name=\"SharedSpelling\"").count(),
        2
    );
    assert_eq!(
        document.source().matches("#SharedSpelling.Content").count(),
        2
    );
    document.undo().unwrap();
    assert_eq!(document.source(), source);
}

#[test]
fn native_rename_undo_and_redo_rebind_element_and_hash_sources_in_the_live_namescope() {
    use crate::PreviewStatus;
    use ferroui_base::threading::Dispatcher;
    use ferroui_controls::{Control, TextBlock, TextBox, Window};
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r##"<StackPanel xmlns="https://github.com/ferroui" xmlns:ui="https://github.com/ferroui" xmlns:n="http://schemas.microsoft.com/winfx/2006/xaml"><TextBox n:Name="Source" Text="Initial"/><TextBlock Name="Named" Text="{ui:Binding Path=Text, ElementName=Source}"/><TextBlock Name="Hashed" Text="{Binding #Source.Text}"/></StackPanel>"##;
    let host = ContentControl::new();
    let mut session = DesignerSession::new(source, host.clone()).unwrap();
    let window = Window::new();
    window.set_content(Some(Control::boxed(host)));
    window.show();
    window.update_layout();
    let target = named(session.document(), "Source");
    let named_text = named(session.document(), "Named");
    let hashed_text = named(session.document(), "Hashed");
    let assert_binding_updates = |session: &DesignerSession, name: &str, value: &str| {
        assert!(
            matches!(session.preview_status(), PreviewStatus::Ready { .. }),
            "{:?}",
            session.preview_status()
        );
        window.update_layout();
        Dispatcher::ui_thread().run_jobs(None);
        let input = session
            .preview
            .visual(target)
            .unwrap()
            .cast::<TextBox>()
            .unwrap();
        assert_eq!(input.name().as_deref(), Some(name));
        input.set_text(Some(value));
        Dispatcher::ui_thread().run_jobs(None);
        for id in [named_text, hashed_text] {
            let label = session
                .preview
                .visual(id)
                .unwrap()
                .cast::<TextBlock>()
                .unwrap();
            assert_eq!(
                label.text().as_deref(),
                Some(value),
                "A binding must follow the current native source after a name/history change"
            );
        }
    };
    assert_binding_updates(&session, "Source", "Before rename");
    let revision = session.document().revision();
    session.rename_element(target, "Renamed").unwrap();
    assert_eq!(session.document().revision(), revision + 1);
    assert!(session.document().source().contains("n:Name=\"Renamed\""));
    assert_binding_updates(&session, "Renamed", "After rename");
    session.undo().unwrap();
    assert_eq!(session.document().source(), source);
    assert_binding_updates(&session, "Source", "After undo");
    session.redo().unwrap();
    assert_binding_updates(&session, "Renamed", "After redo");
    window.close();
}
