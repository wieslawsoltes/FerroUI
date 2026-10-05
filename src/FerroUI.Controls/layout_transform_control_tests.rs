// The reference tests use a rectangle shape as the transformed child; any
// control with an explicit size shows the same behaviour, so a border is used
// here.
use crate::test_support::{test_scope, TestRoot};
use crate::{Border, LayoutTransformControl};
use ferroui_base::media::{ITransform, RotateTransform, ScaleTransform, SkewTransform};
use ferroui_base::{Matrix, Rect, Ref, Size};
use std::rc::Rc;

fn create_with_child_and_measure_and_transform(
    width: f64,
    height: f64,
    transform: impl Into<Rc<dyn ITransform>>,
) -> Ref<LayoutTransformControl> {
    let lt = LayoutTransformControl::new();
    lt.set_layout_transform(Some(transform.into()));

    let child = Border::new();
    child.set_width(width);
    child.set_height(height);
    lt.set_child(child);

    lt.measure(Size::INFINITY);
    lt.arrange(Rect::from_size(lt.desired_size()));

    lt
}

fn transform_measure_size_test(size: Size, transform: impl Into<Rc<dyn ITransform>>, expected_size: Size) {
    let lt = create_with_child_and_measure_and_transform(size.width, size.height, transform);

    let out_size = lt.desired_size();

    assert_eq!(out_size.width, expected_size.width);
    assert_eq!(out_size.height, expected_size.height);
}

fn transform_root_bounds_test(size: Size, transform: impl Into<Rc<dyn ITransform>>, expected_bounds: Rect) {
    let lt = create_with_child_and_measure_and_transform(size.width, size.height, transform);

    let out_bounds = lt.transform_root().unwrap().bounds();

    assert_eq!(out_bounds.x, expected_bounds.x);
    assert_eq!(out_bounds.y, expected_bounds.y);
    assert_eq!(out_bounds.width, expected_bounds.width);
    assert_eq!(out_bounds.height, expected_bounds.height);
}

/// Asserts that the elements of two matrices are equal to 3 decimal places.
fn assert_matrix_close(actual: Matrix, expected: Matrix) {
    for (actual, expected) in [
        (actual.m11, expected.m11),
        (actual.m12, expected.m12),
        (actual.m21, expected.m21),
        (actual.m22, expected.m22),
        (actual.m31, expected.m31),
        (actual.m32, expected.m32),
    ] {
        assert!((actual - expected).abs() < 0.0005, "{actual} != {expected}");
    }
}

fn render_transform_matrix(lt: &LayoutTransformControl) -> Matrix {
    lt.transform_root().unwrap().render_transform().unwrap().value()
}

#[test]
fn measure_on_scale_x2_is_correct() {
    let scale = 2.0;

    transform_measure_size_test(
        Size::new(100.0, 50.0),
        ScaleTransform::with_scale(scale, scale),
        Size::new(200.0, 100.0),
    );
}

#[test]
fn measure_on_scale_x0_5_is_correct() {
    let scale = 0.5;

    transform_measure_size_test(Size::new(100.0, 50.0), ScaleTransform::with_scale(scale, scale), Size::new(50.0, 25.0));
}

#[test]
fn measure_on_skew_x_axis_45_degrees_is_correct() {
    transform_measure_size_test(
        Size::new(100.0, 100.0),
        SkewTransform::with_angles(45.0, 0.0),
        Size::new(200.0, 100.0),
    );
}

#[test]
fn measure_on_skew_y_axis_45_degrees_is_correct() {
    transform_measure_size_test(
        Size::new(100.0, 100.0),
        SkewTransform::with_angles(0.0, 45.0),
        Size::new(100.0, 200.0),
    );
}

#[test]
fn measure_on_skew_x_axis_minus_45_degrees_is_correct() {
    transform_measure_size_test(
        Size::new(100.0, 100.0),
        SkewTransform::with_angles(-45.0, 0.0),
        Size::new(200.0, 100.0),
    );
}

#[test]
fn measure_on_skew_y_axis_minus_45_degrees_is_correct() {
    transform_measure_size_test(
        Size::new(100.0, 100.0),
        SkewTransform::with_angles(0.0, -45.0),
        Size::new(100.0, 200.0),
    );
}

#[test]
fn measure_on_skew_0_degrees_is_correct() {
    transform_measure_size_test(
        Size::new(100.0, 100.0),
        SkewTransform::with_angles(0.0, 0.0),
        Size::new(100.0, 100.0),
    );
}

#[test]
fn measure_on_rotate_90_degrees_is_correct() {
    transform_measure_size_test(Size::new(100.0, 25.0), RotateTransform::with_angle(90.0), Size::new(25.0, 100.0));
}

#[test]
fn measure_on_rotate_minus_90_degrees_is_correct() {
    transform_measure_size_test(Size::new(100.0, 25.0), RotateTransform::with_angle(-90.0), Size::new(25.0, 100.0));
}

#[test]
fn measure_on_rotate_0_degrees_is_correct() {
    transform_measure_size_test(Size::new(100.0, 25.0), RotateTransform::with_angle(0.0), Size::new(100.0, 25.0));
}

#[test]
fn measure_on_rotate_180_degrees_is_correct() {
    transform_measure_size_test(Size::new(100.0, 25.0), RotateTransform::with_angle(180.0), Size::new(100.0, 25.0));
}

#[test]
fn bounds_on_scale_x2_are_correct() {
    let scale = 2.0;

    transform_root_bounds_test(
        Size::new(100.0, 50.0),
        ScaleTransform::with_scale(scale, scale),
        Rect::new(0.0, 0.0, 100.0, 50.0),
    );
}

