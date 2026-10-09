//! Text under comparison: the services text needs of a backend, the text
//! scenes and what the numeric comparisons of typefaces and glyph runs
//! share.
//!
//! Text is drawn through the contracts as an application draws it: a
//! platform typeface from the font manager of the backend, the glyph
//! typeface of the base library over it, shaping by HarfBuzz (the same
//! shaper over the tables each typeface serves), a glyph run of the render
//! interface, `draw_glyph_run`. The scenes that lay text out use the text
//! layout of the base library with the font manager of the backend, which
//! is where fallback fonts come from.
//!
//! The fonts are the test fonts of the Skia backend
//! (`src/Skia/FerroUI.Skia/test_assets`), included from there.

use crate::scenes::Scene;
use crate::Backend;
use ferroui_base::media::immutable::{
    ImmutableGradientStop, ImmutableLinearGradientBrush, ImmutablePen, ImmutableSolidColorBrush,
};
use ferroui_base::media::text_formatting::{
    GlyphInfo, ITextDrawingSink, TextLayout, TextLayoutOptions, TextShaper, TextShaperOptions,
};
use ferroui_base::media::{
    BoxShadows, Color, Colors, EdgeMode, FontFamily, FontSimulations, GlyphInfoList, GlyphRun, GlyphTypeface,
    GradientSpreadMethod, IBrush, IPen, IPlatformTypeface, PenLineCap, PenLineJoin, RenderOptions, TextDecorationCollection,
    TextDecorations, TextWrapping, Typeface,
};
use ferroui_base::platform::{
    register_manifest_resources, IAssetLoader, IDrawingContextImpl, IFontManagerImpl, IGlyphRunImpl,
    IPlatformRenderInterface, StandardAssetLoader,
};
use ferroui_base::utilities::ReadOnlyMemory;
use ferroui_base::{
    CornerRadius, FerroLocator, Matrix, Point, Rect, RelativePoint, RelativeUnit, RoundedRect,
};
use std::rc::Rc;
use std::sync::{Arc, Once};

/// The assembly the fonts of the scenes are manifest resources of.
pub const ASSEMBLY: &str = "ferroui-render-backends-comparison";

macro_rules! font {
    ($folder:literal, $file:literal) => {
        include_bytes!(concat!("../../src/Skia/FerroUI.Skia/test_assets/", $folder, "/", $file)) as &[u8]
    };
}

pub const INTER: &[u8] = font!("assets", "Inter-Regular.ttf");
pub const INTER_VARIABLE: &[u8] = font!("assets", "InterVariable.ttf");
pub const NOTO_SANS_ARABIC: &[u8] = font!("assets", "NotoSansArabic-Regular.ttf");
pub const NOTO_SANS_HEBREW: &[u8] = font!("assets", "NotoSansHebrew-Regular.ttf");

