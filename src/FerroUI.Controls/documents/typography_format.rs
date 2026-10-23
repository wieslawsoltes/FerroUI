//! Value snapshots of assigned or effective typography, independent of element lifetime.

use super::Typography;
use ferroui_base::media::{
    FontCapitals, FontEastAsianLanguage, FontEastAsianWidths, FontFeature, FontFeatureCollection,
    FontFraction, FontNumeralAlignment, FontNumeralStyle, FontVariants,
};
use ferroui_base::FerroObject;

/// A per-property typography patch. `None` keeps the existing assignment.
///
/// Snapshots retain local choices independently, including explicit defaults
/// that override an inherited nondefault value. Resolved snapshots contain
/// every property and can be translated into OpenType feature records.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TypographyFormat {
    pub annotation_alternates: Option<i32>,
    pub east_asian_expert_forms: Option<bool>,
    pub east_asian_language: Option<FontEastAsianLanguage>,
    pub east_asian_widths: Option<FontEastAsianWidths>,
    pub standard_ligatures: Option<bool>,
    pub contextual_ligatures: Option<bool>,
    pub discretionary_ligatures: Option<bool>,
    pub historical_ligatures: Option<bool>,
    pub standard_swashes: Option<i32>,
    pub contextual_swashes: Option<i32>,
    pub contextual_alternates: Option<bool>,
    pub stylistic_alternates: Option<i32>,
    pub stylistic_set1: Option<bool>,
    pub stylistic_set2: Option<bool>,
    pub stylistic_set3: Option<bool>,
    pub stylistic_set4: Option<bool>,
    pub stylistic_set5: Option<bool>,
    pub stylistic_set6: Option<bool>,
    pub stylistic_set7: Option<bool>,
    pub stylistic_set8: Option<bool>,
    pub stylistic_set9: Option<bool>,
    pub stylistic_set10: Option<bool>,
    pub stylistic_set11: Option<bool>,
    pub stylistic_set12: Option<bool>,
    pub stylistic_set13: Option<bool>,
    pub stylistic_set14: Option<bool>,
    pub stylistic_set15: Option<bool>,
    pub stylistic_set16: Option<bool>,
    pub stylistic_set17: Option<bool>,
    pub stylistic_set18: Option<bool>,
    pub stylistic_set19: Option<bool>,
    pub stylistic_set20: Option<bool>,
    pub capitals: Option<FontCapitals>,
    pub capital_spacing: Option<bool>,
    pub kerning: Option<bool>,
    pub case_sensitive_forms: Option<bool>,
    pub historical_forms: Option<bool>,
    pub fraction: Option<FontFraction>,
    pub numeral_style: Option<FontNumeralStyle>,
    pub numeral_alignment: Option<FontNumeralAlignment>,
    pub slashed_zero: Option<bool>,
    pub mathematical_greek: Option<bool>,
    pub variants: Option<FontVariants>,
}

