//! Text tests: the Skia font manager and glyph runs with the HarfBuzz
//! shaper, on real fonts.

use crate::*;
use ferroui_base::media::fonts::OpenTypeTag;
use ferroui_base::media::immutable::{ImmutablePen, ImmutableSolidColorBrush};
use ferroui_base::media::text_formatting::{ITextDrawingSink, TextLayout, TextLayoutOptions, TextShaper, TextShaperOptions};
use ferroui_base::media::{
    BaselinePixelAlignment, BoxShadows, CharacterHit, Colors, EdgeMode, FlowDirection, FontFamily, FontFeature,
    FontManager, FontSimulations, FontStretch, FontStyle, FontWeight, GlyphInfoList, GlyphRun, IBrush, IPen,
    RenderOptions, TextHintingMode, TextOptions, TextRenderingMode, Typeface,
};
use ferroui_base::platform::{
    IAssetLoader, IDrawingContextImpl, IFontManagerImpl,
    IPlatformRenderInterface, IRenderTargetBitmapImpl, PixelFormat, StandardAssetLoader,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::utilities::{CultureInfo, ReadOnlyMemory};
use ferroui_base::{FerroLocator, LocatorExtensions, Matrix, PixelSize, Point, Rect, RoundedRect, Vector};
use std::cell::RefCell;
use std::io::Read;
use std::rc::Rc;

const FONTS: &str = "resm:FerroUI.Skia.UnitTests.Fonts?assembly=ferroui-skia";
const ASSETS: &str = "resm:FerroUI.Skia.UnitTests.Assets?assembly=ferroui-skia";

/// Starts a scope with the Skia render interface and font manager and the
/// HarfBuzz text shaper registered; dispose the result to leave it.
pub(crate) fn start() -> Rc<dyn IDisposable> {
    crate::unit_tests::register_test_assets();

    let scope = FerroLocator::enter_scope();

    let asset_loader: Rc<dyn IAssetLoader> = Rc::new(StandardAssetLoader::new(None));
    FerroLocator::current_mutable().bind::<dyn IAssetLoader>().to_constant(asset_loader);

    SkiaPlatform::initialize();
    ferroui_harfbuzz::HarfBuzzPlatform::initialize();

    scope
}

pub(crate) fn utf16(text: &str) -> ReadOnlyMemory<u16> {
    ReadOnlyMemory::from_str(text)
}

pub(crate) fn test_typeface(family: &str) -> Typeface {
    // Cascadia Code is a font of the test project, the others are render
    // test assets.
    let source = if family == "Cascadia Code" { FONTS } else { ASSETS };

    Typeface::new(FontFamily::parse(&format!("{source}#{family}")).unwrap())
}


fn glyph_clusters(buffer: &ferroui_base::media::text_formatting::ShapedBuffer) -> Vec<i32> {
    buffer.glyph_infos().iter().map(|glyph| glyph.glyph_cluster).collect()
}


// ---------------------------------------------------------------------------
// Font manager and typefaces
// ---------------------------------------------------------------------------

#[test]
fn font_manager_reports_system_fonts() {
    let font_manager = FontManagerImpl::new();

    let default_family = font_manager.get_default_font_family_name();
    assert!(!default_family.is_empty());

    let families = font_manager.get_installed_font_family_names(false);
    assert!(!families.is_empty());
    assert!(families.iter().any(|family| family == &default_family), "{default_family} is installed");
    // Asking for an update gives the same set.
    assert_eq!(families.len(), font_manager.get_installed_font_family_names(true).len());

    let typefaces = font_manager.try_get_family_typefaces(&default_family).expect("the default family has styles");
    assert!(!typefaces.is_empty());
    assert!(typefaces.iter().all(|typeface| typeface.font_family().name() == default_family));

    assert!(font_manager.try_get_family_typefaces("No Such Family 4f1c").is_none());
}

#[test]
fn font_manager_creates_system_typefaces() {
    let font_manager = FontManagerImpl::new();
    let default_family = font_manager.get_default_font_family_name();

    let regular = font_manager
        .try_create_glyph_typeface(&default_family, FontStyle::Normal, FontWeight::Normal, FontStretch::Normal)
        .expect("the default family resolves");
    assert_eq!(default_family, regular.family_name());
    assert_eq!(FontWeight::Normal, regular.weight());
    assert_eq!(FontStyle::Normal, regular.style());
    assert_eq!(FontSimulations::None, regular.font_simulations());

    // A bold request is either served by a bold face or simulated.
    let bold = font_manager
        .try_create_glyph_typeface(&default_family, FontStyle::Normal, FontWeight::Bold, FontStretch::Normal)
        .expect("the default family resolves");
    let skia_bold = SkiaTypeface::try_get(&*bold).expect("a typeface of this backend");
    assert_eq!(
        !skia_bold.sk_typeface().is_bold(),
        bold.font_simulations().contains(FontSimulations::Bold),
        "bold is simulated exactly when the matched face is not bold"
    );

    let italic = font_manager
        .try_create_glyph_typeface(&default_family, FontStyle::Italic, FontWeight::Normal, FontStretch::Normal)
        .expect("the default family resolves");
    let skia_italic = SkiaTypeface::try_get(&*italic).expect("a typeface of this backend");
    assert_eq!(
        !skia_italic.sk_typeface().is_italic(),
        italic.font_simulations().contains(FontSimulations::Oblique)
    );
}

#[test]
fn font_manager_creates_typefaces_from_streams() {
    let font_manager = FontManagerImpl::new();
    let font_data: &[u8] = include_bytes!("test_assets/assets/Inter-Regular.ttf");

    let typeface = font_manager
        .try_create_glyph_typeface_from_stream(&mut &font_data[..], FontSimulations::Bold | FontSimulations::Oblique)
        .expect("the font loads");

    assert_eq!("Inter", typeface.family_name());
    assert_eq!(FontWeight::Normal, typeface.weight());
    assert_eq!(FontStyle::Normal, typeface.style());
    assert_eq!(FontStretch::Normal, typeface.stretch());
    assert_eq!(FontSimulations::Bold | FontSimulations::Oblique, typeface.font_simulations());

    // Table access: the `head` table is 54 bytes and starts with version 1.0.
    let head = typeface.try_get_table(OpenTypeTag::parse("head")).expect("every font has a head table");
    assert_eq!(54, head.len());
    assert_eq!([0, 1, 0, 0], head.span()[..4]);
    // The magic number of the head table.
    assert_eq!([0x5F, 0x0F, 0x3C, 0xF5], head.span()[12..16]);
    assert!(typeface.try_get_table(OpenTypeTag::parse("zzzz")).is_none());

    // The stream gives the font data back.
    let mut stream = typeface.try_get_stream().expect("the font data is available");
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes).unwrap();
    assert_eq!(font_data.len(), bytes.len());
    assert_eq!(font_data[..12], bytes[..12]);

    // The simulations shape the Skia font.
    let skia_typeface = SkiaTypeface::try_get(&*typeface).expect("a typeface of this backend");
    let font = skia_typeface.create_sk_font(20.0);
    assert_eq!(20.0, font.size());
    assert!(font.is_embolden());
    assert!(font.is_linear_metrics());
    assert_eq!(-0.3, font.skew_x());

    // Garbage is rejected.
    assert!(font_manager.try_create_glyph_typeface_from_stream(&mut &[1u8, 2, 3][..], FontSimulations::None).is_none());
}

