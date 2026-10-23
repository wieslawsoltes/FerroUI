use crate::documents::{
    CharacterFormat, Paragraph, RichTextDocument, Run, Span, Typography, TypographyFormat,
};
use crate::rich_text::{EditKind, RichEditTextDocument, RichTextEditor, RtfError, RtfWriter};
use crate::test_support::test_scope;
use crate::{RichEditBox, RichTextBlock};
use ferroui_base::media::{
    FontCapitals, FontEastAsianLanguage, FontEastAsianWidths, FontFeature, FontFeatureCollection,
    FontFraction, FontNumeralAlignment, FontNumeralStyle, FontVariants,
};
use ferroui_base::metadata::{from_markup_value, into_markup_value, MarkupTyped};
use ferroui_base::{FerroObject, FerroPropertyRegistry, Rect, Ref, Size};
use std::cell::Cell;
use std::rc::Rc;

fn feature_values(features: &FontFeatureCollection) -> Vec<(String, i32)> {
    features
        .list()
        .snapshot()
        .iter()
        .map(|feature| {
            assert_eq!((feature.start, feature.end), (0, -1));
            (feature.tag.clone(), feature.value)
        })
        .collect()
}

fn nondefault_tags(format: &TypographyFormat) -> Vec<String> {
    feature_values(&format.to_font_features())
        .into_iter()
        .filter_map(|(tag, value)| {
            assert_eq!(value, 1);
            (!matches!(tag.as_str(), "liga" | "clig" | "calt" | "kern")).then_some(tag)
        })
        .collect()
}

fn defaults() -> TypographyFormat {
    TypographyFormat {
        annotation_alternates: Some(0),
        east_asian_expert_forms: Some(false),
        east_asian_language: Some(FontEastAsianLanguage::Normal),
        east_asian_widths: Some(FontEastAsianWidths::Normal),
        standard_ligatures: Some(true),
        contextual_ligatures: Some(true),
        discretionary_ligatures: Some(false),
        historical_ligatures: Some(false),
        standard_swashes: Some(0),
        contextual_swashes: Some(0),
        contextual_alternates: Some(true),
        stylistic_alternates: Some(0),
        stylistic_set1: Some(false),
        stylistic_set2: Some(false),
        stylistic_set3: Some(false),
        stylistic_set4: Some(false),
        stylistic_set5: Some(false),
        stylistic_set6: Some(false),
        stylistic_set7: Some(false),
        stylistic_set8: Some(false),
        stylistic_set9: Some(false),
        stylistic_set10: Some(false),
        stylistic_set11: Some(false),
        stylistic_set12: Some(false),
        stylistic_set13: Some(false),
        stylistic_set14: Some(false),
        stylistic_set15: Some(false),
        stylistic_set16: Some(false),
        stylistic_set17: Some(false),
        stylistic_set18: Some(false),
        stylistic_set19: Some(false),
        stylistic_set20: Some(false),
        capitals: Some(FontCapitals::Normal),
        capital_spacing: Some(false),
        kerning: Some(true),
        case_sensitive_forms: Some(false),
        historical_forms: Some(false),
        fraction: Some(FontFraction::Normal),
        numeral_style: Some(FontNumeralStyle::Normal),
        numeral_alignment: Some(FontNumeralAlignment::Normal),
        slashed_zero: Some(false),
        mathematical_greek: Some(false),
        variants: Some(FontVariants::Normal),
    }
}

