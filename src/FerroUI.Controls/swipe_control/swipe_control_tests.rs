//! Tests of the swipe control (#443, SW-1), against WinUI's `SwipeControl`, `SwipeItems` and
//! `SwipeItem`. The states of the interaction tracker are driven as WinUI's tracker owner reports
//! them (the tests of the source use the same entry points through their test hooks).

use super::*;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use crate::test_support::test_scope;
use crate::presenters::ContentPresenter;
use crate::{CommandBarButton, Grid, StackPanel};
use ferroui_base::input::ICommand;
use ferroui_base::numerics::Vector3;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::{BoxedValue, Ref};
use std::cell::Cell;
use std::rc::Rc;

fn x(value: f32) -> Vector3 {
    Vector3::new(value, 0.0, 0.0)
}

/// A swipe control with WinUI's template parts.
fn swipe_control() -> Ref<SwipeControl> {
    let control = SwipeControl::new();
    control.set_template(Some(FuncControlTemplate::new(|_, scope| {
        let stack_panel = StackPanel::new();
        stack_panel.set_name(Some("SwipeContentStackPanel".to_string()));
        let swipe_content_root = Grid::new();
        swipe_content_root.set_name(Some("SwipeContentRoot".to_string()));
        swipe_content_root.children().add(stack_panel.register_in_name_scope(&**scope));
        let presenter = ContentPresenter::new();
        presenter.set_name(Some("ContentPresenter".to_string()));
        let content_root = Grid::new();
        content_root.set_name(Some("ContentRoot".to_string()));
        content_root.children().add(presenter.register_in_name_scope(&**scope));
        let root = Grid::new();
        root.set_name(Some("RootGrid".to_string()));
        root.children().add(swipe_content_root.register_in_name_scope(&**scope));
        root.children().add(content_root.register_in_name_scope(&**scope));
        root.register_in_name_scope(&**scope).upcast()
    })));
    control.apply_template();
    control
}

fn stack_panel_children(control: &SwipeControl) -> Vec<Ref<CommandBarButton>> {
    let root = control.visual_children().snapshot()[0].cast::<Grid>().unwrap();
    let swipe_content_root = root.children().snapshot()[0].cast::<Grid>().unwrap();
    let panel = swipe_content_root.children().snapshot()[0].cast::<StackPanel>().unwrap();
    panel.children().snapshot().iter().filter_map(|child| child.cast::<CommandBarButton>()).collect()
}

fn items(mode: SwipeMode, texts: &[&str]) -> Ref<SwipeItems> {
    let items = SwipeItems::with_mode(mode);
    for text in texts {
        items.add(SwipeItem::with_text(*text));
    }
    items
}

/// A command that counts its runs.
struct CountingCommand(Cell<usize>);

impl ICommand for CountingCommand {
    fn can_execute(&self, _parameter: Option<&BoxedValue>) -> bool {
        true
    }

    fn execute(&self, _parameter: Option<&BoxedValue>) {
        self.0.set(self.0.get() + 1);
    }