#[test]
fn font_manager_matches_characters_with_fallback_fonts() {
    let font_manager = FontManagerImpl::new();

    for (codepoint, what) in [(0x0623, "Arabic"), (0x4E2D, "CJK"), (0x05D0, "Hebrew"), (0x1F600, "emoji")] {
        let typeface = font_manager
            .try_match_character(codepoint, FontStyle::Normal, FontWeight::Normal, FontStretch::Normal, None, None)
            .unwrap_or_else(|| panic!("a system font covers {what}"));
        let skia_typeface = SkiaTypeface::try_get(&*typeface).unwrap();
        assert_ne!(0, skia_typeface.sk_typeface().unichar_to_glyph(codepoint), "{what} glyph in {}", typeface.family_name());
        assert_eq!(FontSimulations::None, typeface.font_simulations());
    }

    // Other styles and an explicit culture go through the general path.
    let bold_italic = font_manager.try_match_character(
        'A' as i32,
        FontStyle::Italic,
        FontWeight::Bold,
        FontStretch::Condensed,
        Some(&font_manager.get_default_font_family_name()),
        Some(&CultureInfo::get_culture_info("en-US")),
    );
    assert!(bold_italic.is_some());
}

#[test]
fn skia_platform_registers_the_font_manager() {
    let scope = start();

    assert!(FerroLocator::current().get_service::<dyn IFontManagerImpl>().is_some());

    let font_manager = FontManager::current();
    let default_typeface = Typeface::default_typeface().glyph_typeface();
    assert_eq!(font_manager.default_font_family().name(), default_typeface.family_name());
    assert!(default_typeface.metrics().design_em_height > 0);
    assert!(default_typeface.glyph_count() > 0);
    assert_ne!(0, default_typeface.character_to_glyph_map().get_glyph('A' as i32));

    // Embedded fonts load through the font manager's stream path.
    let cascadia = test_typeface("Cascadia Code").glyph_typeface();
    assert_eq!("Cascadia Code", cascadia.family_name());
    assert!(SkiaTypeface::try_get(&**cascadia.platform_typeface()).is_some());

    scope.dispose();
}

// ---------------------------------------------------------------------------
// Shaping details
// ---------------------------------------------------------------------------