/// Every test font: its file name and its bytes. The numeric comparisons
/// run over all of them.
pub const TEST_FONTS: &[(&str, &[u8])] = &[
    ("AdobeBlank2VF.ttf", font!("assets", "AdobeBlank2VF.ttf")),
    ("Inter-Bold.ttf", font!("assets", "Inter-Bold.ttf")),
    ("Inter-Regular.ttf", INTER),
    ("InterVariable.ttf", INTER_VARIABLE),
    ("Manrope-Light.ttf", font!("assets", "Manrope-Light.ttf")),
    ("MiSans-Normal.ttf", font!("assets", "MiSans-Normal.ttf")),
    ("NISC18030.ttf", font!("assets", "NISC18030.ttf")),
    ("NotoMono-Regular.ttf", font!("assets", "NotoMono-Regular.ttf")),
    ("NotoSans-Italic.ttf", font!("assets", "NotoSans-Italic.ttf")),
    ("NotoSansArabic-Regular.ttf", NOTO_SANS_ARABIC),
    ("NotoSansDeseret-Regular.ttf", font!("assets", "NotoSansDeseret-Regular.ttf")),
    ("NotoSansHebrew-Regular.ttf", NOTO_SANS_HEBREW),
    ("NotoSansMiao-Regular.ttf", font!("assets", "NotoSansMiao-Regular.ttf")),
    ("NotoSansTamil-Regular.ttf", font!("assets", "NotoSansTamil-Regular.ttf")),
    ("PointMatch.ttf", font!("assets", "PointMatch.ttf")),
    ("SourceSerif4_36pt-Italic.ttf", font!("assets", "SourceSerif4_36pt-Italic.ttf")),
    ("TwitterColorEmoji-SVGinOT.ttf", font!("assets", "TwitterColorEmoji-SVGinOT.ttf")),
    ("BareMinimum.ttf", font!("fonts", "BareMinimum.ttf")),
    ("CascadiaCode.ttf", font!("fonts", "CascadiaCode.ttf")),
    ("DF7segHMI.ttf", font!("fonts", "DF7segHMI.ttf")),
    ("DejaVuSans.ttf", font!("fonts", "DejaVuSans.ttf")),
    ("Inter-Regular.LineGap800.ttf", font!("fonts", "Inter-Regular.LineGap800.ttf")),
    ("Manrope-Light.ttf (fonts)", font!("fonts", "Manrope-Light.ttf")),
    ("NotoSansArabic-NoLayout.ttf", font!("fonts", "NotoSansArabic-NoLayout.ttf")),
    ("NotoSansJP-Subset.ttf", font!("fonts", "NotoSansJP-Subset.ttf")),
    ("NotoSansSC-Subset.ttf", font!("fonts", "NotoSansSC-Subset.ttf")),
    ("TestFontNoCmap412.ttf", font!("fonts", "TestFontNoCmap412.ttf")),
    ("WinSymbols3.ttf", font!("fonts", "WinSymbols3.ttf")),
];

/// The fonts the text layouts of the scenes name as families.
static RESOURCES: &[(&str, &[u8])] = &[
    ("FerroUI.RenderBackends.Comparison.Fonts.Inter-Regular.ttf", INTER),
    ("FerroUI.RenderBackends.Comparison.Fonts.NotoSansArabic-Regular.ttf", NOTO_SANS_ARABIC),
    ("FerroUI.RenderBackends.Comparison.Fonts.NotoSansHebrew-Regular.ttf", NOTO_SANS_HEBREW),
];

/// The family of an embedded font of the scenes.
pub fn embedded_typeface(family: &str) -> Typeface {
    Typeface::new(
        FontFamily::parse(&format!("resm:FerroUI.RenderBackends.Comparison.Fonts?assembly={ASSEMBLY}#{family}"))
            .unwrap_or_else(|_| panic!("the family {family} parses")),
    )
}

/// Whether a backend under comparison is the Skia backend.
pub fn is_skia(backend: &Backend) -> bool {
    backend.name.starts_with("Skia")
}

/// The font manager of the backend a render interface belongs to.
pub fn font_manager(backend: &Backend) -> Rc<dyn IFontManagerImpl> {
    if is_skia(backend) {
        Rc::new(ferroui_skia::FontManagerImpl::new())
    } else {
        Rc::new(ferroui_vello::FontManagerImpl::new())
    }
}

/// Runs `f` with the services text needs registered for a backend: its
/// render interface and font manager, the asset loader and the HarfBuzz
/// text shaper. The services are those of `f` only.
pub fn with_text_services<R>(backend: &Backend, f: impl FnOnce(&Rc<dyn IFontManagerImpl>) -> R) -> R {
    static REGISTERED: Once = Once::new();
    REGISTERED.call_once(|| register_manifest_resources(ASSEMBLY, RESOURCES));

    let scope = FerroLocator::enter_scope();

    let asset_loader: Rc<dyn IAssetLoader> = Rc::new(StandardAssetLoader::new(None));
    let font_manager = font_manager(backend);

    FerroLocator::current_mutable()
        .bind::<dyn IAssetLoader>()
        .to_constant(asset_loader)
        .bind::<dyn IPlatformRenderInterface>()
        .to_constant(backend.interface.clone())
        .bind::<dyn IFontManagerImpl>()
        .to_constant(font_manager.clone());
    ferroui_harfbuzz::HarfBuzzPlatform::initialize();

    let result = f(&font_manager);

    scope.dispose();
    result
}

