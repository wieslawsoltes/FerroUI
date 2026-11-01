//! Resolution-independent wallpaper. The curves and gradient stops are created once at
//! mount; the compositor retains the drawing while shell controls change independently.
use super::appearance::{Appearance, Layout};
use ferroui_base::media::{Color, Geometry, GradientStop, IBrush, LinearGradientBrush, Stretch};
use ferroui_view::prelude::*;
use std::fmt::Write;
use std::rc::Rc;

fn gradient(look: Appearance, stops: &[(f64, &str)]) -> Rc<dyn IBrush> {
    let brush = LinearGradientBrush::new();
    for &(offset, value) in stops {
        let tint = Color::parse(value).expect("wallpaper color");
        let color = match look.layout() {
            Layout::Taskbar => {
                Color::from_rgb(tint.r.saturating_sub(12), tint.b, tint.g.saturating_add(49))
            }
            Layout::MenuBarDock => {
                Color::from_rgb(tint.g.saturating_add(30), tint.r, tint.b.saturating_add(24))
            }
            Layout::BottomNavigation => tint,
        };
        brush
            .gradient_stops()
            .add(GradientStop::with_color_and_offset(color, offset));
    }
    brush.into()
}

fn shape(look: Appearance, data: &str, stops: &[(f64, &str)]) -> Element {
    path()
        .data(Geometry::parse(data).expect("wallpaper geometry"))
        .fill(gradient(look, stops))
        .into()
}

pub(super) fn wallpaper(look: Appearance) -> Element {
    // An incoming sheet folds around the upper-right crest and returns across the foreground.
    // Each face shares its edge with the next, so the silhouette stays sharp at any DPI.
    let back = "M0 578 C265 633 489 484 637 279 C754 115 790 19 923 0 H1440 V900 H0 Z";
    let crest = "M0 687 C339 713 568 536 736 333 C914 118 1048 54 1195 99
        C1324 138 1352 252 1299 334 C1294 230 1222 166 1117 187
        C963 218 873 419 676 579 C469 748 254 786 0 763 Z";
    let fold = "M0 763 C254 786 469 748 676 579 C873 419 963 218 1117 187
        C1222 166 1294 230 1299 334 C1267 416 1202 487 1117 540
        C1186 456 1200 378 1140 343 C1063 298 967 388 851 501
        C623 737 330 849 0 832 Z";
    let shadow = "M0 832 C330 849 623 737 851 501 C967 388 1063 298 1140 343
        C1200 378 1186 456 1117 540 C1290 447 1394 359 1440 247
        V900 H0 Z";
    let near = "M239 900 C577 859 855 725 1077 598 C1239 504 1341 445 1440 421
        V900 Z";
    let mut contours = String::new();
    for i in 0..17 {
        let s = f64::from(i);
        write!(
            contours,
            "M{} 938 C{} {} {} {} 1465 {} ",
            193.0 + 27.0 * s,
            650.0 + 16.0 * s,
            836.0 + 4.0 * s,
            1100.0 + 5.0 * s,
            533.0 + 6.0 * s,
            422.0 + 11.5 * s,
        )
        .expect("write to string");
    }
    let drawing = canvas()
        .width(1440.0)
        .height(900.0)
        .background(gradient(
            look,
            &[(0.0, "#0B1927"), (0.68, "#173A45"), (1.0, "#204F58")],
        ))
        .child(shape(
            look,
            back,
            &[(0.0, "#163B47"), (0.52, "#30676D"), (1.0, "#122333")],
        ))
        .child(shape(
            look,
            crest,
            &[
                (0.0, "#237776"),
                (0.35, "#53B6A8"),
                (0.62, "#D8F4D7"),
                (0.79, "#7FD8BF"),
                (1.0, "#497C83"),
            ],
        ))
        .child(shape(
            look,
            fold,
            &[
                (0.0, "#07504F"),
                (0.5, "#257F7E"),
                (0.78, "#366B74"),
                (1.0, "#172F43"),
            ],
        ))
        .child(shape(
            look,
            shadow,
            &[(0.0, "#132734"), (0.62, "#0B202C"), (1.0, "#22545C")],
        ))
        .child(shape(
            look,
            near,
            &[(0.0, "#193A45"), (0.68, "#224C56"), (1.0, "#0D2534")],
        ))
        .child(
            path()
                .data(Geometry::parse(&contours).expect("contour geometry"))
                .stroke("#304C7376")
                .stroke_thickness(1.0),
        );
    viewbox()
        .stretch(Stretch::UniformToFill)
        .clip_to_bounds(true)
        .is_hit_test_visible(false)
        .child(drawing)
        .into()
}
