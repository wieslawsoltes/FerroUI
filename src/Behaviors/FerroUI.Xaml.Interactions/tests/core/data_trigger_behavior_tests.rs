//! Ported from `tests/Xaml.Behaviors.Interactions.UnitTests/Core/DataTriggerBehaviorTests.cs` of
//! Xaml.Behaviors (with its windows `DataTriggerBehavior001.xaml` to `DataTriggerBehavior004.xaml`,
//! loaded with the run-time loader).
//!
//! Adapted: the jobs of the dispatcher run after showing the window and after each input (the
//! headless platform of the reference runs them as it processes the input); the data context of
//! `DataTriggerBehavior004` is set by the test (the window of the reference creates it in its
//! document). The rendered frames the reference saves are not taken.

use super::command::Command;
use crate::tests::support::{boxed, click, find, key_press, load, panics, run_jobs, start_app, text, window_with};
use crate::{ChangePropertyAction, DataTriggerBehavior, InvokeCommandAction};
use ferroui_base::input::Key;
use ferroui_base::{
    ferro_class, ferro_properties, instantiate, BoxedValue, FerroObject, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty,
};
use ferroui_controls::{Border, Button, CheckBox, Control, Slider, TextBlock, TextBox, Window};
use ferroui_xaml_interactivity::{
    ActionCollection, BehaviorCollection, ComparisonConditionType, IReversibleAction, Interaction, StyledElementAction,
    StyledElementActionImpl,
};
use std::cell::Cell;
use std::rc::Rc;

/// `CountingAction`: an action that counts its executions.
#[repr(C)]
pub(crate) struct CountingAction {
    base: StyledElementAction,
    execution_count: Cell<i32>,
}

ferro_class!(CountingAction: StyledElementAction);
ferroui_base::ferro_impl_classes!(CountingAction: FerroObjectImpl, StyledElementImpl);

impl StyledElementActionImpl for CountingAction {
    fn execute(this: &Self, _sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        this.execution_count.set(this.execution_count.get() + 1);
        Some(Rc::new(true))
    }
}

impl CountingAction {
    fn new() -> Ref<Self> {
        instantiate(Self { base: StyledElementAction::construct(), execution_count: Cell::new(0) })
    }

    fn execution_count(&self) -> i32 {
        self.execution_count.get()
    }
}

/// `DataTriggerBehavior004BindingSource`: the data context of `DataTriggerBehavior004`.
#[repr(C)]
pub(crate) struct DataTriggerBehavior004BindingSource {
    base: FerroObject,
}

ferro_class!(DataTriggerBehavior004BindingSource: FerroObject);
ferroui_base::ferro_impl_classes!(DataTriggerBehavior004BindingSource: FerroObjectImpl);

ferro_properties! {
    impl DataTriggerBehavior004BindingSource {
        pub fn test_property_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<DataTriggerBehavior004BindingSource, _>("TestProperty", None)
        }
    }
}

impl DataTriggerBehavior004BindingSource {
    fn new() -> Ref<Self> {
        instantiate(Self { base: FerroObject::construct() })
    }

    fn set_test_property(&self, value: Option<&str>) {
        self.set_value(Self::test_property_property(), value.map(str::to_string))
    }
}

/// `new DataTriggerBehavior004()`: the window, with its data context
/// (`<local:DataTriggerBehavior004BindingSource TestProperty="Initial value" />`).
fn data_trigger_behavior_004() -> (Ref<Window>, Ref<DataTriggerBehavior004BindingSource>) {
    let window = load::<Window>("DataTriggerBehavior004.xaml", include_str!("DataTriggerBehavior004.xaml"));
    let source = DataTriggerBehavior004BindingSource::new();
    source.set_test_property(Some("Initial value"));
    window.set_data_context(Some(boxed(&source)));
    (window, source)
}

/// The one behavior of an element (`Assert.Single(Interaction.GetBehaviors(element))`), as a data
/// trigger.
fn single_data_trigger(behaviors: &BehaviorCollection) -> Ref<DataTriggerBehavior> {
    assert_eq!(1, behaviors.count());
    behaviors.get(0).cast::<DataTriggerBehavior>().expect("a data trigger")
}

