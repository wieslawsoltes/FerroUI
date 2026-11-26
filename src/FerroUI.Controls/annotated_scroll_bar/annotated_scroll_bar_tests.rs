//! Tests of WinUI's `AnnotatedScrollBar` (#1268).

use super::*;
use crate::primitives::ScrollControllerScrollToRequestedEventArgs;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use crate::test_support::{test_scope, TestRoot};
use crate::{Border, Control, Grid, RepeatButton, TextBlock};
use ferroui_base::controls::NameScopeRef;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::{Ref, Upcast};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The template of the bar: the parts of WinUI's `AnnotatedScrollBar.xaml`, a 3 px thumb and
/// 16 px buttons.
fn bar_template(_bar: &Ref<AnnotatedScrollBar>, scope: &NameScopeRef) -> Ref<Control> {
    let root = Grid::new();
    root.set_row_definitions(crate::RowDefinitions::parse("Auto,*,Auto").unwrap());
    let increment = RepeatButton::new();
    increment.set_height(16.0);
    root.children()
        .add(named(increment.clone(), "PART_VerticalIncrementRepeatButton", scope));
    let vertical_grid = Grid::new();
    Grid::set_row(&vertical_grid.clone().upcast(), 1);
    let labels_grid = Grid::new();
    vertical_grid
        .children()
        .add(named(labels_grid.clone(), "PART_LabelsGrid", scope));
    let rail = Border::new();
    vertical_grid
        .children()
        .add(named(rail.clone(), "PART_ToolTipRail", scope));
    let ghost = Border::new();
    ghost.set_height(3.0);
    ghost.set_is_visible(false);
    ghost.set_vertical_alignment(VerticalAlignment::Top);
    vertical_grid
        .children()
        .add(named(ghost.clone(), "PART_VerticalThumbGhost", scope));
    let thumb = Border::new();
    thumb.set_height(3.0);
    thumb.set_vertical_alignment(VerticalAlignment::Top);
    vertical_grid
        .children()
        .add(named(thumb.clone(), "PART_VerticalThumb", scope));
    root.children()
        .add(named(vertical_grid.clone(), "PART_VerticalGrid", scope));
    let decrement = RepeatButton::new();
    decrement.set_height(16.0);
    Grid::set_row(&decrement.clone().upcast(), 2);
    root.children()
        .add(named(decrement.clone(), "PART_VerticalDecrementRepeatButton", scope));
    root.upcast()
}

/// `element` named `name` in the name scope of the template.
fn named<T: ferroui_base::ObjectType + Upcast<ferroui_base::StyledElement>>(
    element: Ref<T>,
    name: &str,
    scope: &NameScopeRef,
) -> Ref<T> {
    let styled: &ferroui_base::StyledElement = (*element).upcast();
    styled.set_name(Some(name.to_string()));
    element.register_in_name_scope(&**scope)
}

fn template_part(bar: &Ref<AnnotatedScrollBar>, name: &str) -> Ref<Control> {
    bar.get_template_descendants()
        .into_iter()
        .filter_map(|visual| visual.cast::<Control>())
        .find(|control| control.name().as_deref() == Some(name))
        .expect("the part")
}

/// A 40 x 232 bar: a 200 px rail between the two buttons.
fn scene() -> (Ref<TestRoot>, Ref<AnnotatedScrollBar>) {
    let bar = AnnotatedScrollBar::new();
    bar.set_template(Some(FuncControlTemplate::for_type::<AnnotatedScrollBar>(bar_template)));
    bar.set_width(40.0);
    bar.set_height(232.0);
    let root = TestRoot::with_child(&bar);
    root.layout_manager().execute_initial_layout_pass();
    root.layout_manager().execute_layout_pass();
    (root, bar)
}

fn recorded_scroll_to(bar: &Ref<AnnotatedScrollBar>) -> (Rc<RefCell<Vec<f64>>>, ferroui_base::reactive::Subscription) {
    let offsets = Rc::new(RefCell::new(Vec::new()));
    let recorded = offsets.clone();
    let subscription = bar.scroll_controller().scroll_to_requested(Rc::new(
        move |args: &ScrollControllerScrollToRequestedEventArgs| {
            recorded.borrow_mut().push(args.offset());
            args.set_correlation_id(1);
        },
    ));
    (offsets, subscription)
}

