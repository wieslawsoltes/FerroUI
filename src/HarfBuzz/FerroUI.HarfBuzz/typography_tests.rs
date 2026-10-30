use crate::HarfBuzzTextShaper;
use ferroui_base::media::fonts::testing::TestPlatformTypeface;
use ferroui_base::media::text_formatting::{GlyphInfo, TextShaperOptions};
use ferroui_base::media::{FontFeature, FontFeatureCollection, FontSimulations, GlyphTypeface};
use ferroui_base::platform::ITextShaperImpl;
use ferroui_base::utilities::ReadOnlyMemory;
use ferroui_controls::documents::{Run, Typography};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use std::rc::Rc;

const INTER_FONT: &[u8] = include_bytes!("../../FerroUI.Base/test_assets/fonts/Inter-Regular.ttf");

fn start() -> (
    Rc<HarfBuzzTextShaper>,
    Rc<GlyphTypeface>,
    UnitTestApplicationScope,
) {
    let shaper = Rc::new(HarfBuzzTextShaper::new());
    let scope = UnitTestApplication::start(
        TestServices::mock_threading_interface().with_text_shaper_impl(shaper.clone()),
    );
    let platform = TestPlatformTypeface::from_bytes(INTER_FONT.to_vec(), FontSimulations::None)
        .expect("the bundled font loads");
    let font = GlyphTypeface::new(platform, FontSimulations::None).expect("the font has tables");
    (shaper, font, scope)
}

fn shape(
    shaper: &HarfBuzzTextShaper,
    font: &Rc<GlyphTypeface>,
    run: &Run,
    text: &str,
) -> Vec<GlyphInfo> {
    let features = Typography::effective_font_features(run)
        .map(|features| Rc::new(features.list().snapshot().as_ref().clone()));
    let options = TextShaperOptions::with_all(font.clone(), 16.0, 0, None, 0.0, 0.0, features);
    shaper
        .shape_text(
            &ReadOnlyMemory::from_vec(text.encode_utf16().collect()),
            &options,
        )
        .glyph_infos()
        .to_vec()
}

#[test]
fn typography_harfbuzz_kerning_changes_real_font_positions_and_raw_override_restores_them() {
    let (shaper, font, _scope) = start();
    let run = Run::with_text(Some("AV"));
    let standard = shape(&shaper, &font, &run, "AV");
    Typography::set_kerning(&run, false);
    let without_kerning = shape(&shaper, &font, &run, "AV");
    assert!(
        without_kerning
            .iter()
            .map(|info| info.glyph_advance)
            .sum::<f64>()
            > standard.iter().map(|info| info.glyph_advance).sum::<f64>()
    );
    assert_eq!(
        standard
            .iter()
            .map(|info| info.glyph_index)
            .collect::<Vec<_>>(),
        without_kerning
            .iter()
            .map(|info| info.glyph_index)
            .collect::<Vec<_>>(),
    );
    run.set_font_features(Some(FontFeatureCollection::from_items([FontFeature {
        tag: "kern".to_owned(),
        value: 1,
        start: 0,
        end: -1,
    }])));
    assert_eq!(shape(&shaper, &font, &run, "AV"), standard);
}

#[test]
fn typography_harfbuzz_slashed_zero_changes_glyphs_and_scoped_override_preserves_source_clusters() {
    let (shaper, font, _scope) = start();
    let run = Run::with_text(Some("00"));
    let standard = shape(&shaper, &font, &run, "00");
    Typography::set_slashed_zero(&run, true);
    let slashed = shape(&shaper, &font, &run, "00");
    assert_eq!(slashed.len(), 2);
    assert_ne!(slashed[0].glyph_index, standard[0].glyph_index);
    assert_eq!(slashed[0].glyph_index, slashed[1].glyph_index);
    run.set_font_features(Some(FontFeatureCollection::from_items([FontFeature {
        tag: "zero".to_owned(),
        value: 0,
        start: 1,
        end: 2,
    }])));
    let mixed = shape(&shaper, &font, &run, "00");
    assert_eq!(mixed[0].glyph_index, slashed[0].glyph_index);
    assert_eq!(mixed[1].glyph_index, standard[1].glyph_index);
    assert_eq!(
        mixed
            .iter()
            .map(|info| info.glyph_cluster)
            .collect::<Vec<_>>(),
        [0, 1]
    );
}
