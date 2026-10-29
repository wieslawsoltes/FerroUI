//! Ported from `tests/Xaml.Behaviors.Interactions.UnitTests/Core/MultiDataTriggerBehaviorTests.cs`
//! of Xaml.Behaviors (with its windows `MultiDataTriggerBehavior001.xaml` to
//! `MultiDataTriggerBehavior003.xaml`, loaded with the run-time loader).
//!
//! Adapted: the jobs of the dispatcher run after showing the window and after each input (the
//! headless platform of the reference runs them as it processes the input). The rendered frames
//! the reference saves are not taken.

use crate::tests::support::{boxed, click, find, key_press, load, run_jobs, start_app, text, window_with};
use crate::{ChangePropertyAction, MultiDataTriggerBehavior};
use ferroui_base::input::{InputElement, Key};
use ferroui_base::{
    ferro_class, instantiate, BoxedValue, FerroObject, FerroObjectImpl, Ref, StyledElementImpl,
};
use ferroui_controls::{Button, CheckBox, Control, Slider, TextBlock, Window};
use ferroui_xaml_interactivity::{
    ActionCollection, BehaviorCollection, Condition, ConditionCollection, IAction, IReversibleAction, Interaction,
    StyledElementAction, StyledElementActionImpl,
};
use std::cell::Cell;
use std::rc::Rc;

/// `ExecutionPathAction`: a reversible action that counts how it is executed.
#[repr(C)]
pub(crate) struct ExecutionPathAction {
    base: StyledElementAction,
    legacy_execution_count: Cell<i32>,
    reversible_execution_count: Cell<i32>,
}

ferro_class!(ExecutionPathAction: StyledElementAction);
ferroui_base::ferro_impl_classes!(ExecutionPathAction: FerroObjectImpl, StyledElementImpl);

impl StyledElementActionImpl for ExecutionPathAction {
    fn execute(this: &Self, _sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        this.legacy_execution_count.set(this.legacy_execution_count.get() + 1);
        Some(Rc::new(true))
    }
}

impl ExecutionPathAction {
    fn static_constructor() {
        <dyn IReversibleAction>::register::<ExecutionPathAction>();
    }

    fn new() -> Ref<Self> {
        instantiate(Self {
            base: StyledElementAction::construct(),
            legacy_execution_count: Cell::new(0),
            reversible_execution_count: Cell::new(0),
        })
    }

    fn legacy_execution_count(&self) -> i32 {
        self.legacy_execution_count.get()
    }

    fn reversible_execution_count(&self) -> i32 {
        self.reversible_execution_count.get()
    }
}

impl IAction for ExecutionPathAction {
    fn execute(&self, sender: Option<&BoxedValue>, parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        StyledElementAction::execute(self, sender, parameter)
    }
}

impl IReversibleAction for ExecutionPathAction {
    fn execute_reversibly(&self, _sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        self.reversible_execution_count.set(self.reversible_execution_count.get() + 1);
        Some(Rc::new(true))
    }

    fn revert(&self, _sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        Some(Rc::new(true))
    }
}

/// `CreateBehavior(revertOnFalse, action)`: a trigger on the enabled state of its button.
fn create_behavior(revert_on_false: bool, action: &Ref<ExecutionPathAction>) -> Ref<MultiDataTriggerBehavior> {
    let behavior = MultiDataTriggerBehavior::new();
    behavior.set_revert_on_false(revert_on_false);
    let conditions = ConditionCollection::new();
    let condition = Condition::new();
    condition.set_property(Some(InputElement::is_enabled_property().as_property()));
    condition.set_condition_value(Some(Rc::new(true)));
    conditions.add(condition);
    behavior.set_conditions(Some(conditions));
    let actions = ActionCollection::new();
    actions.add(action.clone().upcast());
    behavior.set_actions(Some(actions));
    behavior
}

/// The one behavior of an element (`Assert.Single(Interaction.GetBehaviors(element))`), as a
/// trigger on several conditions.
fn single_multi_data_trigger(behaviors: &BehaviorCollection) -> Ref<MultiDataTriggerBehavior> {
    assert_eq!(1, behaviors.count());
    behaviors.get(0).cast::<MultiDataTriggerBehavior>().expect("a trigger on several conditions")
}

fn text_of(window: &Window) -> Option<String> {
    find::<TextBlock>(window, "TargetTextBlock").text()
}

/// Clicks the check box of a window and runs the jobs the click posted.
fn click_check_box(window: &Window) {
    click(&find::<CheckBox>(window, "TargetCheckBox"));
    run_jobs();
}

