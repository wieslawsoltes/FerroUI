//! The text services of the backend: the font manager over the catalog of the engine, the
//! typeface, the shaper of the engine and the glyph run. The shaper of the engine is checked
//! against the HarfBuzz shaper over the same fonts.

use crate::font::FontCatalog;
use crate::{ProGpuFontManager, ProGpuGlyphRun, ProGpuPlatformRenderInterface, ProGpuTextShaper, ProGpuTypeface};
use ferroui_base::media::text_formatting::{GlyphInfo, TextShaperOptions};
use ferroui_base::media::{FontSimulations, FontStretch, FontStyle, FontWeight, GlyphRun, GlyphTypeface, IPlatformTypeface};
use ferroui_base::platform::{IFontManagerImpl, IPlatformRenderInterface, ITextShaperImpl};
use ferroui_base::utilities::ReadOnlyMemory;
use ferroui_base::{FerroLocator, Point};
use std::rc::Rc;

macro_rules! font {
    ($folder:literal, $file:literal) => {
        include_bytes!(concat!("../../Skia/FerroUI.Skia/test_assets/", $folder, "/", $file)) as &[u8]
    };
}

const INTER: &[u8] = font!("assets", "Inter-Regular.ttf");
const INTER_BOLD: &[u8] = font!("assets", "Inter-Bold.ttf");
const INTER_VARIABLE: &[u8] = font!("assets", "InterVariable.ttf");
const NOTO_SANS_ARABIC: &[u8] = font!("assets", "NotoSansArabic-Regular.ttf");
const NOTO_SANS_HEBREW: &[u8] = font!("assets", "NotoSansHebrew-Regular.ttf");
const NOTO_SANS_JP: &[u8] = font!("fonts", "NotoSansJP-Subset.ttf");

#[derive(Clone, Copy)]
enum Shaper {
    Engine,
    HarfBuzz,
}

/// Runs `f` with a text shaper registered: the services are those of `f` only.
fn with_shaper<R>(shaper: Shaper, f: impl FnOnce(&dyn ITextShaperImpl) -> R) -> R {
    let scope = FerroLocator::enter_scope();
    let text_shaper: Rc<dyn ITextShaperImpl> = match shaper {
        Shaper::Engine => Rc::new(ProGpuTextShaper::new()),
        Shaper::HarfBuzz => Rc::new(ferroui_harfbuzz::HarfBuzzTextShaper::new()),
    };
    FerroLocator::current_mutable().bind::<dyn ITextShaperImpl>().to_constant(text_shaper.clone());
    let result = f(&*text_shaper);
    scope.dispose();
    result
}

fn typeface_of(font: &[u8]) -> Rc<dyn IPlatformTypeface> {
    ProGpuFontManager::new().try_create_glyph_typeface_from_stream(&mut &font[..], FontSimulations::None).expect("the font is read")
}

fn glyph_typeface_of(font: &[u8]) -> Rc<GlyphTypeface> {
    GlyphTypeface::new(typeface_of(font), FontSimulations::None).expect("the tables of the font are read")
}

/// The glyphs of `text` in `font` at `size`, by a shaper.
fn shape(shaper: Shaper, font: &[u8], text: &str, size: f64, bidi_level: i8) -> Vec<GlyphInfo> {
    with_shaper(shaper, |text_shaper| {
        let glyph_typeface = glyph_typeface_of(font);
        let units: Vec<u16> = text.encode_utf16().collect();
        let options = TextShaperOptions::with_all(glyph_typeface.clone(), size, bidi_level, None, 0.0, 0.0, None);
        let buffer = text_shaper.shape_text(&ReadOnlyMemory::from_vec(units), &options);
        let glyphs = buffer.glyph_infos().to_vec();
        glyph_typeface.dispose();
        glyphs
    })
}

