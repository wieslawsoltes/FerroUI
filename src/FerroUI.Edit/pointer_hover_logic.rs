use crate::TextArea;
use ferroui_base::input::{
    IPointer, InputElement, KeyModifiers, PointerEventArgs, PointerPointProperties,
};
use ferroui_base::interactivity::{RoutedEvent, RoutingStrategies};
use ferroui_base::reactive::Subscription;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::{Point, Visual, WeakRef};
use ferroui_controls::Control;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

const HOVER_DELAY: Duration = Duration::from_millis(400);
const HOVER_TOLERANCE: f64 = 2.0;

/// Tracks pointer hover on a control. Dropping the attachment cancels its
/// timer and removes the input handlers.
#[must_use = "keep the attachment alive while tracking hover"]
pub struct PointerHoverLogic {
    _handlers: [Subscription; 4],
    _state: Rc<State>,
}

#[derive(Clone)]
struct HoverInput {
    root: WeakRef<Visual>,
    position: Point,
    pointer: Rc<dyn IPointer>,
    timestamp: u64,
    properties: PointerPointProperties,
    modifiers: KeyModifiers,
}

struct Activity {
    start: Point,
    input: HoverInput,
    timer: Option<Subscription>,
}

struct State {
    target: WeakRef<Control>,
    activity: RefCell<Option<Activity>>,
    is_observed: Box<dyn Fn() -> bool>,
    hovered: Box<dyn Fn(&PointerEventArgs)>,
    stopped: Box<dyn Fn(&PointerEventArgs)>,
}

impl PointerHoverLogic {
    /// Invokes the callbacks with the original pointer position after 400 ms
    /// within two units of the starting point on each axis.
    pub fn attach(
        target: &Control,
        hovered: impl Fn(&PointerEventArgs) + 'static,
        stopped: impl Fn(&PointerEventArgs) + 'static,
    ) -> Self {
        Self::attach_when(target, || true, hovered, stopped)
    }

    fn attach_when(
        target: &Control,
        is_observed: impl Fn() -> bool + 'static,
        hovered: impl Fn(&PointerEventArgs) + 'static,
        stopped: impl Fn(&PointerEventArgs) + 'static,
    ) -> Self {
        let state = Rc::new(State {
            target: target.to_ref().downgrade(),
            activity: RefCell::new(None),
            is_observed: Box::new(is_observed),
            hovered: Box::new(hovered),
            stopped: Box::new(stopped),
        });
        let weak = Rc::downgrade(&state);
        let entered = target.add_disposable_handler_with(
            InputElement::pointer_entered_event(),
            move |_, input| {
                if let Some(state) = weak.upgrade() {
                    state.update(input, true);
                }
            },
            RoutingStrategies::DIRECT,
            true,
        );
        let weak = Rc::downgrade(&state);
        let moved = target.add_disposable_handler_with(
            InputElement::pointer_moved_event(),
            move |_, input| {
                if let Some(state) = weak.upgrade() {
                    state.update(input, false);
                }
            },
            RoutingStrategies::BUBBLE,
            true,
        );
        let weak = Rc::downgrade(&state);
        let exited = target.add_disposable_handler_with(
            InputElement::pointer_exited_event(),
            move |_, _| {
                if let Some(state) = weak.upgrade() {
                    let previous = state.activity.take();
                    state.report_stop(previous);
                }
            },
            RoutingStrategies::DIRECT,
            true,
        );
        let weak = Rc::downgrade(&state);
        let detached = target.detached_from_visual_tree(move |_| {
            if let Some(state) = weak.upgrade() {
                state.activity.take();
            }
        });
        Self {
            _handlers: [entered, moved, exited, detached],
            _state: state,
        }
    }
}

impl State {
    fn update(self: &Rc<Self>, input: &PointerEventArgs, entered: bool) {
        if !(self.is_observed)() {
            self.activity.take();
            return;
        }
        let Some(target) = self.target.upgrade() else {
            return;
        };
        let Some(root) = target.visual_root() else {
            return;
        };
        let start = input.get_position(Some(&target));
        if !entered
            && self.activity.borrow().as_ref().is_some_and(|activity| {
                activity.input.pointer.id() == input.pointer().id()
                    && (activity.start.x - start.x).abs() <= HOVER_TOLERANCE
                    && (activity.start.y - start.y).abs() <= HOVER_TOLERANCE
            })
        {
            return;
        }
        let Some(position) = target.translate_point(start, &root) else {
            return;
        };
        let snapshot = HoverInput {
            root: root.downgrade(),
            position,
            pointer: input.pointer().clone(),
            timestamp: input.timestamp(),
            properties: input.properties(),
            modifiers: input.key_modifiers(),
        };
        let weak = Rc::downgrade(self);
        let timer = DispatcherTimer::run_once(
            move || {
                if let Some(state) = weak.upgrade() {
                    state.report_hover();
                }
            },
            HOVER_DELAY,
            DispatcherPriority::BACKGROUND,
        );
        // Install the replacement before notifying handlers so a reentrant
        // exit or detach can cancel it.
        let previous = self.activity.replace(Some(Activity {
            start,
            input: snapshot,
            timer: Some(timer),
        }));
        self.report_stop(previous);
    }

