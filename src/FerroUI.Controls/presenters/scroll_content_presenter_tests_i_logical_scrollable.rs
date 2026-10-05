use crate::presenters::ScrollContentPresenter;
use crate::test_support_scrolling::TestScrollable;
use crate::{Control, ScrollViewer};
use ferroui_base::reactive::ObservableExt;
use ferroui_base::{FerroObjectExtensions, Rect, Ref, Size, Vector};
use std::cell::Cell;
use std::rc::Rc;

fn presenter_with(content: &Ref<TestScrollable>) -> Ref<ScrollContentPresenter> {
    let target = ScrollContentPresenter::new();
    target.set_content(Some(Control::boxed(content)));
    target
}

#[test]
fn measure_should_pass_unchanged_bounds_to_i_logical_scrollable() {
    let scrollable = TestScrollable::new();
    let target = presenter_with(&scrollable);

    target.update_child();
    target.measure(Size::new(100.0, 100.0));

    assert_eq!(Size::new(100.0, 100.0), scrollable.available_size());
}

#[test]
fn arrange_should_not_offset_i_logical_scrollable_bounds() {
    let scrollable =
        TestScrollable::with_state(Size::new(100.0, 100.0), Vector::new(50.0, 50.0), Size::new(25.0, 25.0));
    let target = presenter_with(&scrollable);

    target.update_child();
    target.measure(Size::new(100.0, 100.0));
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    assert_eq!(Rect::new(0.0, 0.0, 100.0, 100.0), scrollable.bounds());
}

#[test]
fn arrange_should_offset_i_logical_scrollable_bounds_when_logical_scroll_disabled() {
    let scrollable = TestScrollable::new();
    scrollable.set_is_logical_scroll_enabled(false);

    let target = ScrollContentPresenter::new();
    target.set_can_horizontally_scroll(true);
    target.set_can_vertically_scroll(true);
    target.set_content(Some(Control::boxed(&scrollable)));

    target.update_child();
    target.measure(Size::new(100.0, 100.0));
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));
    target.set_offset(Vector::new(25.0, 25.0));
    target.measure(Size::new(100.0, 100.0));
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    assert_eq!(Rect::new(-25.0, -25.0, 150.0, 150.0), scrollable.bounds());
}

#[test]
fn arrange_should_not_set_viewport_and_extent_with_i_logical_scrollable() {
    let target = presenter_with(&TestScrollable::new());
    let changed = Rc::new(Cell::new(false));

    target.update_child();
    target.measure(Size::new(100.0, 100.0));

    let skip_first = || {
        let changed = changed.clone();
        let skipped = Cell::new(false);
        move |_: Size| {
            if skipped.replace(true) {
                changed.set(true);
            }
        }
    };
    let _viewport = FerroObjectExtensions::get_observable(fo(&target), ScrollViewer::viewport_property())
        .subscribe_fn(skip_first());
    let _extent =
        FerroObjectExtensions::get_observable(fo(&target), ScrollViewer::extent_property()).subscribe_fn(skip_first());

    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    assert!(!changed.get());
}

#[test]
fn invalidate_scroll_should_be_set_when_set_as_content() {
    let scrollable = TestScrollable::new();
    let target = presenter_with(&scrollable);

    target.update_child();

    assert!(scrollable.has_scroll_invalidated_subscriber());
}

#[test]
fn invalidate_scroll_should_be_cleared_when_removed_from_content() {
    let scrollable = TestScrollable::new();
    let target = presenter_with(&scrollable);

    target.update_child();
    target.set_content(None);
    target.update_child();

    assert!(!scrollable.has_scroll_invalidated_subscriber());
}

#[test]
fn extent_offset_and_viewport_should_be_read_from_i_logical_scrollable() {
    let scrollable =
        TestScrollable::with_state(Size::new(100.0, 100.0), Vector::new(50.0, 50.0), Size::new(25.0, 25.0));
    let target = presenter_with(&scrollable);

    target.update_child();

    assert_eq!(scrollable.extent(), target.extent());
    assert_eq!(scrollable.offset(), target.offset());
    assert_eq!(scrollable.viewport(), target.viewport());

    scrollable.set_extent(Size::new(200.0, 200.0));
    scrollable.set_offset(Vector::new(100.0, 100.0));
    scrollable.set_viewport(Size::new(50.0, 50.0));

    assert_eq!(scrollable.extent(), target.extent());
    assert_eq!(scrollable.offset(), target.offset());
    assert_eq!(scrollable.viewport(), target.viewport());
}

#[test]
fn offset_should_be_written_to_i_logical_scrollable() {
    let scrollable = TestScrollable::new();
    scrollable.set_extent(Size::new(100.0, 100.0));
    scrollable.set_offset(Vector::new(50.0, 50.0));
    let target = presenter_with(&scrollable);

    target.update_child();
    target.set_offset(Vector::new(25.0, 25.0));

    assert_eq!(target.offset(), scrollable.offset());
}