#[test]
fn shaper_applies_ligatures_and_font_features() {
    let scope = start();

    let glyph_typeface = test_typeface("Cascadia Code").glyph_typeface();
    let map = glyph_typeface.character_to_glyph_map();
    let plain = [map.get_glyph('-' as i32), map.get_glyph('>' as i32)];

    // Cascadia Code turns "->" into an arrow through contextual alternates.
    let ligated = TextShaper::current().shape_text(&utf16("->"), &TextShaperOptions::new(glyph_typeface.clone()));
    assert_eq!(2, ligated.length());
    assert_eq!(vec![0, 1], glyph_clusters(&ligated));
    assert_ne!(plain[0], ligated.get(0).glyph_index);
    assert_ne!(plain[1], ligated.get(1).glyph_index);

    // Turning the feature off gives the plain glyphs back.
    let features = Rc::new(vec![FontFeature::parse("-calt")]);
    let options = TextShaperOptions::with_all(glyph_typeface.clone(), 12.0, 0, None, 0.0, 0.0, Some(features));
    let unligated = TextShaper::current().shape_text(&utf16("->"), &options);
    assert_eq!(plain.to_vec(), unligated.glyph_indices().to_vec());

    scope.dispose();
}

#[test]
fn shaper_scales_advances_and_adds_letter_spacing() {
    let scope = start();

    // Cascadia Code is monospaced: 1200 of 2048 units per glyph.
    let glyph_typeface = test_typeface("Cascadia Code").glyph_typeface();
    let design_advance = glyph_typeface.try_get_horizontal_glyph_advance(glyph_typeface.character_to_glyph_map().get_glyph('a' as i32)).unwrap() as f64;
    let em = glyph_typeface.metrics().design_em_height as f64;

    let at_12 = TextShaper::current().shape_text(&utf16("abc"), &TextShaperOptions::new(glyph_typeface.clone()));
    for glyph in at_12.glyph_infos().iter() {
        assert!((glyph.glyph_advance - design_advance * 12.0 / em).abs() < 1e-9);
        assert_eq!(Vector::new(0.0, 0.0), Vector::new(glyph.glyph_offset.x, glyph.glyph_offset.y.abs()));
    }

    let spaced = TextShaperOptions::with_all(glyph_typeface.clone(), 24.0, 0, None, 0.0, 3.0, None);
    let at_24 = TextShaper::current().shape_text(&utf16("abc"), &spaced);
    for glyph in at_24.glyph_infos().iter() {
        assert!((glyph.glyph_advance - (design_advance * 24.0 / em + 3.0)).abs() < 1e-9);
    }

    // Empty text gives an empty buffer.
    let empty = TextShaper::current().shape_text(&utf16(""), &TextShaperOptions::new(glyph_typeface));
    assert_eq!(0, empty.length());

    scope.dispose();
}

#[test]
fn shaper_joins_arabic_and_keeps_visual_order() {
    let scope = start();

    let glyph_typeface = test_typeface("Noto Sans Arabic").glyph_typeface();
    assert_eq!("Noto Sans Arabic", glyph_typeface.family_name());

    // "سلام": four letters that join.
    let text = "\u{0633}\u{0644}\u{0627}\u{0645}";
    let options = TextShaperOptions::with_all(glyph_typeface.clone(), 12.0, 1, None, 0.0, 0.0, None);
    let buffer = TextShaper::current().shape_text(&utf16(text), &options);

    assert!(!buffer.is_left_to_right());
    // Visual order: the clusters run from the end of the text to the start.
    let clusters = glyph_clusters(&buffer);
    assert!(clusters.windows(2).all(|pair| pair[0] >= pair[1]), "descending clusters: {clusters:?}");
    assert_eq!(0, *clusters.last().unwrap());

    // Joining replaces the nominal glyphs with positional forms (lam and
    // alef even form a ligature).
    let map = glyph_typeface.character_to_glyph_map();
    let nominal: Vec<u16> = text.chars().map(|c| map.get_glyph(c as i32)).collect();
    assert!(buffer.glyph_indices().iter().any(|glyph| !nominal.contains(glyph)), "positional forms are used");
    assert!(buffer.length() < 4, "lam-alef is a ligature: {clusters:?}");

    // Shaping a slice uses the surrounding text as context: the middle
    // letters keep their joining forms.
    let whole = utf16(text);
    let middle = TextShaper::current().shape_text(&whole.slice(1, 1), &options);
    let isolated = TextShaper::current().shape_text(&utf16("\u{0644}"), &options);
    assert_eq!(0, middle.get(0).glyph_cluster);
    assert_ne!(isolated.get(0).glyph_index, middle.get(0).glyph_index, "the lam joins its neighbours");

    scope.dispose();
}

#[test]
fn glyph_typefaces_get_a_harf_buzz_shaper_typeface() {
    let scope = start();

    let glyph_typeface = Typeface::default_typeface().glyph_typeface();
    let shaper_typeface = glyph_typeface.text_shaper_typeface();
    assert!(shaper_typeface.as_any().downcast_ref::<ferroui_harfbuzz::HarfBuzzTypeface>().is_some());

    scope.dispose();
}

// ---------------------------------------------------------------------------
// Glyph runs
// ---------------------------------------------------------------------------