/// The platform typeface of a font file, by the font manager of a backend.
pub fn platform_typeface(
    font_manager: &Rc<dyn IFontManagerImpl>,
    font: &[u8],
    font_simulations: FontSimulations,
) -> Option<Rc<dyn IPlatformTypeface>> {
    font_manager.try_create_glyph_typeface_from_stream(&mut &font[..], font_simulations)
}

/// The glyph typeface of a font file.
///
/// # Panics
/// Panics for a font the backend or the base library does not read.
pub fn glyph_typeface(
    font_manager: &Rc<dyn IFontManagerImpl>,
    font: &[u8],
    font_simulations: FontSimulations,
) -> Rc<GlyphTypeface> {
    let typeface = platform_typeface(font_manager, font, font_simulations).expect("the backend reads the font");

    GlyphTypeface::new(typeface, font_simulations).expect("the font has its tables")
}

/// The platform typeface of a variable font at values of its axes, as each
/// backend makes one: the contracts have no member for it.
pub fn variable_typeface(backend: &Backend, font: &[u8], settings: &[(&str, f32)]) -> Rc<dyn IPlatformTypeface> {
    if is_skia(backend) {
        use skia_safe::font_arguments::variation_position::Coordinate;
        use skia_safe::font_arguments::VariationPosition;
        use skia_safe::{Data, FontArguments, FontMgr, FourByteTag};

        let typeface = FontMgr::new().new_from_data(Data::new_copy(font), None).expect("Skia reads the font");
        let coordinates: Vec<Coordinate> = settings
            .iter()
            .map(|(tag, value)| {
                let tag = tag.as_bytes();
                Coordinate {
                    axis: FourByteTag::from_chars(tag[0] as char, tag[1] as char, tag[2] as char, tag[3] as char),
                    value: *value,
                }
            })
            .collect();
        let arguments =
            FontArguments::new().set_variation_design_position(VariationPosition { coordinates: &coordinates });
        let instance = typeface.clone_with_arguments(&arguments).expect("an instance of the font");

        ferroui_skia::SkiaTypeface::new(instance, FontSimulations::None)
    } else {
        ferroui_vello::VelloTypeface::from_bytes_with_variations(font.to_vec(), FontSimulations::None, settings)
            .expect("the font reads")
    }
}

/// Shapes a text with a glyph typeface at an em size; `bidi_level` is 1
/// for a text that runs right to left.
pub fn shape(glyph_typeface: &Rc<GlyphTypeface>, text: &str, em_size: f64, bidi_level: i8) -> Vec<GlyphInfo> {
    let options = TextShaperOptions::with_all(glyph_typeface.clone(), em_size, bidi_level, None, 0.0, 0.0, None);

    TextShaper::current().shape_text(&ReadOnlyMemory::from_str(text), &options).glyph_infos().to_vec()
}

/// The glyph run of a shaped text, by the render interface of a backend.
pub fn glyph_run(
    backend: &Backend,
    glyph_typeface: &Rc<GlyphTypeface>,
    text: &str,
    em_size: f64,
    bidi_level: i8,
    origin: Point,
) -> Arc<dyn IGlyphRunImpl> {
    let glyph_infos = shape(glyph_typeface, text, em_size, bidi_level);

    backend.interface.create_glyph_run(glyph_typeface, em_size, &glyph_infos, origin)
}

/// The glyph run of the base library of a shaped text: what the geometry of
/// a run is built from.
pub fn base_glyph_run(glyph_typeface: &Rc<GlyphTypeface>, text: &str, em_size: f64, origin: Point) -> Rc<GlyphRun> {
    let glyph_infos = shape(glyph_typeface, text, em_size, 0);

    GlyphRun::new(
        glyph_typeface.clone(),
        em_size,
        ReadOnlyMemory::from_str(text),
        GlyphInfoList::from(glyph_infos),
        Some(origin),
        0,
    )
}