#[test]
fn typography_registers_all_43_inherited_properties_with_exact_defaults_and_accessors() {
    let _scope = test_scope();
    let properties = FerroPropertyRegistry::instance().get_declared(Typography::TYPE);
    let mut expected = vec![
        "AnnotationAlternates",
        "EastAsianExpertForms",
        "EastAsianLanguage",
        "EastAsianWidths",
        "StandardLigatures",
        "ContextualLigatures",
        "DiscretionaryLigatures",
        "HistoricalLigatures",
        "StandardSwashes",
        "ContextualSwashes",
        "ContextualAlternates",
        "StylisticAlternates",
        "StylisticSet1",
        "StylisticSet2",
        "StylisticSet3",
        "StylisticSet4",
        "StylisticSet5",
        "StylisticSet6",
        "StylisticSet7",
        "StylisticSet8",
        "StylisticSet9",
        "StylisticSet10",
        "StylisticSet11",
        "StylisticSet12",
        "StylisticSet13",
        "StylisticSet14",
        "StylisticSet15",
        "StylisticSet16",
        "StylisticSet17",
        "StylisticSet18",
        "StylisticSet19",
        "StylisticSet20",
        "Capitals",
        "CapitalSpacing",
        "Kerning",
        "CaseSensitiveForms",
        "HistoricalForms",
        "Fraction",
        "NumeralStyle",
        "NumeralAlignment",
        "SlashedZero",
        "MathematicalGreek",
        "Variants",
    ];
    expected.sort_unstable();
    let mut names = properties
        .iter()
        .map(|property| property.name())
        .collect::<Vec<_>>();
    names.sort_unstable();
    assert_eq!(names, expected);
    assert!(properties.iter().all(|property| property.inherits()
        && property.is_attached()
        && Typography::is_property(property)));
    let markup = <Typography as MarkupTyped>::MARKUP;
    assert_eq!(markup.methods.len(), 86);
    for name in expected {
        for prefix in ["Get", "Set"] {
            assert!(markup
                .methods
                .iter()
                .any(|method| method.name == format!("{prefix}{name}")));
        }
    }
    let run = Run::with_text(Some("test"));
    assert_eq!(TypographyFormat::effective(&run), defaults());
    assert!(TypographyFormat::from_object(&run).is_empty());
    assert_eq!(
        feature_values(&defaults().to_font_features()),
        [
            ("liga".to_owned(), 1),
            ("clig".to_owned(), 1),
            ("calt".to_owned(), 1),
            ("kern".to_owned(), 1)
        ]
    );
    assert!(Typography::effective_font_features(&run).is_none());
}

#[test]
fn typography_markup_accessors_apply_bool_integer_and_typed_enum_values() {
    let _scope = test_scope();
    let run = Run::with_text(Some("test"));
    let object: Ref<FerroObject> = run.cast().unwrap();
    let target = into_markup_value(object);
    let markup = <Typography as MarkupTyped>::MARKUP;
    for (name, value) in [
        ("StandardLigatures", into_markup_value(false)),
        ("StandardSwashes", into_markup_value(-7i32)),
        ("Capitals", into_markup_value(FontCapitals::AllPetiteCaps)),
    ] {
        let setter = markup
            .methods
            .iter()
            .find(|method| method.name == format!("Set{name}"))
            .unwrap();
        (setter.invoke)(&[target.clone(), value]).unwrap();
    }
    let get = |name: &str| {
        let method = markup
            .methods
            .iter()
            .find(|method| method.name == name)
            .unwrap();
        (method.invoke)(&[target.clone()]).unwrap()
    };
    assert_eq!(
        from_markup_value::<bool>(&get("GetStandardLigatures")),
        Some(false)
    );
    assert_eq!(
        from_markup_value::<i32>(&get("GetStandardSwashes")),
        Some(-7)
    );
    assert_eq!(
        from_markup_value::<FontCapitals>(&get("GetCapitals")),
        Some(FontCapitals::AllPetiteCaps)
    );
}

#[test]
fn typography_maps_all_boolean_families_and_explicit_false_values() {
    let _scope = test_scope();
    let cases: &[(&str, fn(&FerroObject, bool), bool)] = &[
        ("expt", Typography::set_east_asian_expert_forms, false),
        ("liga", Typography::set_standard_ligatures, true),
        ("clig", Typography::set_contextual_ligatures, true),
        ("dlig", Typography::set_discretionary_ligatures, false),
        ("hlig", Typography::set_historical_ligatures, false),
        ("calt", Typography::set_contextual_alternates, true),
        ("ss01", Typography::set_stylistic_set1, false),
        ("ss02", Typography::set_stylistic_set2, false),
        ("ss03", Typography::set_stylistic_set3, false),
        ("ss04", Typography::set_stylistic_set4, false),
        ("ss05", Typography::set_stylistic_set5, false),
        ("ss06", Typography::set_stylistic_set6, false),
        ("ss07", Typography::set_stylistic_set7, false),
        ("ss08", Typography::set_stylistic_set8, false),
        ("ss09", Typography::set_stylistic_set9, false),
        ("ss10", Typography::set_stylistic_set10, false),
        ("ss11", Typography::set_stylistic_set11, false),
        ("ss12", Typography::set_stylistic_set12, false),
        ("ss13", Typography::set_stylistic_set13, false),
        ("ss14", Typography::set_stylistic_set14, false),
        ("ss15", Typography::set_stylistic_set15, false),
        ("ss16", Typography::set_stylistic_set16, false),
        ("ss17", Typography::set_stylistic_set17, false),
        ("ss18", Typography::set_stylistic_set18, false),
        ("ss19", Typography::set_stylistic_set19, false),
        ("ss20", Typography::set_stylistic_set20, false),
        ("cpsp", Typography::set_capital_spacing, false),
        ("kern", Typography::set_kerning, true),
        ("case", Typography::set_case_sensitive_forms, false),
        ("hist", Typography::set_historical_forms, false),
        ("zero", Typography::set_slashed_zero, false),
        ("mgrk", Typography::set_mathematical_greek, false),
    ];
    assert_eq!(cases.len(), 32);
    for &(tag, setter, default) in cases {
        let run = Run::new();
        for value in [true, false] {
            setter(&run, value);
            let features = feature_values(&TypographyFormat::effective(&run).to_font_features());
            let actual = features
                .iter()
                .find(|feature| feature.0 == tag)
                .map(|feature| feature.1);
            assert_eq!(
                actual,
                if value || default {
                    Some(i32::from(value))
                } else {
                    None
                },
                "{tag}: {value}"
            );
        }
    }
}

