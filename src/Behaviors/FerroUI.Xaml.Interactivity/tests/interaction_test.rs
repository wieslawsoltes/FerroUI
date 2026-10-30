//! Ported from `tests/Xaml.Behaviors.Interactivity.UnitTests/InteractionTest.cs` of Xaml.Behaviors:
//! the attachment of behavior collections and the replay of the lifecycle of the host.
//!
//! Adapted: the triggers of the reference are classes deriving from `StyledElementTrigger<Button>`
//! (or `<TopLevel>`); here they derive from [`StyledElementTriggerOfT`] with the type of the host,
//! declared by `test_trigger!`, and take their constructor arguments after construction. The event
//! handlers the tests add to the host (`button.Loaded += ..`) are subscriptions of the framework.

use super::stub_action::StubAction;
use super::stub_behavior::StubBehavior;
use super::support::{behavior_collection, boxed, object, run_jobs, start_app, text, window_with};
use super::test_behaviors::TestBehavior;
use crate::{
    ActionCollection, BehaviorCollection, BehaviorImpl, BehaviorImplExt, BehaviorOfT, Interaction,
    StyledElementBehaviorImpl, StyledElementTriggerOfT,
};
use ferroui_base::{
    ferro_class, instantiate, FerroObject, FerroObjectImpl, Ref, StaticType, StyledElement, StyledElementImpl,
};
use ferroui_controls::{Button, Control, Panel, StackPanel, TopLevel};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Declares a trigger class of the tests: a [`StyledElementTriggerOfT`] of the given host type
/// with the given fields (initialised to their defaults) and hooks.
macro_rules! test_trigger {
    ($name:ident ($associated:ty) { $($field:ident : $field_type:ty),* $(,)? } impl { $($hooks:tt)* }) => {
        #[repr(C)]
        struct $name {
            base: StyledElementTriggerOfT,
            $($field: $field_type,)*
        }

        ferro_class!($name: StyledElementTriggerOfT);
        ferroui_base::ferro_impl_classes!($name: FerroObjectImpl, StyledElementImpl);

        impl StyledElementBehaviorImpl for $name {
            $($hooks)*
        }

        impl $name {
            fn create() -> Ref<Self> {
                instantiate(Self {
                    base: StyledElementTriggerOfT::construct(<$associated as StaticType>::TYPE),
                    $($field: Default::default(),)*
                })
            }
        }
    };
}

fn increment(counter: &Cell<i32>) {
    counter.set(counter.get() + 1);
}

/// `Interaction.GetBehaviors(AssociatedObject).Remove(item)`.
fn remove_from_host_behaviors(associated_object: &Ref<FerroObject>, item: &Ref<FerroObject>) {
    Interaction::get_behaviors(associated_object).remove(item);
}

/// `Interaction.GetBehaviors(AssociatedObject).Add(item)`.
fn add_to_host_behaviors(associated_object: &Ref<FerroObject>, item: Ref<FerroObject>) {
    Interaction::get_behaviors(associated_object).add(item);
}

type EventOrder = Rc<RefCell<Vec<String>>>;

fn record(event_order: &RefCell<Option<EventOrder>>, entry: String) {
    if let Some(event_order) = &*event_order.borrow() {
        event_order.borrow_mut().push(entry);
    }
}

test_trigger!(LoadedTrigger(Button) {
    initialized_count: Cell<i32>,
    logical_attach_count: Cell<i32>,
    visual_attach_count: Cell<i32>,
    loaded_count: Cell<i32>,
} impl {
    fn on_initialized_event(this: &Self) {
        increment(&this.initialized_count);
    }

    fn on_attached_to_logical_tree_event(this: &Self) {
        increment(&this.logical_attach_count);
    }

    fn on_attached_to_visual_tree(this: &Self) {
        increment(&this.visual_attach_count);
    }

    fn on_loaded(this: &Self) {
        increment(&this.loaded_count);
    }
});

impl LoadedTrigger {
    fn new() -> Ref<Self> {
        Self::create()
    }
}

test_trigger!(TopLevelLoadedTrigger(TopLevel) {
    initialized_count: Cell<i32>,
    logical_attach_count: Cell<i32>,
    visual_attach_count: Cell<i32>,
    loaded_count: Cell<i32>,
} impl {
    fn on_initialized_event(this: &Self) {
        increment(&this.initialized_count);
    }

    fn on_attached_to_logical_tree_event(this: &Self) {
        increment(&this.logical_attach_count);
    }

    fn on_attached_to_visual_tree(this: &Self) {
        increment(&this.visual_attach_count);
    }

    fn on_loaded(this: &Self) {
        increment(&this.loaded_count);
    }
});

test_trigger!(SiblingAwareLoadedTrigger(Button) {
    sibling: RefCell<Option<Ref<FerroObject>>>,
    sibling_was_attached_when_loaded: Cell<bool>,
} impl {
    fn on_loaded(this: &Self) {
        let sibling = this.sibling.borrow().clone();
        let sibling_associated_object = sibling.as_ref().and_then(|sibling| {
            <dyn crate::IBehavior>::of(sibling).and_then(|sibling| sibling.associated_object())
        });
        this.sibling_was_attached_when_loaded.set(this.associated_object() == sibling_associated_object);
    }
});

impl SiblingAwareLoadedTrigger {
    fn new(sibling: Ref<FerroObject>) -> Ref<Self> {
        let this = Self::create();
        *this.sibling.borrow_mut() = Some(sibling);
        this
    }
}