/// The two shapers give the same glyphs, clusters, advances and offsets.
fn assert_shapers_agree(font: &[u8], text: &str, bidi_level: i8) -> Vec<GlyphInfo> {
    let engine = shape(Shaper::Engine, font, text, 20.0, bidi_level);
    let harfbuzz = shape(Shaper::HarfBuzz, font, text, 20.0, bidi_level);
    let describe = |glyphs: &[GlyphInfo]| {
        glyphs
            .iter()
            .map(|glyph| format!("{}@{} +{:.3} ({:.3},{:.3})", glyph.glyph_index, glyph.glyph_cluster, glyph.glyph_advance, glyph.glyph_offset.x, glyph.glyph_offset.y))
            .collect::<Vec<_>>()
            .join(" | ")
    };
    assert_eq!(engine.len(), harfbuzz.len(), "{text:?}\n engine   {}\n harfbuzz {}", describe(&engine), describe(&harfbuzz));
    for (index, (ours, theirs)) in engine.iter().zip(&harfbuzz).enumerate() {
        let same = ours.glyph_index == theirs.glyph_index
            && ours.glyph_cluster == theirs.glyph_cluster
            && (ours.glyph_advance - theirs.glyph_advance).abs() < 0.011
            && (ours.glyph_offset.x - theirs.glyph_offset.x).abs() < 0.011
            && (ours.glyph_offset.y - theirs.glyph_offset.y).abs() < 0.011;
        assert!(same, "{text:?} glyph {index}\n engine   {}\n harfbuzz {}", describe(&engine), describe(&harfbuzz));
    }
    engine
}

#[test]
fn the_font_manager_lists_and_matches_the_fonts_of_the_system() {
    if FontCatalog::system().is_none_or(|catalog| catalog.face_count() == 0) {
        eprintln!("skipped: the system has no fonts the catalog of the engine finds");
        return;
    }
    let manager = ProGpuFontManager::new();
    let default_family = manager.get_default_font_family_name();
    assert!(!default_family.is_empty());
    let families = manager.get_installed_font_family_names(false);
    assert!(families.contains(&default_family), "{default_family} is installed");
    assert!(families.windows(2).all(|pair| pair[0] < pair[1]), "the families are sorted and unique");

    let regular = manager.try_create_glyph_typeface(&default_family, FontStyle::Normal, FontWeight::Normal, FontStretch::Normal).expect("the default family has a face");
    assert_eq!(regular.family_name(), default_family);
    assert_eq!(regular.font_simulations(), FontSimulations::None);
    assert!(GlyphTypeface::new(regular.clone(), FontSimulations::None).is_ok(), "the base library reads the tables of a system face");

    // A bold of the family is a bold face, a bold instance or a simulated bold.
    let bold = manager.try_create_glyph_typeface(&default_family, FontStyle::Normal, FontWeight::Bold, FontStretch::Normal).expect("the default family answers a bold");
    assert!(bold.weight().0 >= 600 || bold.font_simulations().contains(FontSimulations::Bold));

    assert!(manager.try_create_glyph_typeface("No Such Family 3f9a", FontStyle::Normal, FontWeight::Normal, FontStretch::Normal).is_none());
    assert!(manager.try_get_family_typefaces("No Such Family 3f9a").is_none());
    let typefaces = manager.try_get_family_typefaces(&default_family).expect("the default family has typefaces");
    assert!(!typefaces.is_empty());

    // A character of another script comes from a face that has it.
    let fallback = manager
        .try_match_character(0x4E2D, FontStyle::Normal, FontWeight::Normal, FontStretch::Normal, Some(&default_family), None)
        .expect("a system font has a common Han character");
    let glyph_typeface = GlyphTypeface::new(fallback, FontSimulations::None).expect("the tables of the fallback are read");
    assert_ne!(glyph_typeface.character_to_glyph_map().get_glyph(0x4E2D), 0);
}

#[test]
fn a_typeface_of_font_bytes_serves_its_names_style_tables_and_stream() {
    let regular = typeface_of(INTER);
    assert_eq!(regular.family_name(), "Inter");
    assert_eq!(regular.weight(), FontWeight::Normal);
    assert_eq!(regular.style(), FontStyle::Normal);
    assert_eq!(regular.stretch(), FontStretch::Normal);
    let bold = typeface_of(INTER_BOLD);
    assert_eq!(bold.weight(), FontWeight::Bold);

    let head = regular.try_get_table(ferroui_base::media::fonts::OpenTypeTag::parse("head")).expect("the head table");
    assert_eq!(head.len(), 54);
    assert!(regular.try_get_table(ferroui_base::media::fonts::OpenTypeTag::parse("zzzz")).is_none());
    let mut bytes = Vec::new();
    regular.try_get_stream().expect("the stream of the font").read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, INTER);

    let manager = ProGpuFontManager::new();
    assert!(manager.try_create_glyph_typeface_from_stream(&mut &[0u8; 64][..], FontSimulations::None).is_none(), "bytes that are no font");
    let simulated = manager.try_create_glyph_typeface_from_stream(&mut &INTER[..], FontSimulations::Bold | FontSimulations::Oblique).unwrap();
    assert_eq!(simulated.font_simulations(), FontSimulations::Bold | FontSimulations::Oblique);

    let glyph_typeface = glyph_typeface_of(INTER);
    assert_eq!(glyph_typeface.metrics().design_em_height, 2816);
    assert_ne!(glyph_typeface.character_to_glyph_map().get_glyph('A' as i32), 0);
}