#[test]
fn bounds_on_scale_x0_5_are_correct() {
    let scale = 0.5;

    transform_root_bounds_test(
        Size::new(100.0, 50.0),
        ScaleTransform::with_scale(scale, scale),
        Rect::new(0.0, 0.0, 100.0, 50.0),
    );
}

#[test]
fn bounds_on_rotate_180_degrees_are_correct() {
    transform_root_bounds_test(
        Size::new(100.0, 25.0),
        RotateTransform::with_angle(180.0),
        Rect::new(100.0, 25.0, 100.0, 25.0),
    );
}

#[test]
fn bounds_on_rotate_0_degrees_are_correct() {
    transform_root_bounds_test(
        Size::new(100.0, 25.0),
        RotateTransform::with_angle(0.0),
        Rect::new(0.0, 0.0, 100.0, 25.0),
    );
}

#[test]
fn bounds_on_rotate_90_degrees_are_correct() {
    transform_root_bounds_test(
        Size::new(100.0, 25.0),
        RotateTransform::with_angle(90.0),
        Rect::new(25.0, 0.0, 100.0, 25.0),
    );
}

#[test]
fn bounds_on_rotate_minus_90_degrees_are_correct() {
    transform_root_bounds_test(
        Size::new(100.0, 25.0),
        RotateTransform::with_angle(-90.0),
        Rect::new(0.0, 100.0, 100.0, 25.0),
    );
}

#[test]
fn bounds_on_transform_applied_then_removed_are_correct() {
    let control = create_with_child_and_measure_and_transform(100.0, 25.0, RotateTransform::with_angle(90.0));

    assert_eq!(control.desired_size(), Size::new(25.0, 100.0));

    control.set_layout_transform(None);
    control.measure(Size::INFINITY);
    control.arrange(Rect::from_size(control.desired_size()));

    assert_eq!(control.desired_size(), Size::new(100.0, 25.0));
}

#[test]
fn should_generate_rotate_transform_90_degrees() {
    let lt = create_with_child_and_measure_and_transform(100.0, 25.0, RotateTransform::with_angle(90.0));

    assert_matrix_close(render_transform_matrix(&lt), Matrix::create_rotation(Matrix::to_radians(90.0)));
}

#[test]
fn should_generate_rotate_transform_minus_90_degrees() {
    let lt = create_with_child_and_measure_and_transform(100.0, 25.0, RotateTransform::with_angle(-90.0));

    assert_matrix_close(render_transform_matrix(&lt), Matrix::create_rotation(Matrix::to_radians(-90.0)));
}

#[test]
fn should_generate_scale_transform_x2() {
    let lt = create_with_child_and_measure_and_transform(100.0, 50.0, ScaleTransform::with_scale(2.0, 2.0));

    assert_matrix_close(render_transform_matrix(&lt), Matrix::create_scale(2.0, 2.0));
}

#[test]
fn should_generate_skew_transform_45_degrees() {
    let lt = create_with_child_and_measure_and_transform(100.0, 100.0, SkewTransform::with_angles(45.0, 45.0));

    assert_matrix_close(
        render_transform_matrix(&lt),
        Matrix::create_skew(Matrix::to_radians(45.0), Matrix::to_radians(45.0)),
    );
}

#[test]
fn should_generate_skew_transform_minus_45_degrees() {
    let lt = create_with_child_and_measure_and_transform(100.0, 100.0, SkewTransform::with_angles(-45.0, -45.0));

    assert_matrix_close(
        render_transform_matrix(&lt),
        Matrix::create_skew(Matrix::to_radians(-45.0), Matrix::to_radians(-45.0)),
    );
}

#[test]
fn should_apply_transform_on_attach_to_visual_tree() {
    // The reference test shows the control in a window; a test root attaches
    // it to a visual tree in the same way.
    let _scope = test_scope();
    let transform = SkewTransform::with_angles(-45.0, -45.0);

    let lt = create_with_child_and_measure_and_transform(100.0, 100.0, &transform);

    transform.set_angle_x(45.0);
    transform.set_angle_y(45.0);

    let root = TestRoot::with_child(&lt);
    root.execute_initial_layout_pass();

    assert_matrix_close(
        render_transform_matrix(&lt),
        Matrix::create_skew(Matrix::to_radians(45.0), Matrix::to_radians(45.0)),
    );
}

#[test]
fn transform_changes_are_tracked_only_while_attached() {
    let _scope = test_scope();
    let transform = ScaleTransform::with_scale(2.0, 2.0);
    let lt = create_with_child_and_measure_and_transform(100.0, 50.0, &transform);
    let root = TestRoot::with_child(&lt);
    root.execute_initial_layout_pass();

    transform.set_scale_x(3.0);

    assert_eq!(render_transform_matrix(&lt), Matrix::create_scale(3.0, 2.0));
    assert!(!lt.is_measure_valid());

    root.set_child(None);
    transform.set_scale_x(4.0);

    assert_eq!(render_transform_matrix(&lt), Matrix::create_scale(3.0, 2.0));
}

#[test]
fn use_render_transform_mirrors_the_render_transform() {
    let lt = LayoutTransformControl::new();
    lt.set_use_render_transform(true);

    let transform: Rc<dyn ITransform> = ScaleTransform::with_scale(2.0, 2.0).into();
    lt.set_render_transform(Some(transform.clone()));

    assert!(lt.layout_transform().is_some_and(|t| *t == *transform));

    lt.set_use_render_transform(false);

    assert!(lt.layout_transform().is_none());
}