fn shaped_glyph_run(text: &str, typeface: &Typeface, em_size: f64, origin: Point) -> Rc<GlyphRun> {
    let glyph_typeface = typeface.glyph_typeface();
    let options = TextShaperOptions::with_all(glyph_typeface.clone(), em_size, 0, None, 0.0, 0.0, None);
    let buffer = TextShaper::current().shape_text(&utf16(text), &options);
    let glyph_infos = buffer.glyph_infos().to_vec();

    GlyphRun::new(
        glyph_typeface,
        em_size,
        utf16(text),
        GlyphInfoList::from(glyph_infos),
        Some(origin),
        0,
    )
}

struct TextTarget {
    bitmap: Rc<dyn IRenderTargetBitmapImpl>,
}

impl TextTarget {
    fn new(width: i32, height: i32) -> Self {
        let render_interface = FerroLocator::current().get_service::<dyn IPlatformRenderInterface>().unwrap();
        Self {
            bitmap: render_interface.create_render_target_bitmap(PixelSize::new(width, height), Vector::new(96.0, 96.0)),
        }
    }

    fn draw(&self, f: impl FnOnce(&mut dyn IDrawingContextImpl)) {
        let mut context = self.bitmap.create_drawing_context();
        context.clear(Colors::WHITE);
        f(&mut *context);
        context.dispose();
    }

    /// All pixels as (r, g, b) rows.
    fn pixels(&self) -> Vec<Vec<(u8, u8, u8)>> {
        let size = self.bitmap.pixel_size();
        let framebuffer = self.bitmap.lock();
        let row_bytes = framebuffer.row_bytes() as usize;
        let bgra = framebuffer.format() == PixelFormat::BGRA8888;
        let mut rows = Vec::new();
        framebuffer.with_data(&mut |data| {
            for y in 0..size.height as usize {
                let row = &data[y * row_bytes..y * row_bytes + size.width as usize * 4];
                rows.push(
                    row.chunks_exact(4)
                        .map(|p| if bgra { (p[2], p[1], p[0]) } else { (p[0], p[1], p[2]) })
                        .collect(),
                );
            }
        });
        framebuffer.dispose();
        rows
    }

    /// The bounds of the pixels matching `is_ink`, and their count.
    fn ink(&self, is_ink: impl Fn((u8, u8, u8)) -> bool) -> (Rect, usize) {
        let (mut left, mut top, mut right, mut bottom, mut count) = (i32::MAX, i32::MAX, -1, -1, 0);
        for (y, row) in self.pixels().iter().enumerate() {
            for (x, pixel) in row.iter().enumerate() {
                if is_ink(*pixel) {
                    left = left.min(x as i32);
                    right = right.max(x as i32);
                    top = top.min(y as i32);
                    bottom = bottom.max(y as i32);
                    count += 1;
                }
            }
        }
        if count == 0 {
            return (Rect::default(), 0);
        }
        (Rect::new(left as f64, top as f64, (right - left + 1) as f64, (bottom - top + 1) as f64), count)
    }
}

fn is_dark((r, g, b): (u8, u8, u8)) -> bool {
    (r as u32 + g as u32 + b as u32) < 384
}

fn is_not_white((r, g, b): (u8, u8, u8)) -> bool {
    r < 250 || g < 250 || b < 250
}

#[test]
fn glyph_run_is_created_measured_and_drawn() {
    let scope = start();

    let typeface = test_typeface("Inter");
    let origin = Point::new(10.0, 60.0);
    let glyph_run = shaped_glyph_run("Hxg", &typeface, 40.0, origin);

    let platform_impl = glyph_run.platform_impl();
    assert!(platform_impl.as_any().downcast_ref::<GlyphRunImpl>().is_some());
    assert_eq!(40.0, platform_impl.font_rendering_em_size());
    assert_eq!(origin, platform_impl.baseline_origin());

    // The conservative bounds: start near the origin, rise above the
    // baseline for the capital and descend below it for the "g".
    let bounds = platform_impl.bounds();
    assert!(bounds.x >= origin.x - 1.0 && bounds.x < origin.x + 6.0, "{bounds}");
    assert!(bounds.y < origin.y - 25.0 && bounds.y > origin.y - 40.0, "{bounds}");
    assert!(bounds.bottom() > origin.y + 5.0 && bounds.bottom() < origin.y + 15.0, "{bounds}");
    assert!(bounds.width > 60.0 && bounds.width < 90.0, "{bounds}");

    // The same bounds for a similar run after blobs were created
    // (Similar_Runs_Have_Same_InkBounds_After_Blob_Creation).
    let target = TextTarget::new(120, 90);
    let foreground = ImmutableSolidColorBrush::new(Colors::BLACK);
    target.draw(|context| context.draw_glyph_run(Some(&foreground), &*platform_impl));
    let again = shaped_glyph_run("Hxg", &typeface, 40.0, origin);
    assert_eq!(bounds, again.platform_impl().bounds());
    assert_eq!(glyph_run.ink_bounds(), again.ink_bounds());

    // The ink lies inside the bounds and fills most of their extent.
    let (ink, count) = target.ink(is_dark);
    assert!(count > 300, "the glyphs left ink: {count}");
    assert!(bounds.inflate(1.0).contains_rect(ink), "ink {ink} inside bounds {bounds}");
    assert!(ink.width > bounds.width - 8.0 && ink.height > bounds.height - 4.0, "ink {ink} fills bounds {bounds}");

    // The stems of the "H" cross a band above the baseline.
    let intersections = platform_impl.get_intersections(-20.0, -15.0);
    assert!(intersections.len() >= 4, "{intersections:?}");
    assert_eq!(0, intersections.len() % 2);

    scope.dispose();
}

