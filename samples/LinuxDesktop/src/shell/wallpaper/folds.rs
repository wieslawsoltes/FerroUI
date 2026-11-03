//! Original folded-paper artwork. Each fan shares a crease at the ground plane.
//! Draw the cubic sections back-to-front into one retained drawing per colorway.
use ferroui_base::media::immutable::{ImmutableGradientStop, ImmutableLinearGradientBrush};
use ferroui_base::media::{
    Color, Drawing, DrawingGroup, DrawingImage, Geometry, GradientSpreadMethod, GradientStop,
    IBrush, IImage, IPen, Pen, RadialGradientBrush, StreamGeometry,
};
use ferroui_base::platform::IGeometryContext;
use ferroui_base::{Point, Rect, RelativePoint, RelativeUnit};
use ferroui_view::prelude::*;
use std::rc::Rc;

struct Fold {
    start: [i16; 2],
    cubics: &'static [[i16; 6]],
    layers: i16,
    spread: [i16; 2],
    light_axis: [f64; 4],
    colors: &'static [(f64, u32)],
    rolled_edge: Option<&'static str>,
}

const FOLDS: &[Fold] = &[
    Fold {
        start: [798, 795],
        cubics: &[
            [708, 682, 498, 588, 448, 445],
            [420, 368, 520, 355, 511, 288],
            [500, 198, 525, 132, 594, 150],
            [662, 168, 661, 252, 740, 240],
            [821, 225, 828, 334, 843, 404],
            [891, 596, 935, 681, 883, 793],
        ],
        layers: 8,
        spread: [-16, -6],
        light_axis: [0.04, 0.18, 0.95, 0.82],
        colors: &[
            (0.0, 0xC5FAFF),
            (0.12, 0x78D6FF),
            (0.28, 0x2487EE),
            (0.43, 0x1739AC),
            (0.53, 0x0B208A),
            (0.68, 0x377DE9),
            (0.83, 0x61C8FF),
            (1.0, 0x1142BE),
        ],
        rolled_edge: None,
    },
    Fold {
        start: [761, 792],
        cubics: &[
            [734, 671, 638, 501, 674, 393],
            [703, 304, 782, 311, 793, 231],
            [810, 116, 855, 100, 909, 141],
            [947, 170, 913, 263, 992, 287],
            [1047, 304, 1007, 371, 990, 409],
            [935, 524, 908, 698, 869, 792],
        ],
        layers: 7,
        spread: [12, -5],
        light_axis: [0.04, 0.19, 0.98, 0.75],
        colors: &[
            (0.0, 0x6EBDFF),
            (0.12, 0xC7F6FF),
            (0.25, 0x307EE8),
            (0.44, 0x102282),
            (0.59, 0x2843BF),
            (0.8, 0x559FEE),
            (1.0, 0x102B9E),
        ],
        rolled_edge: None,
    },
    Fold {
        start: [764, 800],
        cubics: &[
            [643, 735, 464, 711, 375, 628],
            [309, 567, 328, 483, 388, 475],
            [434, 469, 450, 518, 493, 515],
            [558, 507, 535, 382, 635, 382],
            [721, 380, 714, 480, 748, 536],
            [794, 614, 860, 682, 879, 796],
        ],
        layers: 7,
        spread: [-8, -4],
        light_axis: [0.05, 0.05, 0.89, 0.94],
        colors: &[
            (0.0, 0xD7F5FF),
            (0.15, 0x69C9FF),
            (0.32, 0x2466DA),
            (0.51, 0x102B99),
            (0.68, 0x3279DF),
            (0.84, 0x61BAEF),
            (1.0, 0x133898),
        ],
        rolled_edge: Some(
            "M375 628
            C309 567 328 483 388 475
            C434 469 450 518 493 515
            C558 507 535 382 635 382
            C721 380 714 480 748 536
            C716 499 685 400 637 412
            C593 424 604 513 536 550
            C474 586 428 516 386 536
            C352 553 360 601 375 628 Z",
        ),
    },
    Fold {
        start: [741, 805],
        cubics: &[
            [711, 751, 715, 671, 776, 638],
            [851, 596, 899, 582, 927, 496],
            [959, 398, 1013, 325, 1090, 336],
            [1151, 345, 1159, 430, 1122, 485],
            [1085, 538, 1142, 551, 1179, 562],
            [1267, 588, 1182, 716, 1068, 750],
            [980, 778, 864, 801, 811, 804],
        ],
        layers: 7,
        spread: [14, 1],
        light_axis: [0.04, 0.12, 0.93, 0.83],
        colors: &[
            (0.0, 0x8FD8FF),
            (0.2, 0x294FC7),
            (0.36, 0x101B79),
            (0.5, 0x2059CC),
            (0.66, 0x51B8FF),
            (0.81, 0x8FE0FF),
            (1.0, 0x1544AD),
        ],
        rolled_edge: Some(
            "M776 638
            C851 596 899 582 927 496
            C959 398 1013 325 1090 336
            C1151 345 1159 430 1122 485
            C1139 425 1106 360 1053 389
            C993 421 1007 497 939 571
            C898 617 825 631 776 638 Z",
        ),
    },
    Fold {
        start: [647, 767],
        cubics: &[
            [603, 732, 610, 698, 649, 680],
            [679, 667, 707, 683, 729, 668],
            [770, 638, 735, 558, 801, 530],
            [869, 498, 908, 558, 907, 619],
            [906, 673, 992, 668, 1023, 704],
            [1061, 747, 972, 790, 888, 800],
            [787, 813, 696, 807, 647, 767],
        ],
        layers: 7,
        spread: [-10, 2],
        light_axis: [0.1, 0.0, 0.67, 0.99],
        colors: &[
            (0.0, 0xBCF3FF),
            (0.18, 0x63C5FF),
            (0.34, 0x2973EA),
            (0.51, 0x193AB1),
            (0.72, 0x317CE5),
            (0.88, 0x79C9FF),
            (1.0, 0x16399C),
        ],
        rolled_edge: Some(
            "M647 767
            C603 732 610 698 649 680
            C679 667 707 683 729 668
            C770 638 735 558 801 530
            C869 498 908 558 907 619
            C893 571 862 548 832 567
            C792 593 815 652 765 692
            C722 729 673 695 650 719
            C639 730 635 751 647 767 Z",
        ),
    },
];

