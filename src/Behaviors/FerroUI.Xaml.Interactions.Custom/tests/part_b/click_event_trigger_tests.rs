//! Ported from `tests/Xaml.Behaviors.Interactions.UnitTests/Custom/ClickEventTriggerTests.cs` of
//! Xaml.Behaviors (with its window `ClickEventTrigger001`: its content is
//! `ClickEventTrigger001.xaml`, loaded with the run-time loader, and its code, the counters of the
//! clicks and the handlers it adds to the window, is [`ClickEventTrigger001`] here).
//!
//! Adapted: the counters of the window are a data context of their own
//! ([`ClickEventTrigger001Context`], whose methods the call method actions call; the window of the
//! reference is its own data context); a click is a press and a release of the pointer raised on
//! the element (the release goes to the element that captured the pointer, as on the headless
//! platform), a key press is raised on the element the reference focuses before pressing it; the
//! awaited jobs of the dispatcher are run with `run_jobs`. The commands of the reference test
//! project (`Command`, `ObservableCommand`) are small commands of this file.

use super::support::{add_behavior, capture_rendered_frame, find, key_down, key_up, load, run_jobs, start_app};
use super::support::MouseTestHelper;
use crate::ClickEventTrigger;
use ferroui_base::input::{ICommand, InputElement, Key, KeyEventArgs, KeyModifiers, MouseButton, PointerPressedEventArgs};
use ferroui_base::input::{PointerReleasedEventArgs, RawInputModifiers};
use ferroui_base::interactivity::{Interactive, RoutedEventArgs, RoutingStrategies};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, instantiate, BoxedValue, FerroObject, FerroObjectImpl, Point, Ref, Visual,
};
use ferroui_controls::primitives::FlyoutBase;
use ferroui_controls::{Border, Button, TextBox, Window};
use ferroui_xaml_interactions::{AsyncActionGroup, InvokeCommandAction, OpenFilePickerAction, SaveFilePickerAction};
use ferroui_xaml_interactivity::{ActionCollection, Interaction};
use std::cell::Cell;
use std::rc::{Rc, Weak};

/// `Command` of the reference test project: a command that runs an action.
struct Command {
    execute: Box<dyn Fn(Option<&BoxedValue>)>,
}

impl Command {
    fn new(execute: impl Fn(Option<&BoxedValue>) + 'static) -> Option<Rc<dyn ICommand>> {
        Some(Rc::new(Self { execute: Box::new(execute) }))
    }
}

impl ICommand for Command {
    fn can_execute(&self, _parameter: Option<&BoxedValue>) -> bool {
        true
    }

    fn execute(&self, parameter: Option<&BoxedValue>) {
        (self.execute)(parameter);
    }

    fn can_execute_changed(&self, _handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        Disposable::create(|| {})
    }
}

/// `ObservableCommand` of the reference test project: a command whose result of `can_execute` the
/// test sets, and which counts the subscriptions to its notification.
struct ObservableCommand {
    this: Weak<ObservableCommand>,
    can_execute_changed: HandlerList<dyn Fn()>,
    can_execute_result: Cell<bool>,
    subscription_count: Cell<i32>,
}

impl ObservableCommand {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            can_execute_changed: HandlerList::new(),
            can_execute_result: Cell::new(true),
            subscription_count: Cell::new(0),
        })
    }

    fn raise_can_execute_changed(&self) {
        for (_, handler) in self.can_execute_changed.snapshot().iter() {
            handler();
        }
    }

    fn as_command(self: &Rc<Self>) -> Option<Rc<dyn ICommand>> {
        Some(self.clone())
    }
}

impl ICommand for ObservableCommand {
    fn can_execute(&self, _parameter: Option<&BoxedValue>) -> bool {
        self.can_execute_result.get()
    }

    fn execute(&self, _parameter: Option<&BoxedValue>) {}

    fn can_execute_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        let token = self.can_execute_changed.add(handler);
        self.subscription_count.set(self.subscription_count.get() + 1);
        let this = self.this.clone();
        Disposable::create(move || {
            if let Some(this) = this.upgrade() {
                this.can_execute_changed.remove(token);
                this.subscription_count.set(this.subscription_count.get() - 1);
            }
        })
    }
}

