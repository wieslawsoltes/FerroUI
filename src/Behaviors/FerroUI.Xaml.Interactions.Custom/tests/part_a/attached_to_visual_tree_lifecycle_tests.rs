//! Ported from `tests/Xaml.Behaviors.Interactions.UnitTests/Custom/AttachedToVisualTreeLifecycleTests.cs`
//! of Xaml.Behaviors.
//!
//! Adapted: the window is shown with the services of the test application (see `support.rs`); a
//! panel removes and adds its child without a rendered frame (the layout is updated where the
//! reference captures a frame); the moves of the mouse raise the pointer events on the elements
//! (see `Mouse`).

use super::command::Command;
use super::support::{run_jobs, start_app};
use crate::{
    AttachedToVisualTreeBehavior, AttachedToVisualTreeBehaviorImpl, AttachedToVisualTreeTriggerBase,
    AttachedToVisualTreeTriggerBaseImpl, PointerEnteredTrigger, RoutedEventTrigger, RoutedEventTriggerImpl,
};
use ferroui_base::input::{
    InputElement, KeyModifiers, Pointer, PointerEventArgs, PointerPointProperties, PointerType, PointerUpdateKind,
    RawInputModifiers,
};
use ferroui_base::interactivity::{Interactive, RoutedEvent};
use ferroui_base::media::Brushes;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Point, Ref, StaticType, StyledElementImpl, Visual};
use ferroui_controls::{Border, Control, Panel, StackPanel, Window};
use ferroui_xaml_interactions::InvokeCommandAction;
use ferroui_xaml_interactivity::{
    DisposableAction, DisposingBehaviorImpl, DisposingTriggerImpl, Interaction, StyledElementBehaviorImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The counts of the subscriptions of a tracking trigger or behavior.
#[derive(Default)]
pub(crate) struct Subscriptions {
    active: Cell<i32>,
    created: Cell<i32>,
    disposed: Cell<i32>,
}

impl Subscriptions {
    /// `OnAttachedToVisualTreeOverride` of the tracking classes: counts a new subscription and
    /// returns its disposable, which counts its disposal.
    fn create(subscriptions: &Rc<Subscriptions>) -> Rc<dyn IDisposable> {
        subscriptions.active.set(subscriptions.active.get() + 1);
        subscriptions.created.set(subscriptions.created.get() + 1);

        let subscriptions = subscriptions.clone();
        DisposableAction::create(move || {
            subscriptions.active.set(subscriptions.active.get() - 1);
            subscriptions.disposed.set(subscriptions.disposed.get() + 1);
        })
    }
}

/// `TrackingTrigger`: an `AttachedToVisualTreeTriggerBase<Border>` that counts its subscriptions.
#[repr(C)]
pub(crate) struct TrackingTrigger {
    base: AttachedToVisualTreeTriggerBase,
    subscriptions: Rc<Subscriptions>,
}

ferro_class!(TrackingTrigger: AttachedToVisualTreeTriggerBase);
ferroui_base::ferro_impl_classes!(
    TrackingTrigger: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingTriggerImpl,
);

ferroui_base::ferro_overrides! {
    impl AttachedToVisualTreeTriggerBaseImpl for TrackingTrigger {
        fn on_attached_to_visual_tree_override(this: &Self) -> Rc<dyn IDisposable> {
            Subscriptions::create(&this.subscriptions)
        }
    }
}

impl TrackingTrigger {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: AttachedToVisualTreeTriggerBase::construct(<Border as StaticType>::TYPE),
            subscriptions: Rc::new(Subscriptions::default()),
        })
    }
}

/// `TrackingBehavior`: an `AttachedToVisualTreeBehavior<Border>` that counts its subscriptions.
#[repr(C)]
pub(crate) struct TrackingBehavior {
    base: AttachedToVisualTreeBehavior,
    subscriptions: Rc<Subscriptions>,
}

ferro_class!(TrackingBehavior: AttachedToVisualTreeBehavior);
ferroui_base::ferro_impl_classes!(
    TrackingBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
);

ferroui_base::ferro_overrides! {
    impl AttachedToVisualTreeBehaviorImpl for TrackingBehavior {
        fn on_attached_to_visual_tree_override(this: &Self) -> Rc<dyn IDisposable> {
            Subscriptions::create(&this.subscriptions)
        }
    }
}