#[test]
fn the_shaper_of_the_engine_agrees_with_harfbuzz_on_latin_text() {
    let glyphs = assert_shapers_agree(INTER, "Hello, World! AVATAR To fi 1/2", 0);
    assert!(glyphs.iter().all(|glyph| glyph.glyph_index != 0));
    assert!(glyphs.windows(2).all(|pair| pair[0].glyph_cluster <= pair[1].glyph_cluster));
    assert_shapers_agree(INTER_BOLD, "Bold text, kerned: AVA Ty", 0);
    // Marks that combine and marks that attach.
    assert_shapers_agree(INTER, "re\u{301}sume\u{301} a\u{308}o\u{303}", 0);
    // Characters outside the basic plane keep their clusters in UTF-16 units.
    assert_shapers_agree(INTER, "a\u{1F600}b", 0);
}

#[test]
fn the_shaper_of_the_engine_agrees_with_harfbuzz_on_other_scripts() {
    // Arabic joins and is shaped right to left: the glyphs come in visual order.
    let arabic = assert_shapers_agree(NOTO_SANS_ARABIC, "\u{627}\u{644}\u{633}\u{644}\u{627}\u{645} \u{639}\u{644}\u{64A}\u{643}\u{645}", 1);
    assert!(arabic.first().unwrap().glyph_cluster > arabic.last().unwrap().glyph_cluster, "the first glyph is the last character");
    // With marks above and below.
    assert_shapers_agree(NOTO_SANS_ARABIC, "\u{628}\u{64E}\u{62D}\u{652}\u{631}\u{650}", 1);
    assert_shapers_agree(NOTO_SANS_HEBREW, "\u{5E9}\u{5C1}\u{5B8}\u{5DC}\u{5D5}\u{5B9}\u{5DD} \u{5E2}\u{5D5}\u{5DC}\u{5DD}", 1);
    assert_shapers_agree(NOTO_SANS_JP, "\u{65E5}\u{672C}\u{8A9E}\u{306E}\u{30C6}\u{30AD}\u{30B9}\u{30C8}", 0);
}

#[test]
fn the_shaper_shapes_line_breaks_and_tabs_as_the_layout_expects() {
    // A trailing line break shapes to nothing wide; a pair is one cluster.
    for shaper in [Shaper::Engine, Shaper::HarfBuzz] {
        let glyphs = shape(shaper, INTER, "ab\r\n", 20.0, 0);
        assert_eq!(glyphs[0].glyph_cluster, 0);
        assert_eq!(glyphs[1].glyph_cluster, 1);
        assert!(glyphs[2..].iter().all(|glyph| glyph.glyph_cluster == 2 && glyph.glyph_advance == 0.0), "the break is the cluster of its first character");
        let single = shape(shaper, INTER, "ab\n", 20.0, 0);
        assert_eq!(single.len(), 3);
        assert_eq!(single[2].glyph_advance, 0.0);
    }
    assert_shapers_agree(INTER, "a\tb", 0);
    assert!(shape(Shaper::Engine, INTER, "", 20.0, 0).is_empty());
    // The clusters of the engine are whole graphemes: a joiner in a text belongs to the cluster
    // before it and is drawn as nothing. (The reference shaper keeps it a cluster of its own.)
    let joined = shape(Shaper::Engine, INTER, "a\u{200C}b", 20.0, 0);
    assert_eq!(joined.iter().map(|glyph| glyph.glyph_cluster).collect::<Vec<_>>(), [0, 0, 2]);
    assert_eq!(joined[1].glyph_advance, 0.0);
}

