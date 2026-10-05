//! The reference test suite has no tests for the toggle switch; these are
//! tests of this port for the knob position, the knob drag and the content
//! parts. The template has the parts of the reference simple theme: a canvas
//! (the switch knob) around a grid (the moving knobs), and the two content
//! presenters.

use crate::presenters::ContentPresenter;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::test_support::{boxed_str, string_of, test_scope, TestRoot};
use crate::{Canvas, ContentControl, Control, Grid, Panel, ToggleSwitch};
use ferroui_base::animation::{DoubleTransition, ITransition, Transitions};
use ferroui_base::data::TemplateBinding;
use ferroui_base::input::{
    InputElement, IPointer, KeyModifiers, MouseButton, Pointer, PointerEventArgs, PointerPointProperties,
    PointerPressedEventArgs, PointerReleasedEventArgs, PointerType,
};
use crate::mouse_test_helper::MouseTestHelper;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::media::{Geometry, GeometryHitTestResult};
use ferroui_base::rendering::IHitTester;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{Point, Ref, StyledElement, Visual};
use std::cell::Cell;
use std::rc::Rc;

fn create_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(|_, ns| {
        let root = Grid::new();

        let off_content_presenter = ContentPresenter::new();
        off_content_presenter.set_name(Some("PART_OffContentPresenter".to_string()));
        off_content_presenter.bind_binding(
            ContentPresenter::content_property().as_property(),
            &TemplateBinding::new(ToggleSwitch::off_content_property().as_property()),
        );
        root.children().add(off_content_presenter.register_in_name_scope(&**ns));

        let on_content_presenter = ContentPresenter::new();
        on_content_presenter.set_name(Some("PART_OnContentPresenter".to_string()));
        on_content_presenter.bind_binding(
            ContentPresenter::content_property().as_property(),
            &TemplateBinding::new(ToggleSwitch::on_content_property().as_property()),
        );
        root.children().add(on_content_presenter.register_in_name_scope(&**ns));

        let content_presenter = ContentPresenter::new();
        content_presenter.set_name(Some("PART_ContentPresenter".to_string()));
        content_presenter.bind_binding(
            ContentPresenter::content_property().as_property(),
            &TemplateBinding::new(ContentControl::content_property().as_property()),
        );
        root.children().add(content_presenter.register_in_name_scope(&**ns));

        let switch_knob = Canvas::new();
        switch_knob.set_name(Some("PART_SwitchKnob".to_string()));
        switch_knob.set_horizontal_alignment(HorizontalAlignment::Left);
        switch_knob.set_width(20.0);
        switch_knob.set_height(20.0);

        let moving_knobs = Grid::new();
        moving_knobs.set_name(Some("PART_MovingKnobs".to_string()));
        moving_knobs.set_width(20.0);
        moving_knobs.set_height(20.0);
        switch_knob.children().add(moving_knobs.register_in_name_scope(&**ns));
        root.children().add(switch_knob.register_in_name_scope(&**ns));

        root.upcast()
    })
}

struct Target {
    switch: Ref<ToggleSwitch>,
    knobs: Ref<Panel>,
    root: Ref<TestRoot>,
    pointer: Rc<dyn IPointer>,
}

fn create_target(is_checked: Option<bool>) -> Target {
    let switch = ToggleSwitch::new();
    switch.set_is_checked(is_checked);
    switch.set_template(Some(create_template()));

    let root = TestRoot::with_child(switch.clone());
    root.execute_initial_layout_pass();

    let knobs = switch
        .get_template_descendants()
        .into_iter()
        .filter_map(|x| x.cast::<Panel>())
        .find(|x| x.name().as_deref() == Some("PART_MovingKnobs"))
        .expect("no moving knobs");
    let pointer: Rc<dyn IPointer> = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
    Target { switch, knobs, root, pointer }
}

impl Target {
    fn root_visual(&self) -> Ref<Visual> {
        self.root.clone().upcast()
    }

    fn left(&self) -> f64 {
        Canvas::get_left(&self.knobs)
    }

    fn press(&self, x: f64) {
        self.knobs.raise_event(&PointerPressedEventArgs::new(
            self.knobs.clone(),
            self.pointer.clone(),
            &self.root_visual(),
            Point::new(x, 5.0),
            0,
            PointerPointProperties::default(),
            KeyModifiers::NONE,
            1,
        ));
    }