test_trigger!(LifecycleAwareLoadedTrigger(Button) {
    sibling: RefCell<Option<Ref<LoadedTrigger>>>,
    sibling_completed_earlier_phases_when_loaded: Cell<bool>,
} impl {
    fn on_loaded(this: &Self) {
        let sibling = this.sibling.borrow().clone().expect("the sibling");
        this.sibling_completed_earlier_phases_when_loaded.set(
            sibling.initialized_count.get() == 1
                && sibling.logical_attach_count.get() == 1
                && sibling.visual_attach_count.get() == 1
                && sibling.loaded_count.get() == 0,
        );
    }
});

impl LifecycleAwareLoadedTrigger {
    fn new(sibling: &Ref<LoadedTrigger>) -> Ref<Self> {
        let this = Self::create();
        *this.sibling.borrow_mut() = Some(sibling.clone());
        this
    }
}

test_trigger!(SelfRemovingLoadedTrigger(Button) {
    loaded_count: Cell<i32>,
} impl {
    fn on_loaded(this: &Self) {
        increment(&this.loaded_count);
        if let Some(associated_object) = this.associated_object() {
            remove_from_host_behaviors(&associated_object, &object(&this.to_ref()));
        }
    }
});

test_trigger!(SelfRemovingInitializedTrigger(Button) {
    initialized_count: Cell<i32>,
    logical_attach_count: Cell<i32>,
    visual_attach_count: Cell<i32>,
    loaded_count: Cell<i32>,
} impl {
    fn on_initialized_event(this: &Self) {
        increment(&this.initialized_count);
        if let Some(associated_object) = this.associated_object() {
            remove_from_host_behaviors(&associated_object, &object(&this.to_ref()));
        }
    }

    fn on_attached_to_logical_tree_event(this: &Self) {
        increment(&this.logical_attach_count);
    }

    fn on_attached_to_visual_tree(this: &Self) {
        increment(&this.visual_attach_count);
    }

    fn on_loaded(this: &Self) {
        increment(&this.loaded_count);
    }
});

test_trigger!(RemovingLoadedTrigger(Button) {
    parent: RefCell<Option<Ref<Panel>>>,
    loaded_count: Cell<i32>,
} impl {
    fn on_loaded(this: &Self) {
        increment(&this.loaded_count);
        if this.loaded_count.get() == 1 {
            if let Some(associated_object) = this.associated_object().and_then(|o| o.cast::<Control>()) {
                let parent = this.parent.borrow().clone().expect("the panel");
                parent.children().remove(&associated_object);
            }
        }
    }
});

impl RemovingLoadedTrigger {
    fn new(parent: Ref<Panel>) -> Ref<Self> {
        let this = Self::create();
        *this.parent.borrow_mut() = Some(parent);
        this
    }
}

/// `SiblingRemovingBehavior`: a `Behavior<Button>` that removes a sibling when it is attached.
#[repr(C)]
struct SiblingRemovingBehavior {
    base: BehaviorOfT,
    sibling: RefCell<Option<Ref<FerroObject>>>,
}

ferro_class!(SiblingRemovingBehavior: BehaviorOfT);
ferroui_base::ferro_impl_classes!(SiblingRemovingBehavior: FerroObjectImpl);

impl BehaviorImpl for SiblingRemovingBehavior {
    fn on_attached(this: &Self) {
        Self::parent_on_attached(this);
        if let Some(associated_object) = this.associated_object() {
            let sibling = this.sibling.borrow().clone().expect("the sibling");
            remove_from_host_behaviors(&associated_object, &sibling);
        }
    }
}

impl SiblingRemovingBehavior {
    fn new(sibling: Ref<FerroObject>) -> Ref<Self> {
        instantiate(Self {
            base: BehaviorOfT::construct(<Button as StaticType>::TYPE),
            sibling: RefCell::new(Some(sibling)),
        })
    }
}

/// `SiblingAddingBehavior`: a `Behavior<Button>` that adds a sibling when it is attached.
#[repr(C)]
struct SiblingAddingBehavior {
    base: BehaviorOfT,
    sibling: RefCell<Option<Ref<FerroObject>>>,
}

ferro_class!(SiblingAddingBehavior: BehaviorOfT);
ferroui_base::ferro_impl_classes!(SiblingAddingBehavior: FerroObjectImpl);

impl BehaviorImpl for SiblingAddingBehavior {
    fn on_attached(this: &Self) {
        Self::parent_on_attached(this);
        if let Some(associated_object) = this.associated_object() {
            let sibling = this.sibling.borrow().clone().expect("the sibling");
            add_to_host_behaviors(&associated_object, sibling);
        }
    }
}

impl SiblingAddingBehavior {
    fn new(sibling: Ref<FerroObject>) -> Ref<Self> {
        instantiate(Self {
            base: BehaviorOfT::construct(<Button as StaticType>::TYPE),
            sibling: RefCell::new(Some(sibling)),
        })
    }
}

test_trigger!(SiblingAddingInitializedTrigger(Button) {
    sibling: RefCell<Option<Ref<FerroObject>>>,
} impl {
    fn on_initialized_event(this: &Self) {
        if let Some(associated_object) = this.associated_object() {
            let sibling = this.sibling.borrow().clone().expect("the sibling");
            add_to_host_behaviors(&associated_object, sibling);
        }
    }
});