fn paint(axis: [f64; 4], colors: &[(f64, u32)], exposure: f64) -> Rc<dyn IBrush> {
    let stops: Vec<_> = colors
        .iter()
        .map(|&(offset, rgb)| {
            let color = Color::from_uint32(rgb | 0xff00_0000);
            let shade = |channel| (f64::from(channel) * exposure).round() as u8;
            ImmutableGradientStop::new(
                offset,
                Color::from_rgb(shade(color.r), shade(color.g), shade(color.b)),
            )
        })
        .collect();
    let [x1, y1, x2, y2] = axis;
    Rc::new(ImmutableLinearGradientBrush::new(
        &stops,
        1.0,
        None,
        None,
        GradientSpreadMethod::Pad,
        Some(RelativePoint::new(x1, y1, RelativeUnit::Relative)),
        Some(RelativePoint::new(x2, y2, RelativeUnit::Relative)),
        None,
    ))
}

impl Fold {
    fn section(&self, depth: i16) -> ferroui_base::Ref<Geometry> {
        // The crown opens while the base stays in contact with the floor.
        let point = |x: i16, y: i16| {
            let spread = ((812.0 - f64::from(y)) / 420.0).clamp(0.06, 1.3) * f64::from(depth);
            Point::new(
                f64::from(x) + spread * f64::from(self.spread[0]),
                f64::from(y) + spread * f64::from(self.spread[1]),
            )
        };
        let shape = StreamGeometry::new();
        {
            let mut path = shape.open();
            path.begin_figure(point(self.start[0], self.start[1]), true);
            for &[x1, y1, x2, y2, x, y] in self.cubics {
                path.cubic_bezier_to(point(x1, y1), point(x2, y2), point(x, y), true);
            }
            path.end_figure(true);
        }
        shape.upcast()
    }
}

