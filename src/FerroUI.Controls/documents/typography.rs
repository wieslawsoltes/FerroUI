//! Inherited typography attached properties over the shared font-feature pipeline.
//! The formatter consumes resolved values; these properties never rewrite text.

use super::{TextElement, TypographyFormat};
use ferroui_base::media::{
    FontCapitals, FontEastAsianLanguage, FontEastAsianWidths, FontFeatureCollection, FontFraction,
    FontNumeralAlignment, FontNumeralStyle, FontVariants,
};
use ferroui_base::{
    ferro_properties, ferro_static_type, AttachedProperty, FerroObject, FerroProperty, Ref,
    StyledPropertyOptions,
};

/// Selects named OpenType features through inherited attached properties.
///
/// Fonts supply the actual glyph substitutions and positioning. An unsupported
/// feature leaves the text as the font shapes it; no synthetic glyph or text
/// conversion is introduced. The existing raw `FontFeatures` collection is an
/// explicit final override layer and retains its original order and ranges.
pub struct Typography;

ferro_static_type!(Typography);

ferro_properties! {
    impl Typography {
        /// The inherited `AnnotationAlternates` property.
        pub fn annotation_alternates_property() -> AttachedProperty<i32> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "AnnotationAlternates", StyledPropertyOptions::new(0).inherits(true),
            )
        }

        /// The inherited `EastAsianExpertForms` property.
        pub fn east_asian_expert_forms_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "EastAsianExpertForms", StyledPropertyOptions::new(false).inherits(true),
            )
        }

        /// The inherited `EastAsianLanguage` property.
        pub fn east_asian_language_property() -> AttachedProperty<FontEastAsianLanguage> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "EastAsianLanguage", StyledPropertyOptions::new(FontEastAsianLanguage::Normal).inherits(true),
            )
        }

        /// The inherited `EastAsianWidths` property.
        pub fn east_asian_widths_property() -> AttachedProperty<FontEastAsianWidths> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "EastAsianWidths", StyledPropertyOptions::new(FontEastAsianWidths::Normal).inherits(true),
            )
        }

        /// The inherited `StandardLigatures` property.
        pub fn standard_ligatures_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "StandardLigatures", StyledPropertyOptions::new(true).inherits(true),
            )
        }

        /// The inherited `ContextualLigatures` property.
        pub fn contextual_ligatures_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "ContextualLigatures", StyledPropertyOptions::new(true).inherits(true),
            )
        }

        /// The inherited `DiscretionaryLigatures` property.
        pub fn discretionary_ligatures_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "DiscretionaryLigatures", StyledPropertyOptions::new(false).inherits(true),
            )
        }

        /// The inherited `HistoricalLigatures` property.
        pub fn historical_ligatures_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "HistoricalLigatures", StyledPropertyOptions::new(false).inherits(true),
            )
        }

        /// The inherited `StandardSwashes` property.
        pub fn standard_swashes_property() -> AttachedProperty<i32> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "StandardSwashes", StyledPropertyOptions::new(0).inherits(true),
            )
        }

        /// The inherited `ContextualSwashes` property.
        pub fn contextual_swashes_property() -> AttachedProperty<i32> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "ContextualSwashes", StyledPropertyOptions::new(0).inherits(true),
            )
        }

        /// The inherited `ContextualAlternates` property.
        pub fn contextual_alternates_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "ContextualAlternates", StyledPropertyOptions::new(true).inherits(true),
            )
        }

        /// The inherited `StylisticAlternates` property.
        pub fn stylistic_alternates_property() -> AttachedProperty<i32> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "StylisticAlternates", StyledPropertyOptions::new(0).inherits(true),
            )
        }

        /// The inherited `StylisticSet1` property.
        pub fn stylistic_set1_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "StylisticSet1", StyledPropertyOptions::new(false).inherits(true),
            )
        }

        /// The inherited `StylisticSet2` property.
        pub fn stylistic_set2_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "StylisticSet2", StyledPropertyOptions::new(false).inherits(true),
            )
        }

        /// The inherited `StylisticSet3` property.
        pub fn stylistic_set3_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "StylisticSet3", StyledPropertyOptions::new(false).inherits(true),
            )
        }

        /// The inherited `StylisticSet4` property.
        pub fn stylistic_set4_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "StylisticSet4", StyledPropertyOptions::new(false).inherits(true),
            )
        }

        /// The inherited `StylisticSet5` property.
        pub fn stylistic_set5_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "StylisticSet5", StyledPropertyOptions::new(false).inherits(true),
            )
        }

        /// The inherited `StylisticSet6` property.
        pub fn stylistic_set6_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "StylisticSet6", StyledPropertyOptions::new(false).inherits(true),
            )
        }

        /// The inherited `StylisticSet7` property.
        pub fn stylistic_set7_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "StylisticSet7", StyledPropertyOptions::new(false).inherits(true),
            )
        }

        /// The inherited `StylisticSet8` property.
        pub fn stylistic_set8_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "StylisticSet8", StyledPropertyOptions::new(false).inherits(true),
            )
        }

        /// The inherited `StylisticSet9` property.
        pub fn stylistic_set9_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "StylisticSet9", StyledPropertyOptions::new(false).inherits(true),
            )
        }

        /// The inherited `StylisticSet10` property.
        pub fn stylistic_set10_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "StylisticSet10", StyledPropertyOptions::new(false).inherits(true),
            )
        }

        /// The inherited `StylisticSet11` property.
        pub fn stylistic_set11_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "StylisticSet11", StyledPropertyOptions::new(false).inherits(true),
            )
        }

        /// The inherited `StylisticSet12` property.
        pub fn stylistic_set12_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "StylisticSet12", StyledPropertyOptions::new(false).inherits(true),
            )
        }

        /// The inherited `StylisticSet13` property.
        pub fn stylistic_set13_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "StylisticSet13", StyledPropertyOptions::new(false).inherits(true),
            )
        }

        /// The inherited `StylisticSet14` property.
        pub fn stylistic_set14_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "StylisticSet14", StyledPropertyOptions::new(false).inherits(true),
            )
        }

        /// The inherited `StylisticSet15` property.
        pub fn stylistic_set15_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "StylisticSet15", StyledPropertyOptions::new(false).inherits(true),
            )
        }

        /// The inherited `StylisticSet16` property.
        pub fn stylistic_set16_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "StylisticSet16", StyledPropertyOptions::new(false).inherits(true),
            )
        }

        /// The inherited `StylisticSet17` property.
        pub fn stylistic_set17_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "StylisticSet17", StyledPropertyOptions::new(false).inherits(true),
            )
        }

        /// The inherited `StylisticSet18` property.
        pub fn stylistic_set18_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "StylisticSet18", StyledPropertyOptions::new(false).inherits(true),
            )
        }

        /// The inherited `StylisticSet19` property.
        pub fn stylistic_set19_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "StylisticSet19", StyledPropertyOptions::new(false).inherits(true),
            )
        }

        /// The inherited `StylisticSet20` property.
        pub fn stylistic_set20_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "StylisticSet20", StyledPropertyOptions::new(false).inherits(true),
            )
        }

        /// The inherited `Capitals` property.
        pub fn capitals_property() -> AttachedProperty<FontCapitals> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "Capitals", StyledPropertyOptions::new(FontCapitals::Normal).inherits(true),
            )
        }

        /// The inherited `CapitalSpacing` property.
        pub fn capital_spacing_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "CapitalSpacing", StyledPropertyOptions::new(false).inherits(true),
            )
        }

        /// The inherited `Kerning` property.
        pub fn kerning_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "Kerning", StyledPropertyOptions::new(true).inherits(true),
            )
        }

        /// The inherited `CaseSensitiveForms` property.
        pub fn case_sensitive_forms_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "CaseSensitiveForms", StyledPropertyOptions::new(false).inherits(true),
            )
        }

        /// The inherited `HistoricalForms` property.
        pub fn historical_forms_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "HistoricalForms", StyledPropertyOptions::new(false).inherits(true),
            )
        }

        /// The inherited `Fraction` property.
        pub fn fraction_property() -> AttachedProperty<FontFraction> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "Fraction", StyledPropertyOptions::new(FontFraction::Normal).inherits(true),
            )
        }

        /// The inherited `NumeralStyle` property.
        pub fn numeral_style_property() -> AttachedProperty<FontNumeralStyle> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "NumeralStyle", StyledPropertyOptions::new(FontNumeralStyle::Normal).inherits(true),
            )
        }

        /// The inherited `NumeralAlignment` property.
        pub fn numeral_alignment_property() -> AttachedProperty<FontNumeralAlignment> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "NumeralAlignment", StyledPropertyOptions::new(FontNumeralAlignment::Normal).inherits(true),
            )
        }

        /// The inherited `SlashedZero` property.
        pub fn slashed_zero_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "SlashedZero", StyledPropertyOptions::new(false).inherits(true),
            )
        }

        /// The inherited `MathematicalGreek` property.
        pub fn mathematical_greek_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "MathematicalGreek", StyledPropertyOptions::new(false).inherits(true),
            )
        }

        /// The inherited `Variants` property.
        pub fn variants_property() -> AttachedProperty<FontVariants> {
            FerroProperty::register_attached_with::<Typography, FerroObject, _>(
                "Variants", StyledPropertyOptions::new(FontVariants::Normal).inherits(true),
            )
        }

    }
}

