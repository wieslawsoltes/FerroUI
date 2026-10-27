use std::rc::Rc;

use ferroui_base::media::text_formatting::testing::{
    format_line, paragraph_properties, run_properties, DrawCall, RecordingDrawingSink,
    SingleBufferTextSource, TestTextShaperImpl, TextTestScope, CJK_FAMILY, DEFAULT_FAMILY,
    EMOJI_FAMILY,
};
use ferroui_base::media::text_formatting::TextShaper;
use ferroui_base::media::{FontManager, TextWrapping};
use ferroui_base::platform::{IFontManagerImpl, ITextShaperImpl};
use ferroui_base::{FerroLocator, Point};
use ferroui_pretext::{
    IPretextTextMeasurer, IPretextTextMeasurerFactory, IPretextTextShaper, LayoutCursor,
    PrepareOptions, PretextError, PretextLayout, PretextShapeOptions, PretextTextDirection,
};

use crate::{create_layout, FerroTextMeasurerFactory};

const FONT: &str = "16px Test Sans";

#[test]
fn absent_font_and_shaper_services_return_errors() {
    let scope = FerroLocator::enter_scope();
    FerroLocator::current_mutable()
        .bind::<FontManager>()
        .to_func(|| None);
    FerroLocator::current_mutable()
        .bind::<dyn IFontManagerImpl>()
        .to_func(|| None);
    FerroLocator::current_mutable()
        .bind::<TextShaper>()
        .to_func(|| None);
    FerroLocator::current_mutable()
        .bind::<dyn ITextShaperImpl>()
        .to_func(|| None);
    assert!(matches!(create_layout(), Err(PretextError::Backend(_))));
    scope.dispose();
}

#[test]
fn direct_shaper_without_typeface_backend_returns_an_error() {
    let _services = TextTestScope::new();
    let _shaper = TextShaper::current();
    let factory = FerroTextMeasurerFactory::try_current().unwrap();
    let font = factory.create_text_service(FONT).unwrap();
    let shadow = FerroLocator::enter_scope();
    FerroLocator::current_mutable()
        .bind::<dyn ITextShaperImpl>()
        .to_func(|| None);
    // TextShaper::current can return the retained service, but fallback still
    // needs ITextShaperImpl to build GlyphTypeface's script-support face.
    assert!(matches!(create_layout(), Err(PretextError::Backend(_))));
    assert!(matches!(
        font.shape_native("世界", PretextShapeOptions::default()),
        Err(PretextError::Backend(_))
    ));
    shadow.dispose();
    assert_eq!(font.try_measure_text("abc").unwrap(), 24.0);
}

#[test]
fn replacing_only_typeface_shaper_invalidates_existing_services() {
    let _services = TextTestScope::new();
    let factory = FerroTextMeasurerFactory::try_current().unwrap();
    let font = factory.create_text_service(FONT).unwrap();
    let shadow = FerroLocator::enter_scope();
    FerroLocator::current_mutable()
        .bind::<dyn ITextShaperImpl>()
        .to_constant(Rc::new(TestTextShaperImpl::new()));
    assert!(!IPretextTextMeasurerFactory::is_supported(&factory));
    assert!(matches!(
        font.shape_native("世界", PretextShapeOptions::default()),
        Err(PretextError::Backend(_))
    ));
    shadow.dispose();
    assert_eq!(font.try_measure_text("abc").unwrap(), 24.0);
}

#[test]
fn combined_factory_measures_and_shapes_using_framework_services() {
    let _scope = TextTestScope::new();
    let factory = FerroTextMeasurerFactory::try_current().unwrap();
    assert!(IPretextTextMeasurerFactory::is_supported(&factory));
    assert!(factory.shaper_factory().is_some());
    let native = factory.create_text_service(FONT).unwrap();
    let shaped = native
        .shape_text("abc ", PretextShapeOptions::default())
        .unwrap();
    assert_eq!(shaped.advance_x(), 32.0);
    assert_eq!(native.measure_text("abc "), shaped.advance_x());
    assert!(shaped.is_shaped());
    assert_eq!(
        shaped
            .glyphs()
            .iter()
            .map(|g| g.cluster)
            .collect::<Vec<_>>(),
        [0, 1, 2, 3]
    );
}