/// Moves the slider of a window one small change to the right and runs the jobs it posted.
fn press_right(window: &Window) {
    key_press(&find::<Slider>(window, "TargetSlider"), Key::Right);
    run_jobs();
}

fn is_checked(window: &Window) -> Option<bool> {
    find::<CheckBox>(window, "TargetCheckBox").is_checked()
}

fn slider_value(window: &Window) -> f64 {
    find::<Slider>(window, "TargetSlider").value()
}

/// The checks of `MultiDataTriggerBehavior_001` and `MultiDataTriggerBehavior_002_Property_Conditions`.
fn check_ready_sequence(window: &Window) {
    window.show();
    run_jobs();

    assert_eq!(Some("Move slider and check".to_string()), text_of(window));
    assert_eq!(Some(false), is_checked(window));
    assert_eq!(0.0, slider_value(window));

    click_check_box(window);

    assert_eq!(Some("Slider too low".to_string()), text_of(window));
    assert_eq!(Some(true), is_checked(window));

    press_right(window);
    press_right(window);

    assert_eq!(Some("Ready".to_string()), text_of(window));
    assert_eq!(50.0, slider_value(window));

    click_check_box(window);

    assert_eq!(Some("Checkbox unchecked".to_string()), text_of(window));
    assert_eq!(Some(false), is_checked(window));
}

/// Shows `MultiDataTriggerBehavior003` and meets its conditions (the check box checked, the slider
/// at 50).
fn show_003_and_meet_conditions() -> Ref<Window> {
    let window = load::<Window>("MultiDataTriggerBehavior003.xaml", include_str!("MultiDataTriggerBehavior003.xaml"));
    window.show();
    run_jobs();
    click_check_box(&window);
    press_right(&window);
    press_right(&window);
    window
}

#[test]
fn multi_data_trigger_behavior_001() {
    let _app = start_app();
    let window = load::<Window>("MultiDataTriggerBehavior001.xaml", include_str!("MultiDataTriggerBehavior001.xaml"));

    check_ready_sequence(&window);
}

#[test]
fn multi_data_trigger_behavior_002_property_conditions() {
    let _app = start_app();
    let window = load::<Window>("MultiDataTriggerBehavior002.xaml", include_str!("MultiDataTriggerBehavior002.xaml"));

    check_ready_sequence(&window);
}

#[test]
fn multi_data_trigger_behavior_003_reverts_on_false_when_enabled() {
    let _app = start_app();
    let window = load::<Window>("MultiDataTriggerBehavior003.xaml", include_str!("MultiDataTriggerBehavior003.xaml"));

    window.show();
    run_jobs();

    assert_eq!(Some("Red".to_string()), text_of(&window));
    assert_eq!(Some(false), is_checked(&window));
    assert_eq!(0.0, slider_value(&window));

    click_check_box(&window);
    press_right(&window);
    press_right(&window);

    assert_eq!(Some("Green".to_string()), text_of(&window));
    assert_eq!(Some(true), is_checked(&window));
    assert_eq!(50.0, slider_value(&window));

    click_check_box(&window);

    assert_eq!(Some("Red".to_string()), text_of(&window));
    assert_eq!(Some(false), is_checked(&window));
}

#[test]
fn multi_data_trigger_behavior_uses_execution_path_selected_by_revert_on_false() {
    let _app = start_app();
    let legacy_action = ExecutionPathAction::new();
    let reversible_action = ExecutionPathAction::new();
    let button = Button::new();
    button.set_is_enabled(true);
    let behaviors = BehaviorCollection::new();
    behaviors.add(create_behavior(false, &legacy_action).upcast());
    behaviors.add(create_behavior(true, &reversible_action).upcast());
    Interaction::set_behaviors(&button, Some(behaviors));
    let window = window_with(Some(button.upcast::<Control>()));

    window.show();
    run_jobs();

    assert!(legacy_action.legacy_execution_count() > 0);
    assert_eq!(0, legacy_action.reversible_execution_count());
    assert_eq!(0, reversible_action.legacy_execution_count());
    assert!(reversible_action.reversible_execution_count() > 0);
    window.close();
}