#[test]
fn the_shaper_shapes_a_part_of_a_text_with_its_surroundings() {
    // The middle letters of an Arabic word, shaped alone as a slice of the word, join to the
    // letters around them; shaped as a text of their own they do not.
    let word: Vec<u16> = "\u{633}\u{644}\u{633}\u{644}\u{633}".encode_utf16().collect();
    let shape_slice = |shaper: Shaper| {
        with_shaper(shaper, |text_shaper| {
            let glyph_typeface = glyph_typeface_of(NOTO_SANS_ARABIC);
            let options = TextShaperOptions::with_all(glyph_typeface.clone(), 20.0, 1, None, 0.0, 0.0, None);
            let whole = ReadOnlyMemory::from_vec(word.clone());
            let buffer = text_shaper.shape_text(&whole.slice(1, 3), &options);
            let glyphs = buffer.glyph_infos().to_vec();
            glyph_typeface.dispose();
            glyphs
        })
    };
    let engine = shape_slice(Shaper::Engine);
    let harfbuzz = shape_slice(Shaper::HarfBuzz);
    let ids = |glyphs: &[GlyphInfo]| glyphs.iter().map(|glyph| (glyph.glyph_index, glyph.glyph_cluster)).collect::<Vec<_>>();
    assert_eq!(ids(&engine), ids(&harfbuzz));
    let alone = shape(Shaper::Engine, NOTO_SANS_ARABIC, "\u{644}\u{633}\u{644}", 20.0, 1);
    assert_ne!(ids(&engine), ids(&alone), "the slice joins to its surroundings");
}

#[test]
fn a_variable_face_is_shaped_at_its_instance() {
    let manager = ProGpuFontManager::new();
    let regular = manager.try_create_glyph_typeface_from_stream(&mut &INTER_VARIABLE[..], FontSimulations::None).unwrap();
    let typeface = ProGpuTypeface::try_get(&*regular).unwrap();
    assert!(typeface.face().axis_count() > 0);
    assert!(typeface.coordinates().is_empty(), "a face of bytes is its default instance");
}

#[test]
fn a_glyph_run_places_its_glyphs_and_its_geometry_is_their_outlines() {
    let interface = ProGpuPlatformRenderInterface::new();
    let glyph_typeface = glyph_typeface_of(INTER);
    let glyphs = shape(Shaper::Engine, INTER, "Hog", 32.0, 0);
    let origin = Point::new(10.0, 40.0);
    let run = interface.create_glyph_run(&glyph_typeface, 32.0, &glyphs, origin);
    assert_eq!(run.font_rendering_em_size(), 32.0);
    assert_eq!(run.baseline_origin(), origin);
    let ours = run.as_any().downcast_ref::<ProGpuGlyphRun>().unwrap();
    assert_eq!(ours.glyphs().len(), 3);
    assert_eq!(ours.glyphs()[0].x, 0.0);
    assert!((f64::from(ours.glyphs()[1].x) - glyphs[0].glyph_advance).abs() < 1e-4);

    // The outlines: above the baseline for the capital, below it for the descender, and all
    // inside the box of the run.
    let geometry_run = GlyphRun::new(glyph_typeface.clone(), 32.0, ReadOnlyMemory::from_vec("Hog".encode_utf16().collect()), glyphs.clone().into(), Some(origin), 0);
    let geometry = interface.build_glyph_run_geometry(&geometry_run);
    let ink = geometry.bounds();
    let width: f64 = glyphs.iter().map(|glyph| glyph.glyph_advance).sum();
    assert!(ink.x >= origin.x && ink.x + ink.width <= origin.x + width, "{ink:?} in the advances");
    assert!(ink.y < origin.y - 20.0 && ink.y > origin.y - 32.0, "the capital rises to its height: {ink:?}");
    assert!(ink.y + ink.height > origin.y + 4.0, "the descender goes below the baseline: {ink:?}");
    let bounds = run.bounds();
    assert!(bounds.x <= ink.x && bounds.y <= ink.y && bounds.x + bounds.width >= ink.x + ink.width && bounds.y + bounds.height >= ink.y + ink.height, "{bounds:?} holds {ink:?}");
    // A point in the stem of the capital is in the fill; a point in its counter is not.
    let stem = Point::new(origin.x + 0.12 * 32.0, origin.y - 10.0);
    assert!(geometry.fill_contains(stem), "the stem of the capital is filled at {stem:?}");
    assert!(!geometry.fill_contains(Point::new(origin.x + 0.36 * 32.0, origin.y - 3.0)), "between the stems, under the bar, is empty");
}