    fn move_to(&self, x: f64) {
        self.knobs.raise_event(&PointerEventArgs::new(
            Some(InputElement::pointer_moved_event()),
            self.knobs.clone(),
            self.pointer.clone(),
            Some(&self.root_visual()),
            Point::new(x, 5.0),
            0,
            PointerPointProperties::default(),
            KeyModifiers::NONE,
        ));
    }

    /// Releases the pointer; returns whether the switch handled the event.
    fn release(&self, x: f64) -> bool {
        let e = PointerReleasedEventArgs::new(
            self.knobs.clone(),
            self.pointer.clone(),
            &self.root_visual(),
            Point::new(x, 5.0),
            0,
            PointerPointProperties::default(),
            KeyModifiers::NONE,
            MouseButton::Left,
        );
        self.knobs.raise_event(&e);
        e.handled()
    }
}

#[test]
fn knob_position_follows_is_checked() {
    let _scope = test_scope();
    let target = create_target(Some(false));

    assert_eq!(0.0, target.left());

    target.switch.set_is_checked(Some(true));
    assert_eq!(20.0, target.left());

    target.switch.set_is_checked(Some(false));
    assert_eq!(0.0, target.left());

    // An indeterminate switch keeps the position of the knob.
    target.switch.set_is_checked(Some(true));
    target.switch.set_is_checked(None);
    assert_eq!(20.0, target.left());
}

#[test]
fn knob_position_is_set_when_the_template_is_applied() {
    let _scope = test_scope();
    let target = create_target(Some(true));

    // The knob has no bounds when the template is applied; the change of the
    // bounds of the switch moves the knob once it is laid out.
    target.switch.set_width(100.0);
    target.root.layout_manager().execute_layout_pass();

    assert_eq!(20.0, target.left());
}

#[test]
fn content_presenters_are_registered_and_contents_are_logical_children() {
    let _scope = test_scope();
    let target = create_target(Some(false));

    let off_presenter = target.switch.off_content_presenter().expect("no off content presenter");
    let on_presenter = target.switch.on_content_presenter().expect("no on content presenter");
    assert_eq!(Some("PART_OffContentPresenter".to_string()), off_presenter.name());
    assert_eq!(Some("PART_OnContentPresenter".to_string()), on_presenter.name());
    assert_eq!(Some("Off".to_string()), target.switch.off_content().as_ref().and_then(string_of));
    assert_eq!(Some("On".to_string()), target.switch.on_content().as_ref().and_then(string_of));

    let logical_children = StyledElement::logical_children(&target.switch);
    let off = Control::new();
    let on = Control::new();
    target.switch.set_off_content(Some(Control::boxed(off.clone())));
    target.switch.set_on_content(Some(Control::boxed(on.clone())));
    assert!(logical_children.contains(&off.clone().upcast::<StyledElement>()));
    assert!(logical_children.contains(&on.clone().upcast::<StyledElement>()));

    target.switch.set_off_content(boxed_str("No"));
    target.switch.set_on_content(None);
    assert!(!logical_children.contains(&off.upcast::<StyledElement>()));
    assert!(!logical_children.contains(&on.upcast::<StyledElement>()));
}

#[test]
fn dragging_the_knob_past_the_middle_checks_the_switch() {
    let _scope = test_scope();
    let target = create_target(Some(false));

    target.press(5.0);
    assert!(!target.switch.classes().contains(":dragging"));

    // A move of three pixels or less is not a drag.
    target.move_to(8.0);
    assert!(!target.switch.classes().contains(":dragging"));
    assert_eq!(0.0, target.left());

    target.move_to(17.0);
    assert!(target.switch.classes().contains(":dragging"));
    assert_eq!(12.0, target.left());

    // The knob stays inside the switch knob.
    target.move_to(100.0);
    assert_eq!(20.0, target.left());
    target.move_to(-100.0);
    assert_eq!(0.0, target.left());

    target.move_to(17.0);
    assert!(target.release(17.0));

    assert!(!target.switch.classes().contains(":dragging"));
    assert_eq!(Some(true), target.switch.is_checked());
    assert_eq!(20.0, target.left());
}

