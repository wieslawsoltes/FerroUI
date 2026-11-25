//! A technical product plate exported through FerroUI's drawing contracts.
//!
//! Run from the workspace root:
//! `cargo run -p ferroui-svg --example layered_export -- samples/SvgExport/output`
//!
//! Add `--benchmark` to measure 100 exports of the same recorded document. Use `--release`
//! on the Cargo command for performance measurements; the program reports its build profile.
//! `--reference` also exports a Skia raster replay for independent pixel comparisons.
//! The drawing uses system fonts at recording time; the exported text is outlined by default.

use ferroui_base::media::immutable::{
    ImmutableGradientStop, ImmutableLinearGradientBrush, ImmutablePen, ImmutableSolidColorBrush,
};
use ferroui_base::media::text_formatting::GlyphInfo;
use ferroui_base::media::{BoxShadows, Color, IBrush};
use ferroui_base::platform::{IDrawingContextImpl, IGeometryContext, IStreamGeometryBuilder};
use ferroui_base::{Point, Rect, RoundedRect, Size, Vector};
use ferroui_skia::{GlyphRunImpl, SkiaPlatform, StreamGeometryBuilder};
use ferroui_svg::{
    SvgCutaway, SvgCutawayMode, SvgDocument, SvgDrawingContext, SvgExplodedView, SvgExportOptions,
    SvgFidelity, SvgInspection, SvgLayerFilter, SvgLayerOptions, SvgView,
};
use skia_safe::{Font, FontMgr, FontStyle};
use std::error::Error;
use std::path::PathBuf;
use std::time::Instant;

const INK: u32 = 0xff193047;
const MUTED: u32 = 0xff667e90;
const RULE: u32 = 0xffcbd7e0;
const PAPER: u32 = 0xfff1f5f8;
const CYAN: u32 = 0xff37ded1;
const DEVICE: Rect = Rect::new(140.0, 330.0, 650.0, 350.0);

struct Fonts {
    regular: Font,
    bold: Font,
    mono: Font,
}

impl Fonts {
    fn new() -> Result<Self, Box<dyn Error>> {
        let manager = FontMgr::new();
        let make = |families: &[&str], style| -> Result<Font, Box<dyn Error>> {
            families
                .iter()
                .find_map(|family| manager.match_family_style(family, style))
                .or_else(|| manager.legacy_make_typeface(None, style))
                .map(|face| Font::new(face, 16.0))
                .ok_or_else(|| {
                    "Install a system font such as DejaVu Sans to run the example".into()
                })
        };
        Ok(Self {
            regular: make(&["Inter", "DejaVu Sans", "Arial"], FontStyle::normal())?,
            bold: make(&["Inter", "DejaVu Sans", "Arial"], FontStyle::bold())?,
            mono: make(
                &["DejaVu Sans Mono", "Menlo", "Consolas"],
                FontStyle::normal(),
            )?,
        })
    }
}

fn color(argb: u32) -> Color {
    Color::from_uint32(argb)
}

fn solid(argb: u32) -> ImmutableSolidColorBrush {
    ImmutableSolidColorBrush::from_uint32(argb)
}

fn gradient(first: u32, last: u32) -> ImmutableLinearGradientBrush {
    ImmutableLinearGradientBrush::from_stops(&[
        ImmutableGradientStop::new(0.0, color(first)),
        ImmutableGradientStop::new(1.0, color(last)),
    ])
}

fn panel(
    ctx: &mut SvgDrawingContext,
    rect: Rect,
    radius: f64,
    fill: &dyn IBrush,
    stroke: Option<u32>,
) {
    let pen = stroke.map(|value| ImmutablePen::from_uint32(value, 1.0));
    ctx.draw_rectangle(
        Some(fill),
        pen.as_ref().map(|p| p as &dyn ferroui_base::media::IPen),
        RoundedRect::from_radius(rect, radius),
        &BoxShadows::default(),
    );
}

fn line(ctx: &mut SvgDrawingContext, start: (f64, f64), end: (f64, f64), argb: u32, width: f64) {
    ctx.draw_line(
        Some(&ImmutablePen::from_uint32(argb, width)),
        Point::new(start.0, start.1),
        Point::new(end.0, end.1),
    );
}