impl SiblingAddingInitializedTrigger {
    fn new(sibling: Ref<FerroObject>) -> Ref<Self> {
        let this = Self::create();
        *this.sibling.borrow_mut() = Some(sibling);
        this
    }
}

test_trigger!(SiblingAddingVisualTrigger(Button) {
    sibling: RefCell<Option<Ref<FerroObject>>>,
} impl {
    fn on_attached_to_visual_tree(this: &Self) {
        if let Some(associated_object) = this.associated_object() {
            let sibling = this.sibling.borrow().clone().expect("the sibling");
            add_to_host_behaviors(&associated_object, sibling);
        }
    }
});

impl SiblingAddingVisualTrigger {
    fn new(sibling: Ref<FerroObject>) -> Ref<Self> {
        let this = Self::create();
        *this.sibling.borrow_mut() = Some(sibling);
        this
    }
}

test_trigger!(NestedSiblingAddingInitializedTrigger(Button) {
    sibling: RefCell<Option<Ref<LoadedTrigger>>>,
    sibling_completed_each_earlier_phase: Cell<bool>,
    logical_check_count: Cell<i32>,
    visual_check_count: Cell<i32>,
} impl {
    fn on_initialized_event(this: &Self) {
        if let Some(associated_object) = this.associated_object() {
            let sibling = this.sibling.borrow().clone().expect("the sibling");
            add_to_host_behaviors(&associated_object, object(&sibling));
        }
    }

    fn on_attached_to_logical_tree_event(this: &Self) {
        increment(&this.logical_check_count);
        let sibling = this.sibling.borrow().clone().expect("the sibling");
        let completed = sibling.initialized_count.get() == 1 && sibling.logical_attach_count.get() == 0;
        this.sibling_completed_each_earlier_phase.set(this.sibling_completed_each_earlier_phase.get() & completed);
    }

    fn on_attached_to_visual_tree(this: &Self) {
        increment(&this.visual_check_count);
        let sibling = this.sibling.borrow().clone().expect("the sibling");
        let completed = sibling.initialized_count.get() == 1
            && sibling.logical_attach_count.get() == 1
            && sibling.visual_attach_count.get() == 0;
        this.sibling_completed_each_earlier_phase.set(this.sibling_completed_each_earlier_phase.get() & completed);
    }
});

impl NestedSiblingAddingInitializedTrigger {
    fn new(sibling: &Ref<LoadedTrigger>) -> Ref<Self> {
        let this = Self::create();
        *this.sibling.borrow_mut() = Some(sibling.clone());
        this.sibling_completed_each_earlier_phase.set(true);
        this
    }
}

test_trigger!(SiblingRemovingLoadedTrigger(Button) {
    sibling: RefCell<Option<Ref<FerroObject>>>,
} impl {
    fn on_loaded(this: &Self) {
        if let Some(associated_object) = this.associated_object() {
            let sibling = this.sibling.borrow().clone().expect("the sibling");
            remove_from_host_behaviors(&associated_object, &sibling);
        }
    }
});

impl SiblingRemovingLoadedTrigger {
    fn new(sibling: Ref<FerroObject>) -> Ref<Self> {
        let this = Self::create();
        *this.sibling.borrow_mut() = Some(sibling);
        this
    }
}

test_trigger!(RecordingLoadedTrigger(Button) {
    name: RefCell<String>,
    event_order: RefCell<Option<EventOrder>>,
    sibling_to_add: RefCell<Option<Ref<FerroObject>>>,
} impl {
    fn on_loaded(this: &Self) {
        record(&this.event_order, this.name.borrow().clone());
        let sibling_to_add = this.sibling_to_add.borrow().clone();
        if let (Some(sibling_to_add), Some(associated_object)) = (sibling_to_add, this.associated_object()) {
            add_to_host_behaviors(&associated_object, sibling_to_add);
        }
    }
});

impl RecordingLoadedTrigger {
    fn new(name: &str, event_order: &EventOrder, sibling_to_add: Option<Ref<FerroObject>>) -> Ref<Self> {
        let this = Self::create();
        *this.name.borrow_mut() = name.to_string();
        *this.event_order.borrow_mut() = Some(event_order.clone());
        *this.sibling_to_add.borrow_mut() = sibling_to_add;
        this
    }
}

test_trigger!(RecordingVisualTrigger(Button) {
    name: RefCell<String>,
    event_order: RefCell<Option<EventOrder>>,
} impl {
    fn on_attached_to_visual_tree(this: &Self) {
        record(&this.event_order, this.name.borrow().clone());
    }
});

impl RecordingVisualTrigger {
    fn new(name: &str, event_order: &EventOrder) -> Ref<Self> {
        let this = Self::create();
        *this.name.borrow_mut() = name.to_string();
        *this.event_order.borrow_mut() = Some(event_order.clone());
        this
    }
}

test_trigger!(RecordingInitializedTrigger(Button) {
    name: RefCell<String>,
    event_order: RefCell<Option<EventOrder>>,
} impl {
    fn on_initialized_event(this: &Self) {
        record(&this.event_order, this.name.borrow().clone());
    }
});

impl RecordingInitializedTrigger {
    fn new(name: &str, event_order: &EventOrder) -> Ref<Self> {
        let this = Self::create();
        *this.name.borrow_mut() = name.to_string();
        *this.event_order.borrow_mut() = Some(event_order.clone());
        this
    }
}