/// The counters of the code of the window `ClickEventTrigger001` of the reference and the methods its call method actions call.
#[repr(C)]
pub(crate) struct ClickEventTrigger001Context {
    base: FerroObject,
    release_clicks: Cell<i32>,
    press_clicks: Cell<i32>,
    modifier_clicks: Cell<i32>,
    source_control_clicks: Cell<i32>,
    handled_events_too_clicks: Cell<i32>,
    handled_events_too_text_box_clicks: Cell<i32>,
    handle_event_false_button_clicks: Cell<i32>,
    handle_event_false_text_box_clicks: Cell<i32>,
    handle_event_false_button_native_clicks: Cell<i32>,
    handle_event_false_button_click_events: Cell<i32>,
    handle_event_false_text_box_click_events: Cell<i32>,
    handle_event_false_text_box_bubbled_key_up: Cell<i32>,
    space_release_clicks: Cell<i32>,
    space_press_clicks: Cell<i32>,
    default_clicks: Cell<i32>,
    cancel_clicks: Cell<i32>,
    flyout_clicks: Cell<i32>,
}

ferro_class!(ClickEventTrigger001Context: FerroObject);
ferroui_base::ferro_impl_classes!(ClickEventTrigger001Context: FerroObjectImpl);
ferro_class_info!(ClickEventTrigger001Context {
    markup: {
        methods: [
            fn OnReleaseClicked() => |this: &Ref<ClickEventTrigger001Context>| increment(&this.release_clicks),
            fn OnPressClicked() => |this: &Ref<ClickEventTrigger001Context>| increment(&this.press_clicks),
            fn OnModifierClicked() => |this: &Ref<ClickEventTrigger001Context>| increment(&this.modifier_clicks),
            fn OnSourceControlClicked() => |this: &Ref<ClickEventTrigger001Context>| {
                increment(&this.source_control_clicks)
            },
            fn OnHandledEventsTooClicked() => |this: &Ref<ClickEventTrigger001Context>| {
                increment(&this.handled_events_too_clicks)
            },
            fn OnHandledEventsTooTextBoxClicked() => |this: &Ref<ClickEventTrigger001Context>| {
                increment(&this.handled_events_too_text_box_clicks)
            },
            fn OnHandleEventFalseButtonClicked() => |this: &Ref<ClickEventTrigger001Context>| {
                increment(&this.handle_event_false_button_clicks)
            },
            fn OnHandleEventFalseTextBoxClicked() => |this: &Ref<ClickEventTrigger001Context>| {
                increment(&this.handle_event_false_text_box_clicks)
            },
            fn OnSpaceReleaseClicked() => |this: &Ref<ClickEventTrigger001Context>| {
                increment(&this.space_release_clicks)
            },
            fn OnSpacePressClicked() => |this: &Ref<ClickEventTrigger001Context>| increment(&this.space_press_clicks),
            fn OnDefaultClicked() => |this: &Ref<ClickEventTrigger001Context>| increment(&this.default_clicks),
            fn OnCancelClicked() => |this: &Ref<ClickEventTrigger001Context>| increment(&this.cancel_clicks),
            fn OnFlyoutClicked() => |this: &Ref<ClickEventTrigger001Context>| increment(&this.flyout_clicks),
        ],
    },
});

fn increment(counter: &Cell<i32>) {
    counter.set(counter.get() + 1);
}

impl ClickEventTrigger001Context {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: FerroObject::construct(),
            release_clicks: Cell::new(0),
            press_clicks: Cell::new(0),
            modifier_clicks: Cell::new(0),
            source_control_clicks: Cell::new(0),
            handled_events_too_clicks: Cell::new(0),
            handled_events_too_text_box_clicks: Cell::new(0),
            handle_event_false_button_clicks: Cell::new(0),
            handle_event_false_text_box_clicks: Cell::new(0),
            handle_event_false_button_native_clicks: Cell::new(0),
            handle_event_false_button_click_events: Cell::new(0),
            handle_event_false_text_box_click_events: Cell::new(0),
            handle_event_false_text_box_bubbled_key_up: Cell::new(0),
            space_release_clicks: Cell::new(0),
            space_press_clicks: Cell::new(0),
            default_clicks: Cell::new(0),
            cancel_clicks: Cell::new(0),
            flyout_clicks: Cell::new(0),
        })
    }
}