/// A text drawing sink over a platform drawing context: what the drawing
/// context of the base library does for a text layout. It notes the family
/// of every glyph run it draws.
pub struct ContextSink<'a> {
    context: &'a mut dyn IDrawingContextImpl,
    transforms: Vec<Matrix>,
    /// The families of the glyph runs that were drawn, in order.
    pub families: Vec<String>,
}

impl<'a> ContextSink<'a> {
    pub fn new(context: &'a mut dyn IDrawingContextImpl) -> Self {
        Self { context, transforms: Vec::new(), families: Vec::new() }
    }
}

impl ITextDrawingSink for ContextSink<'_> {
    fn draw_glyph_run(&mut self, foreground: Option<&Rc<dyn IBrush>>, glyph_run: &Rc<GlyphRun>) {
        self.families.push(glyph_run.glyph_typeface().family_name().to_owned());
        self.context.draw_glyph_run(foreground.map(|brush| &**brush), &*glyph_run.platform_impl());
    }

    fn draw_rectangle(&mut self, brush: Option<&Rc<dyn IBrush>>, pen: Option<&Rc<dyn IPen>>, rect: Rect) {
        self.context.draw_rectangle(
            brush.map(|brush| &**brush),
            pen.map(|pen| &**pen),
            RoundedRect::from_rect(rect),
            &BoxShadows::default(),
        );
    }

    fn draw_line(&mut self, pen: &Rc<dyn IPen>, p1: Point, p2: Point) {
        self.context.draw_line(Some(&**pen), p1, p2);
    }

    fn push_transform(&mut self, matrix: Matrix) {
        let current = self.context.transform();
        self.transforms.push(current);
        self.context.set_transform(matrix * current);
    }

    fn pop_transform(&mut self) {
        let previous = self.transforms.pop().expect("a pushed transform");
        self.context.set_transform(previous);
    }
}

const INK: Color = Color::from_argb(255, 20, 20, 30);
const NAVY: Color = Color::from_argb(255, 20, 40, 120);

fn solid(color: Color) -> ImmutableSolidColorBrush {
    ImmutableSolidColorBrush::new(color)
}

fn background(context: &mut dyn IDrawingContextImpl) {
    context.clear(Colors::WHITE);
}

/// Draws a line of text of a font file with its baseline origin at
/// `origin`.
#[allow(clippy::too_many_arguments)]
fn draw_line_of_text(
    backend: &Backend,
    context: &mut dyn IDrawingContextImpl,
    glyph_typeface: &Rc<GlyphTypeface>,
    text: &str,
    em_size: f64,
    bidi_level: i8,
    origin: Point,
    brush: &dyn IBrush,
) {
    let run = glyph_run(backend, glyph_typeface, text, em_size, bidi_level, origin);
    context.draw_glyph_run(Some(brush), &*run);
    run.dispose();
}

/// A line of Latin text at five sizes, from a small label to a heading.
fn latin_sizes(backend: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    with_text_services(backend, |font_manager| {
        let inter = glyph_typeface(font_manager, INTER, FontSimulations::None);
        let brush = solid(INK);

        for (em_size, baseline) in [(9.0, 18.0), (12.0, 38.0), (16.0, 62.0), (24.0, 96.0), (40.0, 150.0)] {
            draw_line_of_text(backend, context, &inter, "Hamburgefonstiv 123", em_size, 0, Point::new(8.0, baseline), &brush);
        }
    });
}

/// The sizes an interface is set in, at origins between pixels.
fn interface_sizes(backend: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    with_text_services(backend, |font_manager| {
        let inter = glyph_typeface(font_manager, INTER, FontSimulations::None);
        let brush = solid(INK);

        for (line, em_size) in [11.0, 12.0, 13.0, 14.0, 15.0, 17.0, 20.0].into_iter().enumerate() {
            let origin = Point::new(6.3 + line as f64 * 0.21, 22.4 + line as f64 * 25.3);
            draw_line_of_text(backend, context, &inter, "The quick brown fox jumps", em_size, 0, origin, &brush);
        }
    });
}

