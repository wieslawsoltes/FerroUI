use super::*;
use ferroui_base::{Ref, interactivity::RoutedEventArgs};
use ferroui_controls::{
    ContentControl, Window,
    automation::AutomationProperties,
    testing::{TestServices, UnitTestApplication},
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
    assert!(
        slots
            .iter()
            .any(|slot| slot.property == ItemTemplateMember::CellTemplate && !slot.can_reset)
    );
    assert!(
        session
            .edit_item_template(
                node,
                ItemTemplateMember::CellTemplate,
                ItemTemplateChange::Reset
            )
            .unwrap_err()
            .contains("requires an item template")
    );
    assert_eq!(session.document().source(), source);
}