/// `IsSourceInsideTarget` of the window of the reference.
fn is_source_inside_target(source: Option<Ref<FerroObject>>, target: &Visual) -> bool {
    let Some(source_visual) = source.and_then(|source| source.cast::<Visual>()) else {
        return false;
    };

    source_visual.ptr_eq(&target.to_ref()) || target.is_visual_ancestor_of(&source_visual)
}

/// `ClickEventTrigger001`: the window, its counters and the handlers its constructor adds.
struct ClickEventTrigger001 {
    window: Ref<Window>,
    context: Ref<ClickEventTrigger001Context>,
}

impl ClickEventTrigger001 {
    fn new() -> Self {
        ferroui_xaml_interactions::register_types();
        let window = load::<Window>("ClickEventTrigger001.xaml", include_str!("ClickEventTrigger001.xaml"));
        let context = ClickEventTrigger001Context::new();
        let data_context: BoxedValue = Rc::new(context.clone().upcast::<FerroObject>());
        window.set_data_context(Some(data_context));

        let this = Self { window, context };

        let button = this.handle_event_false_button_target();
        let counters = this.context.clone();
        button.add_handler_with(
            Button::click_event(),
            move |_sender: &Interactive, _e: &RoutedEventArgs| {
                increment(&counters.handle_event_false_button_native_clicks);
            },
            RoutingStrategies::DIRECT | RoutingStrategies::BUBBLE,
            false,
        );
        let counters = this.context.clone();
        let target = button.clone();
        button.add_handler_with(
            Button::click_event(),
            move |_sender: &Interactive, e: &RoutedEventArgs| {
                if is_source_inside_target(e.source(), &target) {
                    increment(&counters.handle_event_false_button_click_events);
                }
            },
            RoutingStrategies::BUBBLE,
            false,
        );
        let text_box = this.handle_event_false_text_box_target();
        let counters = this.context.clone();
        let target = text_box.clone();
        text_box.add_handler_with(
            Button::click_event(),
            move |_sender: &Interactive, e: &RoutedEventArgs| {
                if is_source_inside_target(e.source(), &target) {
                    increment(&counters.handle_event_false_text_box_click_events);
                }
            },
            RoutingStrategies::BUBBLE,
            false,
        );

        let handled_events_too_sources: Rc<[Ref<Visual>]> = Rc::new([
            this.handled_events_too_source_target().upcast::<Visual>(),
            this.handled_events_too_text_box_source_target().upcast::<Visual>(),
        ]);
        let sources = handled_events_too_sources.clone();
        this.window.add_handler_with(
            InputElement::pointer_pressed_event(),
            move |_sender: &Interactive, e: &PointerPressedEventArgs| {
                if sources.iter().any(|target| is_source_inside_target(e.source(), target)) {
                    e.set_handled(true);
                }
            },
            RoutingStrategies::TUNNEL,
            false,
        );
        let sources = handled_events_too_sources;
        this.window.add_handler_with(
            InputElement::pointer_released_event(),
            move |_sender: &Interactive, e: &PointerReleasedEventArgs| {
                if sources.iter().any(|target| is_source_inside_target(e.source(), target)) {
                    e.set_handled(true);
                }
            },
            RoutingStrategies::TUNNEL,
            false,
        );
        let text_box_source = this.handled_events_too_text_box_source_target();
        for routed_event in [InputElement::key_down_event(), InputElement::key_up_event()] {
            let target = text_box_source.clone();
            this.window.add_handler_with(
                routed_event,
                move |_sender: &Interactive, e: &KeyEventArgs| {
                    if e.key == Key::Space && is_source_inside_target(e.source(), &target) {
                        e.set_handled(true);
                    }
                },
                RoutingStrategies::TUNNEL,
                false,
            );
        }
        let counters = this.context.clone();
        let target = this.handle_event_false_text_box_target();
        this.window.add_handler_with(
            InputElement::key_up_event(),
            move |_sender: &Interactive, e: &KeyEventArgs| {
                if e.key == Key::Space && is_source_inside_target(e.source(), &target) {
                    increment(&counters.handle_event_false_text_box_bubbled_key_up);
                }
            },
            RoutingStrategies::BUBBLE,
            false,
        );

        this
    }

