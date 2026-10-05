use super::Ellipse;
use crate::test_support_shapes::MockRenderInterfaceScope;
use ferroui_base::media::EllipseGeometry;
use ferroui_base::{Rect, Size};

#[test]
fn measure_does_not_set_rendered_geometry_rect() {
    let _app = MockRenderInterfaceScope::install();

    let target = Ellipse::new();

    target.measure(Size::new(100.0, 100.0));

    let geometry = target.rendered_geometry().unwrap().cast::<EllipseGeometry>().expect("an ellipse geometry");
    assert_eq!(Rect::default(), geometry.rect());
}

#[test]
fn arrange_sets_rendered_geometry_properties() {
    let _app = MockRenderInterfaceScope::install();

    let target = Ellipse::new();

    target.measure(Size::new(100.0, 100.0));
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    let geometry = target.rendered_geometry().unwrap().cast::<EllipseGeometry>().expect("an ellipse geometry");
    assert_eq!(Rect::new(0.0, 0.0, 100.0, 100.0), geometry.rect());
}

#[test]
fn rearranging_updates_rendered_geometry_rect() {
    let _app = MockRenderInterfaceScope::install();

    let target = Ellipse::new();

    target.measure(Size::new(100.0, 100.0));
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    let geometry = target.rendered_geometry().unwrap().cast::<EllipseGeometry>().expect("an ellipse geometry");
    assert_eq!(Rect::new(0.0, 0.0, 100.0, 100.0), geometry.rect());

    target.measure(Size::new(200.0, 200.0));
    target.arrange(Rect::new(0.0, 0.0, 200.0, 200.0));

    let geometry = target.rendered_geometry().unwrap().cast::<EllipseGeometry>().expect("an ellipse geometry");
    assert_eq!(Rect::new(0.0, 0.0, 200.0, 200.0), geometry.rect());
}