impl Typography {
    fn static_constructor() {}

    pub fn get_annotation_alternates(element: &FerroObject) -> i32 {
        element.get_value(Self::annotation_alternates_property())
    }

    pub fn set_annotation_alternates(element: &FerroObject, value: i32) {
        element.set_value(Self::annotation_alternates_property(), value);
    }

    pub fn get_east_asian_expert_forms(element: &FerroObject) -> bool {
        element.get_value(Self::east_asian_expert_forms_property())
    }

    pub fn set_east_asian_expert_forms(element: &FerroObject, value: bool) {
        element.set_value(Self::east_asian_expert_forms_property(), value);
    }

    pub fn get_east_asian_language(element: &FerroObject) -> FontEastAsianLanguage {
        element.get_value(Self::east_asian_language_property())
    }

    pub fn set_east_asian_language(element: &FerroObject, value: FontEastAsianLanguage) {
        element.set_value(Self::east_asian_language_property(), value);
    }

    pub fn get_east_asian_widths(element: &FerroObject) -> FontEastAsianWidths {
        element.get_value(Self::east_asian_widths_property())
    }

    pub fn set_east_asian_widths(element: &FerroObject, value: FontEastAsianWidths) {
        element.set_value(Self::east_asian_widths_property(), value);
    }

    pub fn get_standard_ligatures(element: &FerroObject) -> bool {
        element.get_value(Self::standard_ligatures_property())
    }

