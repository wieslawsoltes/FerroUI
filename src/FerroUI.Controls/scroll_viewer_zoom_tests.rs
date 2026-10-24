//! Tests of the zoom of `ScrollViewer` (WinUI's `ZoomMode`, `ZoomFactor`,
//! `ChangeView`; not upstream, docs/porting/winui-composition.md phase 7).

use crate::presenters::ScrollContentPresenter;
use crate::primitives::ScrollBarVisibility;
use crate::scroll_viewer_tests::create_template;
use crate::templates::FuncControlTemplate;
use crate::test_support::{test_scope, TestRoot};
use crate::{Border, ScrollViewer, ZoomMode};
use ferroui_base::input::{
    IPointer, KeyModifiers, Pointer, PointerPointProperties, PointerType, PointerUpdateKind, PointerWheelEventArgs,
    RawInputModifiers,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::{Matrix, Point, Ref, Size, Vector, Visual};
use std::rc::Rc;

/// A 100 x 100 scroll viewer, scrolling both ways, over a 200 x 300 border.
fn scene() -> (Ref<TestRoot>, Ref<ScrollViewer>, Ref<Border>, Ref<ScrollContentPresenter>) {
    let target = ScrollViewer::new();
    target.set_template(Some(FuncControlTemplate::for_type::<ScrollViewer>(create_template)));
    target.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Hidden);
    target.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Hidden);
    target.set_width(100.0);
    target.set_height(100.0);
    let content = Border::new();
    content.set_width(200.0);
    content.set_height(300.0);
    target.set_content(Some(crate::Control::boxed(&content)));
    let root = TestRoot::with_child(&target);
    root.layout_manager().execute_initial_layout_pass();
    let presenter = target.scroll_content_presenter().expect("the template has a scroll presenter");
    (root, target, content, presenter)
}

fn layout(root: &Ref<TestRoot>) {
    root.layout_manager().execute_layout_pass();
}

fn wheel(target: &ScrollContentPresenter, root: &Visual, position: Point, modifiers: KeyModifiers, delta: f64) -> bool {
    let pointer: Rc<dyn IPointer> = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
    let e = PointerWheelEventArgs::new(
        target.to_ref(),
        pointer,
        root,
        position,
        0,
        PointerPointProperties::new(RawInputModifiers::NONE, PointerUpdateKind::Other),
        modifiers,
        Vector::new(0.0, delta),
    );
    target.raise_event(&e);
    e.handled()
}

#[test]
fn the_content_is_not_zoomed_by_default() {
    let _scope = test_scope();
    let (_root, target, content, _presenter) = scene();
    assert_eq!(1.0, target.zoom_factor());
    assert_eq!(ZoomMode::Disabled, target.zoom_mode());
    assert_eq!((0.1, 10.0), (target.min_zoom_factor(), target.max_zoom_factor()));
    assert_eq!(Size::new(200.0, 300.0), target.extent());
    assert!(content.manipulation_transform().is_none());
}

#[test]
fn change_view_zooms_the_content_and_its_extent() {
    let _scope = test_scope();
    let (root, target, content, presenter) = scene();

    assert!(target.change_view(None, None, Some(2.0)));
    layout(&root);
    assert_eq!(2.0, target.zoom_factor());
    assert_eq!(2.0, presenter.zoom_factor());
    // The layout of the content is not zoomed; the extent is.
    assert_eq!(Size::new(200.0, 300.0), content.bounds().size());
    assert_eq!(Size::new(400.0, 600.0), target.extent());
    assert_eq!(Some(Matrix::create_scale(2.0, 2.0)), content.manipulation_transform());

    // Offsets are in zoomed units, within the zoomed extent.
    assert!(target.change_view(Some(150.0), Some(450.0), None));
    layout(&root);
    assert_eq!(Vector::new(150.0, 450.0), target.offset());
    assert!(target.change_view(Some(1000.0), Some(1000.0), None));
    layout(&root);
    assert_eq!(Vector::new(300.0, 500.0), target.offset());

    // A point of the content is drawn at its zoomed place in the presenter.
    let to_presenter = content.transform_to_visual(&presenter).expect("a transform to the presenter");
    let point = Point::new(160.0, 260.0) * to_presenter;
    assert_eq!(Point::new(20.0, 20.0), point);

    // Back to 1: the content loses its transform.
    target.change_view(Some(0.0), Some(0.0), Some(1.0));
    layout(&root);
    assert!(content.manipulation_transform().is_none());
    assert_eq!(Size::new(200.0, 300.0), target.extent());
}

#[test]
fn the_zoom_factor_stays_between_its_minimum_and_maximum() {
    let _scope = test_scope();
    let (root, target, _content, _presenter) = scene();
    target.set_max_zoom_factor(4.0);
    target.set_min_zoom_factor(0.5);
    target.zoom_to_factor(8.0);
    layout(&root);
    assert_eq!(4.0, target.zoom_factor());
    target.zoom_to_factor(0.1);
    layout(&root);
    assert_eq!(0.5, target.zoom_factor());
    // At half the size the content fits: nothing to scroll.
    assert_eq!(Size::new(100.0, 150.0), target.extent());
    target.zoom_to_factor(f32::NAN);
    assert_eq!(0.5, target.zoom_factor());
}

#[test]
fn ctrl_and_the_wheel_zoom_about_the_pointer_when_zooming_is_enabled() {
    let _scope = test_scope();
    let (root, target, _content, presenter) = scene();

    // Disabled: the wheel scrolls.
    assert!(wheel(&presenter, &target, Point::new(50.0, 50.0), KeyModifiers::NONE, -1.0));
    layout(&root);
    assert_eq!(1.0, target.zoom_factor());
    assert_eq!(Vector::new(0.0, 50.0), target.offset());
    wheel(&presenter, &target, Point::new(50.0, 50.0), KeyModifiers::CONTROL, 1.0);
    assert_eq!(1.0, target.zoom_factor());

    target.set_zoom_mode(ZoomMode::Enabled);
    assert!(wheel(&presenter, &target, Point::new(50.0, 50.0), KeyModifiers::CONTROL, 1.0));
    layout(&root);
    assert!((target.zoom_factor() - 1.1).abs() < 1e-6, "{}", target.zoom_factor());
    // The point under the pointer stays where it is: (0 + 50) * 1.1 - 50 on both axes (the
    // Ctrl and wheel of the disabled zoom scrolled back to the top).
    let offset = target.offset();
    assert!((offset.x - 5.0).abs() < 1e-6 && (offset.y - 5.0).abs() < 1e-6, "{offset:?}");
}