#[test]
fn dragging_the_knob_back_before_the_middle_unchecks_the_switch() {
    let _scope = test_scope();
    let target = create_target(Some(true));
    assert_eq!(20.0, target.left());

    target.press(25.0);
    target.move_to(10.0);
    assert_eq!(5.0, target.left());
    assert!(target.release(10.0));

    assert_eq!(Some(false), target.switch.is_checked());
    assert_eq!(0.0, target.left());
}

#[test]
fn releasing_the_knob_on_the_side_it_came_from_restores_its_position() {
    let _scope = test_scope();
    let target = create_target(Some(false));

    target.press(5.0);
    target.move_to(10.0);
    assert_eq!(5.0, target.left());
    assert!(target.release(10.0));

    assert_eq!(Some(false), target.switch.is_checked());
    assert_eq!(0.0, target.left());
}

#[test]
fn releasing_without_a_drag_is_not_handled() {
    let _scope = test_scope();
    let target = create_target(Some(false));

    target.press(5.0);
    target.move_to(6.0);
    assert!(!target.release(6.0));

    assert_eq!(Some(false), target.switch.is_checked());

    // Moves without a press do nothing.
    target.move_to(50.0);
    assert!(!target.switch.classes().contains(":dragging"));
    assert_eq!(0.0, target.left());
}

#[test]
fn knob_transitions_are_applied_when_loaded_and_suspended_while_dragging() {
    let _scope = test_scope();
    let target = create_target(Some(false));
    Dispatcher::ui_thread().run_jobs(None);
    assert!(target.knobs.transitions().is_none());

    let transition = DoubleTransition::new();
    transition.set_property(Some(Canvas::left_property().as_property()));
    let transition: Rc<dyn ITransition> = transition.into();
    let transitions = Transitions::from_items([transition]);
    target.switch.set_knob_transitions(Some(transitions.clone()));
    assert!(target.knobs.transitions().is_some_and(|x| x.ptr_eq(&transitions)));

    target.press(5.0);
    target.move_to(17.0);
    assert!(target.knobs.transitions().is_none());

    target.release(17.0);
    assert!(target.knobs.transitions().is_some_and(|x| x.ptr_eq(&transitions)));
}

// ---------------------------------------------------------------------------
// The switch as a toggle button: the events of a pressed left button reach
// the button behind the knob, which toggles on a click unless the release
// ended a drag of the knob.
// ---------------------------------------------------------------------------

/// A hit tester that finds the root of the search when it contains the
/// point.
struct BoundsHitTester;

impl IHitTester for BoundsHitTester {
    fn hit_test(&self, p: Point, root: &Visual, _filter: Option<&dyn Fn(&Visual) -> bool>) -> Vec<Ref<Visual>> {
        if root.bounds().contains(p) {
            vec![root.to_ref()]
        } else {
            Vec::new()
        }
    }

    fn hit_test_geometry(
        &self,
        _geometry: &Geometry,
        _root: &Visual,
        _filter: Option<&dyn Fn(&Visual) -> bool>,
    ) -> Vec<GeometryHitTestResult> {
        Vec::new()
    }

    fn hit_test_first(&self, p: Point, root: &Visual, filter: Option<&dyn Fn(&Visual) -> bool>) -> Option<Ref<Visual>> {
        self.hit_test(p, root, filter).into_iter().next()
    }

    fn hit_test_first_geometry(
        &self,
        _geometry: &Geometry,
        _root: &Visual,
        _filter: Option<&dyn Fn(&Visual) -> bool>,
    ) -> Option<GeometryHitTestResult> {
        None
    }
}

/// A target whose root finds the switch under the pointer, and the mouse
/// that presses its left button on the knob.
fn create_clickable_target(is_checked: Option<bool>) -> (Target, MouseTestHelper, Rc<Cell<i32>>) {
    let target = create_target(is_checked);
    target.root.set_hit_tester(Some(Rc::new(BoundsHitTester)));
    let clicks = Rc::new(Cell::new(0));
    target.switch.click({
        let clicks = clicks.clone();
        move |_, _| clicks.set(clicks.get() + 1)
    });
    (target, MouseTestHelper::new(), clicks)
}