#[test]
fn native_shaping_preserves_ligatures_instead_of_mapping_codepoints() {
    let scope = TextTestScope::new();
    scope.shaper().add_ligature("ffi");
    let service = FerroTextMeasurerFactory::try_current()
        .unwrap()
        .create_text_service(FONT)
        .unwrap();
    let shaped = service
        .shape_text("office", PretextShapeOptions::default())
        .unwrap();
    assert_eq!(shaped.glyphs().len(), 4);
    assert_eq!(
        shaped
            .glyphs()
            .iter()
            .map(|g| g.cluster)
            .collect::<Vec<_>>(),
        [0, 1, 4, 5]
    );
    assert_eq!(shaped.advance_x(), 32.0);
    assert_eq!(service.try_measure_text("office").unwrap(), 32.0);
    assert!(scope.shaper().shape_calls().contains(&"office".to_owned()));
}

#[test]
fn font_fallback_preserves_actual_faces_and_utf16_clusters() {
    let _scope = TextTestScope::new();
    let service = FerroTextMeasurerFactory::try_current()
        .unwrap()
        .create_text_service(FONT)
        .unwrap();
    let native = service
        .shape_native("a中😀b", PretextShapeOptions::default())
        .unwrap();
    let shaped = native.shaped_run();
    assert_eq!(
        shaped
            .glyphs()
            .iter()
            .map(|g| g.cluster)
            .collect::<Vec<_>>(),
        [0, 1, 2, 4]
    );
    assert_eq!(shaped.advance_x(), 40.0);
    assert!(!shaped.has_missing_glyphs());
    assert_eq!(
        native
            .glyph_runs()
            .iter()
            .map(|run| run.glyph_typeface().family_name())
            .collect::<Vec<_>>(),
        [DEFAULT_FAMILY, CJK_FAMILY, EMOJI_FAMILY, DEFAULT_FAMILY]
    );
    assert_eq!(
        native
            .glyph_runs()
            .iter()
            .map(|run| run.baseline_origin().x)
            .collect::<Vec<_>>(),
        [0.0, 8.0, 16.0, 32.0]
    );
    for (index, run) in shaped.font_runs().iter().enumerate() {
        assert_eq!(index, run.index);
        assert_eq!(1, run.glyph_count);
        assert_eq!(index, shaped.glyphs()[run.first_glyph_index].font_run_index);
        assert!(run
            .font
            .contains(native.glyph_runs()[index].glyph_typeface().family_name()));
    }
}

#[test]
fn rtl_clusters_are_visual_and_bidi_mixed_runs_are_reordered() {
    let _scope = TextTestScope::new();
    let service = FerroTextMeasurerFactory::try_current()
        .unwrap()
        .create_text_service(FONT)
        .unwrap();
    let rtl = service
        .shape_text("אבג", PretextShapeOptions::default())
        .unwrap();
    assert_eq!(
        rtl.glyphs().iter().map(|g| g.cluster).collect::<Vec<_>>(),
        [2, 1, 0]
    );
    assert_eq!(
        rtl.glyphs().iter().map(|g| g.x).collect::<Vec<_>>(),
        [0.0, 8.0, 16.0]
    );
    let mixed = service
        .shape_text("אב abc", PretextShapeOptions::default())
        .unwrap();
    assert_eq!(
        mixed.glyphs().iter().map(|g| g.cluster).collect::<Vec<_>>(),
        [3, 4, 5, 2, 1, 0]
    );
    let forced = service
        .shape_text(
            "אב abc",
            PretextShapeOptions {
                direction: PretextTextDirection::LeftToRight,
            },
        )
        .unwrap();
    assert_eq!(
        forced
            .glyphs()
            .iter()
            .map(|g| g.cluster)
            .collect::<Vec<_>>(),
        [1, 0, 2, 3, 4, 5]
    );
}

#[test]
fn native_measurement_agrees_with_framework_formatter() {
    let scope = TextTestScope::new();
    scope.shaper().add_ligature("ffi");
    let service = FerroTextMeasurerFactory::try_current()
        .unwrap()
        .create_text_service(FONT)
        .unwrap();
    for text in [
        "office file",
        "abc 世界 😀",
        "According to محمد الأحمد",
        "e\u{301}x",
        "a\0b",
        "abc  ",
    ] {
        let properties = run_properties(16.0);
        let source = SingleBufferTextSource::new(text, properties.clone());
        let line = format_line(
            &source,
            0,
            f64::INFINITY,
            &paragraph_properties(&properties, TextWrapping::NoWrap),
            None,
        )
        .unwrap();
        assert_eq!(
            service.try_measure_text(text).unwrap(),
            line.width_including_trailing_whitespace(),
            "{text:?}"
        );
    }
}