    /// `window.Show()`, with the layout the clicks need.
    fn show(&self) {
        self.window.show();
        run_jobs();
        capture_rendered_frame(&self.window);
    }

    fn border(&self, name: &str) -> Ref<Border> {
        find::<Border>(&self.window, name)
    }

    fn release_target(&self) -> Ref<Border> {
        self.border("ReleaseTarget")
    }

    fn outside_target(&self) -> Ref<Border> {
        self.border("OutsideTarget")
    }

    fn press_target(&self) -> Ref<Border> {
        self.border("PressTarget")
    }

    fn modifier_target(&self) -> Ref<Border> {
        self.border("ModifierTarget")
    }

    fn source_control_source_target(&self) -> Ref<Border> {
        self.border("SourceControlSourceTarget")
    }

    fn source_control_host_target(&self) -> Ref<Border> {
        self.border("SourceControlHostTarget")
    }

    fn handled_events_too_source_target(&self) -> Ref<Button> {
        find::<Button>(&self.window, "HandledEventsTooSourceTarget")
    }

    fn handled_events_too_host_target(&self) -> Ref<Border> {
        self.border("HandledEventsTooHostTarget")
    }

    fn handled_events_too_text_box_source_target(&self) -> Ref<TextBox> {
        find::<TextBox>(&self.window, "HandledEventsTooTextBoxSourceTarget")
    }

    fn handled_events_too_text_box_host_target(&self) -> Ref<Border> {
        self.border("HandledEventsTooTextBoxHostTarget")
    }

    fn handle_event_false_button_target(&self) -> Ref<Button> {
        find::<Button>(&self.window, "HandleEventFalseButtonTarget")
    }

    fn handle_event_false_text_box_target(&self) -> Ref<TextBox> {
        find::<TextBox>(&self.window, "HandleEventFalseTextBoxTarget")
    }

    fn space_release_target(&self) -> Ref<Border> {
        self.border("SpaceReleaseTarget")
    }

    fn space_press_target(&self) -> Ref<Border> {
        self.border("SpacePressTarget")
    }

    fn focus_source(&self) -> Ref<TextBox> {
        find::<TextBox>(&self.window, "FocusSource")
    }

    fn default_target(&self) -> Ref<Border> {
        self.border("DefaultTarget")
    }

    fn cancel_target(&self) -> Ref<Border> {
        self.border("CancelTarget")
    }

    fn flyout_target(&self) -> Ref<Border> {
        self.border("FlyoutTarget")
    }
}

/// The single click trigger of the behaviors of `host`
/// (`Interaction.GetBehaviors(host).OfType<ClickEventTrigger>().Single()`).
fn single_click_event_trigger(host: &FerroObject) -> Ref<ClickEventTrigger> {
    let triggers: Vec<Ref<ClickEventTrigger>> = Interaction::get_behaviors(host)
        .snapshot()
        .iter()
        .filter_map(|behavior| behavior.cast::<ClickEventTrigger>())
        .collect();
    assert_eq!(1, triggers.len());
    triggers[0].clone()
}

/// The actions of a trigger (`trigger.Actions ??= []`).
fn actions_of(trigger: &ClickEventTrigger) -> ActionCollection {
    match trigger.actions() {
        Some(actions) => actions,
        None => {
            let actions = ActionCollection::new();
            trigger.set_actions(Some(actions.clone()));
            actions
        }
    }
}

/// A press of the space key on `target` (the key down and the key up event raised on it).
fn press_space(target: &Interactive) {
    key_down(target, Key::Space, KeyModifiers::NONE);
    key_up(target, Key::Space, KeyModifiers::NONE);
}