    fn can_execute_changed(&self, _handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
}

#[test]
fn execute_items_have_one_item() {
    let _scope = test_scope();
    let execute = SwipeItems::with_mode(SwipeMode::Execute);
    assert!(execute.try_add(SwipeItem::new()).is_ok());
    assert_eq!(execute.try_add(SwipeItem::new()), Err(ExecuteItemsHaveOneItem));
    execute.add(SwipeItem::new());
    assert_eq!(execute.count(), 1);
    let reveal = items(SwipeMode::Reveal, &["a", "b"]);
    assert_eq!(reveal.try_set_mode(SwipeMode::Execute), Err(ExecuteItemsHaveOneItem));
    assert_eq!(reveal.mode(), SwipeMode::Reveal);
}

#[test]
fn the_defaults_are_those_of_winui() {
    let _scope = test_scope();
    let item = SwipeItem::new();
    assert_eq!(item.behavior_on_invoked(), SwipeBehaviorOnInvoked::Auto);
    assert_eq!(item.text(), "");
    assert_eq!(SwipeItems::new().mode(), SwipeMode::Reveal);
    let control = SwipeControl::new();
    assert!(control.left_items().is_none());
    assert!(!control.is_open());
    assert!(control.is_idle());
}

#[test]
fn swiping_creates_the_items_of_the_side_and_resting_open_keeps_them() {
    let _scope = test_scope();
    let control = swipe_control();
    control.set_left_items(Some(items(SwipeMode::Reveal, &["Archive", "Flag"])));
    control.set_right_items(Some(items(SwipeMode::Reveal, &["Delete"])));

    control.interacting_state_entered();
    assert!(!control.is_idle());
    // A negative position shows the left items.
    control.values_changed(x(-30.0));
    assert_eq!(control.created_content(), swipe_control::CreatedContent::Left);
    let buttons = stack_panel_children(&control);
    let labels: Vec<String> = buttons.iter().map(|button| button.label().unwrap_or_default()).collect();
    assert_eq!(labels, ["Archive", "Flag"]);
    // The other way: the right items replace them.
    control.values_changed(x(30.0));
    assert_eq!(control.created_content(), swipe_control::CreatedContent::Right);
    assert_eq!(stack_panel_children(&control).len(), 1);

    control.inertia_state_entered(x(80.0), x(68.0));
    assert!(control.is_open());
    control.idle_state_entered(x(68.0));
    assert!(control.is_open());
    assert!(control.is_idle());
    assert_eq!(stack_panel_children(&control).len(), 1);

    // Resting closed takes the items away.
    control.interacting_state_entered();
    control.inertia_state_entered(x(40.0), x(0.0));
    control.idle_state_entered(x(0.0));
    assert!(!control.is_open());
    assert_eq!(control.created_content(), swipe_control::CreatedContent::None);
    assert!(stack_panel_children(&control).is_empty());
}

#[test]
fn a_flick_to_the_other_side_closes() {
    let _scope = test_scope();
    let control = swipe_control();
    control.set_left_items(Some(items(SwipeMode::Reveal, &["Archive"])));
    control.set_right_items(Some(items(SwipeMode::Reveal, &["Delete"])));
    control.interacting_state_entered();
    control.values_changed(x(-30.0));
    control.inertia_state_entered(x(-30.0), x(68.0));
    assert!(!control.is_open());
}

#[test]
fn execute_items_invoke_their_item_when_they_come_to_rest_open() {
    let _scope = test_scope();
    let control = swipe_control();
    let execute = items(SwipeMode::Execute, &["Delete"]);
    let item = execute.get(0).unwrap();
    let command = Rc::new(CountingCommand(Cell::new(0)));
    item.set_command(Some(command.clone()));
    let invoked = Rc::new(Cell::new(0));
    let count = invoked.clone();
    let sender = control.clone();
    item.invoked(move |_, args| {
        assert_eq!(args.swipe_control(), sender);
        count.set(count.get() + 1);
    })
    .detach();
    control.set_right_items(Some(execute));

    control.interacting_state_entered();
    // Past the threshold (here the size of the items, which have no size without a layout).
    control.values_changed(x(150.0));
    assert!(control.is_threshold_reached());
    control.inertia_state_entered(x(150.0), x(150.0));
    control.idle_state_entered(x(150.0));
    assert_eq!(invoked.get(), 1);
    assert_eq!(command.0.get(), 1);
    // Auto closes: without a compositor at once.
    assert!(!control.is_open());
}

#[test]
fn invoking_an_item_closes_unless_it_remains_open() {
    let _scope = test_scope();
    let control = swipe_control();
    let reveal = items(SwipeMode::Reveal, &["Flag"]);
    control.set_left_items(Some(reveal.clone()));
    control.interacting_state_entered();
    control.values_changed(x(-80.0));
    control.inertia_state_entered(x(-80.0), x(-68.0));
    control.idle_state_entered(x(-68.0));
    assert!(control.is_open());
    let item = reveal.get(0).unwrap();
    item.set_behavior_on_invoked(SwipeBehaviorOnInvoked::RemainOpen);
    item.invoke_swipe(&control);
    assert!(control.is_open());
    item.set_behavior_on_invoked(SwipeBehaviorOnInvoked::Close);
    item.invoke_swipe(&control);
    assert!(!control.is_open());
}

#[test]
fn interacting_with_another_control_closes_the_last_one() {
    let _scope = test_scope();
    let first = swipe_control();
    first.set_left_items(Some(items(SwipeMode::Reveal, &["Flag"])));
    first.interacting_state_entered();
    first.values_changed(x(-80.0));
    first.inertia_state_entered(x(-80.0), x(-68.0));
    first.idle_state_entered(x(-68.0));
    assert!(first.is_open());
    assert_eq!(SwipeControl::last_interacted_with_swipe_control(), Some(first.clone()));

    let second = swipe_control();
    second.set_left_items(Some(items(SwipeMode::Reveal, &["Flag"])));
    second.interacting_state_entered();
    second.values_changed(x(-10.0));
    assert!(!first.is_open());
    assert_eq!(SwipeControl::last_interacted_with_swipe_control(), Some(second));
}

#[test]
fn vertical_items_swipe_on_the_vertical_axis() {
    let _scope = test_scope();
    let control = swipe_control();
    control.set_top_items(Some(items(SwipeMode::Reveal, &["Up"])));
    control.interacting_state_entered();
    control.values_changed(Vector3::new(0.0, -20.0, 0.0));
    assert_eq!(control.created_content(), swipe_control::CreatedContent::Top);
    // The horizontal axis does nothing.
    control.values_changed(Vector3::new(40.0, -20.0, 0.0));
    assert_eq!(control.created_content(), swipe_control::CreatedContent::Top);
}