#[test]
fn typography_maps_all_38_enum_choices_to_exact_feature_tags() {
    macro_rules! choices {
        ($field:ident, $enum:ident, [$($variant:ident => [$($tag:literal),*]),* $(,)?]) => {
            $(assert_eq!(nondefault_tags(&TypographyFormat {
                $field: Some($enum::$variant), ..Default::default()
            }), vec![$($tag.to_owned()),*] as Vec<String>, stringify!($variant));)*
        };
    }
    choices!(east_asian_language, FontEastAsianLanguage, [
        Normal => [], HojoKanji => ["hojo"], Jis04 => ["jp04"], Jis78 => ["jp78"],
        Jis83 => ["jp83"], Jis90 => ["jp90"], NlcKanji => ["nlck"], Simplified => ["smpl"],
        Traditional => ["trad"], TraditionalNames => ["tnam"],
    ]);
    choices!(east_asian_widths, FontEastAsianWidths, [
        Normal => [], Full => ["fwid"], Half => ["half"], Proportional => ["pwid"],
        Quarter => ["qwid"], Third => ["twid"],
    ]);
    choices!(capitals, FontCapitals, [
        Normal => [], AllSmallCaps => ["c2sc", "smcp"], SmallCaps => ["smcp"],
        AllPetiteCaps => ["c2pc", "pcap"], PetiteCaps => ["pcap"], Unicase => ["unic"], Titling => ["titl"],
    ]);
    choices!(fraction, FontFraction, [Normal => [], Stacked => ["afrc"], Slashed => ["frac"]]);
    choices!(numeral_style, FontNumeralStyle, [Normal => [], Lining => ["lnum"], OldStyle => ["onum"]]);
    choices!(numeral_alignment, FontNumeralAlignment, [Normal => [], Proportional => ["pnum"], Tabular => ["tnum"]]);
    choices!(variants, FontVariants, [Normal => [], Superscript => ["sups"], Subscript => ["subs"], Ordinal => ["ordn"], Inferior => ["sinf"], Ruby => ["ruby"]]);
}

#[test]
fn typography_integer_selectors_preserve_signed_parameters_and_zero_clears_features() {
    let _scope = test_scope();
    let cases: &[(&str, fn(&FerroObject, i32))] = &[
        ("nalt", Typography::set_annotation_alternates),
        ("swsh", Typography::set_standard_swashes),
        ("cswh", Typography::set_contextual_swashes),
        ("salt", Typography::set_stylistic_alternates),
    ];
    for &(tag, setter) in cases {
        let run = Run::new();
        for value in [1, 7, -1, i32::MIN, i32::MAX, 0] {
            setter(&run, value);
            let features = feature_values(&TypographyFormat::effective(&run).to_font_features());
            assert_eq!(
                features
                    .iter()
                    .find(|feature| feature.0 == tag)
                    .map(|feature| feature.1),
                (value != 0).then_some(value),
                "{tag}: {value}"
            );
        }
    }
}