test_trigger!(RecordingTopLevelPhaseTrigger(TopLevel) {
    name: RefCell<String>,
    event_order: RefCell<Option<EventOrder>>,
    sibling_to_add: RefCell<Option<Ref<FerroObject>>>,
    add_sibling_on_next_visual: Cell<bool>,
} impl {
    fn on_attached_to_logical_tree_event(this: &Self) {
        record(&this.event_order, format!("{}-Logical", this.name.borrow()));
    }

    fn on_attached_to_visual_tree(this: &Self) {
        record(&this.event_order, format!("{}-Visual", this.name.borrow()));
        let sibling_to_add = this.sibling_to_add.borrow().clone();
        if this.add_sibling_on_next_visual.get() {
            if let (Some(sibling_to_add), Some(associated_object)) = (sibling_to_add, this.associated_object()) {
                this.add_sibling_on_next_visual.set(false);
                add_to_host_behaviors(&associated_object, sibling_to_add);
            }
        }
    }

    fn on_loaded(this: &Self) {
        record(&this.event_order, format!("{}-Loaded", this.name.borrow()));
    }
});

impl RecordingTopLevelPhaseTrigger {
    fn new(name: &str, event_order: &EventOrder, sibling_to_add: Option<Ref<FerroObject>>) -> Ref<Self> {
        let this = Self::create();
        *this.name.borrow_mut() = name.to_string();
        *this.event_order.borrow_mut() = Some(event_order.clone());
        *this.sibling_to_add.borrow_mut() = sibling_to_add;
        this
    }

    fn arm_visual_addition(&self) {
        self.add_sibling_on_next_visual.set(true);
    }
}

fn event_order() -> EventOrder {
    Rc::new(RefCell::new(Vec::new()))
}

fn assert_event_order(expected: &[&str], event_order: &EventOrder) {
    let actual = event_order.borrow().clone();
    assert_eq!(expected.iter().map(|entry| entry.to_string()).collect::<Vec<_>>(), actual);
}

fn contains(behaviors: &BehaviorCollection, item: &Ref<FerroObject>) -> bool {
    behaviors.contains(item)
}

fn assert_current_lifecycle(trigger: &LoadedTrigger, button: &Ref<Button>) {
    assert_eq!(1, trigger.initialized_count.get());
    assert_eq!(1, trigger.logical_attach_count.get());
    assert_eq!(1, trigger.visual_attach_count.get());
    assert_eq!(1, trigger.loaded_count.get());
    assert_eq!(Some(button.clone().upcast::<StyledElement>()), trigger.parent());
}

#[test]
fn set_behaviors_multiple_behaviors_all_attached() {
    let _app = start_app();
    let stubs = [StubBehavior::new(), StubBehavior::new(), StubBehavior::new()];
    let behavior_collection = behavior_collection(&[object(&stubs[0]), object(&stubs[1]), object(&stubs[2])]);

    let button = Button::new();
    Interaction::set_behaviors(&button, Some(behavior_collection));

    for behavior in &stubs {
        assert_eq!(1, behavior.attach_count(), "Should only have called Attach once.");
        assert_eq!(0, behavior.detach_count(), "Should not have called Detach.");
        assert_eq!(Some(object(&button)), behavior.associated_object(), "Should be attached to the host of the BehaviorCollection.");
    }
}

#[test]
fn set_behaviors_on_attached_removal_does_not_attach_removed_sibling() {
    let _app = start_app();
    let sibling = StubBehavior::new();
    let remover = SiblingRemovingBehavior::new(object(&sibling));
    let behaviors = behavior_collection(&[object(&remover), object(&sibling)]);
    let button = Button::new();

    Interaction::set_behaviors(&button, Some(behaviors.clone()));

    assert!(!contains(&behaviors, &object(&sibling)));
    assert_eq!(0, sibling.attach_count());
    assert!(sibling.associated_object().is_none());
    assert_eq!(Some(object(&button)), remover.associated_object());
}

#[test]
fn set_behaviors_multiple_sets_does_not_reattach() {
    let _app = start_app();
    let stub = StubBehavior::new();
    let behavior_collection = behavior_collection(&[object(&stub)]);

    let button = Button::new();
    Interaction::set_behaviors(&button, Some(behavior_collection.clone()));
    Interaction::set_behaviors(&button, Some(behavior_collection));

    assert_eq!(1, stub.attach_count(), "Should only have called Attach once.");
}

#[test]
fn set_behaviors_collection_then_null_deatch_collection() {
    let _app = start_app();
    let stub = StubBehavior::new();
    let behavior_collection = behavior_collection(&[object(&stub)]);

    let button = Button::new();
    Interaction::set_behaviors(&button, Some(behavior_collection));
    Interaction::set_behaviors(&button, None);

    assert_eq!(1, stub.detach_count(), "Should only have called Detach once.");
    assert!(stub.associated_object().is_none(), "AssociatedObject should be null after Detach.");
}

#[test]
fn set_behaviors_null_then_null_no_op() {
    let _app = start_app();
    // As long as this doesn't crash/assert, we're good.

    let button = Button::new();
    Interaction::set_behaviors(&button, None);
    Interaction::set_behaviors(&button, None);
    Interaction::set_behaviors(&button, None);
}

