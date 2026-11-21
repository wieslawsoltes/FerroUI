use super::*;
use ferroui_controls::{automation::AutomationProperties, Window};
use ferroui_designer_animation::BehaviorSource;

const SOURCE: &str = include_str!("../../editors/vscode/test/fixtures/BehaviorsWorkbench.xaml");

#[test]
fn behavior_preview_recovers_when_an_action_fails_after_a_successful_step() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = SOURCE.replace(
        "PropertyName=\"Width\" Value='338'",
        "PropertyName=\"MissingBehaviorField\" Value='338'",
    );
    let workspace = DesignerWorkspace::new(&source).unwrap();
    let session = workspace.session();
    let owner = session
        .borrow()
        .document()
        .ast()
        .find_name("Approve")
        .unwrap();
    let window = Window::new();
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    window.update_layout();
    let behavior = session.borrow().behaviors(owner)[0].id.unwrap();
    let error = session.borrow_mut().preview_behavior(behavior).unwrap_err();
    assert!(error.contains("MissingBehaviorField"), "{error}");
    assert_eq!(session.borrow().active_behavior_preview(), None);
    assert_eq!(session.borrow().authored_source(), source);
    let button = session
        .borrow()
        .preview
        .visual(owner)
        .unwrap()
        .cast::<Button>()
        .unwrap();
    assert_eq!(button.width(), 210.0);
    window.close();
}

#[test]
fn behavior_inspector_previews_and_reorders_native_actions_in_one_transaction() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(SOURCE).unwrap();
    let session = workspace.session();
    let owner = session
        .borrow()
        .document()
        .ast()
        .find_name("Approve")
        .unwrap();
    let window = Window::new();
    window.set_width(1250.0);
    window.set_height(820.0);
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    window.update_layout();
    assert!(matches!(
        session.borrow().preview_status(),
        PreviewStatus::Ready { .. }
    ));
    session.borrow_mut().select([owner]);
    workspace.refresh(true);
    click(&workspace.control().get_control::<Button>("BehaviorsMode"));

    let toolbar = |label: &str| {
        workspace
            .control()
            .get_control::<StackPanel>("AuthoringPanel")
            .get_logical_descendants()
            .find_map(|control| {
                control.cast::<Button>().filter(|button| {
                    AutomationProperties::get_name(button).as_deref() == Some(label)
                })
            })
            .unwrap_or_else(|| panic!("Missing behavior action {label}"))
    };
    let width = || {
        session
            .borrow()
            .preview
            .visual(owner)
            .unwrap()
            .cast::<Button>()
            .unwrap()
            .width()
    };
    assert_eq!(width(), 210.0);
    click(&toolbar("Preview actions"));
    assert_eq!(width(), 338.0);
    assert_eq!(session.borrow().authored_source(), SOURCE);
    click(&toolbar("Reset behavior preview"));
    assert_eq!(width(), 210.0);

    let move_later = toolbar("Move action later");
    click(&move_later);
    let changed = session.borrow().authored_source().to_owned();
    assert!(changed.find("Value='338'").unwrap() < changed.find("Value='278'").unwrap());
    assert!(changed.contains("<!-- Keep the review and archive steps together -->"));
    let sources = BehaviorSource::read(session.borrow().document(), owner).unwrap();
    assert_eq!(sources[0].actions.len(), 3);
    click(&move_later);
    assert_eq!(session.borrow().authored_source(), changed);
    click(&toolbar("Preview actions"));
    assert_eq!(width(), 278.0);
    click(&toolbar("Reset behavior preview"));
    session.borrow_mut().undo().unwrap();
    assert_eq!(session.borrow().authored_source(), SOURCE);
    assert_eq!(width(), 210.0);
    window.close();
}
