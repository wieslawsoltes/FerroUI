use super::*;
use ferroui_base::{interactivity::RoutedEventArgs, Ref};
use ferroui_controls::{
    automation::AutomationProperties,
    testing::{TestServices, UnitTestApplication},
    ContentControl, Window,
};
use ferroui_markup_xaml::FromXamlObject;

const SOURCE: &str = include_str!("../../editors/vscode/test/fixtures/ItemTemplatesWorkbench.xaml");

#[test]
fn item_template_scope_uses_the_selected_fixture_and_restores_the_list_selection() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let host = ContentControl::new();
    let mut session = crate::DesignerSession::new(SOURCE, host.clone()).unwrap();
    let window = Window::new();
    window.set_width(780.0);
    window.set_height(580.0);
    window.set_content(Some(Control::boxed(host)));
    window.show();
    window.update_layout();
    let owner = session.document().ast().find_name("Directory").unwrap();
    let title = session.document().ast().find_name("CustomerName").unwrap();
    session.select([owner]);
    let slots = session.item_template_slots(owner).unwrap();
    assert!(matches!(
        slots[0].value,
        ItemTemplateValue::Resource {
            definition: Some(_),
            ..
        }
    ));
    session
        .preview_item_template(owner, ItemTemplateMember::ItemTemplate)
        .unwrap();
    window.update_layout();
    session.preview.rebuild_map(&session.document);
    assert!(
        session
            .preview_notices()
            .iter()
            .any(|notice| notice.contains("Customers fixture")),
        "{:?}",
        session.preview_notices()
    );
    let text = |session: &crate::DesignerSession| {
        session
            .preview
            .visual(title)
            .and_then(|node| node.cast::<TextBlock>())
            .and_then(|text| text.text())
    };
    assert_eq!(text(&session).as_deref(), Some("Alpine & Co."));
    session.set_design_fixture(Some("Trial".into())).unwrap();
    window.update_layout();
    session.preview.rebuild_map(&session.document);
    assert_eq!(text(&session).as_deref(), Some("Cedar Systems"));
    assert_eq!(session.document().source(), SOURCE);
    session.exit_definition_scope().unwrap();
    assert_eq!(session.selection(), [owner]);
    window.close();
}

#[test]
fn studio_item_template_button_edits_once_and_retires_the_old_action() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(SOURCE).unwrap();
    let window = Window::new();
    window.set_width(1240.0);
    window.set_height(840.0);
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    window.update_layout();
    let session = workspace.session();
    let owner = session
        .borrow()
        .document()
        .ast()
        .find_name("ReviewQueue")
        .unwrap();
    session.borrow_mut().select([owner]);
    workspace.refresh(true);
    let anchor = workspace
        .control()
        .get_control::<Button>("EditItemTemplates");
    assert!(anchor.is_visible());
    let flyout = anchor
        .flyout()
        .and_then(|value| value.cast::<Flyout>())
        .unwrap();
    flyout.show_at(&anchor);
    let body = Ref::<Control>::from_xaml_object(&flyout.content().unwrap()).unwrap();
    let create = body
        .get_logical_descendants()
        .find(|control| {
            AutomationProperties::get_name(control).as_deref() == Some("Create inline template")
        })
        .and_then(|control| control.cast::<Button>())
        .unwrap();
    create.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    let revision = session.borrow().document().revision();
    assert!(matches!(
        session.borrow().item_template_slots(owner).unwrap()[0].value,
        ItemTemplateValue::Inline { .. }
    ));
    create.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(session.borrow().document().revision(), revision);
    session.borrow_mut().undo().unwrap();
    assert_eq!(session.borrow().document().source(), SOURCE);
    flyout.hide();
    window.close();
}

#[test]
fn a_required_tree_grid_cell_template_cannot_be_reset() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = include_str!("../../editors/vscode/test/fixtures/ColumnsWorkbench.xaml");
    let mut session = crate::DesignerSession::new(source, ContentControl::new()).unwrap();
    let node = session
        .document()
        .ast()
        .nodes_in_source_order()
        .find(|node| node.local_name() == "TreeDataGridTemplateColumn")
        .unwrap()
        .id;
    let slots = session.item_template_slots(node).unwrap();
    assert!(slots
        .iter()
        .any(|slot| slot.property == ItemTemplateMember::CellTemplate && !slot.can_reset));
    assert!(session
        .edit_item_template(
            node,
            ItemTemplateMember::CellTemplate,
            ItemTemplateChange::Reset
        )
        .unwrap_err()
        .contains("requires an item template"));
    assert_eq!(session.document().source(), source);
}

