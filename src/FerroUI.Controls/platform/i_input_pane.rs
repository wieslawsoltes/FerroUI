use ferroui_base::animation::easings::IEasing;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::HandlerList;
use ferroui_base::Rect;
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

/// Listener for the platform's input pane (e.g. software keyboard). Provides
/// access to the input pane height and state.
pub trait IInputPane {
    /// The current input pane state.
    fn state(&self) -> InputPaneState;

    /// The current input pane bounds.
    fn occluded_rect(&self) -> Rect;

    /// Occurs when the input pane's state has changed.
    ///
    /// Disposing the returned value removes the handler.
    fn state_changed(&self, handler: Rc<dyn Fn(&InputPaneStateEventArgs)>) -> Rc<dyn IDisposable>;
}

/// A base implementation of [`IInputPane`] for platform backends: keeps the
/// state and raises the state changed event.
#[derive(Default)]
pub struct InputPaneBase {
    state: Cell<InputPaneState>,
    occluded_rect: Cell<Rect>,
    state_changed: Rc<HandlerList<dyn Fn(&InputPaneStateEventArgs)>>,
}

impl InputPaneBase {
    /// Creates a closed input pane.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the current input pane state.
    pub fn set_state(&self, value: InputPaneState) {
        self.state.set(value);
    }

    /// Sets the current input pane bounds.
    pub fn set_occluded_rect(&self, value: Rect) {
        self.occluded_rect.set(value);
    }

    /// Raises the state changed event on the UI thread.
    pub fn on_state_changed(&self, event_args: InputPaneStateEventArgs) {
        let handlers = self.state_changed.clone();
        // The operation can only be canceled by a shutdown of the dispatcher.
        let _ = Dispatcher::ui_thread().invoke_local(move || {
            for (_, handler) in handlers.snapshot().iter() {
                handler(&event_args);
            }
        });
    }
}

impl IInputPane for InputPaneBase {
    fn state(&self) -> InputPaneState {
        self.state.get()
    }

    fn occluded_rect(&self) -> Rect {
        self.occluded_rect.get()
    }

    fn state_changed(&self, handler: Rc<dyn Fn(&InputPaneStateEventArgs)>) -> Rc<dyn IDisposable> {
        let token = self.state_changed.add(handler);
        let handlers = self.state_changed.clone();
        Disposable::create(move || {
            handlers.remove(token);
        })
    }
}

/// The input pane opened state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum InputPaneState {
    /// The input pane is either closed, or doesn't form part of the
    /// platform insets, i.e. it's floating or is an overlay.
    #[default]
    Closed = 0,

    /// The input pane is open.
    Open = 1,
}

/// Provides state change information about the input pane.
#[derive(Clone)]
pub struct InputPaneStateEventArgs {
    new_state: InputPaneState,
    start_rect: Option<Rect>,
    end_rect: Rect,
    animation_duration: Duration,
    easing: Option<Rc<dyn IEasing>>,
}

impl InputPaneStateEventArgs {
    /// Creates the event args for a change that is not animated.
    pub fn new(new_state: InputPaneState, start_rect: Option<Rect>, end_rect: Rect) -> Self {
        Self::with_animation(new_state, start_rect, end_rect, Duration::ZERO, None)
    }

    /// Creates the event args for an animated change.
    pub fn with_animation(
        new_state: InputPaneState,
        start_rect: Option<Rect>,
        end_rect: Rect,
        animation_duration: Duration,
        easing: Option<Rc<dyn IEasing>>,
    ) -> Self {
        Self { new_state, start_rect, end_rect, animation_duration, easing }
    }

    /// The new state of the input pane.
    pub fn new_state(&self) -> InputPaneState {
        self.new_state
    }

    /// The initial bounds of the input pane.
    pub fn start_rect(&self) -> Option<Rect> {
        self.start_rect
    }

    /// The final bounds of the input pane.
    pub fn end_rect(&self) -> Rect {
        self.end_rect
    }

    /// The duration of the input pane's state change animation.
    pub fn animation_duration(&self) -> Duration {
        self.animation_duration
    }

    /// The easing of the input pane's state changed animation.
    pub fn easing(&self) -> Option<&Rc<dyn IEasing>> {
        self.easing.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn base_keeps_state_and_raises_state_changed() {
        let _scope = Dispatcher::unit_test_scope();
        let pane = InputPaneBase::new();
        assert_eq!(pane.state(), InputPaneState::Closed);
        assert_eq!(pane.occluded_rect(), Rect::default());

        let received = Rc::new(RefCell::new(Vec::new()));
        let r = received.clone();
        let subscription = pane.state_changed(Rc::new(move |e| r.borrow_mut().push(e.clone())));

        let open_rect = Rect::new(0.0, 200.0, 200.0, 200.0);
        pane.set_occluded_rect(open_rect);
        pane.set_state(InputPaneState::Open);
        pane.on_state_changed(InputPaneStateEventArgs::new(pane.state(), Some(Rect::default()), open_rect));

        assert_eq!(pane.state(), InputPaneState::Open);
        assert_eq!(pane.occluded_rect(), open_rect);
        assert_eq!(received.borrow().len(), 1);
        assert_eq!(received.borrow()[0].new_state(), InputPaneState::Open);
        assert_eq!(received.borrow()[0].start_rect(), Some(Rect::default()));
        assert_eq!(received.borrow()[0].end_rect(), open_rect);
        assert_eq!(received.borrow()[0].animation_duration(), Duration::ZERO);

        subscription.dispose();
        pane.on_state_changed(InputPaneStateEventArgs::new(InputPaneState::Closed, Some(open_rect), Rect::default()));
        assert_eq!(received.borrow().len(), 1);
    }
}
