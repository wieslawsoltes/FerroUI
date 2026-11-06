//! Tests of the place of a popup whose placement target is moved by a
//! render transform of an ancestor: a page of a navigation page that slides
//! in. Upstream has no such test, so none of these is a port.
//!
//! The question they settle (`docs/porting/decisions/`, the decision on a
//! popup opened while its target moves): a popup takes its anchor rectangle
//! from where its target is at the time of the request (the bounds of the
//! target transformed to the root). Upstream positions an open popup again
//! only when its window moves, when the bounds of its target change in a
//! layout pass, or when a parent popup moves; an animated render transform
//! raises none of the three, so there a popup opened in the middle of a
//! transition stays where its target was then. The port also positions it
//! again when a render transform above the target changes (DEVIATIONS.md,
//! Popup): the second test checks the place given at the opening, the third
//! that the popup ends below its target.
//!
//! The frames are the frames of an application, as in the tests of the
//! native control host.

use crate::native_control_host_tests::{frame, navigation_page_template, page_with, run_to_end};
use crate::page::NavigationPage;
use crate::platform::{IPopupImpl, ITopLevelImpl, IWindowBaseImpl};
use crate::primitives::popup_positioning::{IPopupPositioner, PopupPositionerParameters};
use crate::primitives::Popup;
use crate::testing::{MockWindowImpl, MockWindowingPlatform, TestServices, UnitTestApplication};
use crate::{Border, Control, PlacementMode, Window};
use ferroui_base::animation::{IGlobalClock, IPageTransition, PageSlide, SlideAxis, SignedDuration};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{Brushes, MediaContext};
use ferroui_system::task::JoinHandle;
use ferroui_base::{FerroLocator, PixelPoint, Point, Rect, Ref, Size, Visual};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

/// The place of the target on its page once the page is at rest.
fn at_rest() -> Rect {
    Rect::new(20.0, 30.0, 100.0, 50.0)
}

/// A positioner that records what it is given and hands it on.
struct RecordingPositioner {
    inner: Rc<dyn IPopupPositioner>,
    updates: Rc<RefCell<Vec<PopupPositionerParameters>>>,
}

impl IPopupPositioner for RecordingPositioner {
    fn update(&self, parameters: PopupPositionerParameters) {
        self.updates.borrow_mut().push(parameters);
        self.inner.update(parameters);
    }
}

struct Shell {
    window: Ref<Window>,
    nav: Ref<NavigationPage>,
    /// The parameters every popup of the window was positioned with, in
    /// order.
    updates: Rc<RefCell<Vec<PopupPositionerParameters>>>,
    /// The platform popups the window created.
    popups: Rc<RefCell<Vec<Rc<MockWindowImpl>>>>,
}

/// A window of 400 by 300 whose popups are native ones placed by the
/// managed positioner, with a navigation page that slides its pages in
/// during `transition_milliseconds`, and a first page.
fn shell(transition_milliseconds: f64) -> Shell {
    let clock: Rc<dyn IGlobalClock> = MediaContext::instance().clock();
    FerroLocator::current_mutable().bind::<dyn IGlobalClock>().to_constant(clock);

    let updates = Rc::new(RefCell::new(Vec::new()));
    let popups = Rc::new(RefCell::new(Vec::new()));
    let window_impl = MockWindowingPlatform::create_window_mock_with_size(400.0, 300.0);
    window_impl.setup_create_popup({
        let updates = updates.clone();
        let popups = popups.clone();
        let weak = Rc::downgrade(&window_impl);
        move |_| {
            let parent: Rc<dyn ITopLevelImpl> = weak.upgrade()?;
            let popup = MockWindowingPlatform::create_popup_mock(parent);
            let inner = IPopupImpl::popup_positioner(&*popup).expect("the popup mock has a positioner");
            popup.setup_popup_positioner(Some(Rc::new(RecordingPositioner { inner, updates: updates.clone() })));
            popups.borrow_mut().push(popup.clone());
            let popup: Rc<dyn IPopupImpl> = popup;
            Some(popup)
        }
    });
    let window = Window::with_impl(window_impl);

    let nav = NavigationPage::new();
    let transition: Rc<dyn IPageTransition> = Rc::new(PageSlide::with_duration(
        SignedDuration::from_milliseconds(transition_milliseconds),
        SlideAxis::Horizontal,
    ));
    nav.set_page_transition(Some(transition));
    nav.set_template(Some(navigation_page_template()));
    window.set_content(Some(Control::boxed(nav.clone())));
    window.show();
    frame();

    // The first page is shown without a transition.
    let first = nav.push_async(page_with(None));
    frame();
    assert!(first.is_finished());

    Shell { window, nav, updates, popups }
}

/// The placement target: 100 by 50 at the left top corner of its page.
fn target() -> Ref<Border> {
    let target = Border::new();
    target.set_width(100.0);
    target.set_height(50.0);
    target.set_horizontal_alignment(HorizontalAlignment::Left);
    target.set_vertical_alignment(VerticalAlignment::Top);
    target.set_background(Some(Brushes::red()));
    target
}

/// A popup of 60 by 40 below `target`, centred: at (40, 80) for a target at
/// rest.
fn popup_below(target: &Ref<Border>) -> Ref<Popup> {
    let popup = Popup::new();
    popup.set_placement_target(target.clone());
    popup.set_placement(PlacementMode::Bottom);
    popup.set_width(60.0);
    popup.set_height(40.0);
    popup.set_child(Some(Border::new().upcast()));
    popup.set_parent(popup.placement_target().unwrap());
    popup
}

