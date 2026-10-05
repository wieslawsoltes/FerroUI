use super::ScrollablePullGestureRecognizer;
use crate::test_support::{test_scope, TestRoot};
use crate::text_box_tests_input::TouchTestHelper;
use crate::{ControlImpl, Decorator};
use ferroui_base::input::{IScrollable, InputElement, InputElementImpl, PullDirection};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Point, Ref, Size, StyledElementImpl, Vector,
    VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[repr(C)]
struct TestScrollable {
    base: Decorator,
    offset: Cell<Vector>,
}

ferro_class!(TestScrollable: Decorator);
ferro_impl_classes!(
    TestScrollable: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    ControlImpl
);

impl InputElementImpl for TestScrollable {
    fn as_scrollable(this: &Self) -> Option<&dyn IScrollable> {
        Some(this)
    }
}

impl IScrollable for TestScrollable {
    fn extent(&self) -> Size {
        Size::new(100.0, 1000.0)
    }

    fn offset(&self) -> Vector {
        self.offset.get()
    }

    fn set_offset(&self, value: Vector) {
        self.offset.set(value)
    }

    fn viewport(&self) -> Size {
        Size::new(100.0, 100.0)
    }

    fn can_horizontally_scroll(&self) -> bool {
        false
    }

    fn can_vertically_scroll(&self) -> bool {
        true
    }
}

impl TestScrollable {
    fn new() -> Ref<Self> {
        instantiate(Self { base: Decorator::construct(), offset: Cell::new(Vector::default()) })
    }
}

struct Recorded {
    pull_ids: Rc<RefCell<Vec<i32>>>,
    ended_ids: Rc<RefCell<Vec<i32>>>,
    deltas: Rc<RefCell<Vec<Vector>>>,
}

fn record(root: &TestRoot) -> Recorded {
    let pull_ids = Rc::new(RefCell::new(Vec::new()));
    let ended_ids = Rc::new(RefCell::new(Vec::new()));
    let deltas = Rc::new(RefCell::new(Vec::new()));
    {
        let pull_ids = pull_ids.clone();
        let deltas = deltas.clone();
        root.add_handler(InputElement::pull_gesture_event(), move |_, e| {
            pull_ids.borrow_mut().push(e.id());
            deltas.borrow_mut().push(e.delta());
        });
    }
    {
        let ended_ids = ended_ids.clone();
        root.add_handler(InputElement::pull_gesture_ended_event(), move |_, e| ended_ids.borrow_mut().push(e.id()));
    }
    Recorded { pull_ids, ended_ids, deltas }
}

// Repro for the "pointer capture lost doesn't clean up state" bug.
//
// The pointer released handler clears the pull in progress flag, the tracked pointer
// and the initial position. The pointer capture lost handler only raised the end of
// the pull (so the pull gesture ended event fires) but left the pull in progress and
// the lost pointer tracked. The next gesture re-enters the pointer moved handler with
// a pull in progress and goes straight to `handle_pull`, skipping `begin_pull`. This
// reuses the OLD gesture id for the next pull gesture event - the same id that was
// just used in the pull gesture ended event for the previous gesture.
#[test]
fn gesture_after_pointer_capture_lost_uses_a_new_id() {
    let _scope = test_scope();
    let scrollable = TestScrollable::new();
    let recognizer = ScrollablePullGestureRecognizer::with_direction(PullDirection::TopToBottom, true);
    scrollable.gesture_recognizers().add(recognizer);

    let root = TestRoot::with_child(scrollable.clone());

    let recorded = record(&root);

    let touch = TouchTestHelper::new();

    // First gesture: start pulling downwards so the recognizer begins a pull and captures the pointer.
    touch.down(&scrollable, Point::new(10.0, 20.0));
    touch.move_(&scrollable, Point::new(10.0, 70.0));

    assert_eq!(recorded.pull_ids.borrow().len(), 1);
    let first_gesture_id = recorded.pull_ids.borrow()[0];

    // Capture is lost (e.g. another control steals it, or the visual is detached mid-gesture)
    touch.cancel();

    assert_eq!(*recorded.ended_ids.borrow(), [first_gesture_id]);

    // Second gesture must begin cleanly (`begin_pull`, not `handle_pull`) and therefore get a fresh id.
    touch.down(&scrollable, Point::new(10.0, 20.0));
    touch.move_(&scrollable, Point::new(10.0, 70.0));

    assert_eq!(recorded.pull_ids.borrow().len(), 2);
    assert_ne!(first_gesture_id, recorded.pull_ids.borrow()[1]);
}

// --- Additional tests (not upstream); expectations derived from the reference source. ---

#[test]
fn pull_reports_the_distance_from_the_press_and_ends_on_release() {
    let _scope = test_scope();
    let scrollable = TestScrollable::new();
    let recognizer = ScrollablePullGestureRecognizer::with_direction(PullDirection::TopToBottom, false);
    scrollable.gesture_recognizers().add(recognizer);
    let root = TestRoot::with_child(scrollable.clone());
    let recorded = record(&root);

    let touch = TouchTestHelper::new();
    touch.down(&scrollable, Point::new(10.0, 20.0));

    // Moving against the pull direction does not start a pull.
    touch.move_(&scrollable, Point::new(10.0, 10.0));
    assert!(recorded.pull_ids.borrow().is_empty());

    touch.move_(&scrollable, Point::new(10.0, 50.0));
    touch.move_(&scrollable, Point::new(30.0, 90.0));
    assert_eq!(*recorded.deltas.borrow(), [Vector::new(0.0, 30.0), Vector::new(0.0, 70.0)]);
    let ids = recorded.pull_ids.borrow().clone();
    assert_eq!(ids[0], ids[1]);
    assert!(recorded.ended_ids.borrow().is_empty());

    touch.up(&scrollable, Point::new(30.0, 90.0));
    assert_eq!(recorded.ended_ids.borrow().first(), Some(&ids[0]));
    assert!(touch.captured().is_none());
}

#[test]
fn pull_does_not_start_when_the_target_is_scrolled_away_from_the_edge() {
    let _scope = test_scope();
    let scrollable = TestScrollable::new();
    scrollable.offset.set(Vector::new(0.0, 10.0));
    let recognizer = ScrollablePullGestureRecognizer::with_direction(PullDirection::TopToBottom, false);
    scrollable.gesture_recognizers().add(recognizer.clone());
    let root = TestRoot::with_child(scrollable.clone());
    let recorded = record(&root);

    let touch = TouchTestHelper::new();
    touch.down(&scrollable, Point::new(10.0, 20.0));
    touch.move_(&scrollable, Point::new(10.0, 70.0));
    assert!(recorded.pull_ids.borrow().is_empty());
    touch.up(&scrollable, Point::new(10.0, 70.0));

    // Pulling up from the bottom needs the end of the extent in view.
    recognizer.set_pull_direction(PullDirection::BottomToTop);
    scrollable.offset.set(Vector::new(0.0, 900.0));
    touch.down(&scrollable, Point::new(10.0, 70.0));
    touch.move_(&scrollable, Point::new(10.0, 20.0));
    assert_eq!(*recorded.deltas.borrow(), [Vector::new(0.0, 50.0)]);
    touch.up(&scrollable, Point::new(10.0, 20.0));
}
