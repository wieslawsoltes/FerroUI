use super::*;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{ContentControl, Control, Window};

fn shown(source: &str) -> (DesignerSession, Ref<Window>) {
    let host = ContentControl::new();
    let mut session = DesignerSession::new(source, host.clone()).unwrap();
    let window = Window::new();
    window.set_width(500.0);
    window.set_height(400.0);
    window.set_content(Some(Control::boxed(host)));
    window.show();
    window.update_layout();
    session.preview.rebuild_map(&session.document);
    assert!(
        matches!(session.preview_status(), PreviewStatus::Ready { .. }),
        "{:?}",
        session.preview_status()
    );
    (session, window)
}

fn named(session: &DesignerSession, name: &str) -> NodeId {
    session.document().ast().find_name(name).unwrap()
}

#[test]
fn focus_groups_preserve_native_index_spaces_and_disabled_entries() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<StackPanel xmlns="https://github.com/ferroui"><Button Name="First" TabIndex="3"/><Button Name="Second" TabIndex="3" IsEnabled="False"/><StackPanel Name="Local" KeyboardNavigation.TabNavigation="Local"><Button Name="Inside" TabIndex="-4"/><Button Name="Excluded" IsTabStop="False"/></StackPanel><StackPanel Name="None" KeyboardNavigation.TabNavigation="None"><Button Name="Blocked"/></StackPanel></StackPanel>"#;
    let (session, window) = shown(source);
    let catalog = session.focus_order_catalog().unwrap();
    let root = &catalog.scopes[0];
    let first = named(&session, "First");
    let second = named(&session, "Second");
    assert_eq!(root.items[0].node, Some(first));
    assert_eq!(root.items[1].node, Some(second));
    assert!(!root.items[1].enabled);
    let local = catalog
        .scopes
        .iter()
        .find(|scope| scope.node == Some(named(&session, "Local")))
        .unwrap();
    assert_eq!(local.mode, KeyboardNavigationMode::Local);
    assert_eq!(local.items[0].node, Some(named(&session, "Inside")));
    assert!(
        !root
            .items
            .iter()
            .any(|item| item.node == Some(named(&session, "Inside")))
    );
    assert!(
        !local
            .items
            .iter()
            .find(|item| item.node == Some(named(&session, "Excluded")))
            .unwrap()
            .is_tab_stop
    );
    assert_eq!(session.document().source(), source);
    assert!(!session.can_undo());
    window.close();
}

#[test]
fn focus_reorder_is_one_atomic_undo_and_rejects_cross_group_or_stale_models() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<StackPanel xmlns="https://github.com/ferroui"><Button Name="First"/><Button Name="Second"/><StackPanel Name="Group" KeyboardNavigation.TabNavigation="Cycle"><Button Name="Inside"/></StackPanel></StackPanel>"#;
    let (mut session, window) = shown(source);
    let snapshot = session.focus_order_snapshot().unwrap();
    let root = snapshot.catalog.scopes[0].id;
    let (first, second, group, inside) = (
        named(&session, "First"),
        named(&session, "Second"),
        named(&session, "Group"),
        named(&session, "Inside"),
    );
    assert!(
        session
            .commit_focus_order(
                &snapshot,
                &[FocusOrderEdit::SetOrder {
                    scope: root,
                    order: vec![second, first, inside]
                }]
            )
            .is_err()
    );
    assert_eq!(session.document().source(), source);
    session
        .commit_focus_order(
            &snapshot,
            &[
                FocusOrderEdit::SetOrder {
                    scope: root,
                    order: vec![second, group, first],
                },
                FocusOrderEdit::SetTabStop {
                    node: first,
                    value: false,
                },
            ],
        )
        .unwrap();
    assert_eq!(
        session
            .document()
            .ast()
            .node(second)
            .unwrap()
            .attribute("TabIndex"),
        Some("0")
    );
    assert_eq!(
        session
            .document()
            .ast()
            .node(first)
            .unwrap()
            .attribute("TabIndex"),
        Some("2")
    );
    assert_eq!(
        session
            .document()
            .ast()
            .node(first)
            .unwrap()
            .attribute("IsTabStop"),
        Some("False")
    );
    assert!(!snapshot.is_current(&session));
    session.undo().unwrap();
    assert_eq!(session.document().source(), source);
    assert!(!session.can_undo());
    window.update_layout();
    session.preview.rebuild_map(&session.document);
    let snapshot = session.focus_order_snapshot().unwrap();
    session.set_read_only(true);
    assert!(
        session
            .commit_focus_order(
                &snapshot,
                &[FocusOrderEdit::ResetOrder {
                    scope: snapshot.catalog.scopes[0].id
                }]
            )
            .is_err()
    );
    session.set_read_only(false);
    let current_first = named(&session, "First");
    session
        .preview
        .visual(current_first)
        .unwrap()
        .cast::<InputElement>()
        .unwrap()
        .set_tab_index(7);
    assert!(
        !snapshot.is_current(&session),
        "Native code can alter navigation without a source revision"
    );
    window.close();
}