fn position_in(target: &Border, root: &Visual) -> Option<Rect> {
    let matrix = target.transform_to_visual(root)?;
    Some(Rect::from_position_size(Point::default(), target.bounds().size()).transform_to_aabb(matrix))
}

/// Runs frames until the page of `target` is on its way in: the target is
/// in the tree of the window and at least 120 to the right of its place at
/// rest, with the transition still running. Returns where the target is.
fn run_to_the_middle(shell: &Shell, target: &Border, navigation: &JoinHandle<()>) -> Rect {
    for _ in 0..2000 {
        frame();
        assert!(!navigation.is_finished(), "the transition ended before the target was seen moving");
        if let Some(at) = position_in(target, &shell.window) {
            if at.x > at_rest().x + 120.0 && at.x < 300.0 {
                return at;
            }
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    panic!("the target was never seen moving");
}

fn last_update(shell: &Shell) -> PopupPositionerParameters {
    *shell.updates.borrow().last().expect("the popup was positioned")
}

fn popup_position(shell: &Shell) -> PixelPoint {
    let popups = shell.popups.borrow();
    assert_eq!(1, popups.len(), "one platform popup was created");
    IWindowBaseImpl::position(&*popups[0])
}

/// (b) A popup opened once the page is at rest is given the rectangle of
/// its target and is placed below it.
#[test]
fn popup_opened_after_the_transition_is_placed_at_its_target() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let shell = shell(60.0);
    let target = target();
    let popup = popup_below(&target);

    let navigation = shell.nav.push_async(page_with(Some(target.clone().upcast())));
    run_to_end(&navigation);
    assert_eq!(Some(at_rest()), position_in(&target, &shell.window));

    popup.open();
    frame();

    assert_eq!(at_rest(), last_update(&shell).anchor_rectangle);
    assert_eq!(Size::new(60.0, 40.0), last_update(&shell).size);
    assert_eq!(PixelPoint::new(40, 80), popup_position(&shell));
}

/// (a) at the moment it opens: a popup opened while its page slides in is
/// given the rectangle of its target at that moment, as upstream does.
///
/// A user reaches this: the target is hit at its visual position during the
/// transition (asserted here), so a press there opens the popup.
#[test]
fn popup_opened_during_the_transition_is_placed_where_its_target_is_then() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let shell = shell(1500.0);
    let target = target();
    let popup = popup_below(&target);

    let navigation = shell.nav.push_async(page_with(Some(target.clone().upcast())));
    let moving = run_to_the_middle(&shell, &target, &navigation);

    // Input reaches the target where it is drawn, not where it will rest:
    // at its visual position the target is the topmost visual below the
    // empty presenters the template of these tests lays over its pages (the
    // top command bar and the modal back presenter), and at its place at
    // rest it is not hit at all.
    let target_visual: Ref<Visual> = target.clone().upcast();
    let overlays = ["PART_TopCommandBar", "PART_ModalBackPresenter", "PART_ModalPresenter"];
    let topmost_below_the_overlays = |point: Point| {
        shell.window.get_visuals_at(point).into_iter().find(|visual| {
            let name = visual.clone().downcast::<Control>().ok().and_then(|control| control.name());
            !name.is_some_and(|name| overlays.contains(&name.as_str()))
        })
    };
    assert!(
        topmost_below_the_overlays(moving.center()) == Some(target_visual.clone()),
        "the moving target is hit at its visual position {moving:?}"
    );
    assert!(!shell.window.get_visuals_at(at_rest().center()).contains(&target_visual));

    popup.open();
    frame();
    assert!(!navigation.is_finished(), "the popup was opened in the middle of the transition");

    // The positioner was given the rectangle of the moving target: its
    // size, at the place it had when the popup opened (at or left of where
    // it was seen moving, and right of where it will rest).
    let given = last_update(&shell).anchor_rectangle;
    assert_eq!(at_rest().size(), given.size());
    assert_eq!(at_rest().y, given.y);
    assert!(
        given.x > at_rest().x + 1.0 && given.x <= moving.x,
        "the anchor rectangle {given:?} is that of the moving target (seen at {moving:?})"
    );
    let position_when_opened = popup_position(&shell);
    // Centred below the rectangle it was given: 20 further right, within the
    // pixel the positioner rounds a fractional place to (the target is
    // caught at any fraction of a pixel while it moves).
    assert!(
        (given.x + 20.0 - f64::from(position_when_opened.x)).abs() < 1.0,
        "the popup at {position_when_opened:?} is centred below the anchor rectangle {given:?}"
    );
    assert_eq!(80, position_when_opened.y);

    assert!(popup.is_open());
}

/// (a) to the end, the deviation (DEVIATIONS.md, Popup; the decision
/// document): the popup is positioned again when the transform of its
/// target to the root changes, so it ends below its target. Upstream leaves
/// it where the target was when it opened.
#[test]
fn popup_opened_during_the_transition_ends_at_its_target() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let shell = shell(1500.0);
    let target = target();
    let popup = popup_below(&target);

    let navigation = shell.nav.push_async(page_with(Some(target.clone().upcast())));
    run_to_the_middle(&shell, &target, &navigation);
    popup.open();
    frame();
    run_to_end(&navigation);

    assert_eq!(at_rest(), last_update(&shell).anchor_rectangle);
    assert_eq!(PixelPoint::new(40, 80), popup_position(&shell));
}
