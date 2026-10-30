use crate::documents::{
    CharacterFormat, Paragraph, RichTextDocument, Run, Span, Typography, TypographyFormat,
};
use crate::test_support::test_scope;
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