#[test]
fn set_behaviors_manual_detach_then_null_does_not_double_detach() {
    let _app = start_app();
    let stubs = [StubBehavior::new(), StubBehavior::new(), StubBehavior::new()];
    let behavior_collection = behavior_collection(&[object(&stubs[0]), object(&stubs[1]), object(&stubs[2])]);

    let button = Button::new();
    Interaction::set_behaviors(&button, Some(behavior_collection));

    for behavior in &stubs {
        crate::IBehavior::detach(&**behavior);
    }

    Interaction::set_behaviors(&button, None);

    for behavior in &stubs {
        assert_eq!(
            1,
            behavior.detach_count(),
            "Setting BehaviorCollection to null should not call Detach on already Detached Behaviors."
        );
        assert!(behavior.associated_object().is_none(), "AssociatedObject should be null after Detach.");
    }
}

#[test]
fn lifecycle_events_do_not_create_collection_after_behaviors_are_removed() {
    let _app = start_app();
    let behavior = TestBehavior::new();
    let collection = behavior_collection(&[object(&behavior)]);
    let button = Button::new();
    let panel = Panel::new();
    panel.children().add(&button);
    let window = window_with(Some(panel.clone().upcast()));
    Interaction::set_behaviors(&button, Some(collection.clone()));

    window.show();
    Interaction::set_behaviors(&button, None);
    panel.children().remove(&button);

    assert!(button.get_value(Interaction::behaviors_property()).is_none());
    assert!(collection.associated_object().is_none());
    assert!(behavior.associated_object().is_none());
    assert_eq!(1, behavior.detaching_called());
}

#[test]
fn set_behaviors_on_attached_visual_transfers_visual_tree_lifecycle() {
    let _app = start_app();
    let old_behavior = TestBehavior::new();
    let old_collection = behavior_collection(&[object(&old_behavior)]);
    let button = Button::new();
    let window = window_with(Some(button.clone().upcast()));
    Interaction::set_behaviors(&button, Some(old_collection));

    window.show();

    assert_eq!(1, old_behavior.visual_attach_called());
    assert_eq!(0, old_behavior.visual_detach_called());

    let new_behavior = TestBehavior::new();
    let new_collection = behavior_collection(&[object(&new_behavior)]);

    Interaction::set_behaviors(&button, Some(new_collection));

    assert_eq!(1, old_behavior.visual_detach_called());
    assert_eq!(1, old_behavior.detaching_called());
    assert!(old_behavior.associated_object().is_none());
    assert_eq!(1, new_behavior.attached_called());
    assert_eq!(1, new_behavior.visual_attach_called());
    assert_eq!(0, new_behavior.visual_detach_called());
    assert_eq!(Some(object(&button)), new_behavior.associated_object());
}

#[test]
fn set_behaviors_top_level_close_defers_then_detaches_collection() {
    let _app = start_app();
    let behavior = StubBehavior::new();
    let behaviors = behavior_collection(&[object(&behavior)]);
    let window = window_with(None);
    Interaction::set_behaviors(&window, Some(behaviors.clone()));

    window.show();
    window.close();

    assert_eq!(Some(object(&window)), behaviors.associated_object());
    assert_eq!(Some(object(&window)), behavior.associated_object());

    run_jobs();

    assert!(behaviors.associated_object().is_none());
    assert!(behavior.associated_object().is_none());
    assert_eq!(1, behavior.detach_count());
}

#[test]
fn execute_actions_null_parameters_returns_empty_enumerable() {
    let _app = start_app();
    // Mostly just want to test that this doesn't throw any exceptions.
    let result = Interaction::execute_actions(None, None, None);

    assert!(result.is_empty(), "Calling ExecuteActions with a null ActionCollection should return an empty enumerable.");
}

#[test]
fn execute_actions_multiple_actions_all_actions_executed() {
    let _app = start_app();
    let stubs = [StubAction::new(), StubAction::new(), StubAction::new()];
    let actions = ActionCollection::new();
    for stub in &stubs {
        actions.add(object(stub));
    }

    let sender = boxed(&Button::new());
    let parameter_string = text("TestString");

    Interaction::execute_actions(Some(&sender), Some(&actions), Some(&parameter_string));

    for action in &stubs {
        assert_eq!(1, action.execute_count(), "Each IAction should be executed once.");
        assert_eq!(Some(sender.clone()), action.sender(), "Sender is passed to the actions.");
        assert_eq!(Some(parameter_string.clone()), action.parameter(), "Parameter is passed to the actions.");
    }
}

#[test]
fn execute_actions_actions_with_results_results_in_action_order() {
    let _app = start_app();
    let expected_return_values = ["A", "B", "C"];

    let actions = ActionCollection::new();

    for return_value in expected_return_values {
        actions.add(object(&StubAction::with_return_value(Some(text(return_value)))));
    }

    let results = Interaction::execute_actions(None, Some(&actions), None);

    assert_eq!(expected_return_values.len(), results.len(), "Should have the same number of results as IActions.");

    for (result_index, result) in results.iter().enumerate() {
        assert_eq!(
            &text(expected_return_values[result_index]),
            result,
            "Results should be returned in the order of the actions in the ActionCollection."
        );
    }
}

#[test]
fn set_behaviors_before_show_notifies_loaded_once() {
    let _app = start_app();
    let trigger = LoadedTrigger::new();
    let button = Button::new();
    Interaction::set_behaviors(&button, Some(behavior_collection(&[object(&trigger)])));
    let window = window_with(Some(button.clone().upcast()));

    window.show();
    run_jobs();

    assert_current_lifecycle(&trigger, &button);
}