#[test]
fn typography_inherits_through_document_nodes_and_explicit_defaults_reset_each_family() {
    let _scope = test_scope();
    let view = RichTextBlock::new();
    let paragraph = Paragraph::new();
    let span = Span::new();
    let run = Run::with_text(Some("inherited"));
    span.inlines().add(run.clone());
    paragraph.inlines().add(span.clone());
    view.blocks().add(paragraph.clone());
    let patch = TypographyFormat {
        annotation_alternates: Some(3),
        east_asian_expert_forms: Some(true),
        east_asian_language: Some(FontEastAsianLanguage::Jis90),
        east_asian_widths: Some(FontEastAsianWidths::Full),
        standard_ligatures: Some(false),
        contextual_ligatures: Some(false),
        discretionary_ligatures: Some(true),
        historical_ligatures: Some(true),
        standard_swashes: Some(4),
        contextual_swashes: Some(5),
        contextual_alternates: Some(false),
        stylistic_alternates: Some(6),
        stylistic_set1: Some(true),
        stylistic_set2: Some(true),
        stylistic_set3: Some(true),
        stylistic_set4: Some(true),
        stylistic_set5: Some(true),
        stylistic_set6: Some(true),
        stylistic_set7: Some(true),
        stylistic_set8: Some(true),
        stylistic_set9: Some(true),
        stylistic_set10: Some(true),
        stylistic_set11: Some(true),
        stylistic_set12: Some(true),
        stylistic_set13: Some(true),
        stylistic_set14: Some(true),
        stylistic_set15: Some(true),
        stylistic_set16: Some(true),
        stylistic_set17: Some(true),
        stylistic_set18: Some(true),
        stylistic_set19: Some(true),
        stylistic_set20: Some(true),
        capitals: Some(FontCapitals::SmallCaps),
        capital_spacing: Some(true),
        kerning: Some(false),
        case_sensitive_forms: Some(true),
        historical_forms: Some(true),
        fraction: Some(FontFraction::Stacked),
        numeral_style: Some(FontNumeralStyle::OldStyle),
        numeral_alignment: Some(FontNumeralAlignment::Tabular),
        slashed_zero: Some(true),
        mathematical_greek: Some(true),
        variants: Some(FontVariants::Superscript),
    };
    patch.apply_to(&view);
    assert_eq!(TypographyFormat::effective(&paragraph), patch);
    assert_eq!(TypographyFormat::effective(&span), patch);
    assert_eq!(TypographyFormat::effective(&run), patch);
    assert!(TypographyFormat::from_object(&run).is_empty());
    defaults().apply_to(&span);
    assert_eq!(TypographyFormat::effective(&run), defaults());
    assert_eq!(TypographyFormat::from_object(&span), defaults());
    assert!(Typography::effective_font_features(&run).is_none());
    TypographyFormat::clear_on(&span);
    assert_eq!(TypographyFormat::effective(&run), patch);
}

#[test]
fn typography_raw_features_are_last_and_keep_ranges_duplicates_and_collection_identity() {
    let _scope = test_scope();
    let run = Run::with_text(Some("office"));
    let raw = FontFeatureCollection::from_items([
        FontFeature {
            tag: "liga".into(),
            value: 0,
            start: 0,
            end: -1,
        },
        FontFeature {
            tag: "liga".into(),
            value: 1,
            start: 1,
            end: 4,
        },
        FontFeature {
            tag: "ss01".into(),
            value: 3,
            start: 2,
            end: -1,
        },
    ]);
    run.set_font_features(Some(raw.clone()));
    assert!(
        Typography::effective_font_features(&run) == Some(raw.clone()),
        "default typography preserves the raw collection identity"
    );
    Typography::set_standard_ligatures(&run, false);
    Typography::set_stylistic_set1(&run, true);
    let merged = Typography::effective_font_features(&run).unwrap();
    assert!(merged != raw);
    let values = merged.list().snapshot();
    assert_eq!(
        &values[values.len() - 3..],
        raw.list().snapshot().as_slice()
    );
    assert_eq!(
        values
            .iter()
            .filter(|feature| feature.tag == "liga")
            .map(|feature| (feature.value, feature.start, feature.end))
            .collect::<Vec<_>>(),
        [(0, 0, -1), (0, 0, -1), (1, 1, 4)]
    );
    assert_eq!(raw.len(), 3);
    assert_eq!(merged.len(), 8);
    raw.add(FontFeature {
        tag: "kern".into(),
        value: 0,
        start: 0,
        end: -1,
    });
    assert_eq!(
        merged.len(),
        8,
        "a shaped snapshot does not retain a mutable source list"
    );
}