    pub fn set_standard_ligatures(element: &FerroObject, value: bool) {
        element.set_value(Self::standard_ligatures_property(), value);
    }

    pub fn get_contextual_ligatures(element: &FerroObject) -> bool {
        element.get_value(Self::contextual_ligatures_property())
    }

    pub fn set_contextual_ligatures(element: &FerroObject, value: bool) {
        element.set_value(Self::contextual_ligatures_property(), value);
    }

    pub fn get_discretionary_ligatures(element: &FerroObject) -> bool {
        element.get_value(Self::discretionary_ligatures_property())
    }

    pub fn set_discretionary_ligatures(element: &FerroObject, value: bool) {
        element.set_value(Self::discretionary_ligatures_property(), value);
    }

    pub fn get_historical_ligatures(element: &FerroObject) -> bool {
        element.get_value(Self::historical_ligatures_property())
    }

    pub fn set_historical_ligatures(element: &FerroObject, value: bool) {
        element.set_value(Self::historical_ligatures_property(), value);
    }

    pub fn get_standard_swashes(element: &FerroObject) -> i32 {
        element.get_value(Self::standard_swashes_property())
    }

    pub fn set_standard_swashes(element: &FerroObject, value: i32) {
        element.set_value(Self::standard_swashes_property(), value);
    }

    pub fn get_contextual_swashes(element: &FerroObject) -> i32 {
        element.get_value(Self::contextual_swashes_property())
    }

    pub fn set_contextual_swashes(element: &FerroObject, value: i32) {
        element.set_value(Self::contextual_swashes_property(), value);
    }

    pub fn get_contextual_alternates(element: &FerroObject) -> bool {
        element.get_value(Self::contextual_alternates_property())
    }

    pub fn set_contextual_alternates(element: &FerroObject, value: bool) {
        element.set_value(Self::contextual_alternates_property(), value);
    }

    pub fn get_stylistic_alternates(element: &FerroObject) -> i32 {
        element.get_value(Self::stylistic_alternates_property())
    }

    pub fn set_stylistic_alternates(element: &FerroObject, value: i32) {
        element.set_value(Self::stylistic_alternates_property(), value);
    }

    pub fn get_stylistic_set1(element: &FerroObject) -> bool {
        element.get_value(Self::stylistic_set1_property())
    }

    pub fn set_stylistic_set1(element: &FerroObject, value: bool) {
        element.set_value(Self::stylistic_set1_property(), value);
    }

    pub fn get_stylistic_set2(element: &FerroObject) -> bool {
        element.get_value(Self::stylistic_set2_property())
    }

    pub fn set_stylistic_set2(element: &FerroObject, value: bool) {
        element.set_value(Self::stylistic_set2_property(), value);
    }

    pub fn get_stylistic_set3(element: &FerroObject) -> bool {
        element.get_value(Self::stylistic_set3_property())
    }

    pub fn set_stylistic_set3(element: &FerroObject, value: bool) {
        element.set_value(Self::stylistic_set3_property(), value);
    }

    pub fn get_stylistic_set4(element: &FerroObject) -> bool {
        element.get_value(Self::stylistic_set4_property())
    }