#[test]
fn set_behaviors_after_show_notifies_loaded_once() {
    let _app = start_app();
    let trigger = LoadedTrigger::new();
    let button = Button::new();
    let window = window_with(Some(button.clone().upcast()));
    window.show();
    run_jobs();

    Interaction::set_behaviors(&button, Some(behavior_collection(&[object(&trigger)])));

    assert_current_lifecycle(&trigger, &button);
}

#[test]
fn set_behaviors_after_show_attaches_all_siblings_before_loaded_callbacks() {
    let _app = start_app();
    let button = Button::new();
    let window = window_with(Some(button.clone().upcast()));
    window.show();
    run_jobs();
    let sibling = StubBehavior::new();
    let trigger = SiblingAwareLoadedTrigger::new(object(&sibling));

    Interaction::set_behaviors(&button, Some(behavior_collection(&[object(&trigger), object(&sibling)])));

    assert!(trigger.sibling_was_attached_when_loaded.get());
    assert_eq!(Some(object(&button)), sibling.associated_object());
    window.close();
}

#[test]
fn set_behaviors_after_show_replays_lifecycle_in_collection_phases() {
    let _app = start_app();
    let button = Button::new();
    let window = window_with(Some(button.clone().upcast()));
    window.show();
    run_jobs();
    let sibling = LoadedTrigger::new();
    let observer = LifecycleAwareLoadedTrigger::new(&sibling);

    Interaction::set_behaviors(&button, Some(behavior_collection(&[object(&observer), object(&sibling)])));

    assert!(observer.sibling_completed_earlier_phases_when_loaded.get());
    assert_eq!(1, sibling.loaded_count.get());
    window.close();
}

#[test]
fn set_behaviors_after_show_catches_up_reentrant_additions_before_later_phases() {
    let _app = start_app();
    let button = Button::new();
    let window = window_with(Some(button.clone().upcast()));
    window.show();
    run_jobs();
    let added = LoadedTrigger::new();
    let adder = SiblingAddingInitializedTrigger::new(object(&added));
    let observer = LifecycleAwareLoadedTrigger::new(&added);
    let behaviors = behavior_collection(&[object(&adder), object(&observer)]);

    Interaction::set_behaviors(&button, Some(behaviors.clone()));

    assert!(contains(&behaviors, &object(&added)));
    assert!(observer.sibling_completed_earlier_phases_when_loaded.get());
    assert_eq!(1, added.loaded_count.get());
    window.close();
}

#[test]
fn set_behaviors_after_show_catches_up_nested_additions_between_replayed_phases() {
    let _app = start_app();
    let button = Button::new();
    let window = window_with(Some(button.clone().upcast()));
    window.show();
    run_jobs();
    let nested = LoadedTrigger::new();
    let first_addition = NestedSiblingAddingInitializedTrigger::new(&nested);
    let visual_adder = SiblingAddingVisualTrigger::new(object(&first_addition));
    let behaviors = behavior_collection(&[object(&visual_adder)]);

    Interaction::set_behaviors(&button, Some(behaviors.clone()));

    assert!(contains(&behaviors, &object(&first_addition)));
    assert!(contains(&behaviors, &object(&nested)));
    assert!(first_addition.sibling_completed_each_earlier_phase.get());
    assert_eq!(1, first_addition.logical_check_count.get());
    assert_eq!(1, first_addition.visual_check_count.get());
    assert_current_lifecycle(&nested, &button);
    window.close();
}

#[test]
fn set_behaviors_after_show_defers_reentrant_add_replay_until_all_siblings_attach() {
    let _app = start_app();
    let button = Button::new();
    let window = window_with(Some(button.clone().upcast()));
    window.show();
    run_jobs();
    let sibling = StubBehavior::new();
    let trigger = SiblingAwareLoadedTrigger::new(object(&sibling));
    let adder = SiblingAddingBehavior::new(object(&trigger));
    let behaviors = behavior_collection(&[object(&adder), object(&sibling)]);

    Interaction::set_behaviors(&button, Some(behaviors.clone()));

    assert!(contains(&behaviors, &object(&trigger)));
    assert!(trigger.sibling_was_attached_when_loaded.get());
    assert_eq!(Some(object(&button)), sibling.associated_object());
    window.close();
}

#[test]
fn set_behaviors_after_show_allows_lifecycle_callback_to_remove_behavior() {
    let _app = start_app();
    let button = Button::new();
    let window = window_with(Some(button.clone().upcast()));
    window.show();
    run_jobs();
    let trigger = SelfRemovingLoadedTrigger::create();
    let sibling = StubBehavior::new();
    let behaviors = behavior_collection(&[object(&trigger), object(&sibling)]);

    Interaction::set_behaviors(&button, Some(behaviors.clone()));

    assert_eq!(1, trigger.loaded_count.get());
    assert!(!contains(&behaviors, &object(&trigger)));
    assert!(trigger.associated_object().is_none());
    assert_eq!(Some(object(&button)), sibling.associated_object());
    window.close();
}

#[test]
fn add_behavior_after_show_notifies_loaded_once() {
    let _app = start_app();
    let button = Button::new();
    let behaviors = Interaction::get_behaviors(&button);
    let window = window_with(Some(button.clone().upcast()));
    window.show();
    run_jobs();
    let trigger = LoadedTrigger::new();

    behaviors.add(object(&trigger));

    assert_current_lifecycle(&trigger, &button);
}

