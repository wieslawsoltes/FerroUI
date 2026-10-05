// The reference tests use a rectangle shape as the scaled child; any control
// with an explicit size shows the same behaviour, so a border is used here.
use crate::{Border, Canvas, Control, Viewbox};
use ferroui_base::media::{Stretch, StretchDirection};
use ferroui_base::{Matrix, Point, Rect, Ref, Size, StyledElement, Vector};

fn sized<T>(control: Ref<T>, width: f64, height: f64) -> Ref<T>
where
    T: ferroui_base::ObjectType + ferroui_base::Upcast<ferroui_base::layout::Layoutable>,
{
    {
        let layoutable: &ferroui_base::layout::Layoutable = (*control).upcast();
        layoutable.set_width(width);
        layoutable.set_height(height);
    }
    control
}

fn viewbox_with_rectangle() -> Ref<Viewbox> {
    let target = Viewbox::new();
    target.set_child(sized(Border::new(), 100.0, 50.0));
    target
}

fn try_get_scale(viewbox: &Viewbox) -> Option<Vector> {
    let matrix = viewbox.internal_transform()?.value();

    Some(Matrix::try_decompose_transform(matrix).map(|decomposed| decomposed.scale).unwrap_or_default())
}

#[test]
fn viewbox_stretch_uniform_child() {
    let target = viewbox_with_rectangle();

    target.measure(Size::new(200.0, 200.0));
    target.arrange(Rect::from_position_size(Point::new(0.0, 0.0), target.desired_size()));

    assert_eq!(target.desired_size(), Size::new(200.0, 100.0));

    let scale = try_get_scale(&target).unwrap();
    assert_eq!(scale.x, 2.0);
    assert_eq!(scale.y, 2.0);
}

#[test]
fn viewbox_stretch_none_child() {
    let target = viewbox_with_rectangle();
    target.set_stretch(Stretch::None);

    target.measure(Size::new(200.0, 200.0));
    target.arrange(Rect::from_position_size(Point::new(0.0, 0.0), target.desired_size()));

    assert_eq!(target.desired_size(), Size::new(100.0, 50.0));

    let scale = try_get_scale(&target).unwrap();
    assert_eq!(scale.x, 1.0);
    assert_eq!(scale.y, 1.0);
}

#[test]
fn viewbox_stretch_fill_child() {
    let target = viewbox_with_rectangle();
    target.set_stretch(Stretch::Fill);

    target.measure(Size::new(200.0, 200.0));
    target.arrange(Rect::from_position_size(Point::new(0.0, 0.0), target.desired_size()));

    assert_eq!(target.desired_size(), Size::new(200.0, 200.0));

    let scale = try_get_scale(&target).unwrap();
    assert_eq!(scale.x, 2.0);
    assert_eq!(scale.y, 4.0);
}

#[test]
fn viewbox_stretch_uniform_to_fill_child() {
    let target = viewbox_with_rectangle();
    target.set_stretch(Stretch::UniformToFill);

    target.measure(Size::new(200.0, 200.0));
    target.arrange(Rect::from_position_size(Point::new(0.0, 0.0), target.desired_size()));

    assert_eq!(target.desired_size(), Size::new(200.0, 200.0));

    let scale = try_get_scale(&target).unwrap();
    assert_eq!(scale.x, 4.0);
    assert_eq!(scale.y, 4.0);
}

#[test]
fn viewbox_stretch_uniform_child_with_unrestricted_width() {
    let target = viewbox_with_rectangle();

    target.measure(Size::new(f64::INFINITY, 200.0));
    target.arrange(Rect::from_position_size(Point::new(0.0, 0.0), target.desired_size()));

    assert_eq!(target.desired_size(), Size::new(400.0, 200.0));

    let scale = try_get_scale(&target).unwrap();
    assert_eq!(scale.x, 4.0);
    assert_eq!(scale.y, 4.0);
}