impl TrackingBehavior {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: AttachedToVisualTreeBehavior::construct(<Border as StaticType>::TYPE),
            subscriptions: Rc::new(Subscriptions::default()),
        })
    }
}

/// `new Window { Content = panel }`, shown.
fn show_window_with(panel: &Ref<Panel>) -> Ref<Window> {
    let window = Window::new();
    window.set_content(Some(Control::boxed(panel)));
    window.show();
    window
}

/// `NonGenericPointerEnteredTrigger`: a `RoutedEventTrigger` of `InputElement.PointerEnteredEvent`.
#[repr(C)]
pub(crate) struct NonGenericPointerEnteredTrigger {
    base: RoutedEventTrigger,
}

ferro_class!(NonGenericPointerEnteredTrigger: RoutedEventTrigger);
ferroui_base::ferro_impl_classes!(
    NonGenericPointerEnteredTrigger: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingTriggerImpl,
    AttachedToVisualTreeTriggerBaseImpl,
);

ferroui_base::ferro_overrides! {
    impl RoutedEventTriggerImpl for NonGenericPointerEnteredTrigger {
        fn routed_event(_this: &Self) -> RoutedEvent {
            InputElement::pointer_entered_event().as_routed_event()
        }
    }
}

impl NonGenericPointerEnteredTrigger {
    fn new() -> Ref<Self> {
        instantiate(Self { base: RoutedEventTrigger::construct() })
    }
}

/// The mouse of the test (`MouseMove` of `HeadlessWindowExtensions.cs` of the reference). The
/// headless window of the reference hit-tests the move and its pointer-over processor raises
/// `PointerExited` on the element the pointer leaves and `PointerEntered` on the element it
/// enters; here the elements are given (siblings of a panel, so no ancestor changes its state) and
/// the events are raised on them directly, then the move itself.
struct Mouse {
    pointer: Rc<Pointer>,
    over: RefCell<Option<Ref<Interactive>>>,
    next_stamp: Cell<u64>,
}

impl Mouse {
    fn new() -> Self {
        Self {
            pointer: Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true),
            over: RefCell::new(None),
            next_stamp: Cell::new(1),
        }
    }

    fn raise(&self, target: &Interactive, routed_event: &RoutedEvent<PointerEventArgs>, position: Point) {
        let root: Ref<Visual> = target.visual_root().unwrap_or_else(|| target.to_ref().upcast());
        let position = target.translate_point(position, &root).unwrap_or(position);
        let stamp = self.next_stamp.get();
        self.next_stamp.set(stamp + 1);
        target.raise_event(&PointerEventArgs::new(
            Some(routed_event),
            target.to_ref(),
            self.pointer.clone(),
            Some(&*root),
            position,
            stamp,
            PointerPointProperties::new(RawInputModifiers::NONE, PointerUpdateKind::Other),
            KeyModifiers::NONE,
        ));
    }

    /// `window.MouseMove(target, position)`.
    fn move_to(&self, target: &Interactive, position: Point) {
        let previous = self.over.borrow().clone();
        if previous.as_ref().is_none_or(|previous| !std::ptr::eq(&**previous, target)) {
            if let Some(previous) = previous {
                self.raise(&previous, InputElement::pointer_exited_event(), position);
            }
            *self.over.borrow_mut() = Some(target.to_ref());
            self.raise(target, InputElement::pointer_entered_event(), position);
        }
        self.raise(target, InputElement::pointer_moved_event(), position);
    }
}

