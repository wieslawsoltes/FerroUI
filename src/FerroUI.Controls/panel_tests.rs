use crate::{Control, Panel};
use ferroui_base::{Ref, StyledElement, Visual};
use std::cell::RefCell;
use std::rc::Rc;

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

/// Not from the reference: the panel removes a child at its index in the lists that mirror its
/// children, and reports each removal as the reference does (desktop-performance.md, 9.80).
#[test]
fn children_removed_one_at_a_time_and_as_a_range_leave_the_lists_in_step() {
    let panel = Panel::new();
    let children: Vec<Ref<Control>> = (0..6).map(|_| Control::new()).collect();
    panel.children().add_range(children.iter().cloned());
    let removals = Rc::new(RefCell::new(Vec::new()));
    let seen = removals.clone();
    panel.visual_children().add_collection_changed(Rc::new(move |e| {
        seen.borrow_mut().push((e.old_starting_index, e.old_items.len()));
    }));

    panel.children().remove_at(4);
    panel.children().remove(&children[1]);
    panel.children().remove_range(1, 2);

    let left = [&children[0], &children[5]];
    assert_eq!(panel.logical_children().to_vec(), logical(&left));
    assert_eq!(panel.visual_children().to_vec(), visual(&left));
    assert_eq!(*removals.borrow(), [(4, 1), (1, 1), (1, 2)]);
    assert!(children[4].parent().is_none() && children[4].visual_parent().is_none());
}

/// A list that does not have the children where the panel has them (a derived panel that keeps
/// a visual of its own in front of them) still loses the child, wherever it is.
#[test]
fn a_child_is_removed_from_a_list_that_has_it_at_another_index() {
    let panel = Panel::new();
    let children: Vec<Ref<Control>> = (0..3).map(|_| Control::new()).collect();
    panel.children().add_range(children.iter().cloned());
    let own = Control::new();
    panel.visual_children().insert(0, own.clone().upcast::<Visual>());

    panel.children().remove_at(1);
    panel.children().remove_at(1);

    assert_eq!(panel.logical_children().to_vec(), logical(&[&children[0]]));
    assert_eq!(panel.visual_children().to_vec(), visual(&[&own, &children[0]]));
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