/// `window.KeyPressQwerty(key, RawInputModifiers.None)` with `target` focused: the key down and
/// the key up event raised on it.
fn key_press_qwerty(target: &Interactive, key: Key) {
    key_down(target, key, KeyModifiers::NONE);
    key_up(target, key, KeyModifiers::NONE);
}

/// `new Window { Width = 200, Height = 120, Content = content }`, shown.
fn window_with(content: &ferroui_controls::Control) -> Ref<Window> {
    let window = Window::new();
    window.set_width(200.0);
    window.set_height(120.0);
    window.set_content(Some(ferroui_controls::Control::boxed(content.to_ref())));
    window.show();
    run_jobs();
    capture_rendered_frame(&window);
    window
}

/// A border of 160 by 60 that takes the focus.
fn focusable_border() -> Ref<Border> {
    let border = Border::new();
    border.set_width(160.0);
    border.set_height(60.0);
    border.set_focusable(true);
    border
}

#[test]
fn click_event_trigger_save_file_picker_action_cancel_does_not_execute_command() {
    let _app = start_app();
    ferroui_xaml_interactions::register_types();
    let command_calls = Rc::new(Cell::new(0));

    let target = focusable_border();

    let trigger = ClickEventTrigger::new();
    let action = SaveFilePickerAction::new();
    let calls = command_calls.clone();
    action.set_command(Command::new(move |_| increment(&calls)));

    actions_of(&trigger).add(action.upcast());
    add_behavior(&target, &trigger);

    let _window = window_with(&target);

    MouseTestHelper::new().click(&target);

    run_jobs();
    run_jobs();

    assert_eq!(0, command_calls.get());
}

#[test]
fn click_event_trigger_button_with_picker_action_cancel_does_not_invoke_native_button_command() {
    let _app = start_app();
    ferroui_xaml_interactions::register_types();
    let native_command_calls = Rc::new(Cell::new(0));
    let picker_command_calls = Rc::new(Cell::new(0));

    let button = Button::new();
    button.set_width(160.0);
    button.set_height(60.0);
    let calls = native_command_calls.clone();
    button.set_command(Command::new(move |_| increment(&calls)));

    let trigger = ClickEventTrigger::new();
    let save_file_picker_action = SaveFilePickerAction::new();
    let calls = picker_command_calls.clone();
    save_file_picker_action.set_command(Command::new(move |_| increment(&calls)));
    actions_of(&trigger).add(save_file_picker_action.upcast());
    add_behavior(&button, &trigger);

    let _window = window_with(&button);

    MouseTestHelper::new().click(&button);

    run_jobs();
    run_jobs();

    assert_eq!(0, native_command_calls.get());
    assert_eq!(0, picker_command_calls.get());
}

#[test]
fn click_event_trigger_picker_action_use_command_can_execute_for_is_enabled_disables_associated_panel() {
    let _app = start_app();
    ferroui_xaml_interactions::register_types();
    let command = ObservableCommand::new();
    command.can_execute_result.set(false);

    let target = focusable_border();

    let trigger = ClickEventTrigger::new();
    let action = OpenFilePickerAction::new();
    action.set_command(command.as_command());
    action.set_can_execute_command_parameter(Some(Rc::new("probe".to_string())));
    action.set_use_command_can_execute_for_is_enabled(true);

    actions_of(&trigger).add(action.upcast());
    add_behavior(&target, &trigger);

    let _window = window_with(&target);

    assert!(!target.is_enabled());

    command.can_execute_result.set(true);
    command.raise_can_execute_changed();

    assert!(target.is_enabled());
}

