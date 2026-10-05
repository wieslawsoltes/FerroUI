use ferroui_base::reactive::IDisposable;
use ferroui_base::Rect;
use ferroui_controls::platform::{IInputPane, InputPaneBase, InputPaneState, InputPaneStateEventArgs};
use std::rc::Rc;

/// The input pane of the browser: the part of a view the on-screen
/// keyboard covers, as the virtual keyboard of the page reports it.
#[derive(Default)]
pub struct BrowserInputPane {
    base: InputPaneBase,
}

impl BrowserInputPane {
    /// Creates a closed input pane.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// The page reported the rectangle of the on-screen keyboard, relative
    /// to the element of the view.
    pub fn on_geometry_change(&self, x: f64, y: f64, width: f64, height: f64) -> bool {
        let old_state = (self.base.occluded_rect(), self.base.state());

        let occluded_rect = Rect::new(x, y, width, height);
        self.base.set_occluded_rect(occluded_rect);
        let state = if occluded_rect.width != 0.0 { InputPaneState::Open } else { InputPaneState::Closed };
        self.base.set_state(state);

        if old_state != (occluded_rect, state) {
            self.base.on_state_changed(InputPaneStateEventArgs::new(state, None, occluded_rect));
        }

        true
    }
}

impl IInputPane for BrowserInputPane {
    fn state(&self) -> InputPaneState {
        self.base.state()
    }

    fn occluded_rect(&self) -> Rect {
        self.base.occluded_rect()
    }

    fn state_changed(&self, handler: Rc<dyn Fn(&InputPaneStateEventArgs)>) -> Rc<dyn IDisposable> {
        self.base.state_changed(handler)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::threading::Dispatcher;
    use std::cell::RefCell;

    type Seen = Rc<RefCell<Vec<(InputPaneState, Option<Rect>, Rect)>>>;

    fn pane() -> (Rc<BrowserInputPane>, Seen, Rc<dyn IDisposable>) {
        let pane = BrowserInputPane::new();
        let seen: Seen = Rc::new(RefCell::new(Vec::new()));
        let log = seen.clone();
        let subscription = pane.state_changed(Rc::new(move |e: &InputPaneStateEventArgs| {
            log.borrow_mut().push((e.new_state(), e.start_rect(), e.end_rect()))
        }));
        (pane, seen, subscription)
    }

    #[test]
    fn the_pane_starts_closed() {
        let pane = BrowserInputPane::new();

        assert_eq!(InputPaneState::Closed, pane.state());
        assert_eq!(Rect::default(), pane.occluded_rect());
    }

    #[test]
    fn a_keyboard_with_a_width_opens_the_pane() {
        let _scope = Dispatcher::unit_test_scope();
        let (pane, seen, _subscription) = pane();

        assert!(pane.on_geometry_change(0.0, 300.0, 400.0, 200.0));

        let rect = Rect::new(0.0, 300.0, 400.0, 200.0);
        assert_eq!(InputPaneState::Open, pane.state());
        assert_eq!(rect, pane.occluded_rect());
        assert_eq!(vec![(InputPaneState::Open, None, rect)], *seen.borrow());
    }

    #[test]
    fn a_keyboard_without_a_width_closes_the_pane() {
        let _scope = Dispatcher::unit_test_scope();
        let (pane, seen, _subscription) = pane();
        pane.on_geometry_change(0.0, 300.0, 400.0, 200.0);

        assert!(pane.on_geometry_change(0.0, 0.0, 0.0, 0.0));

        assert_eq!(InputPaneState::Closed, pane.state());
        assert_eq!(Rect::default(), pane.occluded_rect());
        assert_eq!(2, seen.borrow().len());
        assert_eq!((InputPaneState::Closed, None, Rect::default()), seen.borrow()[1]);
    }

    #[test]
    fn the_change_is_raised_only_when_the_rectangle_or_the_state_changes() {
        let _scope = Dispatcher::unit_test_scope();
        let (pane, seen, _subscription) = pane();

        // Closed and empty already.
        pane.on_geometry_change(0.0, 0.0, 0.0, 0.0);
        assert!(seen.borrow().is_empty());

        pane.on_geometry_change(0.0, 300.0, 400.0, 200.0);
        pane.on_geometry_change(0.0, 300.0, 400.0, 200.0);
        assert_eq!(1, seen.borrow().len());

        // The same state with another rectangle is a change.
        pane.on_geometry_change(0.0, 250.0, 400.0, 250.0);
        assert_eq!(2, seen.borrow().len());
        assert_eq!(InputPaneState::Open, seen.borrow()[1].0);

        // A rectangle without a width is closed, whatever its height.
        pane.on_geometry_change(0.0, 250.0, 0.0, 250.0);
        assert_eq!(3, seen.borrow().len());
        assert_eq!(InputPaneState::Closed, seen.borrow()[2].0);
    }

    #[test]
    fn a_disposed_subscription_is_not_called() {
        let _scope = Dispatcher::unit_test_scope();
        let (pane, seen, subscription) = pane();

        subscription.dispose();
        pane.on_geometry_change(0.0, 300.0, 400.0, 200.0);

        assert!(seen.borrow().is_empty());
        assert_eq!(InputPaneState::Open, pane.state());
    }
}
