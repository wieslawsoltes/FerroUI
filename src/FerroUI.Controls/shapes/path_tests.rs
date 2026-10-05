use super::Path;
use crate::test_support::{test_scope, TestRoot};
use crate::test_support_shapes::MockRenderInterfaceScope;
use ferroui_base::media::{EllipseGeometry, RectangleGeometry, Stretch};
use ferroui_base::{Matrix, Rect, Ref, Size};

fn rectangle_path(rect: Rect, stretch: Stretch) -> Ref<Path> {
    let target = Path::new();
    let data = RectangleGeometry::new();
    data.set_rect(rect);
    target.set_data(data);
    target.set_stretch(stretch);
    target
}

#[test]
fn path_with_null_data_does_not_throw_on_measure() {
    let target = Path::new();

    target.measure(Size::INFINITY);
}

#[test]
fn subscribes_to_geometry_changes() {
    let _scope = test_scope();
    let _app = MockRenderInterfaceScope::install();

    let geometry = EllipseGeometry::new();
    geometry.set_rect(Rect::new(0.0, 0.0, 10.0, 10.0));
    let target = Path::new();
    target.set_data(&geometry);

    let root = TestRoot::with_child(&target);

    target.measure(Size::INFINITY);
    assert!(target.is_measure_valid());

    geometry.set_rect(Rect::new(0.0, 0.0, 20.0, 20.0));

    assert!(!target.is_measure_valid());

    root.set_child(None);
}

#[test]
fn calculates_correct_desired_size_for_finite_bounds() {
    for (stretch, expected_width, expected_height) in [
        (Stretch::None, 100.0, 200.0),
        (Stretch::Fill, 500.0, 500.0),
        (Stretch::Uniform, 250.0, 500.0),
        (Stretch::UniformToFill, 500.0, 500.0),
    ] {
        let _app = MockRenderInterfaceScope::install();

        let target = rectangle_path(Rect::new(0.0, 0.0, 100.0, 200.0), stretch);

        target.measure(Size::new(500.0, 500.0));

        assert_eq!(Size::new(expected_width, expected_height), target.desired_size(), "{stretch:?}");
    }
}

#[test]
fn calculates_correct_desired_size_for_infinite_bounds() {
    for stretch in [Stretch::None, Stretch::Fill, Stretch::Uniform, Stretch::UniformToFill] {
        let _app = MockRenderInterfaceScope::install();

        let target = rectangle_path(Rect::new(0.0, 0.0, 100.0, 200.0), stretch);

        target.measure(Size::new(f64::INFINITY, f64::INFINITY));

        assert_eq!(Size::new(100.0, 200.0), target.desired_size(), "{stretch:?}");
    }
}

#[test]
fn measure_does_not_update_rendered_geometry_transform() {
    let _app = MockRenderInterfaceScope::install();

    let target = rectangle_path(Rect::new(0.0, 0.0, 100.0, 200.0), Stretch::Fill);

    target.measure(Size::new(500.0, 500.0));

    let rendered_geometry = target.rendered_geometry().expect("a rendered geometry");
    assert!(rendered_geometry.transform().is_none());
}

#[test]
fn arrange_updates_rendered_geometry_transform() {
    for (stretch, expected_scale_x, expected_scale_y) in [
        (Stretch::None, 1.0, 1.0),
        (Stretch::Fill, 5.0, 2.5),
        (Stretch::Uniform, 2.5, 2.5),
        (Stretch::UniformToFill, 5.0, 5.0),
    ] {
        let _app = MockRenderInterfaceScope::install();

        let target = rectangle_path(Rect::new(0.0, 0.0, 100.0, 200.0), stretch);

        target.measure(Size::new(500.0, 500.0));
        target.arrange(Rect::new(0.0, 0.0, 500.0, 500.0));

        let rendered_geometry = target.rendered_geometry().expect("a rendered geometry");

        if expected_scale_x == 1.0 && expected_scale_y == 1.0 {
            assert!(rendered_geometry.transform().is_none(), "{stretch:?}");
        } else {
            let transform = rendered_geometry.transform().expect("a transform");
            assert_eq!(Matrix::create_scale(expected_scale_x, expected_scale_y), transform.value(), "{stretch:?}");
        }
    }
}

#[test]
fn arrange_reserves_all_of_arrange_rect() {
    let _app = MockRenderInterfaceScope::install();

    let geometry = RectangleGeometry::new();
    geometry.set_rect(Rect::new(0.0, 0.0, 100.0, 200.0));
    let target = Path::new();
    target.set_data(&geometry);
    target.set_stretch(Stretch::Uniform);

    target.measure(Size::new(400.0, 400.0));
    target.arrange(Rect::new(0.0, 0.0, 400.0, 400.0));

    assert_eq!(Rect::new(0.0, 0.0, 100.0, 200.0), geometry.rect());
    let rendered_geometry = target.rendered_geometry().expect("a rendered geometry");
    let transform = rendered_geometry.transform().expect("a transform");
    assert_eq!(Matrix::create_scale(2.0, 2.0), transform.value());
    assert_eq!(Rect::new(0.0, 0.0, 400.0, 400.0), target.bounds());
}

#[test]
fn measure_without_arrange_does_not_clear_rendered_geometry_transform() {
    let _app = MockRenderInterfaceScope::install();

    let target = rectangle_path(Rect::new(0.0, 0.0, 100.0, 100.0), Stretch::Fill);

    target.measure(Size::new(200.0, 200.0));
    target.arrange(Rect::new(0.0, 0.0, 200.0, 200.0));

    let rendered_geometry = target.rendered_geometry().expect("a rendered geometry");
    let transform = rendered_geometry.transform().expect("a transform");
    assert_eq!(Matrix::create_scale(2.0, 2.0), transform.value());

    target.measure(Size::new(300.0, 300.0));

    assert_eq!(Matrix::create_scale(2.0, 2.0), target.rendered_geometry().unwrap().transform().unwrap().value());
}

#[test]
fn arrange_without_measure_updates_rendered_geometry_transform() {
    let _app = MockRenderInterfaceScope::install();

    let target = rectangle_path(Rect::new(0.0, 0.0, 100.0, 100.0), Stretch::Fill);

    target.measure(Size::new(200.0, 200.0));
    target.arrange(Rect::new(0.0, 0.0, 200.0, 200.0));
    let rendered_geometry = target.rendered_geometry().expect("a rendered geometry");
    let transform = rendered_geometry.transform().expect("a transform");
    assert_eq!(Matrix::create_scale(2.0, 2.0), transform.value());

    target.arrange(Rect::new(0.0, 0.0, 300.0, 300.0));
    assert_eq!(Matrix::create_scale(3.0, 3.0), target.rendered_geometry().unwrap().transform().unwrap().value());
}