#[test]
fn click_event_trigger_async_action_group_detaches_child_command_observers() {
    let _app = start_app();
    ferroui_xaml_interactions::register_types();
    let command = ObservableCommand::new();

    let target = focusable_border();

    let trigger = ClickEventTrigger::new();
    let group = AsyncActionGroup::new();
    let action = InvokeCommandAction::new();
    action.set_command(command.as_command());

    let group_actions = match group.actions() {
        Some(actions) => actions,
        None => {
            let actions = ActionCollection::new();
            group.set_actions(Some(actions.clone()));
            actions
        }
    };
    group_actions.add(action.clone().upcast());

    actions_of(&trigger).add(group.clone().upcast());
    add_behavior(&target, &trigger);

    let _window = window_with(&target);

    assert_eq!(1, command.subscription_count.get());

    let trigger_object: Ref<FerroObject> = trigger.upcast();
    Interaction::get_behaviors(&target).remove(&trigger_object);

    assert_eq!(0, command.subscription_count.get());
    assert!(group.parent().is_none());
    assert!(action.parent().is_none());
}

#[test]
fn click_event_trigger_routing_strategies_property_change_rewires_handlers() {
    let _app = start_app();
    ferroui_xaml_interactions::register_types();
    let native_command_calls = Rc::new(Cell::new(0));
    let picker_command_calls = Rc::new(Cell::new(0));

    let button = Button::new();
    button.set_width(160.0);
    button.set_height(60.0);
    let calls = native_command_calls.clone();
    button.set_command(Command::new(move |_| increment(&calls)));

    let trigger = ClickEventTrigger::new();
    trigger.set_routing_strategies(RoutingStrategies::BUBBLE);
    let save_file_picker_action = SaveFilePickerAction::new();
    let calls = picker_command_calls.clone();
    save_file_picker_action.set_command(Command::new(move |_| increment(&calls)));

    actions_of(&trigger).add(save_file_picker_action.upcast());
    add_behavior(&button, &trigger);

    let _window = window_with(&button);
    let mouse = MouseTestHelper::new();

    // Bubble route allows native Button command to run before the trigger consumes input.
    mouse.click(&button);
    run_jobs();
    run_jobs();

    assert_eq!(1, native_command_calls.get());
    assert_eq!(0, picker_command_calls.get());

    // Switching to Tunnel should rewire handlers and suppress native Button command on cancel.
    trigger.set_routing_strategies(RoutingStrategies::TUNNEL);

    mouse.reset_click_count();
    mouse.click(&button);
    run_jobs();
    run_jobs();

    assert_eq!(1, native_command_calls.get());
    assert_eq!(0, picker_command_calls.get());
}

#[test]
fn click_event_trigger_release_mode_does_not_fire_when_released_outside() {
    let _app = start_app();
    let window = ClickEventTrigger001::new();

    window.show();
    let mouse = MouseTestHelper::new();

    mouse.down_at(&window.release_target(), Point::new(10.0, 10.0), MouseButton::Left, KeyModifiers::NONE);
    mouse.up_at(&window.outside_target(), Point::new(10.0, 10.0), MouseButton::Left, KeyModifiers::NONE);

    assert_eq!(0, window.context.release_clicks.get());

    mouse.reset_click_count();
    mouse.click(&window.release_target());

    assert_eq!(1, window.context.release_clicks.get());
}

#[test]
fn click_event_trigger_release_mode_does_not_leak_pressed_state_across_interactions() {
    let _app = start_app();
    let window = ClickEventTrigger001::new();

    window.show();
    let mouse = MouseTestHelper::new();

    mouse.down_at(&window.release_target(), Point::new(10.0, 10.0), MouseButton::Left, KeyModifiers::NONE);
    mouse.up_at(&window.outside_target(), Point::new(10.0, 10.0), MouseButton::Left, KeyModifiers::NONE);
    assert_eq!(0, window.context.release_clicks.get());

    mouse.reset_click_count();
    mouse.down_at(&window.outside_target(), Point::new(10.0, 10.0), MouseButton::Left, KeyModifiers::NONE);
    mouse.up_at(&window.release_target(), Point::new(10.0, 10.0), MouseButton::Left, KeyModifiers::NONE);
    assert_eq!(0, window.context.release_clicks.get());
}

#[test]
fn click_event_trigger_press_mode_fires_on_pointer_down() {
    let _app = start_app();
    let window = ClickEventTrigger001::new();

    window.show();

    MouseTestHelper::new().down_at(
        &window.press_target(),
        Point::new(10.0, 10.0),
        MouseButton::Left,
        KeyModifiers::NONE,
    );

    assert_eq!(1, window.context.press_clicks.get());
}

