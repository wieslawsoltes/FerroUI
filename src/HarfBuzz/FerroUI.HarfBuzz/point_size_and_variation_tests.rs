//! The point size and the position in the variation space the shaper gives HarfBuzz (issue #664).
//!
//! No font of the test assets has a `trak` table, so the tracking test serves the Inter of the
//! base crate with a small `trak` table added by its platform typeface. The variation tests use
//! the variable Inter of the same assets (axes `opsz` 14 to 32 and `wght` 100 to 900).

use crate::{HarfBuzzTextShaper, HarfBuzzTypeface};
use ferroui_base::media::fonts::testing::TestPlatformTypeface;
use ferroui_base::media::fonts::OpenTypeTag;
use ferroui_base::media::text_formatting::TextShaperOptions;
use ferroui_base::media::{
    FontSimulations, FontStretch, FontStyle, FontVariation, FontVariationSettings, FontWeight, GlyphTypeface,
    IFontMemory, IPlatformTypeface,
};
use ferroui_base::platform::ITextShaperImpl;
use ferroui_base::utilities::ReadOnlyMemory;
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use std::any::Any;
use std::io::Read;
use std::rc::Rc;

const INTER_FONT: &[u8] = include_bytes!("../../FerroUI.Base/test_assets/fonts/Inter-Regular.ttf");
const INTER_VARIABLE_FONT: &[u8] = include_bytes!("../../FerroUI.Base/test_assets/fonts/InterVariable.ttf");

const TRAK: OpenTypeTag = OpenTypeTag::from_chars('t', 'r', 'a', 'k');
const WGHT: OpenTypeTag = OpenTypeTag::from_chars('w', 'g', 'h', 't');

/// A `trak` table of one track (the normal one, value 0) at two sizes: a tracking of 100 units
/// of the font at 12 points and of -100 units at 24 points, interpolated between them.
const TRAK_TABLE: [u8; 40] = [
    0x00, 0x01, 0x00, 0x00, // version 1.0
    0x00, 0x00, // format
    0x00, 0x0C, // offset of the horizontal track data
    0x00, 0x00, // offset of the vertical track data: none
    0x00, 0x00, // reserved
    // The horizontal track data, at 12.
    0x00, 0x01, // number of tracks
    0x00, 0x02, // number of sizes
    0x00, 0x00, 0x00, 0x1C, // offset of the sizes: 28
    // The track: value 0, a name index, the offset of its values (36).
    0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x24, //
    // The sizes, 16.16 fixed-point numbers, at 28: 12 and 24.
    0x00, 0x0C, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, //
    // The values of the track, at 36: 100 and -100.
    0x00, 0x64, 0xFF, 0x9C,
];

/// The platform typeface of a font of the test assets, with a `trak` table added and the
/// position in the variation space a backend would report.
struct TestTypeface {
    inner: Rc<TestPlatformTypeface>,
    trak: Option<&'static [u8]>,
    variation_settings: FontVariationSettings,
}

impl IFontMemory for TestTypeface {
    fn try_get_table(&self, tag: OpenTypeTag) -> Option<ReadOnlyMemory<u8>> {
        match self.trak {
            Some(trak) if tag == TRAK => Some(ReadOnlyMemory::from_slice(trak)),
            _ => self.inner.try_get_table(tag),
        }
    }

    fn dispose(&self) {
        IFontMemory::dispose(&*self.inner);
    }
}

impl IPlatformTypeface for TestTypeface {
    fn family_name(&self) -> String {
        self.inner.family_name()
    }

    fn weight(&self) -> FontWeight {
        self.inner.weight()
    }

    fn style(&self) -> FontStyle {
        self.inner.style()
    }

    fn stretch(&self) -> FontStretch {
        self.inner.stretch()
    }

    fn font_simulations(&self) -> FontSimulations {
        self.inner.font_simulations()
    }

    fn variation_settings(&self) -> FontVariationSettings {
        self.variation_settings.clone()
    }