#[test]
fn defaults_are_winui_s() {
    let _scope = test_scope();
    let bar = AnnotatedScrollBar::new();
    assert_eq!(bar.small_change(), 0.0);
    assert_eq!(bar.labels().count(), 0);
    assert!(bar.label_template().is_none());
    assert!(bar.detail_label_template().is_none());
    // The same controller each time; it cannot scroll until a presenter says the axis scrolls.
    assert!(Rc::ptr_eq(&bar.scroll_controller(), &bar.scroll_controller()));
    assert!(!bar.scroll_controller().can_scroll());
    bar.scroll_controller().set_is_scrollable(true);
    assert!(bar.scroll_controller().can_scroll());
}

#[test]
fn the_thumb_follows_the_offset_of_the_presenter() {
    let _scope = test_scope();
    let (_root, bar) = scene();
    let controller = bar.scroll_controller();
    controller.set_values(0.0, 1000.0, 500.0, 100.0);
    // The rail is 200 px, the thumb 3 px: an offset of 1 is 0.197 px of the rail.
    let factor = bar.scroll_offset_to_label_offset_factor();
    assert!((factor - 197.0 / 1000.0).abs() < 1e-9, "{factor}");
    let thumb = template_part(&bar, "PART_VerticalThumb");
    let transform = thumb.render_transform().expect("a translation");
    assert!((transform.value().m32 - 500.0 * factor).abs() < 1e-9);
}

#[test]
fn a_press_on_the_rail_requests_the_offset_under_the_pointer_and_scrolling_can_cancel_it() {
    let _scope = test_scope();
    let (_root, bar) = scene();
    bar.scroll_controller().set_values(0.0, 1000.0, 0.0, 100.0);
    let (offsets, _requests) = recorded_scroll_to(&bar);
    let kinds = Rc::new(RefCell::new(Vec::new()));
    let cancel = Rc::new(Cell::new(false));
    let _scrolling = bar.scrolling({
        let kinds = kinds.clone();
        let cancel = cancel.clone();
        move |_, args| {
            kinds
                .borrow_mut()
                .push((args.scrolling_event_kind(), args.scroll_offset()));
            args.set_cancel(cancel.get());
        }
    });
    bar.scroll_to(250.0, AnnotatedScrollBarScrollingEventKind::Click);
    assert_eq!(*offsets.borrow(), vec![250.0]);
    cancel.set(true);
    bar.scroll_to(300.0, AnnotatedScrollBarScrollingEventKind::Drag);
    assert_eq!(*offsets.borrow(), vec![250.0]);
    assert_eq!(
        *kinds.borrow(),
        vec![
            (AnnotatedScrollBarScrollingEventKind::Click, 250.0),
            (AnnotatedScrollBarScrollingEventKind::Drag, 300.0)
        ]
    );
}

#[test]
fn the_buttons_add_a_velocity_of_an_eighth_of_the_viewport() {
    let _scope = test_scope();
    let (_root, bar) = scene();
    let controller = bar.scroll_controller();
    controller.set_values(0.0, 1000.0, 400.0, 160.0);
    let velocities = Rc::new(RefCell::new(Vec::new()));
    let _velocity = controller.add_scroll_velocity_requested(Rc::new({
        let velocities = velocities.clone();
        move |args: &crate::primitives::ScrollControllerAddScrollVelocityRequestedEventArgs| {
            velocities
                .borrow_mut()
                .push((args.offset_velocity(), args.inertia_decay_rate()));
        }
    }));
    let offsets = Rc::new(RefCell::new(Vec::new()));
    let _scrolling = bar.scrolling({
        let offsets = offsets.clone();
        move |_, args| offsets.borrow_mut().push(args.scroll_offset())
    });
    // The increment button is the top button: it scrolls up.
    bar.scroll_to(0.0, AnnotatedScrollBarScrollingEventKind::IncrementButton);
    bar.scroll_to(0.0, AnnotatedScrollBarScrollingEventKind::DecrementButton);
    assert_eq!(bar.small_change(), 20.0);
    assert_eq!(*offsets.borrow(), vec![380.0, 420.0]);
    let velocities = velocities.borrow();
    assert_eq!(velocities.len(), 2);
    assert!((velocities[0].0 as f64 + 20.0 * 3.688880455092886).abs() < 1e-3);
    assert!((velocities[1].0 as f64 - 20.0 * 3.688880455092886).abs() < 1e-3);
    assert_eq!(velocities[0].1, Some(0.975));
}

