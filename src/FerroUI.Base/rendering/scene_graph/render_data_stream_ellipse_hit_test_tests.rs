//! Port of `Rendering/SceneGraph/RenderDataStreamEllipseHitTestTests.cs`.

use super::scene_graph_test_support::{b, black, black_pen, p};
use crate::rendering::composition::drawing::RenderDataStream;
use crate::{Point, Rect};

/// Declares one test per data row of a parameterized test.
macro_rules! theory {
    ($func:ident: $($name:ident($($arg:expr),* $(,)?));+ $(;)?) => {
        $(
            #[test]
            fn $name() {
                $func($($arg),*)
            }
        )+
    };
}

fn fill_only_hit_test(x: f64, y: f64, inside: bool) {
    let mut stream = RenderDataStream::new();
    stream.draw_ellipse(b(&black()), None, None, Rect::new(0.0, 0.0, 100.0, 100.0));

    assert_eq!(inside, stream.hit_test(Point::new(x, y)));
}

theory!(fill_only_hit_test:
    fill_only_hit_test_1(50.0, 50.0, true);
    fill_only_hit_test_2(50.0, 0.0, true);
    fill_only_hit_test_3(100.0, 50.0, true);
    fill_only_hit_test_4(50.0, 100.0, true);
    fill_only_hit_test_5(-1.0, 0.0, false);
    fill_only_hit_test_6(101.0, 0.0, false);
    fill_only_hit_test_7(101.0, 101.0, false);
    fill_only_hit_test_8(0.0, 101.0, false));

fn stroke_only_hit_test(x: f64, y: f64, inside: bool) {
    let pen = black_pen(2.0);

    let mut stream = RenderDataStream::new();
    stream.draw_ellipse(None, p(&pen), p(&pen), Rect::new(0.0, 0.0, 100.0, 100.0));

    assert_eq!(inside, stream.hit_test(Point::new(x, y)));
}

theory!(stroke_only_hit_test:
    stroke_only_hit_test_1(50.0, 0.0, true);
    stroke_only_hit_test_2(51.0, 0.0, true);
    stroke_only_hit_test_3(100.0, 50.0, true);
    stroke_only_hit_test_4(50.0, 100.0, true);
    stroke_only_hit_test_5(-1.0, 50.0, true);
    stroke_only_hit_test_6(53.0, 50.0, false);
    stroke_only_hit_test_7(101.0, 0.0, false);
    stroke_only_hit_test_8(101.0, 101.0, false);
    stroke_only_hit_test_9(0.0, 101.0, false));
