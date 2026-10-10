//! Port of upstream's `GeometryClippingTests.cs`.

use crate::test_base::TestBase;
use ferroui_base::media::{Brushes, Stretch, StreamGeometry};
use ferroui_controls::shapes::Path;
use ferroui_controls::Canvas;

fn base() -> TestBase {
    TestBase::new("GeometryClipping")
}

#[test]
fn geometry_clip_clips_path() {
    let t = base();
    let target = Canvas::new();
    target.set_background(Some(Brushes::yellow()));
    target.set_clip(StreamGeometry::parse("F1 M 0,0  H 76 V 76 Z").expect("the path data is valid"));
    target.set_width(76.0);
    target.set_height(76.0);
    let path = Path::new();
    path.set_width(32.0);
    path.set_height(40.0);
    Canvas::set_left(&path, 23.0);
    Canvas::set_top(&path, 18.0);
    path.set_stretch(Stretch::Fill);
    path.set_fill(Some(Brushes::black()));
    path.set_data(
        StreamGeometry::parse("F1 M 27,18L 23,26L 33,30L 24,38L 33,46L 23,50L 27,58L 45,58L 55,38L 45,18L 27,18 Z")
            .expect("the path data is valid"),
    );
    target.children().add(path);

    t.render_to_file(&target, "Geometry_Clip_Clips_Path");
    t.compare_images("Geometry_Clip_Clips_Path");
}