    fn report_hover(&self) {
        let input = {
            let mut activity = self.activity.borrow_mut();
            let Some(activity) = activity.as_mut() else {
                return;
            };
            if activity.timer.take().is_none() {
                return;
            }
            activity.input.clone()
        };
        if let Some(args) = self.event_args(input) {
            (self.hovered)(&args);
        }
    }

    fn report_stop(&self, previous: Option<Activity>) {
        if let Some(activity) = previous.filter(|activity| activity.timer.is_none()) {
            if let Some(args) = self.event_args(activity.input) {
                (self.stopped)(&args);
            }
        }
    }

    fn event_args(&self, input: HoverInput) -> Option<PointerEventArgs> {
        let target = self.target.upgrade()?;
        let root = input.root.upgrade()?;
        if target.visual_root().as_ref() != Some(&root) {
            return None;
        }
        Some(PointerEventArgs::new::<PointerEventArgs>(
            None,
            target,
            input.pointer,
            Some(&root),
            input.position,
            input.timestamp,
            input.properties,
            input.modifiers,
        ))
    }
}

impl TextArea {
    ferroui_base::ferro_routed_event!(
        /// Tunnels before the editor reports a pointer hover.
        pub fn preview_pointer_hover_event() -> RoutedEvent<PointerEventArgs> {
            RoutedEvent::register::<TextArea, _>("PreviewPointerHover", RoutingStrategies::TUNNEL)
        }
    );
    ferroui_base::ferro_routed_event!(
        /// Bubbles when the pointer has hovered over the editor.
        pub fn pointer_hover_event() -> RoutedEvent<PointerEventArgs> {
            RoutedEvent::register::<TextArea, _>("PointerHover", RoutingStrategies::BUBBLE)
        }
    );
    ferroui_base::ferro_routed_event!(
        /// Tunnels before the editor reports the end of a hover.
        pub fn preview_pointer_hover_stopped_event() -> RoutedEvent<PointerEventArgs> {
            RoutedEvent::register::<TextArea, _>(
                "PreviewPointerHoverStopped",
                RoutingStrategies::TUNNEL,
            )
        }
    );
    ferroui_base::ferro_routed_event!(
        /// Bubbles when the pointer leaves the original hover position.
        pub fn pointer_hover_stopped_event() -> RoutedEvent<PointerEventArgs> {
            RoutedEvent::register::<TextArea, _>("PointerHoverStopped", RoutingStrategies::BUBBLE)
        }
    );

    pub(crate) fn attach_pointer_hover(&self) -> PointerHoverLogic {
        let hovered = self.to_ref().downgrade();
        let stopped = hovered.clone();
        let preview_hover = Self::preview_pointer_hover_event();
        let hover = Self::pointer_hover_event();
        let preview_stop = Self::preview_pointer_hover_stopped_event();
        let stop = Self::pointer_hover_stopped_event();
        PointerHoverLogic::attach_when(
            self,
            move || {
                [preview_hover, hover, preview_stop, stop]
                    .iter()
                    .any(|event| {
                        event.has_instance_handlers()
                            || event.has_raised_subscriptions()
                            || event.route_finished().has_observers()
                    })
            },
            move |args| {
                if let Some(area) = hovered.upgrade() {
                    area.raise_pointer_hover(args, preview_hover, hover);
                }
            },
            move |args| {
                if let Some(area) = stopped.upgrade() {
                    area.raise_pointer_hover(args, preview_stop, stop);
                }
            },
        )
    }

    fn raise_pointer_hover(
        &self,
        args: &PointerEventArgs,
        preview: &RoutedEvent<PointerEventArgs>,
        bubble: &RoutedEvent<PointerEventArgs>,
    ) {
        let root = self.visual_root();
        args.set_routed_event(Some(preview));
        self.raise_event(args);
        if self.visual_root() == root {
            args.set_routed_event(Some(bubble));
            self.raise_event(args);
        }
    }
}

#[cfg(test)]
#[path = "pointer_hover_logic_tests.rs"]
mod tests;