/// Latin, Arabic and Hebrew, each in a font of its own: the scripts of a
/// mixed text with the fonts fixed.
fn mixed_scripts(backend: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    with_text_services(backend, |font_manager| {
        let brush = solid(INK);
        let inter = glyph_typeface(font_manager, INTER, FontSimulations::None);
        let arabic = glyph_typeface(font_manager, NOTO_SANS_ARABIC, FontSimulations::None);
        let hebrew = glyph_typeface(font_manager, NOTO_SANS_HEBREW, FontSimulations::None);

        draw_line_of_text(backend, context, &inter, "Mixed scripts", 26.0, 0, Point::new(10.0, 40.0), &brush);
        draw_line_of_text(
            backend,
            context,
            &arabic,
            "\u{0627}\u{0644}\u{0639}\u{0631}\u{0628}\u{064A}\u{0629} \u{0646}\u{0635}",
            30.0,
            1,
            Point::new(10.0, 100.0),
            &brush,
        );
        draw_line_of_text(
            backend,
            context,
            &hebrew,
            "\u{05E2}\u{05D1}\u{05E8}\u{05D9}\u{05EA} \u{05E9}\u{05DC}\u{05D5}\u{05DD}",
            30.0,
            1,
            Point::new(10.0, 160.0),
            &brush,
        );
    });
}

/// The text of the fallback scene: Latin, Hebrew, Han, Hiragana and an
/// emoji. The font of the layout has the Latin only.
pub const FALLBACK_TEXT: &str = "Aa \u{05E9}\u{05DC}\u{05D5}\u{05DD} \u{4E2D}\u{6587} \u{3053}\u{3093} \u{1F600}";

/// The layout of the fallback scene.
pub fn fallback_layout() -> TextLayout {
    let foreground: Rc<dyn IBrush> = Rc::new(solid(INK));

    TextLayout::new(
        FALLBACK_TEXT,
        embedded_typeface("Inter"),
        TextLayoutOptions {
            font_size: 26.0,
            foreground: Some(foreground),
            text_wrapping: TextWrapping::Wrap,
            max_width: 180.0,
            ..TextLayoutOptions::default()
        },
    )
}

/// A text of several scripts laid out in a font that has only one of them:
/// the rest is drawn in the fonts the font manager of each backend falls
/// back to.
fn mixed_scripts_with_fallback(backend: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    with_text_services(backend, |_| {
        let layout = fallback_layout();
        layout.draw(&mut ContextSink::new(context), Point::new(10.0, 20.0));
        layout.dispose();
    });
}

/// The bold, the oblique and both simulations.
fn simulations(backend: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    with_text_services(backend, |font_manager| {
        let brush = solid(INK);
        let rows = [
            (FontSimulations::None, 40.0),
            (FontSimulations::Bold, 82.0),
            (FontSimulations::Oblique, 124.0),
            (FontSimulations::Bold | FontSimulations::Oblique, 166.0),
        ];

        for (font_simulations, baseline) in rows {
            let inter = glyph_typeface(font_manager, INTER, font_simulations);
            draw_line_of_text(backend, context, &inter, "Simulated 12", 28.0, 0, Point::new(10.0, baseline), &brush);
        }
    });
}

/// The weight axis of a variable font: thin, the default and heavy.
fn variable_axis(backend: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    with_text_services(backend, |_| {
        let brush = solid(INK);

        for (weight, baseline) in [(100.0, 45.0), (400.0, 100.0), (900.0, 155.0)] {
            let typeface = variable_typeface(backend, INTER_VARIABLE, &[("wght", weight)]);
            let glyph_typeface = GlyphTypeface::new(typeface, FontSimulations::None).expect("the font has its tables");
            draw_line_of_text(backend, context, &glyph_typeface, "Variable", 40.0, 0, Point::new(10.0, baseline), &brush);
        }
    });
}