#[test]
fn add_behavior_from_earlier_loaded_handler_notifies_loaded_once() {
    let _app = start_app();
    let button = Button::new();
    let behaviors: Rc<RefCell<Option<BehaviorCollection>>> = Rc::new(RefCell::new(None));
    let trigger = LoadedTrigger::new();
    {
        let behaviors = behaviors.clone();
        let trigger = object(&trigger);
        button.loaded(move |_, _| {
            let collection = behaviors.borrow().clone().expect("the behaviors");
            collection.add(trigger.clone());
        });
    }
    *behaviors.borrow_mut() = Some(Interaction::get_behaviors(&button));
    let window = window_with(Some(button.clone().upcast()));

    window.show();
    run_jobs();

    assert_current_lifecycle(&trigger, &button);
    window.close();
}

#[test]
fn earlier_loaded_handler_replays_addition_after_existing_behaviors() {
    let _app = start_app();
    let event_order = event_order();
    let button = Button::new();
    let added = RecordingLoadedTrigger::new("B", &event_order, None);
    {
        let host = button.clone();
        let added = object(&added);
        button.loaded(move |_, _| Interaction::get_behaviors(&host).add(added.clone()));
    }
    let first = RecordingLoadedTrigger::new("A", &event_order, None);
    let later_existing = RecordingLoadedTrigger::new("C", &event_order, None);
    let behaviors = behavior_collection(&[object(&first), object(&later_existing)]);
    Interaction::set_behaviors(&button, Some(behaviors.clone()));
    let window = window_with(Some(button.clone().upcast()));

    window.show();
    run_jobs();

    assert_event_order(&["A", "C", "B"], &event_order);
    assert!(contains(&behaviors, &object(&added)));
    window.close();
}

#[test]
fn earlier_visual_handler_replays_addition_after_existing_behaviors() {
    let _app = start_app();
    let event_order = event_order();
    let button = Button::new();
    let added = RecordingVisualTrigger::new("B", &event_order);
    let _subscription = {
        let host = button.downgrade();
        let added = object(&added);
        button.attached_to_visual_tree(move |_| {
            if let Some(host) = host.upgrade() {
                Interaction::get_behaviors(&host).add(added.clone());
            }
        })
    };
    let first = RecordingVisualTrigger::new("A", &event_order);
    let later_existing = RecordingVisualTrigger::new("C", &event_order);
    let behaviors = behavior_collection(&[object(&first), object(&later_existing)]);
    Interaction::set_behaviors(&button, Some(behaviors.clone()));
    let window = window_with(Some(button.clone().upcast()));

    window.show();
    run_jobs();

    assert_event_order(&["A", "C", "B"], &event_order);
    assert!(contains(&behaviors, &object(&added)));
    window.close();
}

#[test]
fn earlier_initialized_handler_replays_addition_after_existing_behaviors() {
    let _app = start_app();
    let event_order = event_order();
    let button = Button::new();
    let added = RecordingInitializedTrigger::new("B", &event_order);
    let _subscription = {
        let host = button.downgrade();
        let added = object(&added);
        button.initialized(move || {
            if let Some(host) = host.upgrade() {
                Interaction::get_behaviors(&host).add(added.clone());
            }
        })
    };
    let first = RecordingInitializedTrigger::new("A", &event_order);
    let later_existing = RecordingInitializedTrigger::new("C", &event_order);
    let behaviors = behavior_collection(&[object(&first), object(&later_existing)]);
    Interaction::set_behaviors(&button, Some(behaviors.clone()));
    let window = window_with(Some(button.clone().upcast()));

    window.show();
    run_jobs();

    assert_event_order(&["A", "C", "B"], &event_order);
    assert!(contains(&behaviors, &object(&added)));
    window.close();
}

#[test]
fn top_level_opened_replays_visual_addition_after_existing_logical_phase() {
    let _app = start_app();
    let event_order = event_order();
    let window = window_with(None);
    let added = RecordingTopLevelPhaseTrigger::new("B", &event_order, None);
    let first = RecordingTopLevelPhaseTrigger::new("A", &event_order, Some(object(&added)));
    let later_existing = RecordingTopLevelPhaseTrigger::new("C", &event_order, None);
    let behaviors = behavior_collection(&[object(&first), object(&later_existing)]);
    Interaction::set_behaviors(&window, Some(behaviors.clone()));
    window.show();
    run_jobs();

    event_order.borrow_mut().clear();
    behaviors.detached_from_visual_tree();
    behaviors.detached_from_logical_tree();
    first.arm_visual_addition();

    behaviors.opened();

    assert_event_order(
        &["A-Visual", "C-Visual", "A-Logical", "C-Logical", "B-Logical", "B-Visual", "B-Loaded"],
        &event_order,
    );
    assert!(contains(&behaviors, &object(&added)));
    window.close();
}

#[test]
fn loaded_dispatch_replays_additions_after_existing_siblings() {
    let _app = start_app();
    let event_order = event_order();
    let added = RecordingLoadedTrigger::new("B", &event_order, None);
    let first = RecordingLoadedTrigger::new("A", &event_order, Some(object(&added)));
    let later_existing = RecordingLoadedTrigger::new("C", &event_order, None);
    let button = Button::new();
    let behaviors = behavior_collection(&[object(&first), object(&later_existing)]);
    Interaction::set_behaviors(&button, Some(behaviors.clone()));
    let window = window_with(Some(button.clone().upcast()));

    window.show();
    run_jobs();

    assert_event_order(&["A", "C", "B"], &event_order);
    assert!(contains(&behaviors, &object(&added)));
    window.close();
}