#[test]
fn viewbox_stretch_uniform_child_with_unrestricted_height() {
    let target = viewbox_with_rectangle();

    target.measure(Size::new(200.0, f64::INFINITY));
    target.arrange(Rect::from_position_size(Point::new(0.0, 0.0), target.desired_size()));

    assert_eq!(target.desired_size(), Size::new(200.0, 100.0));

    let scale = try_get_scale(&target).unwrap();
    assert_eq!(scale.x, 2.0);
    assert_eq!(scale.y, 2.0);
}

/// (child width, child height, viewbox width, viewbox height, expected width,
/// expected height, expected scale)
type SizeAndScaleCase = (f64, f64, f64, f64, f64, f64, f64);

fn check_size_and_scale(stretch_direction: StretchDirection, cases: &[SizeAndScaleCase]) {
    for &(child_width, child_height, viewbox_width, viewbox_height, expected_width, expected_height, expected_scale) in
        cases
    {
        let target = Viewbox::new();
        target.set_child(sized(Control::new(), child_width, child_height));
        target.set_stretch_direction(stretch_direction);

        target.measure(Size::new(viewbox_width, viewbox_height));
        target.arrange(Rect::from_position_size(Point::default(), target.desired_size()));

        assert_eq!(target.desired_size(), Size::new(expected_width, expected_height));

        let scale = try_get_scale(&target).unwrap();
        assert_eq!(scale.x, expected_scale);
        assert_eq!(scale.y, expected_scale);
    }
}

#[test]
fn viewbox_should_return_correct_size_and_scale_stretch_direction_down_only() {
    check_size_and_scale(
        StretchDirection::DownOnly,
        &[
            (50.0, 100.0, 50.0, 100.0, 50.0, 100.0, 1.0),
            (50.0, 100.0, 150.0, 150.0, 50.0, 100.0, 1.0),
            (50.0, 100.0, 25.0, 50.0, 25.0, 50.0, 0.5),
        ],
    );
}

#[test]
fn viewbox_should_return_correct_size_and_scale_stretch_direction_up_only() {
    check_size_and_scale(
        StretchDirection::UpOnly,
        &[
            (50.0, 100.0, 50.0, 100.0, 50.0, 100.0, 1.0),
            (50.0, 100.0, 25.0, 50.0, 25.0, 50.0, 1.0),
            (50.0, 100.0, 150.0, 150.0, 75.0, 150.0, 1.5),
        ],
    );
}

#[test]
fn child_should_be_logical_child_of_viewbox() {
    let target = Viewbox::new();

    assert!(target.logical_children().is_empty());

    let child = Canvas::new();
    target.set_child(&child);

    assert_eq!(target.logical_children().to_vec(), vec![child.clone().upcast::<StyledElement>()]);
    assert_eq!(child.parent().unwrap(), target);

    target.set_child(None);

    assert!(target.logical_children().is_empty());
    assert!(child.parent().is_none());
}

#[test]
fn changing_child_should_invalidate_layout() {
    let target = Viewbox::new();

    target.set_child(sized(Canvas::new(), 100.0, 100.0));

    target.measure(Size::INFINITY);
    target.arrange(Rect::from_size(target.desired_size()));
    assert_eq!(target.desired_size(), Size::new(100.0, 100.0));

    target.set_child(sized(Canvas::new(), 200.0, 200.0));

    target.measure(Size::INFINITY);
    target.arrange(Rect::from_size(target.desired_size()));
    assert_eq!(target.desired_size(), Size::new(200.0, 200.0));
}

#[test]
fn child_is_hosted_in_a_container_visual() {
    let target = Viewbox::new();
    let child = Canvas::new();

    target.set_child(&child);

    assert_eq!(target.visual_children().count(), 1);
    let container = target.visual_children().get(0);
    assert_eq!(child.visual_parent().unwrap(), container);
    assert_eq!(container.visual_parent().unwrap(), target);

    target.set_child(None);

    assert!(child.visual_parent().is_none());
    assert_eq!(target.visual_children().count(), 1);
}
