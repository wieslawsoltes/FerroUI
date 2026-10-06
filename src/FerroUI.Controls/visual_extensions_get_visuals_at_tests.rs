//! Port of `VisualTree/VisualExtensions_GetVisualsAt.cs` (base unit tests).
//! The trees are built from controls on the compositor test services of
//! the compositor hit testing tests, so the tests live with the controls.

use crate::compositor_hit_testing_tests::CompositorTestServices;
use crate::{Border, StackPanel};
use ferroui_base::layout::Orientation;
use ferroui_base::media::{Brushes, GeometryHitTestResult, IBrush, IntersectionResult, RectangleGeometry};
use ferroui_base::{Point, Ref, Size};
use std::rc::Rc;

/// The stack panel of two borders all three tests build; returns it and
/// the first border.
fn tree(background: Option<Rc<dyn IBrush>>) -> (Ref<StackPanel>, Ref<Border>) {
    let border = |brush: Rc<dyn IBrush>| {
        let border = Border::new();
        border.set_width(100.0);
        border.set_height(200.0);
        border.set_background(Some(brush));
        border
    };
    let target = border(Brushes::red());
    let panel = StackPanel::new();
    panel.set_background(background);
    panel.children().add(target.clone());
    panel.children().add(border(Brushes::green()));
    panel.set_orientation(Orientation::Horizontal);
    (panel, target)
}

#[test]
fn should_find_control() {
    let services = CompositorTestServices::new(Size::new(200.0, 200.0));
    let (content, target) = tree(None);
    services.set_content(&content);

    services.run_jobs();
    let result = target.get_visuals_at(Point::new(50.0, 50.0));

    assert_eq!(1, result.len());
    assert_eq!(target.clone().upcast::<ferroui_base::Visual>(), result[0]);
}

#[test]
fn should_find_control_with_geometry() {
    let services = CompositorTestServices::new(Size::new(200.0, 200.0));
    let (content, target) = tree(None);
    services.set_content(&content);

    services.run_jobs();
    let geo = RectangleGeometry::with_rect(target.bounds());
    let result = target.get_visuals_at_geometry(&geo);

    assert_eq!(1, result.len());
    assert_eq!(GeometryHitTestResult::new(target.clone().upcast(), IntersectionResult::FullyInside), result[0]);
}

#[test]
fn should_not_find_sibling_control() {
    let services = CompositorTestServices::new(Size::new(200.0, 200.0));
    let (content, target) = tree(Some(Brushes::white()));
    services.set_content(&content);

    services.run_jobs();
    let result = target.get_visuals_at(Point::new(150.0, 50.0));

    assert!(result.is_empty());
}