#[test]
fn studio_edits_an_included_template_in_its_own_tab_with_the_consumer_fixture() {
    use ferroui_designer_core::{Document, Edit};
    use ferroui_designer_project::{file_uri, Solution};
    use std::{fs, path::PathBuf};

    struct Files(PathBuf);
    impl Drop for Files {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    const VIEW: &str =
        include_str!("../../editors/vscode/test/fixtures/IncludedItemTemplates.xaml");
    const DICTIONARY: &str =
        include_str!("../../editors/vscode/test/fixtures/Templates/Customer Rows.xaml");
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let root = Files(
        std::env::temp_dir().join(format!("ferroui-included-template-{}", std::process::id())),
    );
    fs::create_dir_all(root.0.join("Templates")).unwrap();
    fs::write(root.0.join("Directory.xaml"), VIEW).unwrap();
    let dictionary = root.0.join("Templates/Customer Rows.xaml");
    fs::write(&dictionary, DICTIONARY).unwrap();
    let view_uri = file_uri(root.0.canonicalize().unwrap().join("Directory.xaml")).unwrap();
    let dictionary_uri = file_uri(dictionary.canonicalize().unwrap()).unwrap();
    let workspace = DesignerWorkspace::with_uri(VIEW, view_uri.clone()).unwrap();
    workspace
        .set_solution(Solution::discover(&root.0).unwrap())
        .unwrap();
    let window = Window::new();
    window.set_width(1240.0);
    window.set_height(840.0);
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    window.update_layout();
    let session = workspace.session();
    let owner = session
        .borrow()
        .document()
        .ast()
        .find_name("Directory")
        .unwrap();
    let target = {
        let session = session.borrow();
        let slot = session
            .item_template_slot(owner, ItemTemplateMember::ItemTemplate)
            .unwrap();
        session
            .included_item_template_preview(&slot)
            .unwrap()
            .unwrap()
    };
    assert_eq!(target.source.uri, dictionary_uri);
    let stale = Document::new(DICTIONARY.replace("FontSize=\"16\"", "FontSize=\"17\"")).unwrap();
    assert!(target.source.node_in(&stale, &dictionary_uri).is_err());
    workspace.open_included_template(target).unwrap();
    window.update_layout();
    let mut active = session.borrow_mut();
    assert_eq!(active.document_uri(), dictionary_uri);
    assert_eq!(active.definition_scope().unwrap().title, "CustomerRow");
    let name = active.document().ast().find_name("CustomerName").unwrap();
    active.refresh_runtime_mapping();
    assert_eq!(
        active
            .preview
            .visual(name)
            .and_then(|visual| visual.cast::<TextBlock>())
            .and_then(|text| text.text())
            .as_deref(),
        Some("Alpine & Co.")
    );
    active
        .apply(
            "Change row text size",
            vec![Edit::SetAttribute {
                node: name,
                name: "FontSize".into(),
                value: "24".into(),
            }],
        )
        .unwrap();
    assert!(active
        .authored_source()
        .contains("Name=\"CustomerName\" FontSize=\"24\""));
    active.undo().unwrap();
    assert_eq!(active.authored_source(), DICTIONARY);
    drop(active);
    workspace.return_to_document().unwrap();
    assert_eq!(session.borrow().document_uri(), view_uri);
    assert_eq!(session.borrow().authored_source(), VIEW);
    #[cfg(feature = "mcp")]
    {
        use crate::mcp::{DesignerMcp, Permissions};
        use ferroui_mcp::UiToolProvider;
        use serde_json::json;

        let provider = DesignerMcp::with_workspace(&workspace);
        let read = provider
            .call(
                "designer_item_templates",
                &json!({"node": owner}),
                Permissions::default(),
            )
            .unwrap();
        let revision = session.borrow().document().revision();
        let permission = || Permissions {
            allow_property_writes: true,
            ..Default::default()
        };
        provider
            .call(
                "designer_item_template",
                &json!({
                    "node": owner, "property": "ItemTemplate", "action": "preview",
                    "expected_revision": revision, "expected_model": read["model"],
                }),
                permission(),
            )
            .unwrap();
        assert_eq!(session.borrow().document_uri(), dictionary_uri);
        let revision = session.borrow().document().revision();
        provider
            .call(
                "designer_definition",
                &json!({"action": "exit", "expected_revision": revision}),
                permission(),
            )
            .unwrap();
        assert_eq!(session.borrow().document_uri(), view_uri);
    }
    window.close();
}