#[test]
fn glyph_run_geometry_outlines_the_glyphs() {
    let scope = start();

    let typeface = test_typeface("Inter");
    let origin = Point::new(20.0, 70.0);
    let glyph_run = shaped_glyph_run("Ho", &typeface, 50.0, origin);

    let render_interface = FerroLocator::current().get_service::<dyn IPlatformRenderInterface>().unwrap();
    let geometry = render_interface.build_glyph_run_geometry(&glyph_run);

    let geometry_bounds = geometry.bounds();
    let run_bounds = glyph_run.platform_impl().bounds();
    assert!(run_bounds.inflate(1.5).contains_rect(geometry_bounds), "{geometry_bounds} in {run_bounds}");
    assert!(geometry_bounds.width > run_bounds.width - 8.0);

    // Inside the left stem of the "H" but not in its counter; the "o" has a
    // hole.
    let left = geometry_bounds.x;
    assert!(geometry.fill_contains(Point::new(left + 2.0, origin.y - 18.0)));
    assert!(!geometry.fill_contains(Point::new(left + 14.0, origin.y - 30.0)));

    // Filling the geometry and drawing the run put ink in the same place.
    let foreground = ImmutableSolidColorBrush::new(Colors::BLACK);
    let filled = TextTarget::new(120, 100);
    filled.draw(|context| context.draw_geometry(Some(&foreground), None, &*geometry));
    let drawn = TextTarget::new(120, 100);
    drawn.draw(|context| context.draw_glyph_run(Some(&foreground), &*glyph_run.platform_impl()));

    let (filled_ink, filled_count) = filled.ink(is_dark);
    let (drawn_ink, drawn_count) = drawn.ink(is_dark);
    assert!(filled_count > 400 && drawn_count > 400);
    assert!((filled_ink.x - drawn_ink.x).abs() <= 1.0 && (filled_ink.y - drawn_ink.y).abs() <= 1.0);
    assert!((filled_ink.width - drawn_ink.width).abs() <= 2.0 && (filled_ink.height - drawn_ink.height).abs() <= 2.0);
    let ratio = filled_count as f64 / drawn_count as f64;
    assert!((0.85..1.15).contains(&ratio), "similar coverage: {filled_count} vs {drawn_count}");

    scope.dispose();
}

#[test]
fn text_options_select_the_glyph_rendering() {
    let scope = start();

    let typeface = test_typeface("Inter");
    let glyph_run = shaped_glyph_run("Sample", &typeface, 22.0, Point::new(4.3, 30.4));
    let platform_impl = glyph_run.platform_impl();
    let foreground = ImmutableSolidColorBrush::new(Colors::BLACK);

    let render = |text_options: Option<TextOptions>, render_options: RenderOptions| {
        let target = TextTarget::new(110, 44);
        target.draw(|context| {
            context.push_render_options(render_options);
            if let Some(text_options) = text_options {
                context.push_text_options(text_options);
            }
            context.draw_glyph_run(Some(&foreground), &*platform_impl);
            if text_options.is_some() {
                context.pop_text_options();
            }
            context.pop_render_options();
        });
        target.pixels()
    };

    let is_gray = |pixels: &Vec<Vec<(u8, u8, u8)>>| pixels.iter().flatten().all(|(r, g, b)| r == g && g == b);
    let is_bilevel =
        |pixels: &Vec<Vec<(u8, u8, u8)>>| pixels.iter().flatten().all(|(r, _, _)| *r == 0 || *r == 255);

    let with_mode = |text_rendering_mode| TextOptions { text_rendering_mode, ..TextOptions::default() };

    // Aliased text has no intermediate coverage.
    let alias = render(Some(with_mode(TextRenderingMode::Alias)), RenderOptions::default());
    assert!(is_bilevel(&alias) && is_gray(&alias));

    // Grayscale antialiasing has intermediate levels.
    let antialias = render(Some(with_mode(TextRenderingMode::Antialias)), RenderOptions::default());
    assert!(is_gray(&antialias) && !is_bilevel(&antialias));

    // Render target bitmaps keep subpixel rendering enabled; whether Skia
    // produces coloured fringes depends on the surface, but the mode is a
    // distinct blob.
    let subpixel = render(Some(with_mode(TextRenderingMode::SubpixelAntialias)), RenderOptions::default());
    assert!(!is_bilevel(&subpixel));

    // Without text options the edge mode of the render options decides.
    let aliased_edges =
        render(None, RenderOptions { edge_mode: EdgeMode::Aliased, ..RenderOptions::default() });
    assert_eq!(alias, aliased_edges);

    // The deprecated text rendering mode of the render options is used when
    // the text options leave it unspecified.
    let legacy = render(
        Some(TextOptions::default()),
        RenderOptions { text_rendering_mode: TextRenderingMode::Alias, ..RenderOptions::default() },
    );
    assert_eq!(alias, legacy);

    // Pushed text options merge: the inner unspecified mode inherits.
    let target = TextTarget::new(110, 44);
    target.draw(|context| {
        context.push_text_options(with_mode(TextRenderingMode::Alias));
        context.push_text_options(TextOptions { text_hinting_mode: TextHintingMode::None, ..TextOptions::default() });
        context.draw_glyph_run(Some(&foreground), &*platform_impl);
        context.pop_text_options();
        context.pop_text_options();
    });
    assert!(is_bilevel(&target.pixels()));

    // Hinting and baseline alignment are part of the blob key: each
    // combination gives its own blob.
    let glyph_run_impl = platform_impl.as_any().downcast_ref::<GlyphRunImpl>().unwrap();
    let blob = |text_hinting_mode, baseline_pixel_alignment| {
        glyph_run_impl
            .get_text_blob(
                TextOptions { text_rendering_mode: TextRenderingMode::Antialias, text_hinting_mode, baseline_pixel_alignment },
                RenderOptions::default(),
            )
            .unwrap()
            .unique_id()
    };
    let strong = blob(TextHintingMode::Strong, BaselinePixelAlignment::Aligned);
    assert_eq!(strong, blob(TextHintingMode::Strong, BaselinePixelAlignment::Aligned), "cached");
    assert_ne!(strong, blob(TextHintingMode::None, BaselinePixelAlignment::Aligned));
    assert_ne!(strong, blob(TextHintingMode::Light, BaselinePixelAlignment::Aligned));
    assert_ne!(strong, blob(TextHintingMode::Strong, BaselinePixelAlignment::Unaligned));

    scope.dispose();
}