impl TypographyFormat {
    /// Captures assignments without flattening inherited choices.
    pub fn from_object(element: &FerroObject) -> Self {
        Self {
            annotation_alternates: element
                .get_base_value(Typography::annotation_alternates_property()),
            east_asian_expert_forms: element
                .get_base_value(Typography::east_asian_expert_forms_property()),
            east_asian_language: element.get_base_value(Typography::east_asian_language_property()),
            east_asian_widths: element.get_base_value(Typography::east_asian_widths_property()),
            standard_ligatures: element.get_base_value(Typography::standard_ligatures_property()),
            contextual_ligatures: element
                .get_base_value(Typography::contextual_ligatures_property()),
            discretionary_ligatures: element
                .get_base_value(Typography::discretionary_ligatures_property()),
            historical_ligatures: element
                .get_base_value(Typography::historical_ligatures_property()),
            standard_swashes: element.get_base_value(Typography::standard_swashes_property()),
            contextual_swashes: element.get_base_value(Typography::contextual_swashes_property()),
            contextual_alternates: element
                .get_base_value(Typography::contextual_alternates_property()),
            stylistic_alternates: element
                .get_base_value(Typography::stylistic_alternates_property()),
            stylistic_set1: element.get_base_value(Typography::stylistic_set1_property()),
            stylistic_set2: element.get_base_value(Typography::stylistic_set2_property()),
            stylistic_set3: element.get_base_value(Typography::stylistic_set3_property()),
            stylistic_set4: element.get_base_value(Typography::stylistic_set4_property()),
            stylistic_set5: element.get_base_value(Typography::stylistic_set5_property()),
            stylistic_set6: element.get_base_value(Typography::stylistic_set6_property()),
            stylistic_set7: element.get_base_value(Typography::stylistic_set7_property()),
            stylistic_set8: element.get_base_value(Typography::stylistic_set8_property()),
            stylistic_set9: element.get_base_value(Typography::stylistic_set9_property()),
            stylistic_set10: element.get_base_value(Typography::stylistic_set10_property()),
            stylistic_set11: element.get_base_value(Typography::stylistic_set11_property()),
            stylistic_set12: element.get_base_value(Typography::stylistic_set12_property()),
            stylistic_set13: element.get_base_value(Typography::stylistic_set13_property()),
            stylistic_set14: element.get_base_value(Typography::stylistic_set14_property()),
            stylistic_set15: element.get_base_value(Typography::stylistic_set15_property()),
            stylistic_set16: element.get_base_value(Typography::stylistic_set16_property()),
            stylistic_set17: element.get_base_value(Typography::stylistic_set17_property()),
            stylistic_set18: element.get_base_value(Typography::stylistic_set18_property()),
            stylistic_set19: element.get_base_value(Typography::stylistic_set19_property()),
            stylistic_set20: element.get_base_value(Typography::stylistic_set20_property()),
            capitals: element.get_base_value(Typography::capitals_property()),
            capital_spacing: element.get_base_value(Typography::capital_spacing_property()),
            kerning: element.get_base_value(Typography::kerning_property()),
            case_sensitive_forms: element
                .get_base_value(Typography::case_sensitive_forms_property()),
            historical_forms: element.get_base_value(Typography::historical_forms_property()),
            fraction: element.get_base_value(Typography::fraction_property()),
            numeral_style: element.get_base_value(Typography::numeral_style_property()),
            numeral_alignment: element.get_base_value(Typography::numeral_alignment_property()),
            slashed_zero: element.get_base_value(Typography::slashed_zero_property()),
            mathematical_greek: element.get_base_value(Typography::mathematical_greek_property()),
            variants: element.get_base_value(Typography::variants_property()),
        }
    }