#[test]
fn typography_format_patches_and_character_snapshots_preserve_independent_assignments() {
    let _scope = test_scope();
    let run = Run::with_text(Some("text"));
    Typography::set_capitals(&run, FontCapitals::SmallCaps);
    Typography::set_kerning(&run, false);
    let before = CharacterFormat::from_element(&run);
    let patch = CharacterFormat {
        typography: TypographyFormat {
            capitals: Some(FontCapitals::Normal),
            standard_swashes: Some(9),
            ..Default::default()
        },
        ..Default::default()
    };
    let merged = before.overlay(&patch);
    assert_eq!(merged.typography.kerning, Some(false));
    assert_eq!(merged.typography.capitals, Some(FontCapitals::Normal));
    assert_eq!(merged.typography.standard_swashes, Some(9));
    merged.apply_to(&run);
    assert_eq!(
        CharacterFormat::from_element(&run).typography,
        merged.typography
    );
    let captured = CharacterFormat::from_element(&run);
    Typography::set_kerning(&run, true);
    Typography::set_slashed_zero(&run, true);
    captured.replace_on(&run);
    assert_eq!(CharacterFormat::from_element(&run), captured);
    assert!(!Typography::get_slashed_zero(&run));
    assert!(!TypographyFormat::from_object(&run).is_default());
}

#[test]
fn typography_rich_splits_undo_redo_and_snapshot_restore_retain_full_format_patches() {
    let _scope = test_scope();
    let document = RichTextDocument::from_text("abcd");
    let facade = RichEditTextDocument::new(document.clone());
    let before = document.snapshot();
    let patch = CharacterFormat {
        typography: TypographyFormat {
            capitals: Some(FontCapitals::AllSmallCaps),
            standard_ligatures: Some(false),
            stylistic_set20: Some(true),
            standard_swashes: Some(4),
            ..Default::default()
        },
        ..Default::default()
    };
    facade
        .get_range(1, 3)
        .unwrap()
        .set_character_format(&patch)
        .unwrap();
    let formatted = document.snapshot();
    assert!(facade.undo().unwrap());
    assert_eq!(document.snapshot(), before);
    assert!(facade.redo().unwrap());
    assert_eq!(document.snapshot(), formatted);
    document.replace_text(2..2, "x\ny").unwrap();
    assert_eq!(document.text(), "abx\nycd");
    for position in [2, 3, 5] {
        let format = document.character_format_at(position).unwrap().typography;
        assert_eq!(format.capitals, patch.typography.capitals);
        assert_eq!(
            format.standard_ligatures,
            patch.typography.standard_ligatures
        );
        assert_eq!(format.stylistic_set20, patch.typography.stylistic_set20);
        assert_eq!(format.standard_swashes, patch.typography.standard_swashes);
    }
    document.restore(&formatted).unwrap();
    assert_eq!(document.snapshot(), formatted);
}

#[test]
fn typography_inherited_presentation_invalidates_layout_preserving_model_and_composition() {
    let _scope = test_scope();
    let control = RichEditBox::new();
    control.set_text("original").unwrap();
    control.select(8, 8);
    control.insert_text("!").unwrap();
    control.set_preedit_text(Some("preview"), Some(2)).unwrap();
    let document = control.document();
    let snapshot = document.snapshot();
    let revision = document.revision();
    let mutation = document.mutation_version();
    let composition = control.editor().active_composition_range();
    let changed = Rc::new(Cell::new(0));
    let count = changed.clone();
    let _subscription = document.changed(move |_| count.set(count.get() + 1));
    let presenter = control.text_presenter();
    presenter.measure(Size::new(400.0, 300.0));
    presenter.arrange(Rect::new(0.0, 0.0, 400.0, 300.0));
    Typography::set_kerning(&control, false);
    Typography::set_numeral_alignment(&control, FontNumeralAlignment::Tabular);
    assert!(!presenter.is_measure_valid());
    assert!(!Typography::get_kerning(&document.paragraphs()[0]));
    presenter.measure(Size::new(400.0, 300.0));
    assert_eq!(document.snapshot(), snapshot);
    assert_eq!(document.revision(), revision);
    assert_eq!(document.mutation_version(), mutation);
    assert_eq!(changed.get(), 0);
    assert_eq!(control.editor().active_composition_range(), composition);
    assert!(control.editor().is_composing());
    control.cancel_composition().unwrap();
    assert!(control.undo().unwrap());
    assert_eq!(control.text(), "original");
    assert!(control.redo().unwrap());
    assert_eq!(control.text(), "original!");
    assert!(!Typography::get_kerning(&document.paragraphs()[0]));
}