#[test]
fn focus_source_edits_keep_equivalent_aliases_and_application_lookalikes() {
    let mut document = Document::new(r#"<ui:StackPanel xmlns="using:Application" xmlns:ui="https://github.com/ferroui" xmlns:input="using:FerroUI.Input" xmlns:app="urn:application"><ui:Button Name="A" input:KeyboardNavigation.TabIndex="9" app:KeyboardNavigation.TabIndex="Keep"/><ui:Label Name="Caption"/></ui:StackPanel>"#).unwrap();
    let a = document.ast().find_name("A").unwrap();
    let edits = plan_navigation_property(&document, a, "TabIndex", Some("2"), false).unwrap();
    document.apply("Reorder", edits).unwrap();
    let node = document.ast().node(a).unwrap();
    assert_eq!(
        node.attribute("input:KeyboardNavigation.TabIndex"),
        Some("2")
    );
    assert_eq!(
        node.attribute("app:KeyboardNavigation.TabIndex"),
        Some("Keep")
    );
    assert!(node.attribute("TabIndex").is_none());
    let root = document.ast().root();
    let edits =
        plan_navigation_property(&document, root, "TabNavigation", Some("Once"), false).unwrap();
    document.apply("Group mode", edits).unwrap();
    assert_eq!(
        document
            .ast()
            .node(root)
            .unwrap()
            .attribute("ui:KeyboardNavigation.TabNavigation"),
        Some("Once")
    );
    let edits = plan_navigation_property(&document, a, "TabIndex", None, true).unwrap();
    document.apply("Reset", edits).unwrap();
    assert_eq!(
        document
            .ast()
            .node(a)
            .unwrap()
            .attribute("app:KeyboardNavigation.TabIndex"),
        Some("Keep")
    );
    assert!(
        document
            .ast()
            .node(a)
            .unwrap()
            .attribute("input:KeyboardNavigation.TabIndex")
            .is_none()
    );
}

#[test]
fn focus_order_refuses_bound_indices_and_owner_property_elements_atomically() {
    for source in [
        r#"<StackPanel xmlns="https://github.com/ferroui"><Button Name="A" TabIndex="{Binding Order}"/></StackPanel>"#,
        r#"<StackPanel xmlns="https://github.com/ferroui"><Button Name="A"><Button.TabIndex>3</Button.TabIndex></Button></StackPanel>"#,
        r#"<StackPanel xmlns="https://github.com/ferroui" xmlns:ui="https://github.com/ferroui"><Button Name="A" TabIndex="2" ui:KeyboardNavigation.TabIndex="3"/></StackPanel>"#,
    ] {
        let document = Document::new(source).unwrap();
        let node = document.ast().find_name("A").unwrap();
        assert!(plan_navigation_property(&document, node, "TabIndex", Some("0"), false).is_err());
        assert_eq!(document.source(), source);
    }
}

#[test]
fn label_target_can_name_and_link_an_input_in_one_source_transaction() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<StackPanel xmlns="https://github.com/ferroui"><Label Name="Caption" Content="_Name"/><TextBox/><TextBox Name="FocusTarget"/></StackPanel>"#;
    let (mut session, window) = shown(source);
    let label = named(&session, "Caption");
    let target = session
        .document()
        .ast()
        .elements()
        .find(|node| node.local_name() == "TextBox" && node.xaml_name().is_none())
        .unwrap()
        .id;
    let snapshot = session.focus_order_snapshot().unwrap();
    let association = snapshot
        .catalog
        .labels
        .iter()
        .find(|entry| entry.node == label)
        .unwrap();
    assert!(
        association
            .candidates
            .iter()
            .any(|candidate| candidate.node == target)
    );
    session
        .commit_focus_order(
            &snapshot,
            &[FocusOrderEdit::SetLabelTarget {
                label,
                target: Some(target),
            }],
        )
        .unwrap();
    window.update_layout();
    session.preview.rebuild_map(&session.document);
    assert_eq!(
        session
            .document()
            .ast()
            .node(target)
            .unwrap()
            .attribute("Name"),
        Some("FocusTarget1")
    );
    assert_eq!(
        session
            .document()
            .ast()
            .node(label)
            .unwrap()
            .attribute("Target"),
        Some("{Binding ElementName=FocusTarget1}")
    );
    let label_native = session
        .preview
        .visual(label)
        .unwrap()
        .cast::<Label>()
        .unwrap();
    assert_eq!(
        label_native.target(),
        session
            .preview
            .visual(target)
            .and_then(|visual| visual.cast::<InputElement>())
    );
    session.undo().unwrap();
    assert_eq!(session.document().source(), source);
    window.close();
}

#[test]
fn focus_snapshot_retires_after_definition_scope_round_trip_without_source_edits() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<StackPanel xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><StackPanel.Resources><ControlTemplate x:Key="Card" TargetType="Button"><TextBox Name="TemplateInput"/></ControlTemplate></StackPanel.Resources><Button Name="Instance"/></StackPanel>"#;
    let (mut session, window) = shown(source);
    let snapshot = session.focus_order_snapshot().unwrap();
    let revision = session.document().revision();
    let template = session
        .document()
        .ast()
        .elements()
        .find(|node| node.local_name() == "ControlTemplate")
        .unwrap()
        .id;
    session.enter_definition_scope(template).unwrap();
    session.exit_definition_scope().unwrap();
    assert_eq!(session.document().revision(), revision);
    assert!(!snapshot.is_current(&session));
    assert_eq!(session.document().source(), source);
    window.close();
}