fn text_of(window: &Window, name: &str) -> Option<String> {
    find::<TextBlock>(window, name).text()
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

#[test]
fn data_trigger_behavior_001() {
    let _app = start_app();
    let window = load::<Window>("DataTriggerBehavior001.xaml", include_str!("DataTriggerBehavior001.xaml"));

    window.show();
    run_jobs();

    assert_eq!(Some("Less than or equal 50".to_string()), text_of(&window, "TargetTextBlock"));
    assert_eq!(Some("0".to_string()), find::<TextBox>(&window, "TargetTextBox").text());
    assert_eq!(0.0, find::<Slider>(&window, "TargetSlider").value());

    press_right(&window);
    press_right(&window);
    press_right(&window);

    assert_eq!(Some("More than 50".to_string()), text_of(&window, "TargetTextBlock"));
    assert_eq!(Some("75".to_string()), find::<TextBox>(&window, "TargetTextBox").text());
    assert_eq!(75.0, find::<Slider>(&window, "TargetSlider").value());
}

#[test]
fn data_trigger_behavior_002() {
    let _app = start_app();
    let window = load::<Window>("DataTriggerBehavior002.xaml", include_str!("DataTriggerBehavior002.xaml"));

    window.show();
    run_jobs();

    assert_eq!(Some("Unchecked".to_string()), text_of(&window, "TargetTextBlock"));
    assert_eq!(Some(false), find::<CheckBox>(&window, "TargetCheckBox").is_checked());

    click_check_box(&window);

    assert_eq!(Some("Checked".to_string()), text_of(&window, "TargetTextBlock"));
    assert_eq!(Some(true), find::<CheckBox>(&window, "TargetCheckBox").is_checked());
}

#[test]
fn data_trigger_behavior_003() {
    let _app = start_app();
    let window = load::<Window>("DataTriggerBehavior003.xaml", include_str!("DataTriggerBehavior003.xaml"));

    window.show();
    run_jobs();

    assert_eq!(Some("Less than 50".to_string()), text_of(&window, "TargetTextBlock"));
    assert_eq!(0.0, find::<Slider>(&window, "TargetSlider").value());

    press_right(&window);
    press_right(&window);

    assert_eq!(Some("50 or more".to_string()), text_of(&window, "TargetTextBlock"));
    assert_eq!(50.0, find::<Slider>(&window, "TargetSlider").value());
}

#[test]
fn data_trigger_behavior_non_convertible_value_does_not_throw_or_execute_actions() {
    let _app = start_app();
    let command_calls = Rc::new(Cell::new(0));
    let target = Border::new();
    let trigger = DataTriggerBehavior::new();
    trigger.set_binding(Some(Rc::new(42i32)));
    trigger.set_comparison_condition(ComparisonConditionType::Equal);
    trigger.set_trigger_value(Some(text("not-an-integer")));
    let action = InvokeCommandAction::new();
    let calls = command_calls.clone();
    action.set_command(Some(Command::new(move |_| calls.set(calls.get() + 1))));
    if trigger.actions().is_none() {
        trigger.set_actions(Some(ActionCollection::new()));
    }
    trigger.actions().expect("the actions").add(action.upcast());
    Interaction::get_behaviors(&target).add(trigger.upcast());
    let window = window_with(Some(target.upcast::<Control>()));
    window.show();

    let exception = panics(run_jobs);

    assert!(!exception);
    assert_eq!(0, command_calls.get());
}

#[test]
fn data_trigger_behavior_004_reverts_on_false() {
    let _app = start_app();
    let (window, _source) = data_trigger_behavior_004();

    window.show();
    run_jobs();

    assert_eq!(Some("Red".to_string()), text_of(&window, "TargetTextBlock"));
    assert_eq!(Some(false), find::<CheckBox>(&window, "TargetCheckBox").is_checked());

    click_check_box(&window);

    assert_eq!(Some("Green".to_string()), text_of(&window, "TargetTextBlock"));
    assert_eq!(Some(true), find::<CheckBox>(&window, "TargetCheckBox").is_checked());

    click_check_box(&window);

    assert_eq!(Some("Red".to_string()), text_of(&window, "TargetTextBlock"));
    assert_eq!(Some(false), find::<CheckBox>(&window, "TargetCheckBox").is_checked());
}

#[test]
fn data_trigger_behavior_004_clears_reversible_state_when_disabled() {
    let _app = start_app();
    let (window, _source) = data_trigger_behavior_004();
    window.show();
    run_jobs();
    let target_text_block = find::<TextBlock>(&window, "TargetTextBlock");
    let behavior = single_data_trigger(&Interaction::get_behaviors(&target_text_block));

    click_check_box(&window);
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
fn data_trigger_behavior_004_reconciles_reversible_state_when_is_enabled_changes() {
    let _app = start_app();
    let (window, _source) = data_trigger_behavior_004();
    window.show();
    run_jobs();
    let target_text_block = find::<TextBlock>(&window, "TargetTextBlock");
    let behavior = single_data_trigger(&Interaction::get_behaviors(&target_text_block));
    click_check_box(&window);
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
fn data_trigger_behavior_does_not_execute_replaced_actions_in_legacy_mode() {
    let _app = start_app();
    let initial_action = CountingAction::new();
    let replacement_action = CountingAction::new();
    let button = Button::new();
    let behavior = DataTriggerBehavior::new();
    behavior.set_binding(Some(Rc::new(true)));
    behavior.set_trigger_value(Some(Rc::new(true)));
    let initial_actions = ActionCollection::new();
    initial_actions.add(initial_action.clone().upcast());
    behavior.set_actions(Some(initial_actions));
    let behaviors = BehaviorCollection::new();
    behaviors.add(behavior.clone().upcast());
    Interaction::set_behaviors(&button, Some(behaviors));
    let window = window_with(Some(button.upcast::<Control>()));
    window.show();
    run_jobs();
    assert!(initial_action.execution_count() > 0);

    let replacement_actions = ActionCollection::new();
    replacement_actions.add(replacement_action.clone().upcast());
    behavior.set_actions(Some(replacement_actions));
    run_jobs();

    assert_eq!(0, replacement_action.execution_count());
    window.close();
}

#[test]
fn data_trigger_behavior_004_reverts_and_clears_state_when_removed() {
    let _app = start_app();
    let (window, _source) = data_trigger_behavior_004();
    window.show();
    run_jobs();
    let target_text_block = find::<TextBlock>(&window, "TargetTextBlock");
    let behaviors = Interaction::get_behaviors(&target_text_block);
    let behavior = single_data_trigger(&behaviors);
    click_check_box(&window);
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
fn data_trigger_behavior_004_reconciles_actions_while_active() {
    let _app = start_app();
    let (window, _source) = data_trigger_behavior_004();
    window.show();
    run_jobs();
    let target_text_block = find::<TextBlock>(&window, "TargetTextBlock");
    let behavior = single_data_trigger(&Interaction::get_behaviors(&target_text_block));
    click_check_box(&window);
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

#[test]
fn data_trigger_behavior_executes_when_bound_value_becomes_null() {
    let _app = start_app();
    let (window, source) = data_trigger_behavior_004();

    window.show();
    run_jobs();

    assert_eq!(Some("Not empty".to_string()), text_of(&window, "NullTargetTextBlock"));
    assert_eq!(Some("Unchanged".to_string()), text_of(&window, "UnconfiguredTextBlock"));
    assert_eq!(Some("Unchanged".to_string()), text_of(&window, "RelationalTextBlock"));

    source.set_test_property(None);
    run_jobs();

    assert_eq!(Some("Empty".to_string()), text_of(&window, "NullTargetTextBlock"));
    assert_eq!(Some("Unchanged".to_string()), text_of(&window, "UnconfiguredTextBlock"));
    assert_eq!(Some("Unchanged".to_string()), text_of(&window, "RelationalTextBlock"));
}

#[test]
fn data_trigger_behavior_executes_when_initial_bound_value_is_null() {
    let _app = start_app();
    let (window, source) = data_trigger_behavior_004();
    source.set_test_property(None);

    window.show();
    run_jobs();

    assert_eq!(Some("Empty".to_string()), text_of(&window, "NullTargetTextBlock"));
    assert_eq!(Some("Unchanged".to_string()), text_of(&window, "UnconfiguredTextBlock"));
    assert_eq!(Some("Unchanged".to_string()), text_of(&window, "RelationalTextBlock"));
}