    pub fn set_stylistic_set4(element: &FerroObject, value: bool) {
        element.set_value(Self::stylistic_set4_property(), value);
    }

    pub fn get_stylistic_set5(element: &FerroObject) -> bool {
        element.get_value(Self::stylistic_set5_property())
    }

    pub fn set_stylistic_set5(element: &FerroObject, value: bool) {
        element.set_value(Self::stylistic_set5_property(), value);
    }

    pub fn get_stylistic_set6(element: &FerroObject) -> bool {
        element.get_value(Self::stylistic_set6_property())
    }

    pub fn set_stylistic_set6(element: &FerroObject, value: bool) {
        element.set_value(Self::stylistic_set6_property(), value);
    }

    pub fn get_stylistic_set7(element: &FerroObject) -> bool {
        element.get_value(Self::stylistic_set7_property())
    }

    pub fn set_stylistic_set7(element: &FerroObject, value: bool) {
        element.set_value(Self::stylistic_set7_property(), value);
    }

    pub fn get_stylistic_set8(element: &FerroObject) -> bool {
        element.get_value(Self::stylistic_set8_property())
    }

    pub fn set_stylistic_set8(element: &FerroObject, value: bool) {
        element.set_value(Self::stylistic_set8_property(), value);
    }

    pub fn get_stylistic_set9(element: &FerroObject) -> bool {
        element.get_value(Self::stylistic_set9_property())
    }

    pub fn set_stylistic_set9(element: &FerroObject, value: bool) {
        element.set_value(Self::stylistic_set9_property(), value);
    }

    pub fn get_stylistic_set10(element: &FerroObject) -> bool {
        element.get_value(Self::stylistic_set10_property())
    }

    pub fn set_stylistic_set10(element: &FerroObject, value: bool) {
        element.set_value(Self::stylistic_set10_property(), value);
    }

    pub fn get_stylistic_set11(element: &FerroObject) -> bool {
        element.get_value(Self::stylistic_set11_property())
    }

    pub fn set_stylistic_set11(element: &FerroObject, value: bool) {
        element.set_value(Self::stylistic_set11_property(), value);
    }

    pub fn get_stylistic_set12(element: &FerroObject) -> bool {
        element.get_value(Self::stylistic_set12_property())
    }

    pub fn set_stylistic_set12(element: &FerroObject, value: bool) {
        element.set_value(Self::stylistic_set12_property(), value);
    }

    pub fn get_stylistic_set13(element: &FerroObject) -> bool {
        element.get_value(Self::stylistic_set13_property())
    }

    pub fn set_stylistic_set13(element: &FerroObject, value: bool) {
        element.set_value(Self::stylistic_set13_property(), value);
    }

    pub fn get_stylistic_set14(element: &FerroObject) -> bool {
        element.get_value(Self::stylistic_set14_property())
    }

    pub fn set_stylistic_set14(element: &FerroObject, value: bool) {
        element.set_value(Self::stylistic_set14_property(), value);
    }

    pub fn get_stylistic_set15(element: &FerroObject) -> bool {
        element.get_value(Self::stylistic_set15_property())
    }

    pub fn set_stylistic_set15(element: &FerroObject, value: bool) {
        element.set_value(Self::stylistic_set15_property(), value);
    }

    pub fn get_stylistic_set16(element: &FerroObject) -> bool {
        element.get_value(Self::stylistic_set16_property())
    }

    pub fn set_stylistic_set16(element: &FerroObject, value: bool) {
        element.set_value(Self::stylistic_set16_property(), value);
    }

    pub fn get_stylistic_set17(element: &FerroObject) -> bool {
        element.get_value(Self::stylistic_set17_property())
    }

    pub fn set_stylistic_set17(element: &FerroObject, value: bool) {
        element.set_value(Self::stylistic_set17_property(), value);
    }

    pub fn get_stylistic_set18(element: &FerroObject) -> bool {
        element.get_value(Self::stylistic_set18_property())
    }

    pub fn set_stylistic_set18(element: &FerroObject, value: bool) {
        element.set_value(Self::stylistic_set18_property(), value);
    }

    pub fn get_stylistic_set19(element: &FerroObject) -> bool {
        element.get_value(Self::stylistic_set19_property())
    }

    pub fn set_stylistic_set19(element: &FerroObject, value: bool) {
        element.set_value(Self::stylistic_set19_property(), value);
    }

    pub fn get_stylistic_set20(element: &FerroObject) -> bool {
        element.get_value(Self::stylistic_set20_property())
    }

    pub fn set_stylistic_set20(element: &FerroObject, value: bool) {
        element.set_value(Self::stylistic_set20_property(), value);
    }