// ---------------------------------------------------------------------------
// Text layout through the real font manager, shaper and drawing context
// ---------------------------------------------------------------------------

/// A text drawing sink over a platform drawing context: what the drawing
/// context does for text.
struct ContextSink {
    context: RefCell<Box<dyn IDrawingContextImpl>>,
    transforms: RefCell<Vec<Matrix>>,
    glyph_runs: RefCell<Vec<(Rect, String)>>,
}

impl ContextSink {
    fn new(context: Box<dyn IDrawingContextImpl>) -> Self {
        Self { context: RefCell::new(context), transforms: RefCell::new(Vec::new()), glyph_runs: RefCell::new(Vec::new()) }
    }
}

impl ITextDrawingSink for ContextSink {
    fn draw_glyph_run(&mut self, foreground: Option<&Rc<dyn IBrush>>, glyph_run: &Rc<GlyphRun>) {
        let mut context = self.context.borrow_mut();
        let platform_impl = glyph_run.platform_impl();
        let transform = context.transform();
        self.glyph_runs.borrow_mut().push((
            platform_impl.bounds().transform_to_aabb(transform),
            glyph_run.glyph_typeface().family_name().to_owned(),
        ));
        context.draw_glyph_run(foreground.map(|brush| &**brush), &*platform_impl);
    }

    fn draw_rectangle(&mut self, brush: Option<&Rc<dyn IBrush>>, pen: Option<&Rc<dyn IPen>>, rect: Rect) {
        self.context.borrow_mut().draw_rectangle(
            brush.map(|brush| &**brush),
            pen.map(|pen| &**pen),
            RoundedRect::from_rect(rect),
            &BoxShadows::default(),
        );
    }

    fn draw_line(&mut self, pen: &Rc<dyn IPen>, p1: Point, p2: Point) {
        self.context.borrow_mut().draw_line(Some(&**pen), p1, p2);
    }

    fn push_transform(&mut self, matrix: Matrix) {
        let mut context = self.context.borrow_mut();
        let current = context.transform();
        self.transforms.borrow_mut().push(current);
        context.set_transform(matrix * current);
    }

    fn pop_transform(&mut self) {
        let previous = self.transforms.borrow_mut().pop().expect("a pushed transform");
        self.context.borrow_mut().set_transform(previous);
    }
}