#[test]
fn multi_data_trigger_behavior_003_clears_reversible_state_when_disabled() {
    let _app = start_app();
    let window = show_003_and_meet_conditions();
    let target_text_block = find::<TextBlock>(&window, "TargetTextBlock");
    let behavior = single_multi_data_trigger(&Interaction::get_behaviors(&target_text_block));
    assert_eq!(Some("Green".to_string()), target_text_block.text());

    behavior.set_revert_on_false(false);
    run_jobs();
    target_text_block.set_text(Some("External"));
    behavior.set_revert_on_false(true);
    run_jobs();

    assert_eq!(Some("Green".to_string()), target_text_block.text());
    click_check_box(&window);
    assert_eq!(Some("External".to_string()), target_text_block.text());
    window.close();
}

#[test]
fn multi_data_trigger_behavior_003_reconciles_reversible_state_when_is_enabled_changes() {
    let _app = start_app();
    let window = show_003_and_meet_conditions();
    let target_text_block = find::<TextBlock>(&window, "TargetTextBlock");
    let behavior = single_multi_data_trigger(&Interaction::get_behaviors(&target_text_block));
    assert_eq!(Some("Green".to_string()), target_text_block.text());

    behavior.set_is_enabled(false);
    assert_eq!(Some("Red".to_string()), target_text_block.text());

    behavior.set_is_enabled(true);
    run_jobs();
    assert_eq!(Some("Green".to_string()), target_text_block.text());

    behavior.set_is_enabled(false);
    click_check_box(&window);
    behavior.set_is_enabled(true);
    run_jobs();

    assert_eq!(Some("Red".to_string()), target_text_block.text());
    window.close();
}

#[test]
fn multi_data_trigger_behavior_does_not_execute_replaced_actions_in_legacy_mode() {
    let _app = start_app();
    let initial_action = ExecutionPathAction::new();
    let replacement_action = ExecutionPathAction::new();
    let behavior = create_behavior(false, &initial_action);
    let button = Button::new();
    button.set_is_enabled(true);
    let behaviors = BehaviorCollection::new();
    behaviors.add(behavior.clone().upcast());
    Interaction::set_behaviors(&button, Some(behaviors));
    let window = window_with(Some(button.upcast::<Control>()));
    window.show();
    run_jobs();
    assert!(initial_action.legacy_execution_count() > 0);

    let replacement_actions = ActionCollection::new();
    replacement_actions.add(replacement_action.clone().upcast());
    behavior.set_actions(Some(replacement_actions));
    run_jobs();

    assert_eq!(0, replacement_action.legacy_execution_count());
    assert_eq!(0, replacement_action.reversible_execution_count());
    window.close();
}

#[test]
fn multi_data_trigger_behavior_003_reverts_and_clears_state_when_removed() {
    let _app = start_app();
    let window = show_003_and_meet_conditions();
    let target_text_block = find::<TextBlock>(&window, "TargetTextBlock");
    let behaviors = Interaction::get_behaviors(&target_text_block);
    let behavior = single_multi_data_trigger(&behaviors);
    assert_eq!(Some("Green".to_string()), target_text_block.text());

    let behavior_object: Ref<FerroObject> = behavior.clone().upcast();
    behaviors.remove(&behavior_object);

    assert_eq!(Some("Red".to_string()), target_text_block.text());
    let later_action = ChangePropertyAction::new();
    later_action.set_property_name(Some("Text".to_string()));
    later_action.set_action_value(Some(text("Blue")));
    let sender = boxed(&target_text_block);
    assert!(IReversibleAction::execute_reversibly(&*later_action, Some(&sender), None)
        .and_then(|result| result.downcast_ref::<bool>().copied())
        .unwrap_or(false));
    assert!(later_action
        .revert(Some(&sender), None)
        .and_then(|result| result.downcast_ref::<bool>().copied())
        .unwrap_or(false));
    assert_eq!(Some("Red".to_string()), target_text_block.text());
    window.close();
}

#[test]
fn multi_data_trigger_behavior_003_reconciles_actions_while_active() {
    let _app = start_app();
    let window = show_003_and_meet_conditions();
    let target_text_block = find::<TextBlock>(&window, "TargetTextBlock");
    let behavior = single_multi_data_trigger(&Interaction::get_behaviors(&target_text_block));
    assert_eq!(Some("Green".to_string()), target_text_block.text());

    let actions = behavior.actions().expect("the actions");
    actions.clear();
    run_jobs();
    assert_eq!(Some("Red".to_string()), target_text_block.text());

    let action = ChangePropertyAction::new();
    action.set_property_name(Some("Text".to_string()));
    action.set_action_value(Some(text("Blue")));
    actions.add(action.upcast());
    run_jobs();
    assert_eq!(Some("Blue".to_string()), target_text_block.text());

    click_check_box(&window);
    assert_eq!(Some("Red".to_string()), target_text_block.text());
    window.close();
}