#[test]
fn loaded_dispatch_does_not_notify_removed_sibling_after_detach() {
    let _app = start_app();
    let button = Button::new();
    let removed_trigger = LoadedTrigger::new();
    let remover = SiblingRemovingLoadedTrigger::new(object(&removed_trigger));
    let behaviors = behavior_collection(&[object(&remover), object(&removed_trigger)]);
    Interaction::set_behaviors(&button, Some(behaviors.clone()));
    let window = window_with(Some(button.clone().upcast()));

    window.show();
    run_jobs();

    assert_eq!(0, removed_trigger.loaded_count.get());
    assert!(removed_trigger.associated_object().is_none());

    behaviors.add(object(&removed_trigger));

    assert_eq!(1, removed_trigger.loaded_count.get());
    assert_eq!(Some(object(&button)), removed_trigger.associated_object());
    window.close();
}

#[test]
fn add_behavior_from_earlier_visual_attach_handler_notifies_lifecycle_once() {
    let _app = start_app();
    let button = Button::new();
    let behaviors: Rc<RefCell<Option<BehaviorCollection>>> = Rc::new(RefCell::new(None));
    let trigger = LoadedTrigger::new();
    let _subscription = {
        let behaviors = behaviors.clone();
        let trigger = object(&trigger);
        button.attached_to_visual_tree(move |_| {
            let collection = behaviors.borrow().clone().expect("the behaviors");
            collection.add(trigger.clone());
        })
    };
    *behaviors.borrow_mut() = Some(Interaction::get_behaviors(&button));
    let window = window_with(Some(button.clone().upcast()));

    window.show();
    run_jobs();

    assert_current_lifecycle(&trigger, &button);
    window.close();
}

#[test]
fn add_behavior_from_earlier_initialized_handler_notifies_lifecycle_once() {
    let _app = start_app();
    let button = Button::new();
    let behaviors: Rc<RefCell<Option<BehaviorCollection>>> = Rc::new(RefCell::new(None));
    let trigger = LoadedTrigger::new();
    let _subscription = {
        let behaviors = behaviors.clone();
        let trigger = object(&trigger);
        button.initialized(move || {
            let collection = behaviors.borrow().clone().expect("the behaviors");
            collection.add(trigger.clone());
        })
    };
    *behaviors.borrow_mut() = Some(Interaction::get_behaviors(&button));
    let window = window_with(Some(button.clone().upcast()));

    window.show();
    run_jobs();

    assert_current_lifecycle(&trigger, &button);
    window.close();
}

#[test]
fn add_behavior_after_show_allows_behavior_to_remove_itself_during_loaded() {
    let _app = start_app();
    let button = Button::new();
    let behaviors = Interaction::get_behaviors(&button);
    let window = window_with(Some(button.clone().upcast()));
    window.show();
    run_jobs();
    let trigger = SelfRemovingLoadedTrigger::create();

    behaviors.add(object(&trigger));

    assert_eq!(1, trigger.loaded_count.get());
    assert!(!contains(&behaviors, &object(&trigger)));
    assert!(trigger.associated_object().is_none());
    assert!(behaviors.is_empty());
    window.close();
}

#[test]
fn add_behavior_after_show_stops_replay_when_initialized_removes_behavior() {
    let _app = start_app();
    let button = Button::new();
    let behaviors = Interaction::get_behaviors(&button);
    let window = window_with(Some(button.clone().upcast()));
    window.show();
    run_jobs();
    let trigger = SelfRemovingInitializedTrigger::create();

    behaviors.add(object(&trigger));

    assert_eq!(1, trigger.initialized_count.get());
    assert_eq!(0, trigger.logical_attach_count.get());
    assert_eq!(0, trigger.visual_attach_count.get());
    assert_eq!(0, trigger.loaded_count.get());
    assert!(trigger.associated_object().is_none());
    window.close();
}

#[test]
fn set_behaviors_after_show_subscribes_host_events_before_loaded_replay() {
    let _app = start_app();
    let button = Button::new();
    let panel = StackPanel::new();
    panel.children().add(&button);
    let window = window_with(Some(panel.clone().upcast()));
    window.show();
    run_jobs();
    let trigger = RemovingLoadedTrigger::new(panel.clone().upcast());

    Interaction::set_behaviors(&button, Some(behavior_collection(&[object(&trigger)])));
    assert!(!panel.children().contains(&button.clone().upcast()));

    panel.children().add(&button);
    run_jobs();

    assert_eq!(2, trigger.loaded_count.get());
    window.close();
}

#[test]
fn add_behavior_to_open_top_level_notifies_current_lifecycle_once() {
    let _app = start_app();
    let window = window_with(None);
    let behaviors = Interaction::get_behaviors(&window);
    window.show();
    run_jobs();
    let trigger = TopLevelLoadedTrigger::create();

    behaviors.add(object(&trigger));

    assert_eq!(1, trigger.initialized_count.get());
    assert_eq!(1, trigger.logical_attach_count.get());
    assert_eq!(1, trigger.visual_attach_count.get());
    assert_eq!(1, trigger.loaded_count.get());
    assert_eq!(Some(window.clone().upcast::<StyledElement>()), trigger.parent());

    window.close();
}