#[test]
fn clicking_the_knob_without_dragging_toggles_the_switch() {
    let _scope = test_scope();
    let (target, mouse, clicks) = create_clickable_target(Some(false));

    mouse.down_at(&target.knobs, MouseButton::Left, Point::new(5.0, 5.0), 1);
    // A move within the drag threshold keeps it a click.
    mouse.move_(&target.knobs, Point::new(7.0, 5.0));
    mouse.up_at(&target.knobs, MouseButton::Left, Point::new(7.0, 5.0));

    assert_eq!(1, clicks.get());
    assert!(!target.switch.classes().contains(":dragging"));
    assert_eq!(Some(true), target.switch.is_checked());
    assert_eq!(20.0, target.left());

    mouse.down_at(&target.knobs, MouseButton::Left, Point::new(5.0, 5.0), 1);
    mouse.up_at(&target.knobs, MouseButton::Left, Point::new(5.0, 5.0));

    assert_eq!(2, clicks.get());
    assert_eq!(Some(false), target.switch.is_checked());
    assert_eq!(0.0, target.left());
}

#[test]
fn releasing_a_dragged_knob_does_not_click_the_switch() {
    let _scope = test_scope();
    let (target, mouse, clicks) = create_clickable_target(Some(false));

    // Past the middle: checked by the drag, and not toggled back by a click.
    mouse.down_at(&target.knobs, MouseButton::Left, Point::new(5.0, 5.0), 1);
    mouse.move_(&target.knobs, Point::new(17.0, 5.0));
    assert!(target.switch.classes().contains(":dragging"));
    mouse.up_at(&target.knobs, MouseButton::Left, Point::new(17.0, 5.0));

    assert_eq!(0, clicks.get());
    assert!(!target.switch.classes().contains(":dragging"));
    assert_eq!(Some(true), target.switch.is_checked());
    assert_eq!(20.0, target.left());

    // Released on the side it came from: still checked, and no click.
    mouse.down_at(&target.knobs, MouseButton::Left, Point::new(25.0, 5.0), 1);
    mouse.move_(&target.knobs, Point::new(20.0, 5.0));
    assert_eq!(15.0, target.left());
    mouse.up_at(&target.knobs, MouseButton::Left, Point::new(20.0, 5.0));

    assert_eq!(0, clicks.get());
    assert_eq!(Some(true), target.switch.is_checked());
    assert_eq!(20.0, target.left());

    // The next press starts over: without a drag it is a click again. (The
    // switch is as wide as its knob: the pointer is released inside it.)
    mouse.down_at(&target.knobs, MouseButton::Left, Point::new(5.0, 5.0), 1);
    mouse.up_at(&target.knobs, MouseButton::Left, Point::new(5.0, 5.0));

    assert_eq!(1, clicks.get());
    assert_eq!(Some(false), target.switch.is_checked());
}

#[test]
fn dragging_the_knob_of_an_indeterminate_switch_unchecks_it() {
    let _scope = test_scope();
    let target = create_target(None);

    // The knob of an indeterminate switch has no position.
    assert!(target.left().is_nan());

    target.press(5.0);
    target.move_to(17.0);
    assert!(target.switch.classes().contains(":dragging"));
    // The position stays unset: it is computed from the unset one.
    assert!(target.left().is_nan());

    assert!(target.release(17.0));

    // No position is not past the middle.
    assert!(!target.switch.classes().contains(":dragging"));
    assert_eq!(Some(false), target.switch.is_checked());
    assert_eq!(0.0, target.left());
}

#[test]
fn a_press_after_a_drag_starts_from_the_current_knob_position() {
    let _scope = test_scope();
    let target = create_target(Some(false));

    target.press(5.0);
    target.move_to(17.0);
    assert!(target.release(17.0));
    assert_eq!(Some(true), target.switch.is_checked());

    // The second drag starts at the checked position and is not a drag
    // until the pointer moves more than three pixels again.
    target.press(25.0);
    target.move_to(22.0);
    assert!(!target.switch.classes().contains(":dragging"));
    assert_eq!(20.0, target.left());
    target.move_to(21.0);
    assert!(target.switch.classes().contains(":dragging"));
    assert_eq!(16.0, target.left());
    assert!(target.release(21.0));

    assert_eq!(Some(true), target.switch.is_checked());
    assert_eq!(20.0, target.left());
}