#[test]
fn text_layout_draws_mixed_direction_ligature_and_emoji_text() {
    let scope = start();

    // Latin with a programming ligature, Hebrew (right-to-left), and an
    // emoji and a CJK ideograph that only fallback fonts cover.
    let text = "ab -> \u{05E9}\u{05DC}\u{05D5}\u{05DD} \u{1F600} \u{4E2D} xy";
    let units: Vec<u16> = text.encode_utf16().collect();
    let foreground: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::new(Colors::BLACK));

    let layout = TextLayout::new(
        text,
        test_typeface("Cascadia Code"),
        TextLayoutOptions { font_size: 32.0, foreground: Some(foreground), ..TextLayoutOptions::default() },
    );

    assert_eq!(1, layout.text_lines().len());
    let line = layout.text_lines()[0].clone();
    assert_eq!(units.len() as i32, line.length());
    assert!(layout.width() > 250.0 && layout.width() < 440.0, "width {}", layout.width());
    assert!(layout.height() > 32.0 && layout.height() < 60.0, "height {}", layout.height());
    assert!(layout.baseline() > 20.0 && layout.baseline() < layout.height());

    // Draw at an offset into a raster surface.
    let origin = Point::new(12.0, 20.0);
    let target = TextTarget::new(460, 100);
    let mut context = target.bitmap.create_drawing_context();
    context.clear(Colors::WHITE);
    let mut sink = ContextSink::new(context);
    layout.draw(&mut sink, origin);
    assert!(sink.transforms.borrow().is_empty(), "transform pushes are balanced");
    let glyph_runs = sink.glyph_runs.borrow().clone();
    sink.context.borrow_mut().dispose();

    // The fallback fonts: the emoji and the ideograph are not in Cascadia
    // Code (the Hebrew is).
    let families: Vec<&str> = glyph_runs.iter().map(|(_, family)| family.as_str()).collect();
    assert!(glyph_runs.len() >= 4, "runs: {families:?}");
    assert_eq!("Cascadia Code", families[0]);
    assert!(families.iter().any(|family| family.contains("Emoji")), "an emoji font was matched: {families:?}");
    assert!(families.iter().filter(|family| **family != "Cascadia Code").count() >= 2, "{families:?}");

    // The runs are laid out left to right without leaving the layout box.
    let layout_box = Rect::new(origin.x, origin.y, layout.width(), layout.height());
    for (bounds, family) in &glyph_runs {
        assert!(layout_box.inflate(6.0).contains_rect(*bounds), "{family} run {bounds} inside {layout_box}");
    }
    for pair in glyph_runs.windows(2) {
        assert!(pair[0].0.x < pair[1].0.x, "runs advance: {} then {}", pair[0].0, pair[1].0);
    }

    // The ink: non-empty, inside the layout box, and spanning most of it.
    let (ink, count) = target.ink(is_not_white);
    assert!(count > 1500, "ink pixels: {count}");
    assert!(layout_box.inflate(3.0).contains_rect(ink), "ink {ink} inside {layout_box}");
    assert!(ink.width > layout.width() * 0.9, "ink {ink} spans the layout width {}", layout.width());
    assert!(ink.x - origin.x < 6.0);

    // The emoji is drawn in colour; the text is black.
    let pixels = target.pixels();
    let coloured = pixels
        .iter()
        .flatten()
        .filter(|(r, g, b)| (*r as i32 - *b as i32).abs() > 60 || (*r as i32 - *g as i32).abs() > 60)
        .count();
    assert!(coloured > 100, "the emoji has colour: {coloured} pixels");
    let emoji_index = units.iter().position(|unit| *unit == 0xD83D).unwrap() as i32;
    let emoji_rect = layout.hit_test_text_position(emoji_index);
    let coloured_in_emoji = pixels
        .iter()
        .enumerate()
        .flat_map(|(y, row)| row.iter().enumerate().map(move |(x, pixel)| (x, y, *pixel)))
        .filter(|(x, y, (r, g, b))| {
            ((*r as i32 - *b as i32).abs() > 60 || (*r as i32 - *g as i32).abs() > 60)
                && emoji_rect.translate(Vector::new(origin.x, origin.y)).inflate(4.0).contains(Point::new(*x as f64, *y as f64))
        })
        .count();
    assert_eq!(coloured, coloured_in_emoji, "the colour is where the emoji is hit-tested");

    // --- Hit testing ---

    // Left-to-right characters advance to the right, one cell each
    // (Cascadia Code is monospaced: 1200/2048 em).
    let cell = 32.0 * 1200.0 / 2048.0;
    for index in 0..6 {
        let rect = layout.hit_test_text_position(index);
        assert!((rect.x - index as f64 * cell).abs() < 0.01, "character {index} at {}", rect.x);
        assert!((rect.width - cell).abs() < 0.01);
    }

    // The ligature keeps one cluster per character: both halves of "->"
    // can be hit.
    let arrow = units.iter().position(|unit| *unit == '-' as u16).unwrap() as i32;
    let hit = layout.hit_test_point(Point::new((arrow as f64 + 0.25) * cell, 10.0));
    assert!(hit.is_inside());
    assert_eq!(arrow, hit.text_position());
    assert_eq!(0, hit.character_hit().trailing_length());
    let hit = layout.hit_test_point(Point::new((arrow as f64 + 1.75) * cell, 10.0));
    assert_eq!(arrow + 1, hit.character_hit().first_character_index());
    assert_eq!(1, hit.character_hit().trailing_length());

    // The Hebrew word runs right to left: its first character is at the
    // right end of the word.
    let hebrew = units.iter().position(|unit| *unit == 0x05E9).unwrap() as i32;
    let first = layout.hit_test_text_position(hebrew);
    let last = layout.hit_test_text_position(hebrew + 3);
    assert!(first.x > last.x, "the first Hebrew letter ({}) is right of the last ({})", first.x, last.x);
    let word = layout.hit_test_text_range(hebrew, 4);
    assert_eq!(1, word.len());
    // The rectangle of a right-to-left character starts at its leading
    // (right) edge and extends to the left: its width is negative.
    assert!((first.width + cell).abs() < 0.01 && (last.width + cell).abs() < 0.01);
    assert!((word[0].x - (last.x + last.width)).abs() < 0.01 && (word[0].right() - first.x).abs() < 0.01);
    assert!((word[0].width - 4.0 * cell).abs() < 0.01);

    // A point near the leading (right) edge of the last Hebrew letter hits
    // the letter itself, a point near its trailing (left) edge hits it
    // trailing.
    let hit = layout.hit_test_point(Point::new(last.x + last.width * 0.25, 10.0));
    assert_eq!(hebrew + 3, hit.character_hit().first_character_index());
    assert_eq!(0, hit.character_hit().trailing_length());
    let hit = layout.hit_test_point(Point::new(last.x + last.width * 0.75, 10.0));
    assert_eq!(hebrew + 3, hit.character_hit().first_character_index());
    assert_eq!(1, hit.character_hit().trailing_length());

    // The emoji is one cluster of two code units.
    let hit = layout.hit_test_point(Point::new(emoji_rect.x + emoji_rect.width * 0.25, 10.0));
    assert_eq!(emoji_index, hit.character_hit().first_character_index());
    assert_eq!(emoji_rect, layout.hit_test_text_position(emoji_index + 1).with_x(emoji_rect.x).with_width(emoji_rect.width));

    // The distance of a character hit agrees with the hit-tested position.
    let distance = line.get_distance_from_character_hit(CharacterHit::new(3));
    assert!((distance - layout.hit_test_text_position(3).x).abs() < 0.01);
    assert_eq!(3, line.get_character_hit_from_distance(distance + 1.0).first_character_index());

    // Points outside the layout are reported as such.
    let outside = layout.hit_test_point(Point::new(layout.width() + 50.0, 10.0));
    assert!(!outside.is_inside());

    layout.dispose();
    scope.dispose();
}