/// The layout of the decorations scene: descenders under an underline, and
/// a strikethrough.
pub fn decorations_layout(text_decorations: TextDecorationCollection) -> TextLayout {
    let foreground: Rc<dyn IBrush> = Rc::new(solid(NAVY));

    TextLayout::new(
        "jumping gypsy",
        embedded_typeface("Inter"),
        TextLayoutOptions {
            font_size: 28.0,
            foreground: Some(foreground),
            text_decorations: Some(text_decorations),
            ..TextLayoutOptions::default()
        },
    )
}

/// An underline, a strikethrough and an overline.
fn decorations(backend: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    with_text_services(backend, |_| {
        let rows = [
            (TextDecorations::underline(), 20.0),
            (TextDecorations::strikethrough(), 80.0),
            (TextDecorations::overline(), 140.0),
        ];

        for (text_decorations, top) in rows {
            let layout = decorations_layout(text_decorations);
            layout.draw(&mut ContextSink::new(context), Point::new(8.0, top));
            layout.dispose();
        }
    });
}

/// A run under a rotation, and one under a rotation and a scale.
fn rotated(backend: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    with_text_services(backend, |font_manager| {
        let inter = glyph_typeface(font_manager, INTER, FontSimulations::None);
        let brush = solid(INK);

        context.set_transform(Matrix::create_rotation(-0.45) * Matrix::create_translation(20.0, 120.0));
        draw_line_of_text(backend, context, &inter, "Rotated text", 26.0, 0, Point::new(0.0, 0.0), &brush);

        context.set_transform(
            Matrix::create_scale(1.6, 1.6) * Matrix::create_rotation(0.6) * Matrix::create_translation(60.0, 40.0),
        );
        draw_line_of_text(backend, context, &inter, "and scaled", 14.0, 0, Point::new(0.0, 0.0), &brush);

        context.set_transform(Matrix::IDENTITY);
    });
}

/// A run under a scale alone: the size the glyphs are drawn at is not the
/// em size of the run.
fn scaled(backend: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    with_text_services(backend, |font_manager| {
        let inter = glyph_typeface(font_manager, INTER, FontSimulations::None);
        let brush = solid(INK);

        context.set_transform(Matrix::create_scale(2.5, 2.5));
        draw_line_of_text(backend, context, &inter, "Scale 2.5", 12.0, 0, Point::new(4.0, 24.0), &brush);
        context.set_transform(Matrix::create_scale(0.5, 0.5));
        draw_line_of_text(backend, context, &inter, "Scale one half", 48.0, 0, Point::new(20.0, 280.0), &brush);
        context.set_transform(Matrix::IDENTITY);
    });
}

/// Large text behind a rounded clip.
fn clipped(backend: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    with_text_services(backend, |font_manager| {
        let inter = glyph_typeface(font_manager, INTER, FontSimulations::None);
        let brush = solid(NAVY);

        context.push_clip_rounded(RoundedRect::from_corner_radius(
            Rect::new(30.0, 40.0, 140.0, 110.0),
            CornerRadius::uniform(40.0),
        ));
        draw_line_of_text(backend, context, &inter, "Clip", 100.0, 0, Point::new(4.0, 130.0), &brush);
        draw_line_of_text(backend, context, &inter, "clipped text", 20.0, 0, Point::new(20.0, 60.0), &brush);
        context.pop_clip();
    });
}