#[test]
fn pointer_entered_trigger_executes_once_after_visual_tree_reattach() {
    let _app = start_app();
    let command_calls = Rc::new(Cell::new(0));
    let non_generic_command_calls = Rc::new(Cell::new(0));
    let target = Border::new();
    target.set_width(100.0);
    target.set_height(100.0);
    target.set_background(Some(Brushes::red()));
    let outside = Border::new();
    outside.set_width(100.0);
    outside.set_height(100.0);
    outside.set_background(Some(Brushes::blue()));
    let panel = StackPanel::new();
    panel.children().add(&target);
    panel.children().add(&outside);
    let trigger = PointerEnteredTrigger::new();
    let action = InvokeCommandAction::new();
    let calls = command_calls.clone();
    action.set_command(Some(Command::new(move |_| calls.set(calls.get() + 1))));
    trigger.actions().expect("the actions of the trigger").add(action.upcast());
    let non_generic_trigger = NonGenericPointerEnteredTrigger::new();
    let non_generic_action = InvokeCommandAction::new();
    let calls = non_generic_command_calls.clone();
    non_generic_action.set_command(Some(Command::new(move |_| calls.set(calls.get() + 1))));
    non_generic_trigger.actions().expect("the actions of the trigger").add(non_generic_action.upcast());
    let behaviors = Interaction::get_behaviors(&target);
    behaviors.add(trigger.clone().upcast());
    behaviors.add(non_generic_trigger.clone().upcast());
    let window = Window::new();
    window.set_width(240.0);
    window.set_height(240.0);
    window.set_content(Some(Control::boxed(panel.clone())));
    window.show();
    window.update_layout();
    let mouse = Mouse::new();

    mouse.move_to(&outside, Point::new(10.0, 10.0));
    mouse.move_to(&target, Point::new(10.0, 10.0));

    assert_eq!(1, command_calls.get());
    assert_eq!(1, non_generic_command_calls.get());

    panel.children().remove(&target);
    run_jobs();
    panel.children().insert(0, &target);
    run_jobs();
    window.update_layout();

    mouse.move_to(&outside, Point::new(10.0, 10.0));
    let calls_before_second_entry = command_calls.get();
    let non_generic_calls_before_second_entry = non_generic_command_calls.get();
    mouse.move_to(&target, Point::new(10.0, 10.0));

    assert_eq!(calls_before_second_entry + 1, command_calls.get());
    assert_eq!(non_generic_calls_before_second_entry + 1, non_generic_command_calls.get());

    Interaction::set_behaviors(&target, None);
    mouse.move_to(&outside, Point::new(10.0, 10.0));
    let calls_before_detached_entry = command_calls.get();
    let non_generic_calls_before_detached_entry = non_generic_command_calls.get();
    mouse.move_to(&target, Point::new(10.0, 10.0));

    assert_eq!(calls_before_detached_entry, command_calls.get());
    assert_eq!(non_generic_calls_before_detached_entry, non_generic_command_calls.get());
}

#[test]
fn attached_to_visual_tree_trigger_base_disposes_for_each_visual_tree_lifetime() {
    let _app = start_app();
    let target = Border::new();
    let panel = Panel::new();
    panel.children().add(&target);
    let trigger = TrackingTrigger::new();
    Interaction::get_behaviors(&target).add(trigger.clone().upcast());
    let _window = show_window_with(&panel);
    let subscriptions = trigger.subscriptions.clone();

    assert_eq!(1, subscriptions.active.get());

    panel.children().remove(&target);

    assert_eq!(0, subscriptions.active.get());
    assert_eq!(1, subscriptions.disposed.get());

    panel.children().add(&target);

    assert_eq!(1, subscriptions.active.get());
    assert_eq!(2, subscriptions.created.get());

    panel.children().remove(&target);

    assert_eq!(0, subscriptions.active.get());
    assert_eq!(2, subscriptions.disposed.get());
}

#[test]
fn attached_to_visual_tree_behavior_disposes_for_each_visual_tree_lifetime() {
    let _app = start_app();
    let target = Border::new();
    let panel = Panel::new();
    panel.children().add(&target);
    let behavior = TrackingBehavior::new();
    Interaction::get_behaviors(&target).add(behavior.clone().upcast());
    let _window = show_window_with(&panel);
    let subscriptions = behavior.subscriptions.clone();

    assert_eq!(1, subscriptions.active.get());

    panel.children().remove(&target);

    assert_eq!(0, subscriptions.active.get());
    assert_eq!(1, subscriptions.disposed.get());

    panel.children().add(&target);

    assert_eq!(1, subscriptions.active.get());
    assert_eq!(2, subscriptions.created.get());

    panel.children().remove(&target);

    assert_eq!(0, subscriptions.active.get());
    assert_eq!(2, subscriptions.disposed.get());
}
