//! Reproducible export benchmarks with separate recording, fresh-document export,
//! cache-cold export, warm export, view switching, definition processing and compression phases.
//!
//! Run `cargo run --release -p ferroui-svg --example export_bench -- --help`.
//! CSV is written to stdout (or --output); setup information goes to stderr.
//! Fonts, shaping and bitmap preparation happen before the measured recording.

use ferroui_base::media::immutable::{
    ImmutableGradientStop, ImmutableLinearGradientBrush, ImmutablePen, ImmutableSolidColorBrush,
};
use ferroui_base::media::{
    BaselinePixelAlignment, BoxShadows, Colors, DrawingContext, FlowDirection, FontFamily,
    FontManager, FormattedText, PlatformDrawingContext, TextHintingMode, TextOptions, Typeface,
};
use ferroui_base::platform::{
    IAssetLoader, IGeometryContext, IGeometryImpl, IStreamGeometryBuilder, StandardAssetLoader,
};
use ferroui_base::utilities::locale;
use ferroui_base::{FerroLocator, Matrix, Point, Rect, RoundedRect, Size, Vector};
use ferroui_skia::{ImmutableBitmap, StreamGeometryBuilder};
use ferroui_svg::{
    SvgCutaway, SvgCutawayMode, SvgDocument, SvgExplodedView, SvgExportOptions, SvgExportReport,
    SvgFidelity, SvgInspection, SvgLayerFilter, SvgLayerOptions, SvgOutput, SvgPlatform, SvgView,
};
use std::error::Error;
use std::hint::black_box;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const LAYERS: usize = 8;
const LAYER_BOUNDS: Rect = Rect::new(0.0, 0.0, 640.0, 240.0);
const SIZE: Size = Size::new(1280.0, 960.0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Workload {
    Geometry,
    Definitions,
    Text,
    Bitmap,
    Raster,
}

impl Workload {
    const ALL: [Self; 5] = [
        Self::Geometry,
        Self::Definitions,
        Self::Text,
        Self::Bitmap,
        Self::Raster,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::Geometry => "geometry",
            Self::Definitions => "definitions",
            Self::Text => "text",
            Self::Bitmap => "bitmap",
            Self::Raster => "raster",
        }
    }
}

struct Arguments {
    iterations: usize,
    cold_iterations: usize,
    warmup: usize,
    only: Option<Workload>,
    output: Option<PathBuf>,
    fixtures: Option<PathBuf>,
    reference_scale: f64,
    unhinted_text: bool,
}