#[test]
fn click_event_trigger_key_modifiers_filter_blocks_and_allows() {
    let _app = start_app();
    let window = ClickEventTrigger001::new();

    window.show();
    let mouse = MouseTestHelper::new();

    mouse.click(&window.modifier_target());
    assert_eq!(0, window.context.modifier_clicks.get());

    mouse.reset_click_count();
    mouse.click_with(&window.modifier_target(), KeyModifiers::CONTROL);
    assert_eq!(1, window.context.modifier_clicks.get());
}

#[test]
fn click_event_trigger_source_control_uses_external_source_control() {
    let _app = start_app();
    let window = ClickEventTrigger001::new();

    window.show();
    let mouse = MouseTestHelper::new();

    mouse.click(&window.source_control_host_target());
    assert_eq!(0, window.context.source_control_clicks.get());

    mouse.reset_click_count();
    mouse.click(&window.source_control_source_target());
    assert_eq!(1, window.context.source_control_clicks.get());
}

#[test]
fn click_event_trigger_handled_events_too_default_false_and_property_change_rewires_handlers_for_button_and_text_box_sources(
) {
    let _app = start_app();
    let window = ClickEventTrigger001::new();

    window.show();
    let mouse = MouseTestHelper::new();
    let click = |target: &Interactive| {
        mouse.reset_click_count();
        mouse.click(target);
    };

    let button_trigger = single_click_event_trigger(&window.handled_events_too_host_target());
    let text_box_trigger = single_click_event_trigger(&window.handled_events_too_text_box_host_target());

    assert!(!button_trigger.handled_events_too());
    assert!(!text_box_trigger.handled_events_too());

    click(&window.handled_events_too_source_target());
    assert_eq!(0, window.context.handled_events_too_clicks.get());
    click(&window.handled_events_too_text_box_source_target());
    assert_eq!(0, window.context.handled_events_too_text_box_clicks.get());

    button_trigger.set_handled_events_too(true);
    text_box_trigger.set_handled_events_too(true);

    click(&window.handled_events_too_source_target());
    assert_eq!(1, window.context.handled_events_too_clicks.get());
    click(&window.handled_events_too_text_box_source_target());
    assert_eq!(1, window.context.handled_events_too_text_box_clicks.get());

    button_trigger.set_handled_events_too(false);
    text_box_trigger.set_handled_events_too(false);

    click(&window.handled_events_too_source_target());
    assert_eq!(1, window.context.handled_events_too_clicks.get());
    click(&window.handled_events_too_text_box_source_target());
    assert_eq!(1, window.context.handled_events_too_text_box_clicks.get());
}

#[test]
fn click_event_trigger_handled_events_too_also_controls_keyboard_handlers() {
    let _app = start_app();
    let window = ClickEventTrigger001::new();

    window.show();

    let text_box_trigger = single_click_event_trigger(&window.handled_events_too_text_box_host_target());
    let source = window.handled_events_too_text_box_source_target();

    source.focus();
    press_space(&source);
    assert_eq!(0, window.context.handled_events_too_text_box_clicks.get());

    text_box_trigger.set_handled_events_too(true);

    source.focus();
    press_space(&source);
    assert_eq!(1, window.context.handled_events_too_text_box_clicks.get());

    text_box_trigger.set_handled_events_too(false);

    source.focus();
    press_space(&source);
    assert_eq!(1, window.context.handled_events_too_text_box_clicks.get());
}

#[test]
fn click_event_trigger_handle_event_false_button_preserves_native_click() {
    let _app = start_app();
    let window = ClickEventTrigger001::new();

    window.show();

    MouseTestHelper::new().click(&window.handle_event_false_button_target());

    assert_eq!(1, window.context.handle_event_false_button_clicks.get());
    assert_eq!(1, window.context.handle_event_false_button_native_clicks.get());
    assert_eq!(1, window.context.handle_event_false_button_click_events.get());
}

