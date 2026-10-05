use crate::{Control, Panel};
use ferroui_base::{Ref, StyledElement, Visual};

fn logical(children: &[&Ref<Control>]) -> Vec<Ref<StyledElement>> {
    children.iter().map(|c| (*c).clone().upcast()).collect()
}

fn visual(children: &[&Ref<Control>]) -> Vec<Ref<Visual>> {
    children.iter().map(|c| (*c).clone().upcast()).collect()
}

#[test]
fn adding_control_to_panel_should_set_child_controls_parent() {
    let panel = Panel::new();
    let child = Control::new();

    panel.children().add(&child);

    assert_eq!(child.parent().unwrap(), panel);
    assert_eq!(child.visual_parent().unwrap(), panel);
}

#[test]
fn removing_control_from_panel_should_clear_child_controls_parent() {
    let panel = Panel::new();
    let child = Control::new();

    panel.children().add(&child);
    panel.children().remove(&child);

    assert!(child.parent().is_none());
    assert!(child.visual_parent().is_none());
}

#[test]
fn clearing_panel_children_should_clear_child_controls_parent() {
    let panel = Panel::new();
    let child1 = Control::new();
    let child2 = Control::new();

    panel.children().add(&child1);
    panel.children().add(&child2);
    panel.children().clear();

    assert!(child1.parent().is_none());
    assert!(child1.visual_parent().is_none());
    assert!(child2.parent().is_none());
    assert!(child2.visual_parent().is_none());
}

#[test]
fn replacing_panel_children_should_clear_and_set_control_parent() {
    let panel = Panel::new();
    let child1 = Control::new();
    let child2 = Control::new();

    panel.children().add(&child1);
    panel.children().set(0, &child2);

    assert!(child1.parent().is_none());
    assert!(child1.visual_parent().is_none());
    assert_eq!(child2.parent().unwrap(), panel);
    assert_eq!(child2.visual_parent().unwrap(), panel);
}

#[test]
fn child_control_should_appear_in_panel_logical_and_visual_children() {
    let panel = Panel::new();
    let child = Control::new();

    panel.children().add(&child);

    assert_eq!(panel.children().to_vec(), vec![child.clone()]);
    assert_eq!(panel.logical_children().to_vec(), logical(&[&child]));
    assert_eq!(panel.visual_children().to_vec(), visual(&[&child]));
}

#[test]
fn removing_child_control_should_remove_from_panel_logical_and_visual_children() {
    let panel = Panel::new();
    let child = Control::new();

    panel.children().add(&child);
    panel.children().remove(&child);

    assert!(panel.children().is_empty());
    assert!(panel.logical_children().is_empty());
    assert!(panel.visual_children().is_empty());
}

#[test]
fn moving_panel_children_should_reoder_logical_and_visual_children() {
    let panel = Panel::new();
    let child1 = Control::new();
    let child2 = Control::new();

    panel.children().add(&child1);
    panel.children().add(&child2);
    panel.children().move_item(1, 0);

    assert_eq!(panel.logical_children().to_vec(), logical(&[&child2, &child1]));
    assert_eq!(panel.visual_children().to_vec(), visual(&[&child2, &child1]));
}

#[test]
fn adding_control_to_items_host_panel_should_not_affect_logical_children() {
    // The reference test uses a content control as the real parent; any
    // logical parent shows the same behaviour.
    let child = Control::new();
    let real_parent = Control::new();
    real_parent.logical_children().add(child.clone().upcast());
    let panel = Panel::new();
    panel.set_is_items_host(true);

    panel.children().add(&child);

    assert!(panel.logical_children().is_empty());
    assert_eq!(child.parent().unwrap(), real_parent);
    assert_eq!(child.visual_parent().unwrap(), panel);
}

#[test]
fn child_index_provider_reports_children_and_changes() {
    use ferroui_base::logical_tree::ChildIndexChangedAction;
    use std::cell::RefCell;
    use std::rc::Rc;

    let panel = Panel::new();
    let child1 = Control::new();
    let child2 = Control::new();
    let provider = panel.child_index_provider().unwrap();
    let actions = Rc::new(RefCell::new(Vec::new()));
    let recorded = actions.clone();
    let subscription = provider.child_index_changed(Rc::new(move |e| recorded.borrow_mut().push(e.action())));

    panel.children().add(&child1);
    panel.children().add(&child2);

    assert_eq!(provider.get_child_index(&child2), 1);
    assert_eq!(provider.try_get_total_count(), Some(2));
    assert_eq!(
        *actions.borrow(),
        vec![
            ChildIndexChangedAction::ChildIndexesReset,
            ChildIndexChangedAction::TotalCountChanged,
            ChildIndexChangedAction::ChildIndexesReset,
            ChildIndexChangedAction::TotalCountChanged,
        ]
    );

    subscription.dispose();
    panel.children().clear();
    assert_eq!(actions.borrow().len(), 4);
}
