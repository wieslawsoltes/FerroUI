//! Resolution-independent wallpaper. The curves and gradient stops are created once at
//! mount; the compositor retains the drawing while shell controls change independently.
mod folds;

use super::appearance::{Appearance, ColorMode, Wallpaper};
use ferroui_base::media::{Color, Geometry, GradientStop, IBrush, LinearGradientBrush, Stretch};
use ferroui_controls::Canvas;
use ferroui_view::prelude::*;
use std::fmt::Write;
use std::rc::Rc;

fn gradient(look: Appearance, stops: &[(f64, &str)]) -> Rc<dyn IBrush> {
    let brush = LinearGradientBrush::new();
    for &(offset, value) in stops {
        let tint = Color::parse(value).expect("wallpaper color");
        let color = look.tint(tint);
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

/// Low sun and atmospheric perspective across a coastal inlet. All ridges are closed
/// silhouettes; the water reflects the sunset between the two nearer headlands.
fn horizon(look: Appearance) -> El<Canvas> {
    canvas()
        .width(1440.0)
        .height(900.0)
        .background(gradient(
            look,
            &[
                (0.0, "#413354"),
                (0.45, "#BC7590"),
                (0.8, "#F1B29F"),
                (1.0, "#FFCF9F"),
            ],
        ))
        .child(shape(
            look,
            "M1048 202 A124 124 0 1 1 1048 450 A124 124 0 1 1 1048 202 Z",
            &[(0.0, "#FFF1C6"), (1.0, "#FFD893")],
        ))
        .child(shape(
            look,
            "M0 512 C139 447 199 475 300 430 S455 384 551 416
            S725 470 828 421 S999 332 1095 356 S1310 434 1440 358 V900 H0 Z",
            &[(0.0, "#A46A89"), (0.75, "#CB8AA0"), (1.0, "#D593A3")],
        ))
        .child(shape(
            look,
            "M0 535 C152 507 267 468 364 485 S506 534 613 527
            C794 516 871 435 955 428 C1053 420 1125 465 1196 461 S1365 402 1440 394 V900 H0 Z",
            &[(0.0, "#846682"), (1.0, "#B57D9D")],
        ))
        .child(shape(
            look,
            "M0 537 H1440 V900 H0 Z",
            &[
                (0.0, "#DE939F"),
                (0.49, "#FFC39F"),
                (0.83, "#CA8CA4"),
                (1.0, "#677390"),
            ],
        ))
        .child(shape(
            look,
            "M0 398 C87 410 149 467 249 497 S478 541 605 552
            C511 564 434 569 392 584 C478 584 539 590 574 601 C411 614 313 632 176 667 L0 715 Z",
            &[(0.0, "#483B63"), (0.72, "#6F5478"), (1.0, "#8E6589")],
        ))
        .child(shape(
            look,
            "M1440 445 C1356 454 1304 497 1238 517 C1177 537 1073 542 995 552
            C1066 561 1194 566 1248 583 C1161 591 1119 598 1093 607 L1440 633 Z",
            &[(0.0, "#8F668A"), (1.0, "#654F73")],
        ))
        .child(shape(
            look,
            "M1440 596 C1241 599 1095 645 900 656 S624 678 543 700
            C620 705 683 714 707 721 C554 728 408 767 306 794 L0 883 V900 H1440 Z",
            &[(0.0, "#665878"), (1.0, "#353A57")],
        ))
        .child(shape(
            look,
            "M0 601 C118 608 226 683 361 707 C425 721 463 735 515 752
            C392 776 315 828 244 900 H0 Z",
            &[(0.0, "#312E49"), (1.0, "#4B4566")],
        ))
        .child(shape(
            look,
            "M1440 686 C1252 712 1157 777 1029 815 C946 839 881 866 814 900 H1440 Z",
            &[(0.0, "#444360"), (1.0, "#25273D")],
        ))
}

/// Paper-cut flowers and leaves. The same sprig on two scales gives the composition
/// rhythm without a texture, an animation or a full-screen CPU bitmap.
fn garden(look: Appearance) -> El<Canvas> {
    let mut petals = String::new();
    for i in 0..12 {
        use std::f64::consts::TAU;
        let a = TAU * i as f64 / 12.0;
        let point = |r: f64, angle: f64| (1008.0 + r * angle.cos(), 332.0 + r * angle.sin());
        let (ax, ay) = point(195.0, a - TAU / 24.0);
        let (bx, by) = point(325.0, a - TAU / 31.0);
        let (cx, cy) = point(325.0, a + TAU / 31.0);
        let (dx, dy) = point(195.0, a + TAU / 24.0);
        if i == 0 {
            write!(petals, "M{ax:.2} {ay:.2} ").expect("string write");
        }
        write!(petals, "C{bx:.2} {by:.2} {cx:.2} {cy:.2} {dx:.2} {dy:.2} ").expect("string write");
    }
    petals.push('Z');
    canvas()
        .width(1440.0)
        .height(900.0)
        .child(shape(
            look,
            "M0 642 C167 477 293 556 398 665 C482 755 566 801 690 801 V900 H0 Z",
            &[(0.0, "#7C9E7B"), (1.0, "#809C71")],
        ))
        .child(shape(
            look,
            "M0 785 C163 588 317 612 423 775 L522 900 H0 Z",
            &[(0.0, "#B8CFA4"), (1.0, "#9FB78D")],
        ))
        .child(shape(
            look,
            "M970 900 C1015 728 1128 564 1454 501 C1453 698 1307 888 1093 900 Z",
            &[(0.0, "#AAC4AC"), (1.0, "#5C957D")],
        ))
        .child(shape(
            look,
            "M946 900 C907 678 838 613 682 552 C676 683 760 853 901 900 Z",
            &[(0.0, "#A5D1AF"), (1.0, "#6CAC88")],
        ))
        .child(shape(
            look,
            "M918 900 C987 713 1009 513 1001 380 L1021 379 C1033 545 1017 733 953 900 Z",
            &[(0.0, "#547963"), (1.0, "#24483D")],
        ))
        .child(shape(
            look,
            &petals,
            &[(0.0, "#F6DAAA"), (0.55, "#ECD7A7"), (1.0, "#D0BB89")],
        ))
        .child(shape(
            look,
            "M1008 242 A90 90 0 1 1 1008 422 A90 90 0 1 1 1008 242 Z",
            &[(0.0, "#B66350"), (1.0, "#8E523F")],
        ))
        .child(shape(
            look,
            "M116 900 C200 740 312 717 452 733 C445 818 387 875 328 900 Z",
            &[(0.0, "#436953"), (1.0, "#345A48")],
        ))
}

/// Keep the flower's center at 70% of the viewport width, the same fraction
/// it occupies on the design canvas. On narrow outputs cap the flower's
/// diameter and root the leaves at the bottom, leaving clear space for the
/// clock. Only the transform changes during output resize.
fn garden_viewport(look: Appearance, composed: bool) -> Element {
    use ferroui_base::media::MatrixTransform;
    use ferroui_base::{Matrix, RelativePoint};
    let transform = MatrixTransform::new();
    let drawing = garden(look).with({
        let transform = transform.clone();
        move |drawing| {
            drawing.set_render_transform_origin(RelativePoint::TOP_LEFT);
            drawing.set_render_transform(Some(transform.clone().into()));
        }
    });
    canvas()
        .background(gradient(
            look,
            &[(0.0, "#233F36"), (0.65, "#345847"), (1.0, "#385449")],
        ))
        .clip_to_bounds(true)
        .is_hit_test_visible(false)
        .with(move |viewport| cache_composed(viewport, composed))
        .on_size_changed(move |event| {
            let size = event.new_size();
            let cover = (size.width / 1440.0).max(size.height / 900.0);
            let scale = cover.min(size.width / 800.0);
            let offset_x = (size.width - 1440.0 * scale).min(0.0) * (1008.0 / 1440.0);
            let offset_y = (size.height - 900.0 * scale).max(0.0);
            transform.set_matrix(Matrix::new(scale, 0.0, 0.0, scale, offset_x, offset_y));
        })
        .child(drawing)
        .into()
}

fn cache_composed(host: &ferroui_base::Visual, composed: bool) {
    // The private compositor repaints clients and live backdrops over this
    // visual. A delegated Wayland wallpaper already has its own surface.
    if composed {
        use ferroui_base::media::{BitmapCache, CacheMode};
        host.set_cache_mode(BitmapCache::new().upcast::<CacheMode>());
    }
}

pub(super) fn wallpaper(look: Appearance, composed: bool) -> Element {
    if look
        .winui()
        .is_some_and(|s| s.mode == ColorMode::HighContrast)
    {
        return border()
            .background(super::chrome::resource("SolidBackgroundFillColorBaseBrush"))
            .is_hit_test_visible(false)
            .into();
    }
    let drawing = match look.wallpaper() {
        Wallpaper::Fold => {
            folds::wallpaper(look.winui().is_some_and(|s| s.mode == ColorMode::Dark))
        }
        Wallpaper::Horizon => horizon(look).into_element(),
        Wallpaper::Garden => return garden_viewport(look, composed),
    };
    viewbox()
        .with(move |host| cache_composed(host, composed))
        .stretch(Stretch::UniformToFill)
        .clip_to_bounds(true)
        .is_hit_test_visible(false)
        .set(ferroui_controls::Viewbox::child_property(), drawing)
        .into()
}