    pub fn get_capitals(element: &FerroObject) -> FontCapitals {
        element.get_value(Self::capitals_property())
    }

    pub fn set_capitals(element: &FerroObject, value: FontCapitals) {
        element.set_value(Self::capitals_property(), value);
    }

    pub fn get_capital_spacing(element: &FerroObject) -> bool {
        element.get_value(Self::capital_spacing_property())
    }

    pub fn set_capital_spacing(element: &FerroObject, value: bool) {
        element.set_value(Self::capital_spacing_property(), value);
    }

    pub fn get_kerning(element: &FerroObject) -> bool {
        element.get_value(Self::kerning_property())
    }

    pub fn set_kerning(element: &FerroObject, value: bool) {
        element.set_value(Self::kerning_property(), value);
    }

    pub fn get_case_sensitive_forms(element: &FerroObject) -> bool {
        element.get_value(Self::case_sensitive_forms_property())
    }

    pub fn set_case_sensitive_forms(element: &FerroObject, value: bool) {
        element.set_value(Self::case_sensitive_forms_property(), value);
    }

    pub fn get_historical_forms(element: &FerroObject) -> bool {
        element.get_value(Self::historical_forms_property())
    }

    pub fn set_historical_forms(element: &FerroObject, value: bool) {
        element.set_value(Self::historical_forms_property(), value);
    }

    pub fn get_fraction(element: &FerroObject) -> FontFraction {
        element.get_value(Self::fraction_property())
    }

    pub fn set_fraction(element: &FerroObject, value: FontFraction) {
        element.set_value(Self::fraction_property(), value);
    }

    pub fn get_numeral_style(element: &FerroObject) -> FontNumeralStyle {
        element.get_value(Self::numeral_style_property())
    }

    pub fn set_numeral_style(element: &FerroObject, value: FontNumeralStyle) {
        element.set_value(Self::numeral_style_property(), value);
    }

    pub fn get_numeral_alignment(element: &FerroObject) -> FontNumeralAlignment {
        element.get_value(Self::numeral_alignment_property())
    }

    pub fn set_numeral_alignment(element: &FerroObject, value: FontNumeralAlignment) {
        element.set_value(Self::numeral_alignment_property(), value);
    }

    pub fn get_slashed_zero(element: &FerroObject) -> bool {
        element.get_value(Self::slashed_zero_property())
    }

    pub fn set_slashed_zero(element: &FerroObject, value: bool) {
        element.set_value(Self::slashed_zero_property(), value);
    }

    pub fn get_mathematical_greek(element: &FerroObject) -> bool {
        element.get_value(Self::mathematical_greek_property())
    }

    pub fn set_mathematical_greek(element: &FerroObject, value: bool) {
        element.set_value(Self::mathematical_greek_property(), value);
    }

    pub fn get_variants(element: &FerroObject) -> FontVariants {
        element.get_value(Self::variants_property())
    }

    pub fn set_variants(element: &FerroObject, value: FontVariants) {
        element.set_value(Self::variants_property(), value);
    }

    pub(crate) fn is_property(property: &FerroProperty) -> bool {
        property.owner_type() == Self::TYPE
    }

    /// Resolves inherited typography before appending explicit font features.
    /// The assigned collection is not modified or retained by a derived collection.
    pub fn effective_font_features(element: &FerroObject) -> Option<FontFeatureCollection> {
        TypographyFormat::effective(element)
            .with_font_features(element.get_value(TextElement::font_features_property()))
    }
}