#[test]
fn the_value_waits_for_the_requests_of_the_bar() {
    let _scope = test_scope();
    let (_root, bar) = scene();
    let controller = bar.scroll_controller();
    controller.set_values(0.0, 1000.0, 100.0, 160.0);
    let (_offsets, _requests) = recorded_scroll_to(&bar);
    bar.scroll_to(600.0, AnnotatedScrollBarScrollingEventKind::Click);
    // While the request runs, the value stays; the increment button scrolls from it.
    controller.set_values(0.0, 1000.0, 300.0, 160.0);
    let offsets = Rc::new(RefCell::new(Vec::new()));
    let _scrolling = bar.scrolling({
        let offsets = offsets.clone();
        move |_, args| offsets.borrow_mut().push(args.scroll_offset())
    });
    bar.scroll_to(0.0, AnnotatedScrollBarScrollingEventKind::IncrementButton);
    assert_eq!(*offsets.borrow(), vec![80.0]);
    controller.notify_requested_scroll_completed(1);
    bar.scroll_to(0.0, AnnotatedScrollBarScrollingEventKind::IncrementButton);
    assert_eq!(offsets.borrow()[1], 280.0);
}

#[test]
fn labels_sit_at_their_offsets_and_colliding_labels_collapse() {
    let _scope = test_scope();
    let (root, bar) = scene();
    bar.scroll_controller().set_values(0.0, 1000.0, 0.0, 100.0);
    let template: Rc<dyn crate::templates::IDataTemplate> = crate::templates::FuncDataTemplate::new(
        |_| true,
        |_, _| {
            let text = TextBlock::new();
            text.set_height(20.0);
            Some(text.upcast())
        },
        false,
    );
    bar.set_label_template(Some(template));
    for (content, offset) in [("A", 0.0), ("B", 50.0), ("C", 500.0), ("D", 900.0)] {
        let content: ferroui_base::BoxedValue = Rc::new(content.to_string());
        bar.labels().add(AnnotatedScrollBarLabel::new(Some(content), offset));
    }
    bar.layout_labels();
    root.layout_manager().execute_layout_pass();
    let labels_grid = template_part(&bar, "PART_LabelsGrid")
        .cast::<crate::Panel>()
        .expect("a panel");
    bar.layout_labels();
    let children = labels_grid.children().snapshot();
    assert_eq!(children.len(), 4);
    let factor = bar.scroll_offset_to_label_offset_factor();
    let tops: Vec<f64> = children.iter().map(|c| c.margin().top).collect();
    assert_eq!(tops, vec![0.0, 50.0 * factor, 500.0 * factor, 900.0 * factor]);
    // B (at 9.85 px) overlaps A (0 to 20 px); the first label wins over B.
    let visible: Vec<bool> = children.iter().map(|c| c.is_visible()).collect();
    assert_eq!(visible, vec![true, false, true, true]);
}

#[test]
fn the_detail_label_asks_its_handlers_for_the_offset_under_the_pointer() {
    let _scope = test_scope();
    let bar = AnnotatedScrollBar::new();
    bar.scroll_controller().set_values(0.0, 1000.0, 0.0, 100.0);
    let requested = Rc::new(RefCell::new(Vec::new()));
    let _detail = bar.detail_label_requested({
        let requested = requested.clone();
        move |_, args| {
            requested.borrow_mut().push(args.scroll_offset());
            let content: ferroui_base::BoxedValue = Rc::new(format!("{}", args.scroll_offset()));
            args.set_content(Some(content));
        }
    });
    // Without a template the factor is 1: 50 px of the rail is the offset 50, at most the end of
    // the content (the maximum and the viewport).
    let content = bar
        .raise_detail_label_requested(50.0)
        .expect("the content of the handler");
    assert_eq!(content.downcast_ref::<String>().map(String::as_str), Some("50"));
    assert_eq!(
        bar.raise_detail_label_requested(5000.0)
            .unwrap()
            .downcast_ref::<String>()
            .unwrap(),
        "1100"
    );
    assert_eq!(*requested.borrow(), vec![50.0, 1100.0]);
    let args = AnnotatedScrollBarDetailLabelRequestedEventArgs::new(12.0);
    let shared = args.clone();
    shared.set_content(None);
    assert!(args.content().is_none());
    let label = AnnotatedScrollBarLabel::new(None, 3.0);
    assert_eq!(label, label.clone());
    assert_ne!(label, AnnotatedScrollBarLabel::new(None, 3.0));
}