impl Arguments {
    fn parse() -> Result<Self> {
        let mut result = Self {
            iterations: 40,
            cold_iterations: 8,
            warmup: 5,
            only: None,
            output: None,
            fixtures: None,
            reference_scale: 1.0,
            unhinted_text: false,
        };
        let mut arguments = std::env::args().skip(1);
        while let Some(argument) = arguments.next() {
            if argument == "--help" || argument == "-h" {
                println!("export_bench [--iterations N] [--cold-iterations N] [--warmup N] \
                    [--only geometry|definitions|text|bitmap|raster] [--output FILE] [--fixtures DIR] \
                    [--reference-scale N] [--unhinted-text]\n\n\
                    Defaults: 40 warm iterations, 8 cache-cold/fresh-document/recording iterations, 5 warmups.\n\
                    Fresh-document times recording plus export as one operation using prepared assets.\n\
                    Cold means cleared document export caches, not a new process or font cache.\n\
                    Each multi_view sample exports normal, x-ray, wireframe, cutaway and exploded views.\n\
                    Linux RSS fields are process-wide measurements taken outside timed loops.\n\
                    Prepared fonts, shaped text and bitmap input creation are excluded from recording.\n\
                    --fixtures writes each normal SVG, Skia raster reference and inspection variants.\n\
                    --reference-scale changes only the diagnostic raster reference (default 1, maximum 8).\n\
                    --unhinted-text records text with no hinting or baseline pixel alignment for geometry comparisons.");
                std::process::exit(0);
            }
            if argument == "--unhinted-text" {
                result.unhinted_text = true;
                continue;
            }
            let value = arguments
                .next()
                .ok_or_else(|| invalid(format!("{argument} needs a value")))?;
            match argument.as_str() {
                "--iterations" => result.iterations = parse_count(&value, false)?,
                "--cold-iterations" => result.cold_iterations = parse_count(&value, false)?,
                "--warmup" => result.warmup = parse_count(&value, true)?,
                "--output" => result.output = Some(value.into()),
                "--fixtures" => result.fixtures = Some(value.into()),
                "--reference-scale" => {
                    let scale: f64 = value
                        .parse()
                        .map_err(|_| invalid("reference scale must be a number"))?;
                    if !scale.is_finite() || !(0.0 < scale && scale <= 8.0) {
                        return Err(invalid("reference scale must be finite and in (0,8]").into());
                    }
                    result.reference_scale = scale;
                }
                "--only" => {
                    result.only = Some(
                        Workload::ALL
                            .into_iter()
                            .find(|case| case.name() == value)
                            .ok_or_else(|| invalid(format!("unknown workload '{value}'")))?,
                    );
                }
                _ => return Err(invalid(format!("unknown option '{argument}'")).into()),
            }
        }
        Ok(result)
    }
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

fn parse_count(value: &str, zero_allowed: bool) -> io::Result<usize> {
    let count: usize = value
        .parse()
        .map_err(|_| invalid("iteration count must be an integer"))?;
    if count > 10_000 || (count == 0 && !zero_allowed) {
        return Err(invalid(
            "iteration count must be in 1..=10000 (warmup also allows zero)",
        ));
    }
    Ok(count)
}

struct Assets {
    gradient: ImmutableLinearGradientBrush,
    fill: ImmutableSolidColorBrush,
    accent: ImmutableSolidColorBrush,
    pen: ImmutablePen,
    geometry: Arc<dyn IGeometryImpl>,
    bitmap: ImmutableBitmap,
    text: Vec<FormattedText>,
}

impl Assets {
    fn new() -> Result<Self> {
        SvgPlatform::initialize();
        ferroui_harfbuzz::HarfBuzzPlatform::initialize();
        let loader: Rc<dyn IAssetLoader> = Rc::new(StandardAssetLoader::new(None));
        FerroLocator::current_mutable()
            .bind::<dyn IAssetLoader>()
            .to_constant(loader);
        FontManager::current()
            .add_font_collection(Rc::new(ferroui_fonts_inter::InterFontCollection::new()));

        let mut geometry = StreamGeometryBuilder::new(None);
        geometry.begin_figure(Point::new(4.0, 2.0), true);
        geometry.line_to(Point::new(56.0, 2.0), true);
        geometry.cubic_bezier_to(
            Point::new(70.0, 7.0),
            Point::new(44.0, 22.0),
            Point::new(28.0, 18.0),
            true,
        );
        geometry.line_to(Point::new(6.0, 20.0), true);
        geometry.end_figure(true);

        let mut surface = skia_safe::surfaces::raster_n32_premul((96, 96))
            .ok_or_else(|| invalid("unable to create the small benchmark bitmap"))?;
        let mut paint = skia_safe::Paint::default();
        for row in 0..12 {
            for column in 0..12 {
                paint.set_color(skia_safe::Color::from_rgb(
                    (24 + column * 17) as u8,
                    (32 + row * 16) as u8,
                    (64 + ((row + column) % 8) * 20) as u8,
                ));
                surface.canvas().draw_rect(
                    skia_safe::Rect::from_xywh((column * 8) as f32, (row * 8) as f32, 8.0, 8.0),
                    &paint,
                );
            }
        }
        let bitmap = ImmutableBitmap::from_image(surface.image_snapshot(), None);
        let typeface = Typeface::new(FontFamily::parse("fonts:Inter#Inter")?);
        let text: Vec<_> = (0..10)
            .map(|line| {
                let text = FormattedText::new(
                    &format!("{line:02}  office ffi — Zażółć gęślą jaźń — e\u{301}  0123456789"),
                    locale!("pl-PL"),
                    FlowDirection::LeftToRight,
                    typeface.clone(),
                    14.0,
                    Some(Rc::new(ImmutableSolidColorBrush::new(Colors::BLACK))),
                );
                // Force layout and shaping outside the recording/export measurements.
                black_box(text.width());
                text
            })
            .collect();
        Ok(Self {
            gradient: ImmutableLinearGradientBrush::from_stops(&[
                ImmutableGradientStop::new(0.0, Colors::CORNFLOWER_BLUE),
                ImmutableGradientStop::new(1.0, Colors::DARK_SLATE_BLUE),
            ]),
            fill: ImmutableSolidColorBrush::new(Colors::CORNFLOWER_BLUE),
            accent: ImmutableSolidColorBrush::new(Colors::ORANGE),
            pen: ImmutablePen::from_uint32(0xff183048, 0.75),
            geometry: geometry.to_geometry(),
            bitmap,
            text,
        })
    }
}

fn record(workload: Workload, assets: &Assets, unhinted_text: bool) -> Result<SvgDocument> {
    use ferroui_base::platform::IDrawingContextImpl;
    let mut document = SvgDocument::new(SIZE)?;
    for layer in 0..LAYERS {
        let transform =
            Matrix::create_translation((layer % 2) as f64 * 640.0, (layer / 2) as f64 * 240.0);
        document.record_layer(
            SvgLayerOptions::new(format!("layer-{layer}"), format!("Layer {layer}"))
                .with_bounds(LAYER_BOUNDS)
                .with_transform(transform)
                .with_depth((layer % 4) as f64)
                .with_tags(["assembly"]),
            |context| {
                if workload == Workload::Text {
                    if unhinted_text {
                        context.push_text_options(TextOptions {
                            text_hinting_mode: TextHintingMode::None,
                            baseline_pixel_alignment: BaselinePixelAlignment::Unaligned,
                            ..TextOptions::default()
                        });
                    }
                    {
                        let mut platform = PlatformDrawingContext::borrowed(context);
                        let mut drawing = DrawingContext::new(&mut platform);
                        for (line, text) in assets.text.iter().enumerate() {
                            drawing.draw_text(text, Point::new(8.0, 8.0 + line as f64 * 22.0));
                        }
                        drawing.dispose();
                    }
                    if unhinted_text {
                        context.pop_text_options();
                    }
                    return;
                }
                if workload == Workload::Bitmap {
                    for row in 0..4 {
                        for column in 0..4 {
                            context.draw_bitmap(
                                &assets.bitmap,
                                0.85,
                                Rect::new(8.0, 8.0, 80.0, 80.0),
                                Rect::new(
                                    8.0 + column as f64 * 158.0,
                                    6.0 + row as f64 * 58.0,
                                    144.0,
                                    46.0,
                                ),
                            );
                        }
                    }
                    return;
                }
                if workload == Workload::Raster {
                    context.push_opacity(0.65, Some(LAYER_BOUNDS));
                }
                for row in 0..8 {
                    for column in 0..8 {
                        context.set_transform(Matrix::create_translation(
                            8.0 + column as f64 * 78.0,
                            4.0 + row as f64 * 29.0,
                        ));
                        let rect = Rect::new(0.0, 0.0, 66.0, 23.0);
                        match workload {
                            Workload::Geometry => {
                                context.draw_rectangle(
                                    Some(&assets.fill),
                                    Some(&assets.pen),
                                    RoundedRect::from_radius(rect, 3.0),
                                    &BoxShadows::default(),
                                );
                                context.draw_ellipse(
                                    Some(&assets.accent),
                                    None,
                                    Rect::new(5.0, 5.0, 13.0, 13.0),
                                );
                                context.draw_line(
                                    Some(&assets.pen),
                                    Point::new(24.0, 7.0),
                                    Point::new(58.0, 17.0),
                                );
                            }
                            Workload::Definitions => {
                                context.push_clip(Rect::new(1.0, 1.0, 63.0, 21.0));
                                // Background and border redraw deliberately share the
                                // same gradient and inherited clip context. Other tiles
                                // and semantic layers retain distinct clip/transform
                                // contexts, exercising both successful and refused
                                // resource interning in the same workload.
                                context.draw_rectangle(
                                    Some(&assets.gradient),
                                    None,
                                    RoundedRect::from(rect),
                                    &BoxShadows::default(),
                                );
                                context.draw_rectangle(
                                    Some(&assets.gradient),
                                    Some(&assets.pen),
                                    RoundedRect::from(rect),
                                    &BoxShadows::default(),
                                );
                                context.draw_geometry(
                                    Some(&assets.gradient),
                                    Some(&assets.pen),
                                    &*assets.geometry,
                                );
                                context.pop_clip();
                            }
                            Workload::Raster => {
                                context.draw_rectangle(
                                    Some(&assets.gradient),
                                    None,
                                    RoundedRect::from_radius(rect, 5.0),
                                    &BoxShadows::default(),
                                );
                                context.draw_ellipse(
                                    Some(&assets.accent),
                                    None,
                                    Rect::new(20.0, 4.0, 28.0, 16.0),
                                );
                            }
                            _ => unreachable!(),
                        }
                    }
                }
                if workload == Workload::Raster {
                    context.pop_opacity();
                }
            },
        )?;
    }
    Ok(document)
}

fn options(workload: Workload) -> SvgExportOptions {
    SvgExportOptions {
        fidelity: if workload == Workload::Raster {
            SvgFidelity::Automatic
        } else {
            SvgFidelity::VectorOnly
        },
        raster_scale: 1.0,
        title: Some(format!("FerroUI SVG benchmark: {}", workload.name())),
        ..SvgExportOptions::compact()
    }
}

fn views(normal: &SvgExportOptions) -> Vec<(&'static str, SvgExportOptions)> {
    vec![
        ("normal", normal.clone()),
        (
            "xray",
            SvgExportOptions {
                view: SvgView {
                    inspection: SvgInspection::x_ray(),
                    ..SvgView::default()
                },
                ..normal.clone()
            },
        ),
        (
            "wireframe",
            SvgExportOptions {
                view: SvgView {
                    inspection: SvgInspection::wireframe(),
                    ..SvgView::default()
                },
                ..normal.clone()
            },
        ),
        (
            "cutaway",
            SvgExportOptions {
                view: SvgView {
                    cutaway: Some(SvgCutaway {
                        rect: Rect::new(420.0, 180.0, 380.0, 600.0),
                        layers: SvgLayerFilter::all(),
                        mode: SvgCutawayMode::RemoveInside,
                        show_boundary: true,
                    }),
                    ..SvgView::default()
                },
                ..normal.clone()
            },
        ),
        (
            "exploded",
            SvgExportOptions {
                view: SvgView {
                    exploded: Some(SvgExplodedView {
                        step: Vector::new(10.0, -6.0),
                        show_connectors: true,
                        show_labels: false,
                    }),
                    ..SvgView::default()
                },
                ..normal.clone()
            },
        ),
    ]
}

#[derive(Default)]
struct Observation {
    output_bytes: usize,
    exported_layers: usize,
    images: u64,
    definitions_deduplicated: usize,
    vector_cache_hits: usize,
    raster_cache_hits: usize,
}

impl Observation {
    fn add(&mut self, bytes: usize, report: &SvgExportReport) {
        self.output_bytes += bytes;
        self.exported_layers += report.layers;
        self.images += report.embedded_images;
        self.definitions_deduplicated += report.definitions_deduplicated;
        self.vector_cache_hits += report.vector_cache_hits;
        self.raster_cache_hits += report.raster_cache_hits;
    }
}

struct Distribution {
    minimum: f64,
    median: f64,
    p95: f64,
}

impl Distribution {
    fn new(mut values: Vec<f64>) -> Self {
        values.sort_by(f64::total_cmp);
        let n = values.len();
        Self {
            minimum: values[0],
            median: (values[(n - 1) / 2] + values[n / 2]) / 2.0,
            p95: values[(n * 95).div_ceil(100).saturating_sub(1)],
        }
    }
}

fn memory_kib() -> (String, String) {
    // Optional Linux process measurements, deliberately outside the timed operations.
    let Ok(status) = std::fs::read_to_string("/proc/self/status") else {
        return (String::new(), String::new());
    };
    let read = |key: &str| {
        status
            .lines()
            .find_map(|line| line.strip_prefix(key))
            .and_then(|line| line.split_whitespace().next())
            .unwrap_or("")
            .to_string()
    };
    (read("VmRSS:"), read("VmHWM:"))
}

#[allow(clippy::too_many_arguments)]
fn write_row(
    writer: &mut dyn Write,
    workload: Workload,
    phase: &str,
    samples: usize,
    exports: usize,
    times: Distribution,
    document: &SvgDocument,
    observation: Observation,
) -> io::Result<()> {
    let (rss, peak) = memory_kib();
    let draws: u64 = document
        .layers()
        .iter()
        .map(|layer| layer.audit().draw_calls)
        .sum();
    let recording_bytes: usize = document
        .layers()
        .iter()
        .map(|layer| layer.approximate_recording_bytes())
        .sum();
    writeln!(
        writer,
        "{},{},{},{},{:.6},{:.6},{:.6},{},{},{},{},{},{},{},{},{},{},{}",
        workload.name(),
        phase,
        samples,
        exports,
        times.minimum,
        times.median,
        times.p95,
        observation.output_bytes,
        document.layers().len(),
        draws,
        observation.exported_layers,
        observation.images,
        observation.definitions_deduplicated,
        observation.vector_cache_hits,
        observation.raster_cache_hits,
        recording_bytes,
        rss,
        peak
    )
}

fn export_distribution(
    document: &SvgDocument,
    variants: &[(&str, SvgExportOptions)],
    iterations: usize,
    warmup: usize,
    clear_cache: bool,
) -> Result<(Distribution, Observation)> {
    for _ in 0..warmup {
        for (_, options) in variants {
            black_box(document.export(options)?);
        }
    }
    let mut times = Vec::with_capacity(iterations);
    let mut observation = Observation::default();
    for sample in 0..iterations {
        if clear_cache {
            document.clear_export_cache();
        }
        let start = Instant::now();
        let outputs: Vec<SvgOutput> = variants
            .iter()
            .map(|(_, options)| document.export(options))
            .collect::<std::result::Result<_, _>>()?;
        times.push(start.elapsed().as_secs_f64() * 1000.0);
        if sample + 1 == iterations {
            for output in &outputs {
                observation.add(output.as_bytes().len(), &output.report);
            }
        }
        black_box(outputs);
    }
    Ok((Distribution::new(times), observation))
}

fn validate_output(output: &SvgOutput) -> Result<()> {
    let document = roxmltree::Document::parse(output.as_str())?;
    if !document.root_element().has_tag_name("svg") {
        return Err(invalid("benchmark produced a non-SVG root").into());
    }
    let mut ids = std::collections::HashSet::new();
    for node in document.descendants() {
        if let Some(id) = node.attribute("id") {
            if !ids.insert(id) {
                return Err(invalid(format!("benchmark produced duplicate ID '{id}'")).into());
            }
        }
    }
    Ok(())
}

fn validate_views(document: &SvgDocument, normal: &SvgExportOptions) -> Result<()> {
    // Keep structural validation and deterministic cold/warm comparisons outside
    // every timing window, including workloads without a fixture output directory.
    for (view, options) in views(normal) {
        document.clear_export_cache();
        let cold = document.export(&options)?;
        let warm = document.export(&options)?;
        validate_output(&cold)?;
        if cold.as_bytes() != warm.as_bytes() {
            return Err(invalid(format!(
                "benchmark produced different cache-cold/warm bytes for {view}"
            ))
            .into());
        }
    }
    Ok(())
}

fn fixtures(
    directory: &Path,
    workload: Workload,
    document: &SvgDocument,
    normal: &SvgExportOptions,
    reference_scale: f64,
) -> Result<()> {
    std::fs::create_dir_all(directory)?;
    for (view, options) in views(normal) {
        let filename = format!("{}-{view}.svg", workload.name());
        document.export(&options)?.save(directory.join(filename))?;
    }
    document
        .export(&SvgExportOptions {
            fidelity: SvgFidelity::Raster,
            raster_scale: reference_scale,
            ..normal.clone()
        })?
        .save(directory.join(format!("{}-raster-reference.svg", workload.name())))?;
    Ok(())
}

fn main() -> Result<()> {
    let args = Arguments::parse()?;
    let assets = Assets::new()?;
    let mut writer: Box<dyn Write> = match &args.output {
        Some(path) => Box::new(BufWriter::new(std::fs::File::create(path)?)),
        None => Box::new(BufWriter::new(io::stdout())),
    };
    eprintln!(
        "SVG export benchmark; {} {}; debug_assertions={}; warm={} cold={} warmup={}; unhinted_text={}",
        std::env::consts::OS,
        std::env::consts::ARCH,
        cfg!(debug_assertions),
        args.iterations,
        args.cold_iterations,
        args.warmup,
        args.unhinted_text
    );
    writeln!(writer, "workload,phase,samples,exports_per_operation,min_ms,median_ms,p95_ms,output_bytes,scene_layers,scene_draw_calls,exported_layers,images,definitions_deduplicated,vector_cache_hits,raster_cache_hits,recording_bytes,rss_kib,peak_rss_kib")?;
    for workload in Workload::ALL
        .into_iter()
        .filter(|case| args.only.is_none_or(|selected| selected == *case))
    {
        let mut samples = Vec::with_capacity(args.cold_iterations);
        let mut recorded = None;
        for _ in 0..args.cold_iterations {
            let start = Instant::now();
            let document = record(workload, &assets, args.unhinted_text)?;
            samples.push(start.elapsed().as_secs_f64() * 1000.0);
            recorded = Some(document);
        }
        let document = recorded.unwrap();
        write_row(
            &mut *writer,
            workload,
            "record",
            args.cold_iterations,
            0,
            Distribution::new(samples),
            &document,
            Observation::default(),
        )?;
        let normal = options(workload);
        let mut fresh_times = Vec::with_capacity(args.cold_iterations);
        let mut observation = Observation::default();
        for sample in 0..args.cold_iterations {
            let start = Instant::now();
            let fresh = record(workload, &assets, args.unhinted_text)?;
            let output = fresh.export(&normal)?;
            fresh_times.push(start.elapsed().as_secs_f64() * 1000.0);
            if sample + 1 == args.cold_iterations {
                observation.add(output.as_bytes().len(), &output.report);
            }
            // Destruction remains outside the timed operation, matching the
            // separate recording/export phases and an export retained by its caller.
            black_box((fresh, output));
        }
        write_row(
            &mut *writer,
            workload,
            "fresh_document",
            args.cold_iterations,
            1,
            Distribution::new(fresh_times),
            &document,
            observation,
        )?;
        let variants = [("normal", normal.clone())];
        let (times, observation) =
            export_distribution(&document, &variants, args.cold_iterations, 0, true)?;
        write_row(
            &mut *writer,
            workload,
            "cold",
            args.cold_iterations,
            1,
            times,
            &document,
            observation,
        )?;
        let (times, observation) =
            export_distribution(&document, &variants, args.iterations, args.warmup, false)?;
        write_row(
            &mut *writer,
            workload,
            "warm",
            args.iterations,
            1,
            times,
            &document,
            observation,
        )?;
        let variants = views(&normal);
        let (times, observation) =
            export_distribution(&document, &variants, args.iterations, args.warmup, false)?;
        write_row(
            &mut *writer,
            workload,
            "multi_view",
            args.iterations,
            variants.len(),
            times,
            &document,
            observation,
        )?;
        let no_dedup = [(
            "normal",
            SvgExportOptions {
                deduplicate: false,
                ..normal.clone()
            },
        )];
        let (times, observation) =
            export_distribution(&document, &no_dedup, args.iterations, args.warmup, false)?;
        write_row(
            &mut *writer,
            workload,
            "no_dedup",
            args.iterations,
            1,
            times,
            &document,
            observation,
        )?;

        let output = document.export(&normal)?;
        validate_output(&output)?;
        let mut gzip_times = Vec::with_capacity(args.iterations);
        let mut observation = Observation::default();
        for sample in 0..args.iterations {
            let mut encoded = Vec::new();
            let start = Instant::now();
            output.write_svgz(&mut encoded, 6)?;
            gzip_times.push(start.elapsed().as_secs_f64() * 1000.0);
            if sample + 1 == args.iterations {
                observation.output_bytes = encoded.len();
            }
            black_box(encoded);
        }
        write_row(
            &mut *writer,
            workload,
            "gzip",
            args.iterations,
            0,
            Distribution::new(gzip_times),
            &document,
            observation,
        )?;
        validate_views(&document, &normal)?;
        if let Some(directory) = &args.fixtures {
            fixtures(
                directory,
                workload,
                &document,
                &normal,
                args.reference_scale,
            )?;
        }
        writer.flush()?;
        eprintln!("Completed {}", workload.name());
    }
    Ok(())
}