fn artwork(dark: bool) -> Rc<dyn IImage> {
    const DAY: &[(f64, u32)] = &[
        (0.0, 0xEDF7FF),
        (0.44, 0xC5E6FC),
        (0.74, 0x9DC8F2),
        (1.0, 0x6C9ED9),
    ];
    const NIGHT: &[(f64, u32)] = &[(0.0, 0x050D24), (0.55, 0x0E2241), (1.0, 0x132D50)];
    const LIP: &[(f64, u32)] = &[
        (0.0, 0x235DBA),
        (0.18, 0x4BA9FA),
        (0.32, 0x94E2FF),
        (0.43, 0x367DD1),
        (0.53, 0x16298A),
        (0.77, 0x3170C6),
        (1.0, 0x5DB8EF),
    ];
    let sheet = DrawingGroup::new();
    {
        let mut dc = sheet.open();
        let sky = paint([0.12, 0.0, 0.88, 1.0], if dark { NIGHT } else { DAY }, 1.0);
        dc.fill_rectangle(&sky, Rect::new(0.0, 0.0, 1440.0, 900.0), 0.0);
        let contact = RadialGradientBrush::new();
        for (offset, argb) in [(0.0, 0x7D05316C), (0.42, 0x3D0D4993), (1.0, 0x00227DC8)] {
            contact
                .gradient_stops()
                .add(GradientStop::with_color_and_offset(
                    Color::from_uint32(argb),
                    offset,
                ));
        }
        let contact: Rc<dyn IBrush> = contact.into();
        dc.draw_ellipse(Some(&contact), None, Rect::new(328.0, 757.0, 912.0, 86.0));
        dc.draw_ellipse(Some(&contact), None, Rect::new(606.0, 788.0, 450.0, 28.0));

        let light = if dark { 0.77 } else { 0.94 };
        let edge: Rc<dyn IPen> =
            Pen::from_uint32(if dark { 0x9969A4CB } else { 0x998EDAFF }, 1.4).into();
        let lip = paint([0.0, 0.2, 1.0, 0.67], LIP, 1.0);
        for fold in FOLDS {
            let face = paint(fold.light_axis, fold.colors, light + 0.035);
            let shadow = paint(fold.light_axis, fold.colors, light * 0.78);
            let reflected = paint(fold.light_axis, fold.colors, light * 1.01);
            for depth in (0..fold.layers).rev() {
                let brush = if depth == 0 {
                    &face
                } else if depth % 2 == 0 {
                    &reflected
                } else {
                    &shadow
                };
                dc.draw_geometry(Some(brush), Some(&edge), &fold.section(depth));
            }
            if let Some(contour) = fold.rolled_edge {
                dc.draw_geometry(
                    Some(&lip),
                    None,
                    &Geometry::parse(contour).expect("rolled edge geometry"),
                );
            }
        }
    }
    let artwork = DrawingImage::new();
    artwork.set_drawing(sheet.upcast::<Drawing>());
    artwork.into()
}

pub(super) fn wallpaper(dark: bool) -> Element {
    // Drawing data is independent of the output size and the WinUI density.
    // A monitor hotplug or profile round-trip can share the same immutable scene.
    thread_local! {
        static CACHE: [std::cell::OnceCell<Rc<dyn IImage>>; 2] = Default::default();
    }
    let source = CACHE.with(|cache| {
        cache[usize::from(dark)]
            .get_or_init(|| artwork(dark))
            .clone()
    });
    image().width(1440.0).height(900.0).source(source).into()
}