#[test]
fn combining_marks_and_embedded_nuls_keep_source_coordinates() {
    let _scope = TextTestScope::new();
    let service = FerroTextMeasurerFactory::try_current()
        .unwrap()
        .create_text_service(FONT)
        .unwrap();
    let shaped = service
        .shape_text("e\u{301}\0😀x", PretextShapeOptions::default())
        .unwrap();
    assert_eq!(
        shaped
            .glyphs()
            .iter()
            .map(|g| g.cluster)
            .collect::<Vec<_>>(),
        [0, 0, 2, 3, 5]
    );
    assert_eq!(shaped.advance_x(), 32.0);
}

#[test]
fn glyph_runs_draw_at_the_requested_baseline() {
    let _scope = TextTestScope::new();
    let service = FerroTextMeasurerFactory::try_current()
        .unwrap()
        .create_text_service(FONT)
        .unwrap();
    let native = service
        .shape_native("a中😀", PretextShapeOptions::default())
        .unwrap();
    let mut sink = RecordingDrawingSink::new();
    native.draw(&mut sink, None, Point::new(12.0, 30.0));
    assert_eq!(sink.transform_depth(), 0);
    assert_eq!(
        sink.calls(),
        [
            DrawCall::GlyphRun {
                text: "a".into(),
                origin: Point::new(12.0, 30.0)
            },
            DrawCall::GlyphRun {
                text: "中".into(),
                origin: Point::new(12.0, 30.0)
            },
            DrawCall::GlyphRun {
                text: "😀".into(),
                origin: Point::new(12.0, 30.0)
            },
        ]
    );
}

#[test]
fn engine_integrates_wrapping_and_cached_shaped_materialization() {
    let _scope = TextTestScope::new();
    let engine = create_layout().unwrap();
    let prepared = engine
        .prepare_with_segments("Hello 世界 😀 end", FONT, PrepareOptions::default())
        .unwrap();
    let shaped = engine
        .shape_prepared_text(prepared.clone(), PretextShapeOptions::default())
        .unwrap();
    let line =
        PretextLayout::layout_next_line_range(&prepared, LayoutCursor::default(), 72.0).unwrap();
    let materialized = PretextLayout::materialize_shaped_line_range(&shaped, &line).unwrap();
    assert!(!materialized.shaped_run.glyphs().is_empty());
    assert!(!materialized.shaped_run.has_missing_glyphs());
    assert_eq!(materialized.width, line.width);
    assert_eq!(materialized.start, line.start);
    assert_eq!(materialized.end, line.end);
}

#[test]
fn invalid_fonts_fail_and_unknown_families_fall_back() {
    let _scope = TextTestScope::new();
    let factory = FerroTextMeasurerFactory::try_current().unwrap();
    assert!(matches!(
        factory.create_text_service("0px Test Sans"),
        Err(PretextError::InvalidInput(_))
    ));
    assert!(matches!(
        factory.create_text_service("0 16px Test Sans"),
        Err(PretextError::InvalidInput(_))
    ));
    let native = factory
        .create_text_service("16px Nonexistent Font")
        .unwrap()
        .shape_native("abc", PretextShapeOptions::default())
        .unwrap();
    assert_eq!(
        native.glyph_runs()[0].glyph_typeface().family_name(),
        DEFAULT_FAMILY
    );
    assert_eq!(
        factory
            .create_text_service("16px system-ui")
            .unwrap()
            .try_measure_text("abc")
            .unwrap(),
        24.0
    );
}

#[test]
fn changed_font_service_scope_reports_an_error_but_retained_glyphs_keep_faces() {
    let scope = TextTestScope::new();
    let factory = FerroTextMeasurerFactory::try_current().unwrap();
    let service = factory.create_text_service(FONT).unwrap();
    let native = service
        .shape_native("abc", PretextShapeOptions::default())
        .unwrap();
    drop(scope);
    assert!(!IPretextTextMeasurerFactory::is_supported(&factory));
    assert!(matches!(
        service.try_measure_text("abc"),
        Err(PretextError::Backend(_))
    ));
    assert!(service.measure_text("abc").is_nan());
    assert_eq!(
        native.glyph_runs()[0].glyph_typeface().family_name(),
        DEFAULT_FAMILY
    );
    assert_eq!(native.shaped_run().advance_x(), 24.0);
}

#[test]
fn factory_handles_are_shareable_without_global_pretext_configuration() {
    let _scope = TextTestScope::new();
    let factory: Rc<dyn IPretextTextMeasurerFactory> =
        Rc::new(FerroTextMeasurerFactory::try_current().unwrap());
    let first = PretextLayout::new(factory.clone());
    let second = PretextLayout::new(factory);
    assert_eq!(
        first
            .shape_text("abc", FONT, PretextShapeOptions::default())
            .unwrap()
            .advance_x(),
        second
            .shape_text("abc", FONT, PretextShapeOptions::default())
            .unwrap()
            .advance_x()
    );
}