/// Text painted with a linear gradient over the bounds of its run.
fn gradient(backend: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    with_text_services(backend, |font_manager| {
        let inter = glyph_typeface(font_manager, INTER, FontSimulations::None);
        let brush = ImmutableLinearGradientBrush::new(
            &[
                ImmutableGradientStop::new(0.0, Colors::RED),
                ImmutableGradientStop::new(0.5, Color::from_argb(255, 240, 140, 20)),
                ImmutableGradientStop::new(1.0, Colors::BLUE),
            ],
            1.0,
            None,
            None,
            GradientSpreadMethod::Pad,
            Some(RelativePoint::new(0.0, 0.0, RelativeUnit::Relative)),
            Some(RelativePoint::new(1.0, 1.0, RelativeUnit::Relative)),
            None,
        );

        draw_line_of_text(backend, context, &inter, "Grad", 80.0, 0, Point::new(6.0, 90.0), &brush);
        draw_line_of_text(backend, context, &inter, "gradient brush", 26.0, 0, Point::new(8.0, 160.0), &brush);
    });
}

/// Text without anti-aliasing, at whole pixel positions.
fn aliased(backend: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    with_text_services(backend, |font_manager| {
        let inter = glyph_typeface(font_manager, INTER, FontSimulations::None);
        let brush = solid(INK);

        context.push_render_options(RenderOptions { edge_mode: EdgeMode::Aliased, ..RenderOptions::default() });
        draw_line_of_text(backend, context, &inter, "Aliased", 44.0, 0, Point::new(8.0, 70.0), &brush);
        draw_line_of_text(backend, context, &inter, "aliased text 123", 18.0, 0, Point::new(8.3, 130.4), &brush);
        context.pop_render_options();
    });
}

/// Translucent text over a shape and under an opacity.
fn translucent(backend: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    with_text_services(backend, |font_manager| {
        let inter = glyph_typeface(font_manager, INTER, FontSimulations::None);

        context.draw_rectangle(
            Some(&solid(Color::from_argb(255, 240, 140, 20))),
            None,
            RoundedRect::from_rect(Rect::new(20.0, 30.0, 160.0, 60.0)),
            &BoxShadows::default(),
        );
        let pen = ImmutablePen::new(Some(Rc::new(solid(NAVY))), 3.0, None, PenLineCap::Flat, PenLineJoin::Miter, 10.0);
        context.draw_line(Some(&pen), Point::new(10.0, 140.0), Point::new(190.0, 140.0));

        let half = solid(Color::from_argb(128, 20, 40, 120));
        draw_line_of_text(backend, context, &inter, "Half", 60.0, 0, Point::new(30.0, 85.0), &half);

        context.push_opacity(0.4, None);
        draw_line_of_text(backend, context, &inter, "opacity", 44.0, 0, Point::new(20.0, 155.0), &solid(INK));
        context.pop_opacity();
    });
}

/// The text scenes, with the bound of each: the share of pixels that may
/// differ from the Skia backend by more than the tolerance.
///
/// The bounds of text are wider than those of shapes: on macOS Skia draws
/// glyphs with the rasterizer of the system, which makes stems heavier than
/// the outline of the font is, and the renderers of the Vello project fill
/// the outline.
pub fn text_scenes() -> Vec<Scene> {
    vec![
        Scene { name: "text_latin_sizes", draw: latin_sizes, bound: 6.41 },
        Scene { name: "text_interface_sizes", draw: interface_sizes, bound: 4.88 },
        Scene { name: "text_mixed_scripts", draw: mixed_scripts, bound: 5.00 },
        Scene { name: "text_mixed_scripts_with_fallback", draw: mixed_scripts_with_fallback, bound: 8.99 },
        Scene { name: "text_simulations", draw: simulations, bound: 8.26 },
        Scene { name: "text_variable_axis", draw: variable_axis, bound: 7.71 },
        Scene { name: "text_decorations", draw: decorations, bound: 6.45 },
        Scene { name: "text_rotated", draw: rotated, bound: 2.23 },
        Scene { name: "text_scaled", draw: scaled, bound: 2.70 },
        Scene { name: "text_clipped", draw: clipped, bound: 3.17 },
        Scene { name: "text_gradient", draw: gradient, bound: 5.17 },
        Scene { name: "text_aliased", draw: aliased, bound: 0.34 },
        Scene { name: "text_translucent", draw: translucent, bound: 1.83 },
    ]
}