#[test]
fn click_event_trigger_handle_event_false_text_box_preserves_key_bubbling() {
    let _app = start_app();
    let window = ClickEventTrigger001::new();

    window.show();

    let text_box = window.handle_event_false_text_box_target();
    text_box.focus();
    press_space(&text_box);

    assert_eq!(1, window.context.handle_event_false_text_box_clicks.get());
    assert_eq!(1, window.context.handle_event_false_text_box_click_events.get());
    assert_eq!(1, window.context.handle_event_false_text_box_bubbled_key_up.get());
}

#[test]
fn click_event_trigger_handle_event_false_text_box_pointer_click_fires_trigger_and_click_event() {
    let _app = start_app();
    let window = ClickEventTrigger001::new();

    window.show();

    MouseTestHelper::new().click(&window.handle_event_false_text_box_target());

    assert_eq!(1, window.context.handle_event_false_text_box_clicks.get());
    assert_eq!(1, window.context.handle_event_false_text_box_click_events.get());
}

#[test]
fn click_event_trigger_handle_event_false_text_box_allows_pointer_drag_selection() {
    let _app = start_app();
    let window = ClickEventTrigger001::new();

    window.show();

    let text_box = window.handle_event_false_text_box_target();
    text_box.focus();
    text_box.set_caret_index(0);

    let y = text_box.bounds().height / 2.0;
    let width = text_box.bounds().width;
    let mouse = MouseTestHelper::new();
    mouse.down_at(&text_box, Point::new(4.0, y), MouseButton::Left, KeyModifiers::NONE);
    mouse.move_at(&text_box, Point::new(width - 6.0, y), RawInputModifiers::LEFT_MOUSE_BUTTON);
    mouse.up_at(&text_box, Point::new(width - 6.0, y), MouseButton::Left, KeyModifiers::NONE);

    assert!((text_box.selection_end() - text_box.selection_start()).abs() > 0);
}

#[test]
fn click_event_trigger_space_handling_works_for_press_and_release_modes() {
    let _app = start_app();
    let window = ClickEventTrigger001::new();

    window.show();

    let space_release_target = window.space_release_target();
    space_release_target.focus();
    press_space(&space_release_target);
    assert_eq!(1, window.context.space_release_clicks.get());

    let space_press_target = window.space_press_target();
    space_press_target.focus();
    press_space(&space_press_target);
    assert_eq!(1, window.context.space_press_clicks.get());
}

#[test]
fn click_event_trigger_is_default_root_enter_triggers_and_does_not_duplicate() {
    let _app = start_app();
    let window = ClickEventTrigger001::new();

    window.show();

    let focus_source = window.focus_source();
    focus_source.focus();
    key_press_qwerty(&focus_source, Key::Enter);
    assert_eq!(1, window.context.default_clicks.get());

    let default_target = window.default_target();
    default_target.focus();
    key_press_qwerty(&default_target, Key::Enter);
    assert_eq!(2, window.context.default_clicks.get());
}

#[test]
fn click_event_trigger_is_cancel_root_escape_triggers_and_does_not_duplicate() {
    let _app = start_app();
    let window = ClickEventTrigger001::new();

    window.show();

    let focus_source = window.focus_source();
    focus_source.focus();
    key_press_qwerty(&focus_source, Key::Escape);
    assert_eq!(1, window.context.cancel_clicks.get());

    let cancel_target = window.cancel_target();
    cancel_target.focus();
    key_press_qwerty(&cancel_target, Key::Escape);
    assert_eq!(1, window.context.cancel_clicks.get());
}

#[test]
fn click_event_trigger_flyout_toggles_open_and_close() {
    let _app = start_app();
    let window = ClickEventTrigger001::new();

    window.show();

    let flyout = FlyoutBase::get_attached_flyout(&window.flyout_target()).expect("the attached flyout");
    assert!(!flyout.is_open());

    let mouse = MouseTestHelper::new();
    mouse.click(&window.flyout_target());

    assert!(flyout.is_open());
    assert_eq!(1, window.context.flyout_clicks.get());

    mouse.reset_click_count();
    mouse.click(&window.flyout_target());

    assert!(!flyout.is_open());
    assert_eq!(2, window.context.flyout_clicks.get());
}