    /// Captures every current inherited or assigned value.
    pub fn effective(element: &FerroObject) -> Self {
        Self {
            annotation_alternates: Some(Typography::get_annotation_alternates(element)),
            east_asian_expert_forms: Some(Typography::get_east_asian_expert_forms(element)),
            east_asian_language: Some(Typography::get_east_asian_language(element)),
            east_asian_widths: Some(Typography::get_east_asian_widths(element)),
            standard_ligatures: Some(Typography::get_standard_ligatures(element)),
            contextual_ligatures: Some(Typography::get_contextual_ligatures(element)),
            discretionary_ligatures: Some(Typography::get_discretionary_ligatures(element)),
            historical_ligatures: Some(Typography::get_historical_ligatures(element)),
            standard_swashes: Some(Typography::get_standard_swashes(element)),
            contextual_swashes: Some(Typography::get_contextual_swashes(element)),
            contextual_alternates: Some(Typography::get_contextual_alternates(element)),
            stylistic_alternates: Some(Typography::get_stylistic_alternates(element)),
            stylistic_set1: Some(Typography::get_stylistic_set1(element)),
            stylistic_set2: Some(Typography::get_stylistic_set2(element)),
            stylistic_set3: Some(Typography::get_stylistic_set3(element)),
            stylistic_set4: Some(Typography::get_stylistic_set4(element)),
            stylistic_set5: Some(Typography::get_stylistic_set5(element)),
            stylistic_set6: Some(Typography::get_stylistic_set6(element)),
            stylistic_set7: Some(Typography::get_stylistic_set7(element)),
            stylistic_set8: Some(Typography::get_stylistic_set8(element)),
            stylistic_set9: Some(Typography::get_stylistic_set9(element)),
            stylistic_set10: Some(Typography::get_stylistic_set10(element)),
            stylistic_set11: Some(Typography::get_stylistic_set11(element)),
            stylistic_set12: Some(Typography::get_stylistic_set12(element)),
            stylistic_set13: Some(Typography::get_stylistic_set13(element)),
            stylistic_set14: Some(Typography::get_stylistic_set14(element)),
            stylistic_set15: Some(Typography::get_stylistic_set15(element)),
            stylistic_set16: Some(Typography::get_stylistic_set16(element)),
            stylistic_set17: Some(Typography::get_stylistic_set17(element)),
            stylistic_set18: Some(Typography::get_stylistic_set18(element)),
            stylistic_set19: Some(Typography::get_stylistic_set19(element)),
            stylistic_set20: Some(Typography::get_stylistic_set20(element)),
            capitals: Some(Typography::get_capitals(element)),
            capital_spacing: Some(Typography::get_capital_spacing(element)),
            kerning: Some(Typography::get_kerning(element)),
            case_sensitive_forms: Some(Typography::get_case_sensitive_forms(element)),
            historical_forms: Some(Typography::get_historical_forms(element)),
            fraction: Some(Typography::get_fraction(element)),
            numeral_style: Some(Typography::get_numeral_style(element)),
            numeral_alignment: Some(Typography::get_numeral_alignment(element)),
            slashed_zero: Some(Typography::get_slashed_zero(element)),
            mathematical_greek: Some(Typography::get_mathematical_greek(element)),
            variants: Some(Typography::get_variants(element)),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.annotation_alternates.is_none()
            && self.east_asian_expert_forms.is_none()
            && self.east_asian_language.is_none()
            && self.east_asian_widths.is_none()
            && self.standard_ligatures.is_none()
            && self.contextual_ligatures.is_none()
            && self.discretionary_ligatures.is_none()
            && self.historical_ligatures.is_none()
            && self.standard_swashes.is_none()
            && self.contextual_swashes.is_none()
            && self.contextual_alternates.is_none()
            && self.stylistic_alternates.is_none()
            && self.stylistic_set1.is_none()
            && self.stylistic_set2.is_none()
            && self.stylistic_set3.is_none()
            && self.stylistic_set4.is_none()
            && self.stylistic_set5.is_none()
            && self.stylistic_set6.is_none()
            && self.stylistic_set7.is_none()
            && self.stylistic_set8.is_none()
            && self.stylistic_set9.is_none()
            && self.stylistic_set10.is_none()
            && self.stylistic_set11.is_none()
            && self.stylistic_set12.is_none()
            && self.stylistic_set13.is_none()
            && self.stylistic_set14.is_none()
            && self.stylistic_set15.is_none()
            && self.stylistic_set16.is_none()
            && self.stylistic_set17.is_none()
            && self.stylistic_set18.is_none()
            && self.stylistic_set19.is_none()
            && self.stylistic_set20.is_none()
            && self.capitals.is_none()
            && self.capital_spacing.is_none()
            && self.kerning.is_none()
            && self.case_sensitive_forms.is_none()
            && self.historical_forms.is_none()
            && self.fraction.is_none()
            && self.numeral_style.is_none()
            && self.numeral_alignment.is_none()
            && self.slashed_zero.is_none()
            && self.mathematical_greek.is_none()
            && self.variants.is_none()
    }

    /// Whether the resolved values match the standard property defaults.
    pub fn is_default(&self) -> bool {
        self.annotation_alternates.unwrap_or(0) == 0
            && !self.east_asian_expert_forms.unwrap_or(false)
            && self
                .east_asian_language
                .unwrap_or(FontEastAsianLanguage::Normal)
                == FontEastAsianLanguage::Normal
            && self
                .east_asian_widths
                .unwrap_or(FontEastAsianWidths::Normal)
                == FontEastAsianWidths::Normal
            && self.standard_ligatures.unwrap_or(true)
            && self.contextual_ligatures.unwrap_or(true)
            && !self.discretionary_ligatures.unwrap_or(false)
            && !self.historical_ligatures.unwrap_or(false)
            && self.standard_swashes.unwrap_or(0) == 0
            && self.contextual_swashes.unwrap_or(0) == 0
            && self.contextual_alternates.unwrap_or(true)
            && self.stylistic_alternates.unwrap_or(0) == 0
            && !self.stylistic_set1.unwrap_or(false)
            && !self.stylistic_set2.unwrap_or(false)
            && !self.stylistic_set3.unwrap_or(false)
            && !self.stylistic_set4.unwrap_or(false)
            && !self.stylistic_set5.unwrap_or(false)
            && !self.stylistic_set6.unwrap_or(false)
            && !self.stylistic_set7.unwrap_or(false)
            && !self.stylistic_set8.unwrap_or(false)
            && !self.stylistic_set9.unwrap_or(false)
            && !self.stylistic_set10.unwrap_or(false)
            && !self.stylistic_set11.unwrap_or(false)
            && !self.stylistic_set12.unwrap_or(false)
            && !self.stylistic_set13.unwrap_or(false)
            && !self.stylistic_set14.unwrap_or(false)
            && !self.stylistic_set15.unwrap_or(false)
            && !self.stylistic_set16.unwrap_or(false)
            && !self.stylistic_set17.unwrap_or(false)
            && !self.stylistic_set18.unwrap_or(false)
            && !self.stylistic_set19.unwrap_or(false)
            && !self.stylistic_set20.unwrap_or(false)
            && self.capitals.unwrap_or(FontCapitals::Normal) == FontCapitals::Normal
            && !self.capital_spacing.unwrap_or(false)
            && self.kerning.unwrap_or(true)
            && !self.case_sensitive_forms.unwrap_or(false)
            && !self.historical_forms.unwrap_or(false)
            && self.fraction.unwrap_or(FontFraction::Normal) == FontFraction::Normal
            && self.numeral_style.unwrap_or(FontNumeralStyle::Normal) == FontNumeralStyle::Normal
            && self
                .numeral_alignment
                .unwrap_or(FontNumeralAlignment::Normal)
                == FontNumeralAlignment::Normal
            && !self.slashed_zero.unwrap_or(false)
            && !self.mathematical_greek.unwrap_or(false)
            && self.variants.unwrap_or(FontVariants::Normal) == FontVariants::Normal
    }

    /// Replaces only the individual values present in `patch`.
    pub fn overlay(&self, patch: &Self) -> Self {
        Self {
            annotation_alternates: patch.annotation_alternates.or(self.annotation_alternates),
            east_asian_expert_forms: patch
                .east_asian_expert_forms
                .or(self.east_asian_expert_forms),
            east_asian_language: patch.east_asian_language.or(self.east_asian_language),
            east_asian_widths: patch.east_asian_widths.or(self.east_asian_widths),
            standard_ligatures: patch.standard_ligatures.or(self.standard_ligatures),
            contextual_ligatures: patch.contextual_ligatures.or(self.contextual_ligatures),
            discretionary_ligatures: patch
                .discretionary_ligatures
                .or(self.discretionary_ligatures),
            historical_ligatures: patch.historical_ligatures.or(self.historical_ligatures),
            standard_swashes: patch.standard_swashes.or(self.standard_swashes),
            contextual_swashes: patch.contextual_swashes.or(self.contextual_swashes),
            contextual_alternates: patch.contextual_alternates.or(self.contextual_alternates),
            stylistic_alternates: patch.stylistic_alternates.or(self.stylistic_alternates),
            stylistic_set1: patch.stylistic_set1.or(self.stylistic_set1),
            stylistic_set2: patch.stylistic_set2.or(self.stylistic_set2),
            stylistic_set3: patch.stylistic_set3.or(self.stylistic_set3),
            stylistic_set4: patch.stylistic_set4.or(self.stylistic_set4),
            stylistic_set5: patch.stylistic_set5.or(self.stylistic_set5),
            stylistic_set6: patch.stylistic_set6.or(self.stylistic_set6),
            stylistic_set7: patch.stylistic_set7.or(self.stylistic_set7),
            stylistic_set8: patch.stylistic_set8.or(self.stylistic_set8),
            stylistic_set9: patch.stylistic_set9.or(self.stylistic_set9),
            stylistic_set10: patch.stylistic_set10.or(self.stylistic_set10),
            stylistic_set11: patch.stylistic_set11.or(self.stylistic_set11),
            stylistic_set12: patch.stylistic_set12.or(self.stylistic_set12),
            stylistic_set13: patch.stylistic_set13.or(self.stylistic_set13),
            stylistic_set14: patch.stylistic_set14.or(self.stylistic_set14),
            stylistic_set15: patch.stylistic_set15.or(self.stylistic_set15),
            stylistic_set16: patch.stylistic_set16.or(self.stylistic_set16),
            stylistic_set17: patch.stylistic_set17.or(self.stylistic_set17),
            stylistic_set18: patch.stylistic_set18.or(self.stylistic_set18),
            stylistic_set19: patch.stylistic_set19.or(self.stylistic_set19),
            stylistic_set20: patch.stylistic_set20.or(self.stylistic_set20),
            capitals: patch.capitals.or(self.capitals),
            capital_spacing: patch.capital_spacing.or(self.capital_spacing),
            kerning: patch.kerning.or(self.kerning),
            case_sensitive_forms: patch.case_sensitive_forms.or(self.case_sensitive_forms),
            historical_forms: patch.historical_forms.or(self.historical_forms),
            fraction: patch.fraction.or(self.fraction),
            numeral_style: patch.numeral_style.or(self.numeral_style),
            numeral_alignment: patch.numeral_alignment.or(self.numeral_alignment),
            slashed_zero: patch.slashed_zero.or(self.slashed_zero),
            mathematical_greek: patch.mathematical_greek.or(self.mathematical_greek),
            variants: patch.variants.or(self.variants),
        }
    }

    pub fn apply_to(&self, element: &FerroObject) {
        if let Some(value) = self.annotation_alternates {
            Typography::set_annotation_alternates(element, value);
        }
        if let Some(value) = self.east_asian_expert_forms {
            Typography::set_east_asian_expert_forms(element, value);
        }
        if let Some(value) = self.east_asian_language {
            Typography::set_east_asian_language(element, value);
        }
        if let Some(value) = self.east_asian_widths {
            Typography::set_east_asian_widths(element, value);
        }
        if let Some(value) = self.standard_ligatures {
            Typography::set_standard_ligatures(element, value);
        }
        if let Some(value) = self.contextual_ligatures {
            Typography::set_contextual_ligatures(element, value);
        }
        if let Some(value) = self.discretionary_ligatures {
            Typography::set_discretionary_ligatures(element, value);
        }
        if let Some(value) = self.historical_ligatures {
            Typography::set_historical_ligatures(element, value);
        }
        if let Some(value) = self.standard_swashes {
            Typography::set_standard_swashes(element, value);
        }
        if let Some(value) = self.contextual_swashes {
            Typography::set_contextual_swashes(element, value);
        }
        if let Some(value) = self.contextual_alternates {
            Typography::set_contextual_alternates(element, value);
        }
        if let Some(value) = self.stylistic_alternates {
            Typography::set_stylistic_alternates(element, value);
        }
        if let Some(value) = self.stylistic_set1 {
            Typography::set_stylistic_set1(element, value);
        }
        if let Some(value) = self.stylistic_set2 {
            Typography::set_stylistic_set2(element, value);
        }
        if let Some(value) = self.stylistic_set3 {
            Typography::set_stylistic_set3(element, value);
        }
        if let Some(value) = self.stylistic_set4 {
            Typography::set_stylistic_set4(element, value);
        }
        if let Some(value) = self.stylistic_set5 {
            Typography::set_stylistic_set5(element, value);
        }
        if let Some(value) = self.stylistic_set6 {
            Typography::set_stylistic_set6(element, value);
        }
        if let Some(value) = self.stylistic_set7 {
            Typography::set_stylistic_set7(element, value);
        }
        if let Some(value) = self.stylistic_set8 {
            Typography::set_stylistic_set8(element, value);
        }
        if let Some(value) = self.stylistic_set9 {
            Typography::set_stylistic_set9(element, value);
        }
        if let Some(value) = self.stylistic_set10 {
            Typography::set_stylistic_set10(element, value);
        }
        if let Some(value) = self.stylistic_set11 {
            Typography::set_stylistic_set11(element, value);
        }
        if let Some(value) = self.stylistic_set12 {
            Typography::set_stylistic_set12(element, value);
        }
        if let Some(value) = self.stylistic_set13 {
            Typography::set_stylistic_set13(element, value);
        }
        if let Some(value) = self.stylistic_set14 {
            Typography::set_stylistic_set14(element, value);
        }
        if let Some(value) = self.stylistic_set15 {
            Typography::set_stylistic_set15(element, value);
        }
        if let Some(value) = self.stylistic_set16 {
            Typography::set_stylistic_set16(element, value);
        }
        if let Some(value) = self.stylistic_set17 {
            Typography::set_stylistic_set17(element, value);
        }
        if let Some(value) = self.stylistic_set18 {
            Typography::set_stylistic_set18(element, value);
        }
        if let Some(value) = self.stylistic_set19 {
            Typography::set_stylistic_set19(element, value);
        }
        if let Some(value) = self.stylistic_set20 {
            Typography::set_stylistic_set20(element, value);
        }
        if let Some(value) = self.capitals {
            Typography::set_capitals(element, value);
        }
        if let Some(value) = self.capital_spacing {
            Typography::set_capital_spacing(element, value);
        }
        if let Some(value) = self.kerning {
            Typography::set_kerning(element, value);
        }
        if let Some(value) = self.case_sensitive_forms {
            Typography::set_case_sensitive_forms(element, value);
        }
        if let Some(value) = self.historical_forms {
            Typography::set_historical_forms(element, value);
        }
        if let Some(value) = self.fraction {
            Typography::set_fraction(element, value);
        }
        if let Some(value) = self.numeral_style {
            Typography::set_numeral_style(element, value);
        }
        if let Some(value) = self.numeral_alignment {
            Typography::set_numeral_alignment(element, value);
        }
        if let Some(value) = self.slashed_zero {
            Typography::set_slashed_zero(element, value);
        }
        if let Some(value) = self.mathematical_greek {
            Typography::set_mathematical_greek(element, value);
        }
        if let Some(value) = self.variants {
            Typography::set_variants(element, value);
        }
    }

    pub(crate) fn clear_on(element: &FerroObject) {
        element.clear_value(Typography::annotation_alternates_property());
        element.clear_value(Typography::east_asian_expert_forms_property());
        element.clear_value(Typography::east_asian_language_property());
        element.clear_value(Typography::east_asian_widths_property());
        element.clear_value(Typography::standard_ligatures_property());
        element.clear_value(Typography::contextual_ligatures_property());
        element.clear_value(Typography::discretionary_ligatures_property());
        element.clear_value(Typography::historical_ligatures_property());
        element.clear_value(Typography::standard_swashes_property());
        element.clear_value(Typography::contextual_swashes_property());
        element.clear_value(Typography::contextual_alternates_property());
        element.clear_value(Typography::stylistic_alternates_property());
        element.clear_value(Typography::stylistic_set1_property());
        element.clear_value(Typography::stylistic_set2_property());
        element.clear_value(Typography::stylistic_set3_property());
        element.clear_value(Typography::stylistic_set4_property());
        element.clear_value(Typography::stylistic_set5_property());
        element.clear_value(Typography::stylistic_set6_property());
        element.clear_value(Typography::stylistic_set7_property());
        element.clear_value(Typography::stylistic_set8_property());
        element.clear_value(Typography::stylistic_set9_property());
        element.clear_value(Typography::stylistic_set10_property());
        element.clear_value(Typography::stylistic_set11_property());
        element.clear_value(Typography::stylistic_set12_property());
        element.clear_value(Typography::stylistic_set13_property());
        element.clear_value(Typography::stylistic_set14_property());
        element.clear_value(Typography::stylistic_set15_property());
        element.clear_value(Typography::stylistic_set16_property());
        element.clear_value(Typography::stylistic_set17_property());
        element.clear_value(Typography::stylistic_set18_property());
        element.clear_value(Typography::stylistic_set19_property());
        element.clear_value(Typography::stylistic_set20_property());
        element.clear_value(Typography::capitals_property());
        element.clear_value(Typography::capital_spacing_property());
        element.clear_value(Typography::kerning_property());
        element.clear_value(Typography::case_sensitive_forms_property());
        element.clear_value(Typography::historical_forms_property());
        element.clear_value(Typography::fraction_property());
        element.clear_value(Typography::numeral_style_property());
        element.clear_value(Typography::numeral_alignment_property());
        element.clear_value(Typography::slashed_zero_property());
        element.clear_value(Typography::mathematical_greek_property());
        element.clear_value(Typography::variants_property());
    }

    /// Generates feature tags/parameters, leaving font support to the shaper.
    /// Selectors preserve their signed parameter values; the shaping backend
    /// applies the same bit representation used by raw font features.
    pub fn to_font_features(&self) -> FontFeatureCollection {
        let mut features = Vec::new();
        let mut add = |tag: &str, value: i32| {
            features.push(FontFeature {
                tag: tag.to_owned(),
                value,
                start: 0,
                end: -1,
            });
        };
        let value = self.annotation_alternates.unwrap_or(0);
        if value != 0 {
            add("nalt", value);
        }
        if self.east_asian_expert_forms.unwrap_or(false) {
            add("expt", 1);
        }
        match self
            .east_asian_language
            .unwrap_or(FontEastAsianLanguage::Normal)
        {
            FontEastAsianLanguage::Normal => {}
            FontEastAsianLanguage::HojoKanji => {
                add("hojo", 1);
            }
            FontEastAsianLanguage::Jis04 => {
                add("jp04", 1);
            }
            FontEastAsianLanguage::Jis78 => {
                add("jp78", 1);
            }
            FontEastAsianLanguage::Jis83 => {
                add("jp83", 1);
            }
            FontEastAsianLanguage::Jis90 => {
                add("jp90", 1);
            }
            FontEastAsianLanguage::NlcKanji => {
                add("nlck", 1);
            }
            FontEastAsianLanguage::Simplified => {
                add("smpl", 1);
            }
            FontEastAsianLanguage::Traditional => {
                add("trad", 1);
            }
            FontEastAsianLanguage::TraditionalNames => {
                add("tnam", 1);
            }
        }
        match self
            .east_asian_widths
            .unwrap_or(FontEastAsianWidths::Normal)
        {
            FontEastAsianWidths::Normal => {}
            FontEastAsianWidths::Full => {
                add("fwid", 1);
            }
            FontEastAsianWidths::Half => {
                add("half", 1);
            }
            FontEastAsianWidths::Proportional => {
                add("pwid", 1);
            }
            FontEastAsianWidths::Quarter => {
                add("qwid", 1);
            }
            FontEastAsianWidths::Third => {
                add("twid", 1);
            }
        }
        add("liga", i32::from(self.standard_ligatures.unwrap_or(true)));
        add("clig", i32::from(self.contextual_ligatures.unwrap_or(true)));
        if self.discretionary_ligatures.unwrap_or(false) {
            add("dlig", 1);
        }
        if self.historical_ligatures.unwrap_or(false) {
            add("hlig", 1);
        }
        let value = self.standard_swashes.unwrap_or(0);
        if value != 0 {
            add("swsh", value);
        }
        let value = self.contextual_swashes.unwrap_or(0);
        if value != 0 {
            add("cswh", value);
        }
        add(
            "calt",
            i32::from(self.contextual_alternates.unwrap_or(true)),
        );
        let value = self.stylistic_alternates.unwrap_or(0);
        if value != 0 {
            add("salt", value);
        }
        if self.stylistic_set1.unwrap_or(false) {
            add("ss01", 1);
        }
        if self.stylistic_set2.unwrap_or(false) {
            add("ss02", 1);
        }
        if self.stylistic_set3.unwrap_or(false) {
            add("ss03", 1);
        }
        if self.stylistic_set4.unwrap_or(false) {
            add("ss04", 1);
        }
        if self.stylistic_set5.unwrap_or(false) {
            add("ss05", 1);
        }
        if self.stylistic_set6.unwrap_or(false) {
            add("ss06", 1);
        }
        if self.stylistic_set7.unwrap_or(false) {
            add("ss07", 1);
        }
        if self.stylistic_set8.unwrap_or(false) {
            add("ss08", 1);
        }
        if self.stylistic_set9.unwrap_or(false) {
            add("ss09", 1);
        }
        if self.stylistic_set10.unwrap_or(false) {
            add("ss10", 1);
        }
        if self.stylistic_set11.unwrap_or(false) {
            add("ss11", 1);
        }
        if self.stylistic_set12.unwrap_or(false) {
            add("ss12", 1);
        }
        if self.stylistic_set13.unwrap_or(false) {
            add("ss13", 1);
        }
        if self.stylistic_set14.unwrap_or(false) {
            add("ss14", 1);
        }
        if self.stylistic_set15.unwrap_or(false) {
            add("ss15", 1);
        }
        if self.stylistic_set16.unwrap_or(false) {
            add("ss16", 1);
        }
        if self.stylistic_set17.unwrap_or(false) {
            add("ss17", 1);
        }
        if self.stylistic_set18.unwrap_or(false) {
            add("ss18", 1);
        }
        if self.stylistic_set19.unwrap_or(false) {
            add("ss19", 1);
        }
        if self.stylistic_set20.unwrap_or(false) {
            add("ss20", 1);
        }
        match self.capitals.unwrap_or(FontCapitals::Normal) {
            FontCapitals::Normal => {}
            FontCapitals::AllSmallCaps => {
                add("c2sc", 1);
                add("smcp", 1);
            }
            FontCapitals::SmallCaps => {
                add("smcp", 1);
            }
            FontCapitals::AllPetiteCaps => {
                add("c2pc", 1);
                add("pcap", 1);
            }
            FontCapitals::PetiteCaps => {
                add("pcap", 1);
            }
            FontCapitals::Unicase => {
                add("unic", 1);
            }
            FontCapitals::Titling => {
                add("titl", 1);
            }
        }
        if self.capital_spacing.unwrap_or(false) {
            add("cpsp", 1);
        }
        add("kern", i32::from(self.kerning.unwrap_or(true)));
        if self.case_sensitive_forms.unwrap_or(false) {
            add("case", 1);
        }
        if self.historical_forms.unwrap_or(false) {
            add("hist", 1);
        }
        match self.fraction.unwrap_or(FontFraction::Normal) {
            FontFraction::Normal => {}
            FontFraction::Stacked => {
                add("afrc", 1);
            }
            FontFraction::Slashed => {
                add("frac", 1);
            }
        }
        match self.numeral_style.unwrap_or(FontNumeralStyle::Normal) {
            FontNumeralStyle::Normal => {}
            FontNumeralStyle::Lining => {
                add("lnum", 1);
            }
            FontNumeralStyle::OldStyle => {
                add("onum", 1);
            }
        }
        match self
            .numeral_alignment
            .unwrap_or(FontNumeralAlignment::Normal)
        {
            FontNumeralAlignment::Normal => {}
            FontNumeralAlignment::Proportional => {
                add("pnum", 1);
            }
            FontNumeralAlignment::Tabular => {
                add("tnum", 1);
            }
        }
        if self.slashed_zero.unwrap_or(false) {
            add("zero", 1);
        }
        if self.mathematical_greek.unwrap_or(false) {
            add("mgrk", 1);
        }
        match self.variants.unwrap_or(FontVariants::Normal) {
            FontVariants::Normal => {}
            FontVariants::Superscript => {
                add("sups", 1);
            }
            FontVariants::Subscript => {
                add("subs", 1);
            }
            FontVariants::Ordinal => {
                add("ordn", 1);
            }
            FontVariants::Inferior => {
                add("sinf", 1);
            }
            FontVariants::Ruby => {
                add("ruby", 1);
            }
        }
        FontFeatureCollection::from_items(features)
    }

    pub(crate) fn with_font_features(
        &self,
        explicit: Option<FontFeatureCollection>,
    ) -> Option<FontFeatureCollection> {
        // Preserve the existing default path and collection identity. Default
        // typography is already the normal shaping behavior; the explicit
        // collection remains the final override even when a value is default.
        if self.is_default() {
            return explicit;
        }
        let features = self.to_font_features();
        if let Some(explicit) = explicit {
            for feature in explicit.list().snapshot().iter() {
                features.add(feature.clone());
            }
        }
        Some(features)
    }
}