#[test]
fn offset_should_not_be_written_to_i_logical_scrollable_after_removal() {
    let scrollable = TestScrollable::new();
    scrollable.set_extent(Size::new(100.0, 100.0));
    scrollable.set_offset(Vector::new(50.0, 50.0));
    let target = presenter_with(&scrollable);

    target.set_content(None);
    target.set_offset(Vector::new(25.0, 25.0));

    assert_eq!(Vector::new(50.0, 50.0), scrollable.offset());
}

#[test]
fn toggling_is_logical_scroll_enabled_should_update_state() {
    let scrollable =
        TestScrollable::with_state(Size::new(100.0, 100.0), Vector::new(50.0, 50.0), Size::new(25.0, 25.0));

    let target = ScrollContentPresenter::new();
    target.set_can_horizontally_scroll(true);
    target.set_can_vertically_scroll(true);
    target.set_content(Some(Control::boxed(&scrollable)));

    target.update_child();
    target.measure(Size::new(100.0, 100.0));
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    assert_eq!(scrollable.extent(), target.extent());
    assert_eq!(scrollable.offset(), target.offset());
    assert_eq!(scrollable.viewport(), target.viewport());
    assert_eq!(Rect::new(0.0, 0.0, 100.0, 100.0), scrollable.bounds());

    scrollable.set_is_logical_scroll_enabled(false);
    scrollable.raise_scroll_invalidated();
    target.measure(Size::new(100.0, 100.0));
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    assert_eq!(Size::new(150.0, 150.0), target.extent());
    assert_eq!(Vector::new(0.0, 0.0), target.offset());
    assert_eq!(Size::new(100.0, 100.0), target.viewport());
    assert_eq!(Rect::new(0.0, 0.0, 150.0, 150.0), scrollable.bounds());

    scrollable.set_is_logical_scroll_enabled(true);
    scrollable.raise_scroll_invalidated();
    target.measure(Size::new(100.0, 100.0));
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    assert_eq!(scrollable.extent(), target.extent());
    assert_eq!(scrollable.offset(), target.offset());
    assert_eq!(scrollable.viewport(), target.viewport());
    assert_eq!(Rect::new(0.0, 0.0, 100.0, 100.0), scrollable.bounds());
}

#[test]
fn changing_content_should_update_state() {
    let logical_scrollable =
        TestScrollable::with_state(Size::new(100.0, 100.0), Vector::new(50.0, 50.0), Size::new(25.0, 25.0));

    let non_logical_scrollable = TestScrollable::new();
    non_logical_scrollable.set_is_logical_scroll_enabled(false);

    let target = ScrollContentPresenter::new();
    target.set_can_horizontally_scroll(true);
    target.set_can_vertically_scroll(true);
    target.set_content(Some(Control::boxed(&logical_scrollable)));

    target.update_child();
    target.measure(Size::new(100.0, 100.0));
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    assert_eq!(logical_scrollable.extent(), target.extent());
    assert_eq!(logical_scrollable.offset(), target.offset());
    assert_eq!(logical_scrollable.viewport(), target.viewport());
    assert_eq!(Rect::new(0.0, 0.0, 100.0, 100.0), logical_scrollable.bounds());

    target.set_content(Some(Control::boxed(&non_logical_scrollable)));
    target.update_child();
    target.measure(Size::new(100.0, 100.0));
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    assert_eq!(Size::new(150.0, 150.0), target.extent());
    assert_eq!(Vector::new(0.0, 0.0), target.offset());
    assert_eq!(Size::new(100.0, 100.0), target.viewport());
    assert_eq!(Rect::new(0.0, 0.0, 150.0, 150.0), non_logical_scrollable.bounds());

    target.set_content(Some(Control::boxed(&logical_scrollable)));
    target.update_child();
    target.measure(Size::new(100.0, 100.0));
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    assert_eq!(logical_scrollable.extent(), target.extent());
    assert_eq!(logical_scrollable.offset(), target.offset());
    assert_eq!(logical_scrollable.viewport(), target.viewport());
    assert_eq!(Rect::new(0.0, 0.0, 100.0, 100.0), logical_scrollable.bounds());
}

#[test]
fn should_set_i_logical_scrolable_can_horizontally_scroll() {
    let logical_scrollable = TestScrollable::new();
    let target = presenter_with(&logical_scrollable);

    target.update_child();
    assert!(!logical_scrollable.can_horizontally_scroll());
    target.set_can_horizontally_scroll(true);
    assert!(logical_scrollable.can_horizontally_scroll());
}

#[test]
fn should_set_i_logical_scrolable_can_vertically_scroll() {
    let logical_scrollable = TestScrollable::new();
    let target = presenter_with(&logical_scrollable);

    target.update_child();
    assert!(!logical_scrollable.can_vertically_scroll());
    target.set_can_vertically_scroll(true);
    assert!(logical_scrollable.can_vertically_scroll());
}

/// The object as its root class, for calls that would otherwise resolve to
/// a member of an intermediate class.
fn fo(object: &ferroui_base::FerroObject) -> &ferroui_base::FerroObject {
    object
}