    fn try_get_stream(&self) -> Option<Box<dyn Read>> {
        self.inner.try_get_stream()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn start() -> (Rc<HarfBuzzTextShaper>, UnitTestApplicationScope) {
    let shaper = Rc::new(HarfBuzzTextShaper::new());
    let services = TestServices::mock_threading_interface().with_text_shaper_impl(shaper.clone());
    (shaper, UnitTestApplication::start(services))
}

fn typeface(
    font: &[u8],
    trak: Option<&'static [u8]>,
    variation_settings: FontVariationSettings,
) -> Rc<GlyphTypeface> {
    let inner = TestPlatformTypeface::from_bytes(font.to_vec(), FontSimulations::None).expect("the font loads");
    let platform_typeface: Rc<dyn IPlatformTypeface> = Rc::new(TestTypeface { inner, trak, variation_settings });

    GlyphTypeface::new(platform_typeface, FontSimulations::None).expect("the font has its tables")
}

fn advances(shaper: &HarfBuzzTextShaper, typeface: &Rc<GlyphTypeface>, text: &str, em_size: f64) -> Vec<f64> {
    let options = TextShaperOptions::with_all(typeface.clone(), em_size, 0, None, 0.0, 0.0, None);
    let text = ReadOnlyMemory::from_vec(text.encode_utf16().collect());

    shaper.shape_text(&text, &options).glyph_infos().iter().map(|info| info.glyph_advance).collect()
}

/// Runs `f` with the HarfBuzz font of a typeface.
fn with_font<R>(typeface: &GlyphTypeface, f: impl FnOnce(&crate::hb::Font) -> R) -> R {
    let text_shaper_typeface = typeface.text_shaper_typeface();
    let harf_buzz_typeface =
        text_shaper_typeface.as_any().downcast_ref::<HarfBuzzTypeface>().expect("the typeface of this shaper");

    harf_buzz_typeface.with_font(|font| f(font))
}

fn weight(value: f64) -> FontVariationSettings {
    FontVariationSettings::new([FontVariation::new(WGHT, value)])
}

#[test]
fn shaping_sets_the_point_size_of_the_font_to_the_em_size() {
    let (shaper, _app) = start();
    let typeface = typeface(INTER_FONT, None, FontVariationSettings::empty());

    advances(&shaper, &typeface, "Hello", 12.0);
    assert_eq!(12.0, with_font(&typeface, |font| font.ptem()));

    advances(&shaper, &typeface, "Hello", 24.5);
    assert_eq!(24.5, with_font(&typeface, |font| font.ptem()));
}

#[test]
fn the_tracking_of_a_trak_table_follows_the_em_size() {
    let (shaper, _app) = start();
    let plain = typeface(INTER_FONT, None, FontVariationSettings::empty());
    let tracked = typeface(INTER_FONT, Some(&TRAK_TABLE), FontVariationSettings::empty());
    let units_per_em = plain.metrics().design_em_height as f64;

    // 100 units at 12 points, -100 at 24, 0 halfway between.
    for (em_size, tracking) in [(12.0, 100.0), (24.0, -100.0), (18.0, 0.0)] {
        let plain_advances = advances(&shaper, &plain, "Hello", em_size);
        let tracked_advances = advances(&shaper, &tracked, "Hello", em_size);

        assert_eq!(plain_advances.len(), tracked_advances.len());
        for (plain_advance, tracked_advance) in plain_advances.iter().zip(&tracked_advances) {
            let expected = plain_advance + tracking * em_size / units_per_em;
            assert!(
                (tracked_advance - expected).abs() < 1e-9,
                "at {em_size}: the advance {tracked_advance} is not {plain_advance} with a tracking of {tracking}"
            );
        }
    }
}

#[test]
fn a_typeface_at_the_default_instance_sets_no_coordinates() {
    let (shaper, _app) = start();

    let unvaried = typeface(INTER_VARIABLE_FONT, None, FontVariationSettings::empty());
    advances(&shaper, &unvaried, "Hello", 16.0);
    assert!(with_font(&unvaried, |font| font.var_coords_normalized().is_empty()));

    // The default of each axis, as a backend reports a font that is not varied.
    let defaults = FontVariationSettings::new([
        FontVariation::new(OpenTypeTag::from_chars('o', 'p', 's', 'z'), 14.0),
        FontVariation::new(WGHT, 400.0),
    ]);
    let at_defaults = typeface(INTER_VARIABLE_FONT, None, defaults);
    advances(&shaper, &at_defaults, "Hello", 16.0);
    assert!(with_font(&at_defaults, |font| font.var_coords_normalized().is_empty()));
}

#[test]
fn the_variation_of_the_typeface_reaches_the_font() {
    let (shaper, _app) = start();

    // The axes in the order of the `fvar` table: `opsz`, `wght`. The ends of an axis are -1 and
    // 1 (16384 in 2.14) also through the `avar` table.
    let bold = typeface(INTER_VARIABLE_FONT, None, weight(900.0));
    advances(&shaper, &bold, "Hello", 16.0);
    assert_eq!(vec![0, 16384], with_font(&bold, |font| font.var_coords_normalized().to_vec()));

    let thin = typeface(INTER_VARIABLE_FONT, None, weight(100.0));
    advances(&shaper, &thin, "Hello", 16.0);
    assert_eq!(vec![0, -16384], with_font(&thin, |font| font.var_coords_normalized().to_vec()));
}

#[test]
fn a_variable_font_is_shaped_at_the_position_of_the_typeface() {
    let (shaper, _app) = start();
    let regular = typeface(INTER_VARIABLE_FONT, None, FontVariationSettings::empty());
    let bold = typeface(INTER_VARIABLE_FONT, None, weight(900.0));

    let regular_width: f64 = advances(&shaper, &regular, "Hello World", 16.0).iter().sum();
    let bold_width: f64 = advances(&shaper, &bold, "Hello World", 16.0).iter().sum();

    // The heavy instance of Inter is wider (its `HVAR` table).
    assert!(bold_width > regular_width, "the heavy text is {bold_width} wide, the regular one {regular_width}");
}