ferroui_base::ferro_markup_type!(static Typography {
    namespace: "FerroUI.Controls.Documents",
    type_info: Typography,
    methods: [
        static fn GetAnnotationAlternates(Ref<FerroObject>) -> i32 =>
            |element: Ref<FerroObject>| Typography::get_annotation_alternates(&element),
        static fn SetAnnotationAlternates(Ref<FerroObject>, i32) =>
            |element: Ref<FerroObject>, value: i32| Typography::set_annotation_alternates(&element, value),
        static fn GetEastAsianExpertForms(Ref<FerroObject>) -> bool =>
            |element: Ref<FerroObject>| Typography::get_east_asian_expert_forms(&element),
        static fn SetEastAsianExpertForms(Ref<FerroObject>, bool) =>
            |element: Ref<FerroObject>, value: bool| Typography::set_east_asian_expert_forms(&element, value),
        static fn GetEastAsianLanguage(Ref<FerroObject>) -> FontEastAsianLanguage =>
            |element: Ref<FerroObject>| Typography::get_east_asian_language(&element),
        static fn SetEastAsianLanguage(Ref<FerroObject>, FontEastAsianLanguage) =>
            |element: Ref<FerroObject>, value: FontEastAsianLanguage| Typography::set_east_asian_language(&element, value),
        static fn GetEastAsianWidths(Ref<FerroObject>) -> FontEastAsianWidths =>
            |element: Ref<FerroObject>| Typography::get_east_asian_widths(&element),
        static fn SetEastAsianWidths(Ref<FerroObject>, FontEastAsianWidths) =>
            |element: Ref<FerroObject>, value: FontEastAsianWidths| Typography::set_east_asian_widths(&element, value),
        static fn GetStandardLigatures(Ref<FerroObject>) -> bool =>
            |element: Ref<FerroObject>| Typography::get_standard_ligatures(&element),
        static fn SetStandardLigatures(Ref<FerroObject>, bool) =>
            |element: Ref<FerroObject>, value: bool| Typography::set_standard_ligatures(&element, value),
        static fn GetContextualLigatures(Ref<FerroObject>) -> bool =>
            |element: Ref<FerroObject>| Typography::get_contextual_ligatures(&element),
        static fn SetContextualLigatures(Ref<FerroObject>, bool) =>
            |element: Ref<FerroObject>, value: bool| Typography::set_contextual_ligatures(&element, value),
        static fn GetDiscretionaryLigatures(Ref<FerroObject>) -> bool =>
            |element: Ref<FerroObject>| Typography::get_discretionary_ligatures(&element),
        static fn SetDiscretionaryLigatures(Ref<FerroObject>, bool) =>
            |element: Ref<FerroObject>, value: bool| Typography::set_discretionary_ligatures(&element, value),
        static fn GetHistoricalLigatures(Ref<FerroObject>) -> bool =>
            |element: Ref<FerroObject>| Typography::get_historical_ligatures(&element),
        static fn SetHistoricalLigatures(Ref<FerroObject>, bool) =>
            |element: Ref<FerroObject>, value: bool| Typography::set_historical_ligatures(&element, value),
        static fn GetStandardSwashes(Ref<FerroObject>) -> i32 =>
            |element: Ref<FerroObject>| Typography::get_standard_swashes(&element),
        static fn SetStandardSwashes(Ref<FerroObject>, i32) =>
            |element: Ref<FerroObject>, value: i32| Typography::set_standard_swashes(&element, value),
        static fn GetContextualSwashes(Ref<FerroObject>) -> i32 =>
            |element: Ref<FerroObject>| Typography::get_contextual_swashes(&element),
        static fn SetContextualSwashes(Ref<FerroObject>, i32) =>
            |element: Ref<FerroObject>, value: i32| Typography::set_contextual_swashes(&element, value),
        static fn GetContextualAlternates(Ref<FerroObject>) -> bool =>
            |element: Ref<FerroObject>| Typography::get_contextual_alternates(&element),
        static fn SetContextualAlternates(Ref<FerroObject>, bool) =>
            |element: Ref<FerroObject>, value: bool| Typography::set_contextual_alternates(&element, value),
        static fn GetStylisticAlternates(Ref<FerroObject>) -> i32 =>
            |element: Ref<FerroObject>| Typography::get_stylistic_alternates(&element),
        static fn SetStylisticAlternates(Ref<FerroObject>, i32) =>
            |element: Ref<FerroObject>, value: i32| Typography::set_stylistic_alternates(&element, value),
        static fn GetStylisticSet1(Ref<FerroObject>) -> bool =>
            |element: Ref<FerroObject>| Typography::get_stylistic_set1(&element),
        static fn SetStylisticSet1(Ref<FerroObject>, bool) =>
            |element: Ref<FerroObject>, value: bool| Typography::set_stylistic_set1(&element, value),
        static fn GetStylisticSet2(Ref<FerroObject>) -> bool =>
            |element: Ref<FerroObject>| Typography::get_stylistic_set2(&element),
        static fn SetStylisticSet2(Ref<FerroObject>, bool) =>
            |element: Ref<FerroObject>, value: bool| Typography::set_stylistic_set2(&element, value),
        static fn GetStylisticSet3(Ref<FerroObject>) -> bool =>
            |element: Ref<FerroObject>| Typography::get_stylistic_set3(&element),
        static fn SetStylisticSet3(Ref<FerroObject>, bool) =>
            |element: Ref<FerroObject>, value: bool| Typography::set_stylistic_set3(&element, value),
        static fn GetStylisticSet4(Ref<FerroObject>) -> bool =>
            |element: Ref<FerroObject>| Typography::get_stylistic_set4(&element),
        static fn SetStylisticSet4(Ref<FerroObject>, bool) =>
            |element: Ref<FerroObject>, value: bool| Typography::set_stylistic_set4(&element, value),
        static fn GetStylisticSet5(Ref<FerroObject>) -> bool =>
            |element: Ref<FerroObject>| Typography::get_stylistic_set5(&element),
        static fn SetStylisticSet5(Ref<FerroObject>, bool) =>
            |element: Ref<FerroObject>, value: bool| Typography::set_stylistic_set5(&element, value),
        static fn GetStylisticSet6(Ref<FerroObject>) -> bool =>
            |element: Ref<FerroObject>| Typography::get_stylistic_set6(&element),
        static fn SetStylisticSet6(Ref<FerroObject>, bool) =>
            |element: Ref<FerroObject>, value: bool| Typography::set_stylistic_set6(&element, value),
        static fn GetStylisticSet7(Ref<FerroObject>) -> bool =>
            |element: Ref<FerroObject>| Typography::get_stylistic_set7(&element),
        static fn SetStylisticSet7(Ref<FerroObject>, bool) =>
            |element: Ref<FerroObject>, value: bool| Typography::set_stylistic_set7(&element, value),
        static fn GetStylisticSet8(Ref<FerroObject>) -> bool =>
            |element: Ref<FerroObject>| Typography::get_stylistic_set8(&element),
        static fn SetStylisticSet8(Ref<FerroObject>, bool) =>
            |element: Ref<FerroObject>, value: bool| Typography::set_stylistic_set8(&element, value),
        static fn GetStylisticSet9(Ref<FerroObject>) -> bool =>
            |element: Ref<FerroObject>| Typography::get_stylistic_set9(&element),
        static fn SetStylisticSet9(Ref<FerroObject>, bool) =>
            |element: Ref<FerroObject>, value: bool| Typography::set_stylistic_set9(&element, value),
        static fn GetStylisticSet10(Ref<FerroObject>) -> bool =>
            |element: Ref<FerroObject>| Typography::get_stylistic_set10(&element),
        static fn SetStylisticSet10(Ref<FerroObject>, bool) =>
            |element: Ref<FerroObject>, value: bool| Typography::set_stylistic_set10(&element, value),
        static fn GetStylisticSet11(Ref<FerroObject>) -> bool =>
            |element: Ref<FerroObject>| Typography::get_stylistic_set11(&element),
        static fn SetStylisticSet11(Ref<FerroObject>, bool) =>
            |element: Ref<FerroObject>, value: bool| Typography::set_stylistic_set11(&element, value),
        static fn GetStylisticSet12(Ref<FerroObject>) -> bool =>
            |element: Ref<FerroObject>| Typography::get_stylistic_set12(&element),
        static fn SetStylisticSet12(Ref<FerroObject>, bool) =>
            |element: Ref<FerroObject>, value: bool| Typography::set_stylistic_set12(&element, value),
        static fn GetStylisticSet13(Ref<FerroObject>) -> bool =>
            |element: Ref<FerroObject>| Typography::get_stylistic_set13(&element),
        static fn SetStylisticSet13(Ref<FerroObject>, bool) =>
            |element: Ref<FerroObject>, value: bool| Typography::set_stylistic_set13(&element, value),
        static fn GetStylisticSet14(Ref<FerroObject>) -> bool =>
            |element: Ref<FerroObject>| Typography::get_stylistic_set14(&element),
        static fn SetStylisticSet14(Ref<FerroObject>, bool) =>
            |element: Ref<FerroObject>, value: bool| Typography::set_stylistic_set14(&element, value),
        static fn GetStylisticSet15(Ref<FerroObject>) -> bool =>
            |element: Ref<FerroObject>| Typography::get_stylistic_set15(&element),
        static fn SetStylisticSet15(Ref<FerroObject>, bool) =>
            |element: Ref<FerroObject>, value: bool| Typography::set_stylistic_set15(&element, value),
        static fn GetStylisticSet16(Ref<FerroObject>) -> bool =>
            |element: Ref<FerroObject>| Typography::get_stylistic_set16(&element),
        static fn SetStylisticSet16(Ref<FerroObject>, bool) =>
            |element: Ref<FerroObject>, value: bool| Typography::set_stylistic_set16(&element, value),
        static fn GetStylisticSet17(Ref<FerroObject>) -> bool =>
            |element: Ref<FerroObject>| Typography::get_stylistic_set17(&element),
        static fn SetStylisticSet17(Ref<FerroObject>, bool) =>
            |element: Ref<FerroObject>, value: bool| Typography::set_stylistic_set17(&element, value),
        static fn GetStylisticSet18(Ref<FerroObject>) -> bool =>
            |element: Ref<FerroObject>| Typography::get_stylistic_set18(&element),
        static fn SetStylisticSet18(Ref<FerroObject>, bool) =>
            |element: Ref<FerroObject>, value: bool| Typography::set_stylistic_set18(&element, value),
        static fn GetStylisticSet19(Ref<FerroObject>) -> bool =>
            |element: Ref<FerroObject>| Typography::get_stylistic_set19(&element),
        static fn SetStylisticSet19(Ref<FerroObject>, bool) =>
            |element: Ref<FerroObject>, value: bool| Typography::set_stylistic_set19(&element, value),
        static fn GetStylisticSet20(Ref<FerroObject>) -> bool =>
            |element: Ref<FerroObject>| Typography::get_stylistic_set20(&element),
        static fn SetStylisticSet20(Ref<FerroObject>, bool) =>
            |element: Ref<FerroObject>, value: bool| Typography::set_stylistic_set20(&element, value),
        static fn GetCapitals(Ref<FerroObject>) -> FontCapitals =>
            |element: Ref<FerroObject>| Typography::get_capitals(&element),
        static fn SetCapitals(Ref<FerroObject>, FontCapitals) =>
            |element: Ref<FerroObject>, value: FontCapitals| Typography::set_capitals(&element, value),
        static fn GetCapitalSpacing(Ref<FerroObject>) -> bool =>
            |element: Ref<FerroObject>| Typography::get_capital_spacing(&element),
        static fn SetCapitalSpacing(Ref<FerroObject>, bool) =>
            |element: Ref<FerroObject>, value: bool| Typography::set_capital_spacing(&element, value),
        static fn GetKerning(Ref<FerroObject>) -> bool =>
            |element: Ref<FerroObject>| Typography::get_kerning(&element),
        static fn SetKerning(Ref<FerroObject>, bool) =>
            |element: Ref<FerroObject>, value: bool| Typography::set_kerning(&element, value),
        static fn GetCaseSensitiveForms(Ref<FerroObject>) -> bool =>
            |element: Ref<FerroObject>| Typography::get_case_sensitive_forms(&element),
        static fn SetCaseSensitiveForms(Ref<FerroObject>, bool) =>
            |element: Ref<FerroObject>, value: bool| Typography::set_case_sensitive_forms(&element, value),
        static fn GetHistoricalForms(Ref<FerroObject>) -> bool =>
            |element: Ref<FerroObject>| Typography::get_historical_forms(&element),
        static fn SetHistoricalForms(Ref<FerroObject>, bool) =>
            |element: Ref<FerroObject>, value: bool| Typography::set_historical_forms(&element, value),
        static fn GetFraction(Ref<FerroObject>) -> FontFraction =>
            |element: Ref<FerroObject>| Typography::get_fraction(&element),
        static fn SetFraction(Ref<FerroObject>, FontFraction) =>
            |element: Ref<FerroObject>, value: FontFraction| Typography::set_fraction(&element, value),
        static fn GetNumeralStyle(Ref<FerroObject>) -> FontNumeralStyle =>
            |element: Ref<FerroObject>| Typography::get_numeral_style(&element),
        static fn SetNumeralStyle(Ref<FerroObject>, FontNumeralStyle) =>
            |element: Ref<FerroObject>, value: FontNumeralStyle| Typography::set_numeral_style(&element, value),
        static fn GetNumeralAlignment(Ref<FerroObject>) -> FontNumeralAlignment =>
            |element: Ref<FerroObject>| Typography::get_numeral_alignment(&element),
        static fn SetNumeralAlignment(Ref<FerroObject>, FontNumeralAlignment) =>
            |element: Ref<FerroObject>, value: FontNumeralAlignment| Typography::set_numeral_alignment(&element, value),
        static fn GetSlashedZero(Ref<FerroObject>) -> bool =>
            |element: Ref<FerroObject>| Typography::get_slashed_zero(&element),
        static fn SetSlashedZero(Ref<FerroObject>, bool) =>
            |element: Ref<FerroObject>, value: bool| Typography::set_slashed_zero(&element, value),
        static fn GetMathematicalGreek(Ref<FerroObject>) -> bool =>
            |element: Ref<FerroObject>| Typography::get_mathematical_greek(&element),
        static fn SetMathematicalGreek(Ref<FerroObject>, bool) =>
            |element: Ref<FerroObject>, value: bool| Typography::set_mathematical_greek(&element, value),
        static fn GetVariants(Ref<FerroObject>) -> FontVariants =>
            |element: Ref<FerroObject>| Typography::get_variants(&element),
        static fn SetVariants(Ref<FerroObject>, FontVariants) =>
            |element: Ref<FerroObject>, value: FontVariants| Typography::set_variants(&element, value),
    ],
});

#[cfg(test)]
#[path = "typography_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "typography_layout_tests.rs"]
mod layout_tests;