fn dot(ctx: &mut SvgDrawingContext, x: f64, y: f64, radius: f64, fill: u32, stroke: Option<u32>) {
    let pen = stroke.map(|value| ImmutablePen::from_uint32(value, 1.0));
    ctx.draw_ellipse(
        Some(&solid(fill)),
        pen.as_ref().map(|p| p as &dyn ferroui_base::media::IPen),
        Rect::new(x - radius, y - radius, radius * 2.0, radius * 2.0),
    );
}

fn route(ctx: &mut SvgDrawingContext, points: &[(f64, f64)], argb: u32, width: f64) {
    if let Some(first) = points.first() {
        let mut path = StreamGeometryBuilder::new(None);
        path.begin_figure(Point::new(first.0, first.1), false);
        for point in &points[1..] {
            path.line_to(Point::new(point.0, point.1), true);
        }
        path.end_figure(false);
        let geometry = path.to_geometry();
        ctx.draw_geometry(
            None,
            Some(&ImmutablePen::from_uint32(argb, width)),
            &*geometry,
        );
    }
}

/// The sample's labels are ASCII, so a direct glyph mapping suffices here. Applications should
/// retain their normal FerroUI/HarfBuzz layout for shaped scripts, ligatures and bidirectional text.
#[allow(clippy::too_many_arguments)]
fn label(
    ctx: &mut SvgDrawingContext,
    font: &Font,
    text: &str,
    x: f64,
    y: f64,
    size: f64,
    argb: u32,
    tracking: f64,
) {
    let mut font = font.clone();
    font.set_size(size as f32);
    let indices = font.str_to_glyphs_vec(text);
    let mut advances = vec![0.0; indices.len()];
    font.get_widths(&indices, &mut advances);
    let glyphs: Vec<_> = indices
        .iter()
        .zip(advances)
        .enumerate()
        .map(|(index, (&glyph, advance))| {
            GlyphInfo::new(glyph, index as i32, f64::from(advance) + tracking)
        })
        .collect();
    let run = GlyphRunImpl::from_font(&font, size, &glyphs, Point::new(x, y));
    ctx.draw_glyph_run(Some(&solid(argb)), &run);
}

fn screw(ctx: &mut SvgDrawingContext, x: f64, y: f64) {
    dot(ctx, x, y, 9.0, 0xff203141, Some(0xff8193a3));
    dot(ctx, x, y, 5.0, 0xff536779, Some(0xff243848));
    line(ctx, (x - 3.0, y + 3.0), (x + 3.0, y - 3.0), 0xff0b1b28, 1.4);
}

fn annotation(
    ctx: &mut SvgDrawingContext,
    fonts: &Fonts,
    index: &str,
    y: f64,
    name: &str,
    details: [&str; 2],
    swatch: u32,
) {
    dot(ctx, 976.0, y - 7.0, 17.0, 0xffe4edf3, None);
    label(ctx, &fonts.mono, index, 967.0, y - 2.0, 11.0, INK, 0.0);
    label(ctx, &fonts.bold, name, 1006.0, y, 13.0, INK, 0.1);
    label(
        ctx,
        &fonts.regular,
        details[0],
        1006.0,
        y + 26.0,
        11.0,
        MUTED,
        0.0,
    );
    label(
        ctx,
        &fonts.regular,
        details[1],
        1006.0,
        y + 44.0,
        11.0,
        MUTED,
        0.0,
    );
    panel(
        ctx,
        Rect::new(1006.0, y + 62.0, 36.0, 4.0),
        2.0,
        &solid(swatch),
        None,
    );
}