#[test]
fn typography_local_property_assignment_and_clear_are_document_mutations() {
    let _scope = test_scope();
    let view = RichTextBlock::new();
    Typography::set_kerning(&view, false);
    let document = RichTextDocument::from_text("abc");
    view.set_document(document.clone());
    let editor = RichTextEditor::new(document.clone());
    editor.insert_text("!", EditKind::Typing).unwrap();
    assert!(editor.can_undo());
    let run = document.paragraphs()[0]
        .inlines()
        .get(0)
        .cast::<Run>()
        .unwrap();
    let revision = document.revision();
    Typography::set_kerning(&run, true);
    assert!(document.revision() > revision);
    assert!(!editor.can_undo());
    let revision = document.revision();
    let mutation = document.mutation_version();
    let batch = document.begin_edit();
    run.clear_value(Typography::kerning_property());
    assert!(!Typography::get_kerning(&run));
    assert_eq!(document.revision(), revision);
    assert!(document.mutation_version() > mutation);
    batch.finish();
    assert!(document.revision() > revision);
    assert_eq!(TypographyFormat::from_object(&run).kerning, None);
}

#[test]
fn typography_structured_fragment_retains_nested_default_resets_after_source_drop() {
    let _scope = test_scope();
    let fragment = {
        let source = RichTextDocument::new();
        let paragraph = Paragraph::new();
        let span = Span::new();
        let run = Run::with_text(Some("abcd"));
        Typography::set_capitals(&paragraph, FontCapitals::SmallCaps);
        Typography::set_capitals(&span, FontCapitals::Normal);
        Typography::set_standard_ligatures(&span, false);
        Typography::set_standard_ligatures(&run, true);
        span.inlines().add(run);
        paragraph.inlines().add(span);
        source.blocks().add(paragraph);
        source.slice(1..3).unwrap()
    };
    let target = RichTextDocument::from_text("");
    let before = target.snapshot();
    let facade = RichEditTextDocument::new(target.clone());
    facade
        .editor()
        .insert_fragment(&fragment, EditKind::Paste)
        .unwrap();
    assert_eq!(target.text(), "bc");
    let paragraph_format = target.paragraphs()[0]
        .inlines()
        .get(0)
        .cast::<Span>()
        .unwrap();
    assert_eq!(
        TypographyFormat::from_object(&paragraph_format).capitals,
        Some(FontCapitals::SmallCaps)
    );
    let span = paragraph_format.inlines().get(0).cast::<Span>().unwrap();
    let run = span.inlines().get(0).cast::<Run>().unwrap();
    assert_eq!(Typography::get_capitals(&run), FontCapitals::Normal);
    assert!(Typography::get_standard_ligatures(&run));
    assert_eq!(
        TypographyFormat::from_object(&span).capitals,
        Some(FontCapitals::Normal)
    );
    assert_eq!(
        TypographyFormat::from_object(&run).standard_ligatures,
        Some(true)
    );
    let copied = target.snapshot();
    assert!(facade.undo().unwrap());
    assert_eq!(target.snapshot(), before);
    assert!(facade.redo().unwrap());
    assert_eq!(target.snapshot(), copied);
}

#[test]
fn typography_rtf_rejects_nondefault_features_and_exports_resolved_default_typing() {
    let _scope = test_scope();
    for typography in [
        TypographyFormat {
            capitals: Some(FontCapitals::SmallCaps),
            ..Default::default()
        },
        TypographyFormat {
            standard_ligatures: Some(false),
            ..Default::default()
        },
    ] {
        let document = RichTextDocument::from_text("text");
        document
            .apply_format(
                0..4,
                &CharacterFormat {
                    typography,
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(
            RtfWriter::write(&document.snapshot()).unwrap_err(),
            RtfError::UnsupportedFeature("OpenType feature overrides")
        );
    }
    assert!(RtfWriter::write(&RichTextDocument::from_text("plain").snapshot()).is_ok());
    let control = RichEditBox::new();
    control.set_text("typed").unwrap();
    control.select(5, 5);
    control.insert_text(" text").unwrap();
    assert!(RtfWriter::write(&control.document().snapshot()).is_ok());
    let document = RichTextDocument::from_text("text");
    document
        .apply_format(
            0..4,
            &CharacterFormat {
                typography: TypographyFormat {
                    standard_ligatures: Some(true),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();
    assert!(RtfWriter::write(&document.snapshot()).is_ok());
}
