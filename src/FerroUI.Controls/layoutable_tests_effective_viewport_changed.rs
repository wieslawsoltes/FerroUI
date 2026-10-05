//! The render-transform cases of the effective viewport tests; they need
//! controls, so they live with the controls.

use crate::test_support::{test_scope, TestRoot};
use crate::{Border, Canvas};
use ferroui_base::media::{ITransform, RotateTransform, ScaleTransform, TranslateTransform};
use ferroui_base::{Rect, Ref, RelativePoint, RelativeUnit, Size};
use std::cell::Cell;
use std::rc::Rc;

fn create_root() -> Ref<TestRoot> {
    let root = TestRoot::new();
    root.set_client_size(Size::new(1200.0, 900.0));
    root.set_width(1200.0);
    root.set_height(900.0);
    root
}

fn sized_canvas() -> Ref<Canvas> {
    let target = Canvas::new();
    target.set_width(100.0);
    target.set_height(100.0);
    target
}

fn sized_border(size: f64, child: &Ref<Canvas>) -> Ref<Border> {
    let parent = Border::new();
    parent.set_width(size);
    parent.set_height(size);
    parent.set_child(child);
    parent
}

fn transform<T: ferroui_base::ObjectType>(value: Ref<T>) -> Option<Rc<dyn ITransform>>
where
    Ref<T>: Into<Rc<dyn ITransform>>,
{
    Some(value.into())
}

#[test]
fn translate_transform_doesnt_affect_effective_viewport() {
    let _scope = test_scope();
    let root = create_root();
    let target = sized_canvas();
    let parent = sized_border(200.0, &target);
    let raised = Rc::new(Cell::new(0));

    root.set_child(&parent);

    let _token = target.effective_viewport_changed({
        let raised = raised.clone();
        move |_| raised.set(raised.get() + 1)
    });
    root.execute_initial_layout_pass();

    raised.set(0); // The initial layout pass is expected to raise.

    target.set_render_transform(transform(TranslateTransform::with_offset(8.0, 0.0)));
    target.invalidate_measure();
    root.layout_manager().execute_layout_pass();

    assert_eq!(raised.get(), 0);
}

#[test]
fn translate_transform_on_parent_affects_effective_viewport() {
    let _scope = test_scope();
    let root = create_root();
    let target = sized_canvas();
    let parent = sized_border(200.0, &target);
    let raised = Rc::new(Cell::new(0));

    root.set_child(&parent);
    root.execute_initial_layout_pass();

    let _token = target.effective_viewport_changed({
        let raised = raised.clone();
        move |e| {
            assert_eq!(e.effective_viewport(), Rect::new(-558.0, -400.0, 1200.0, 900.0));
            raised.set(raised.get() + 1);
        }
    });

    // Change the parent render transform to move it. A layout is then needed
    // before the effective viewport change is raised.
    parent.set_render_transform(transform(TranslateTransform::with_offset(8.0, 0.0)));
    parent.invalidate_measure();
    root.layout_manager().execute_layout_pass();

    assert_eq!(raised.get(), 1);
}

#[test]
fn rotate_transform_on_parent_affects_effective_viewport() {
    let _scope = test_scope();
    let root = create_root();
    let target = sized_canvas();
    let parent = sized_border(200.0, &target);
    let raised = Rc::new(Cell::new(0));

    root.set_child(&parent);
    root.execute_initial_layout_pass();

    let _token = target.effective_viewport_changed({
        let raised = raised.clone();
        move |e| {
            // Pixel equality: the values are compared after truncation.
            let actual = e.effective_viewport();
            assert_eq!(
                (actual.x as i32, actual.y as i32, actual.width as i32, actual.height as i32),
                (-651, -792, 1484, 1484)
            );
            raised.set(raised.get() + 1);
        }
    });

    parent.set_render_transform_origin(RelativePoint::new(0.0, 0.0, RelativeUnit::Absolute));
    parent.set_render_transform(transform(RotateTransform::with_angle(45.0)));
    parent.invalidate_measure();
    root.layout_manager().execute_layout_pass();

    assert_eq!(raised.get(), 1);
}

#[test]
fn zero_scale_transform_sets_empty_effective_viewport() {
    let _scope = test_scope();
    let effective_viewport = Rc::new(Cell::new(Rect::from_size(Size::INFINITY)));

    let root = create_root();
    let target = sized_canvas();
    let parent = sized_border(100.0, &target);

    let _token = target.effective_viewport_changed({
        let effective_viewport = effective_viewport.clone();
        move |e| effective_viewport.set(e.effective_viewport())
    });

    root.set_child(&parent);
    root.execute_initial_layout_pass();

    parent.set_render_transform(transform(ScaleTransform::with_scale(0.0, 0.0)));
    root.layout_manager().execute_layout_pass();

    assert_eq!(effective_viewport.get(), Rect::new(0.0, 0.0, 0.0, 0.0));
}