fn record_document(fonts: &Fonts) -> Result<SvgDocument, Box<dyn Error>> {
    let mut doc = SvgDocument::new(Size::new(1280.0, 880.0))?;

    doc.record_layer(
        SvgLayerOptions::new("plate", "Engineering plate").with_tags(["annotation"]),
        |ctx| {
            label(
                ctx,
                &fonts.bold,
                "FERROUI / VECTOR SYSTEMS",
                64.0,
                63.0,
                11.0,
                MUTED,
                1.7,
            );
            label(
                ctx,
                &fonts.bold,
                "Inside the interface.",
                64.0,
                125.0,
                43.0,
                INK,
                -0.5,
            );
            label(
                ctx,
                &fonts.mono,
                "ENGINEERING PLATE / 001",
                964.0,
                65.0,
                10.0,
                MUTED,
                0.1,
            );
            label(ctx, &fonts.bold, "AERIS M1", 1060.0, 106.0, 23.0, INK, 0.2);
            label(
                ctx,
                &fonts.mono,
                "VECTOR ASSEMBLY / REV. A",
                964.0,
                129.0,
                10.0,
                MUTED,
                0.0,
            );
            line(ctx, (64.0, 163.0), (1216.0, 163.0), RULE, 1.0);
            label(
                ctx,
                &fonts.bold,
                "CONTROL MODULE",
                64.0,
                214.0,
                13.0,
                INK,
                1.5,
            );
            label(
                ctx,
                &fonts.regular,
                "One recorded scene. Five ways to inspect it.",
                64.0,
                241.0,
                13.0,
                MUTED,
                0.0,
            );
            line(ctx, (938.0, 208.0), (938.0, 744.0), RULE, 1.0);

            annotation(
                ctx,
                fonts,
                "01",
                278.0,
                "CHASSIS",
                ["Anodized aluminum", "Mounts / ports / thermal vents"],
                0xff52697e,
            );
            annotation(
                ctx,
                fonts,
                "02",
                400.0,
                "LOGIC BOARD",
                ["Routed copper traces", "Processor / memory / power"],
                0xff278575,
            );
            annotation(
                ctx,
                fonts,
                "03",
                522.0,
                "DISPLAY",
                [
                    "Live telemetry interface",
                    "Clipped paths / outlined glyphs",
                ],
                CYAN,
            );
            annotation(
                ctx,
                fonts,
                "04",
                644.0,
                "COVER GLASS",
                ["Optical surface and bezel", "Sectionable outer shell"],
                0xffa7bccb,
            );

            // Registration marks, dimension lines and scale are ordinary vector geometry.
            for (x, y) in [
                (106.0, 296.0),
                (824.0, 296.0),
                (106.0, 714.0),
                (824.0, 714.0),
            ] {
                line(ctx, (x - 8.0, y), (x + 8.0, y), RULE, 1.0);
                line(ctx, (x, y - 8.0), (x, y + 8.0), RULE, 1.0);
            }
            line(ctx, (140.0, 729.0), (790.0, 729.0), 0xffa0b2bf, 0.8);
            for x in [140.0, 790.0] {
                line(ctx, (x, 722.0), (x, 736.0), MUTED, 1.0);
            }
            label(
                ctx,
                &fonts.mono,
                "650 LOGICAL PX",
                407.0,
                752.0,
                10.0,
                MUTED,
                0.5,
            );
            line(ctx, (64.0, 790.0), (1216.0, 790.0), RULE, 1.0);
            label(ctx, &fonts.bold, "RECORD ONCE", 64.0, 821.0, 11.0, INK, 0.8);
            label(
                ctx,
                &fonts.regular,
                "Reuse geometry, resources and text across export views.",
                64.0,
                843.0,
                11.0,
                MUTED,
                0.0,
            );
            label(
                ctx,
                &fonts.mono,
                "NORMAL / X-RAY / SECTION / EXPLODED / WIREFRAME",
                760.0,
                825.0,
                10.0,
                MUTED,
                0.0,
            );
            label(
                ctx,
                &fonts.regular,
                "Named layers. Portable outlines. No external resources.",
                826.0,
                845.0,
                11.0,
                MUTED,
                0.0,
            );
        },
    )?;

    doc.record_layer(
        SvgLayerOptions::new("chassis", "01 Aluminum chassis")
            .with_depth(0.0)
            .with_bounds(Rect::new(138.0, 328.0, 654.0, 361.0))
            .with_tags(["assembly", "structure"]),
        |ctx| {
            panel(
                ctx,
                DEVICE,
                32.0,
                &gradient(0xff536a80, 0xff122739),
                Some(0xff667e91),
            );
            panel(
                ctx,
                Rect::new(151.0, 341.0, 628.0, 328.0),
                24.0,
                &gradient(0xff1c3245, 0xff314d62),
                Some(0xff101f2e),
            );
            for y in [356.0, 654.0] {
                for x in [166.0, 764.0] {
                    screw(ctx, x, y);
                }
            }
            for index in 0..16 {
                let x = 238.0 + f64::from(index) * 27.0;
                panel(
                    ctx,
                    Rect::new(x, 660.0, 15.0, 5.0),
                    2.0,
                    &solid(0xff071925),
                    None,
                );
            }
            // Connector housings extend below the chassis and remain visible in exploded exports.
            for (x, width) in [(220.0, 72.0), (335.0, 42.0), (420.0, 86.0)] {
                panel(
                    ctx,
                    Rect::new(x, 677.0, width, 10.0),
                    3.0,
                    &solid(0xff263d50),
                    Some(0xff7b919f),
                );
                panel(
                    ctx,
                    Rect::new(x + 6.0, 678.0, width - 12.0, 5.0),
                    1.0,
                    &solid(0xff0b1925),
                    None,
                );
            }
        },
    )?;

    doc.record_layer(
        SvgLayerOptions::new("logic-board", "02 Logic board")
            .with_depth(1.0)
            .with_bounds(Rect::new(178.0, 360.0, 574.0, 290.0))
            .with_tags(["assembly", "electronics"]),
        |ctx| {
            panel(
                ctx,
                Rect::new(180.0, 362.0, 570.0, 286.0),
                15.0,
                &gradient(0xff235b53, 0xff123d39),
                Some(0xff568879),
            );
            let copper = 0xff81b792;
            for row in 0..12 {
                let y = 386.0 + f64::from(row) * 20.0;
                let elbow = 286.0 + f64::from(row % 4) * 12.0;
                route(
                    ctx,
                    &[
                        (194.0, y),
                        (elbow, y),
                        (elbow + 24.0, y + 12.0),
                        (716.0, y + 12.0),
                    ],
                    copper,
                    1.2,
                );
                dot(ctx, 196.0, y, 2.8, 0xffd5bb75, Some(0xff466c5b));
                dot(ctx, 716.0, y + 12.0, 2.8, 0xffd5bb75, Some(0xff466c5b));
            }
            for (x, y, width, height) in [
                (354.0, 437.0, 124.0, 114.0),
                (536.0, 400.0, 68.0, 112.0),
                (535.0, 548.0, 98.0, 44.0),
            ] {
                for pin in 0..8 {
                    let py = y + 8.0 + f64::from(pin) * ((height - 16.0) / 7.0);
                    line(ctx, (x - 8.0, py), (x + width + 8.0, py), 0xffc0b88f, 3.5);
                }
                panel(
                    ctx,
                    Rect::new(x, y, width, height),
                    4.0,
                    &gradient(0xff344346, 0xff15272b),
                    Some(0xff6c8580),
                );
                dot(ctx, x + 9.0, y + 9.0, 2.0, 0xff9ab4a8, None);
            }
            label(
                ctx,
                &fonts.mono,
                "FERRO",
                375.0,
                480.0,
                15.0,
                0xffb9cbc1,
                1.5,
            );
            label(
                ctx,
                &fonts.mono,
                "M1 / 64",
                382.0,
                505.0,
                10.0,
                0xff819e90,
                0.4,
            );
            for col in 0..6 {
                let x = 216.0 + f64::from(col) * 20.0;
                panel(
                    ctx,
                    Rect::new(x, 580.0, 10.0, 24.0),
                    2.0,
                    &solid(0xffc7c19e),
                    Some(0xff5d7661),
                );
            }
            for (x, y) in [
                (196.0, 378.0),
                (734.0, 378.0),
                (196.0, 632.0),
                (734.0, 632.0),
            ] {
                dot(ctx, x, y, 5.0, 0xffc5b279, Some(0xff132e2e));
                dot(ctx, x, y, 2.2, 0xff122d30, None);
            }
        },
    )?;

    doc.record_layer(
        SvgLayerOptions::new("display", "03 Telemetry display")
            .with_depth(2.0)
            .with_bounds(Rect::new(215.0, 387.0, 500.0, 236.0))
            .with_tags(["assembly", "shell", "display"]),
        |ctx| {
            panel(
                ctx,
                Rect::new(217.0, 389.0, 496.0, 232.0),
                16.0,
                &gradient(0xff122d40, 0xff071c2b),
                Some(0xff406678),
            );
            ctx.push_clip_rounded(RoundedRect::from_radius(
                Rect::new(223.0, 395.0, 484.0, 220.0),
                12.0,
            ));
            label(
                ctx,
                &fonts.bold,
                "AERIS",
                244.0,
                420.0,
                12.0,
                0xffe8f8fa,
                1.6,
            );
            label(
                ctx,
                &fonts.mono,
                "SYSTEM NOMINAL",
                557.0,
                420.0,
                9.0,
                CYAN,
                0.5,
            );
            dot(ctx, 543.0, 416.0, 3.0, CYAN, None);
            line(ctx, (244.0, 435.0), (685.0, 435.0), 0xff254657, 1.0);

            ctx.draw_ellipse(
                None,
                Some(&ImmutablePen::from_uint32(0xff1c4b5a, 9.0)),
                Rect::new(253.0, 455.0, 112.0, 112.0),
            );
            let arc: Vec<_> = (0..=48)
                .map(|i| {
                    let angle = (135.0 + f64::from(i) * 5.0).to_radians();
                    (309.0 + 56.0 * angle.cos(), 511.0 + 56.0 * angle.sin())
                })
                .collect();
            route(ctx, &arc, CYAN, 9.0);
            label(
                ctx,
                &fonts.bold,
                "72.4",
                274.0,
                516.0,
                29.0,
                0xffedfcff,
                -0.5,
            );
            label(
                ctx,
                &fonts.mono,
                "EFFICIENCY",
                276.0,
                538.0,
                8.0,
                0xff8eafb9,
                0.5,
            );
            label(
                ctx,
                &fonts.regular,
                "Operational envelope",
                397.0,
                462.0,
                11.0,
                0xffd2e9ef,
                0.0,
            );
            for row in 0..4 {
                let y = 482.0 + f64::from(row) * 23.0;
                line(ctx, (399.0, y), (682.0, y), 0xff1b4051, 0.75);
            }
            for col in 0..9 {
                let x = 399.0 + f64::from(col) * 35.0;
                line(ctx, (x, 477.0), (x, 554.0), 0xff1b4051, 0.75);
            }
            let wave: Vec<_> = (0..=84)
                .map(|i| {
                    let t = f64::from(i);
                    (
                        399.0 + t * 3.4,
                        520.0 - 18.0 * (t * 0.16).sin() - 6.0 * (t * 0.37).cos() - t * 0.17,
                    )
                })
                .collect();
            route(ctx, &wave, CYAN, 2.2);
            label(
                ctx,
                &fonts.mono,
                "24.8 C",
                400.0,
                579.0,
                11.0,
                0xffdeeff3,
                0.0,
            );
            label(
                ctx,
                &fonts.mono,
                "120 Hz",
                513.0,
                579.0,
                11.0,
                0xffdeeff3,
                0.0,
            );
            label(
                ctx,
                &fonts.mono,
                "12.0 V",
                619.0,
                579.0,
                11.0,
                0xffdeeff3,
                0.0,
            );
            panel(
                ctx,
                Rect::new(249.0, 589.0, 114.0, 17.0),
                8.5,
                &solid(0xff194d58),
                None,
            );
            label(
                ctx,
                &fonts.bold,
                "LIVE TELEMETRY",
                262.0,
                601.0,
                7.8,
                CYAN,
                0.5,
            );
            ctx.pop_clip();
        },
    )?;

    doc.record_layer(
        SvgLayerOptions::new("glass", "04 Optical cover")
            .with_depth(3.0)
            .with_bounds(Rect::new(200.0, 372.0, 530.0, 266.0))
            .with_tags(["assembly", "shell", "glass"]),
        |ctx| {
            // The open bezel and faint surface tint remain independent editable shapes.
            panel(
                ctx,
                Rect::new(202.0, 374.0, 526.0, 262.0),
                22.0,
                &solid(0x0c9bdeef),
                Some(0xff7396aa),
            );
            for rect in [
                Rect::new(211.0, 380.0, 508.0, 7.0),
                Rect::new(211.0, 623.0, 508.0, 7.0),
            ] {
                panel(ctx, rect, 3.5, &gradient(0xff7c96a7, 0xff334d60), None);
            }
            line(ctx, (207.0, 400.0), (207.0, 610.0), 0xff557b90, 2.0);
            line(ctx, (723.0, 400.0), (723.0, 610.0), 0xff557b90, 2.0);
            let mut reflection = StreamGeometryBuilder::new(None);
            reflection.begin_figure(Point::new(220.0, 389.0), true);
            reflection.line_to(Point::new(452.0, 389.0), true);
            reflection.line_to(Point::new(221.0, 544.0), true);
            reflection.end_figure(true);
            let geometry = reflection.to_geometry();
            ctx.draw_geometry(Some(&gradient(0x17e4ffff, 0x00e4ffff)), None, &*geometry);
            dot(ctx, 695.0, 382.0, 2.0, 0xffa7cbd9, None);
        },
    )?;

    Ok(doc)
}