#[test]
fn text_layout_right_to_left_paragraph_with_arabic() {
    let scope = start();

    let text = "\u{0633}\u{0644}\u{0627}\u{0645} abc";
    let foreground: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::new(Colors::BLACK));
    let layout = TextLayout::new(
        text,
        test_typeface("Noto Sans Arabic"),
        TextLayoutOptions {
            font_size: 28.0,
            foreground: Some(foreground),
            flow_direction: FlowDirection::RightToLeft,
            text_alignment: ferroui_base::media::TextAlignment::Start,
            max_width: 300.0,
            ..TextLayoutOptions::default()
        },
    );

    // In a right-to-left paragraph the Arabic word is at the right edge and
    // the Latin run to its left.
    let arabic = layout.hit_test_text_range(0, 4);
    let latin = layout.hit_test_text_range(5, 3);
    assert_eq!(1, arabic.len());
    assert_eq!(1, latin.len());
    assert!((arabic[0].right() - 300.0).abs() < 0.5, "the Arabic word ends at the right edge: {}", arabic[0]);
    assert!(latin[0].right() <= arabic[0].x + 0.01, "Latin {} is left of Arabic {}", latin[0], arabic[0]);

    // Latin characters still advance left to right inside their run.
    assert!(layout.hit_test_text_position(5).x < layout.hit_test_text_position(6).x);
    // Arabic characters advance right to left.
    assert!(layout.hit_test_text_position(0).x > layout.hit_test_text_position(3).x);

    let target = TextTarget::new(320, 70);
    let mut context = target.bitmap.create_drawing_context();
    context.clear(Colors::WHITE);
    let mut sink = ContextSink::new(context);
    layout.draw(&mut sink, Point::new(10.0, 10.0));
    sink.context.borrow_mut().dispose();

    let (ink, count) = target.ink(is_dark);
    assert!(count > 300);
    // The ink hugs the right edge of the layout box and leaves the left
    // part of the 300 wide box empty.
    assert!(ink.right() > 300.0 && ink.right() <= 311.0, "ink {ink}");
    assert!(ink.x > 10.0 + 300.0 - layout.width_including_trailing_whitespace() - 4.0, "ink {ink}");

    // An underline pen draws through the same sink.
    let pen: Rc<dyn IPen> = Rc::new(ImmutablePen::from_uint32(0xFF000000, 2.0));
    let underlined = TextTarget::new(60, 20);
    let mut context = underlined.bitmap.create_drawing_context();
    context.clear(Colors::WHITE);
    let mut sink = ContextSink::new(context);
    sink.push_transform(Matrix::create_translation(5.0, 0.0));
    sink.draw_line(&pen, Point::new(0.0, 10.0), Point::new(40.0, 10.0));
    sink.pop_transform();
    sink.context.borrow_mut().dispose();
    let (line_ink, _) = underlined.ink(is_dark);
    assert_eq!(Rect::new(5.0, 9.0, 40.0, 2.0), line_ink);

    layout.dispose();
    scope.dispose();
}
