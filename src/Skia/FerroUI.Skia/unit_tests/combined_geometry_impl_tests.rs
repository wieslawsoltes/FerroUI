//! Port of upstream's `CombinedGeometryImplTests.cs` of the Skia unit tests.

use crate::geometry_impl::FillPath;
use crate::CombinedGeometryImpl;
use ferroui_base::platform::IGeometryImpl;
use ferroui_base::Rect;
use skia_safe::{Path, PathBuilder};

#[test]
fn combining_fill_with_empty_stroke_returns_fill_bounds() {
    let mut fill = PathBuilder::new();
    fill.line_to((100.0, 0.0));
    fill.line_to((100.0, 100.0));
    fill.line_to((0.0, 100.0));
    fill.close();

    let stroke = Path::new();

    let result = CombinedGeometryImpl::new(Some(stroke), FillPath::Separate(fill.detach()));

    assert_eq!(Rect::new(0.0, 0.0, 100.0, 100.0), result.bounds());
}