fn export_options() -> SvgExportOptions {
    SvgExportOptions {
        fidelity: SvgFidelity::VectorOnly,
        background: Some(color(PAPER)),
        title: Some("Aeris M1 — FerroUI SVG engineering plate".into()),
        description: Some("A layered control module rendered through FerroUI's Skia drawing contracts. Named chassis, logic board, telemetry display and optical cover support non-destructive inspection exports.".into()),
        ..SvgExportOptions::default()
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let benchmark = args.iter().any(|arg| arg == "--benchmark");
    let reference = args.iter().any(|arg| arg == "--reference");
    let directory = args
        .iter()
        .find(|arg| !arg.starts_with("--"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("samples/SvgExport/output"));
    std::fs::create_dir_all(&directory)?;
    SkiaPlatform::initialize();
    let fonts = Fonts::new()?;
    let recorded_at = Instant::now();
    let document = record_document(&fonts)?;
    println!(
        "Recorded layered scene in {:.2} ms",
        recorded_at.elapsed().as_secs_f64() * 1000.0
    );

    let normal = export_options();
    let views = [
        ("aeris-normal.svg", normal.clone()),
        (
            "aeris-xray.svg",
            SvgExportOptions {
                view: SvgView {
                    inspection_layers: SvgLayerFilter::tag("assembly"),
                    inspection: SvgInspection::XRay {
                        fill_opacity: 0.06,
                        stroke: color(0xff187c94),
                        stroke_width: 0.65,
                    },
                    ..SvgView::default()
                },
                ..normal.clone()
            },
        ),
        (
            "aeris-cutaway.svg",
            SvgExportOptions {
                view: SvgView {
                    cutaway: Some(SvgCutaway {
                        rect: Rect::new(456.0, 395.0, 255.0, 220.0),
                        layers: SvgLayerFilter::tag("shell"),
                        mode: SvgCutawayMode::RemoveInside,
                        show_boundary: true,
                    }),
                    ..SvgView::default()
                },
                ..normal.clone()
            },
        ),
        (
            "aeris-exploded.svg",
            SvgExportOptions {
                view: SvgView {
                    exploded: Some(SvgExplodedView {
                        step: Vector::new(42.0, -48.0),
                        show_connectors: true,
                        show_labels: true,
                    }),
                    ..SvgView::default()
                },
                ..normal.clone()
            },
        ),
        (
            "aeris-wireframe.svg",
            SvgExportOptions {
                view: SvgView {
                    inspection_layers: SvgLayerFilter::tag("assembly"),
                    inspection: SvgInspection::Wireframe {
                        stroke: color(0xff215d79),
                        stroke_width: 0.6,
                    },
                    ..SvgView::default()
                },
                ..normal.clone()
            },
        ),
    ];
    for (filename, options) in views {
        let started = Instant::now();
        let output = document.export(&options)?;
        output.save(directory.join(filename))?;
        let bytes = std::fs::metadata(directory.join(filename))?.len();
        println!(
            "{}: {} bytes, {:.2} ms; {} vector layers, {} draw calls, {} shared definitions",
            filename,
            bytes,
            started.elapsed().as_secs_f64() * 1000.0,
            output.report.vector_layers,
            output.report.draw_calls,
            output.report.definitions_deduplicated,
        );
    }
    if reference {
        let raster = SvgExportOptions {
            fidelity: SvgFidelity::Raster,
            raster_scale: 1.0,
            ..normal.clone()
        };
        document
            .export(&raster)?
            .save(directory.join("aeris-raster-reference.svg"))?;
    }
    let compact = SvgExportOptions {
        pretty: false,
        ..normal
    };
    document
        .export(&compact)?
        .save_svgz(directory.join("aeris-normal.svgz"))?;

    if benchmark {
        // Warm the serializer before measuring. Recording and filesystem I/O are excluded.
        for _ in 0..5 {
            std::hint::black_box(document.export(&compact)?);
        }
        let mut samples = Vec::with_capacity(100);
        for _ in 0..100 {
            let started = Instant::now();
            std::hint::black_box(document.export(&compact)?);
            samples.push(started.elapsed().as_secs_f64() * 1000.0);
        }
        samples.sort_by(f64::total_cmp);
        let profile = if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        };
        println!(
            "100 warm compact exports ({profile}): median {:.3} ms, p95 {:.3} ms, min {:.3} ms",
            (samples[49] + samples[50]) / 2.0,
            samples[94],
            samples[0]
        );
    }
    Ok(())
}
