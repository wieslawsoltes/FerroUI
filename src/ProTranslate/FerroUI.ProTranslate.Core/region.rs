use crate::{locale_data, Culture, TranslationError};

pub(crate) type LocaleData = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
);
pub(crate) fn locale_metadata(culture: &Culture) -> &'static LocaleData {
    let mut current = Some(culture.clone());
    while let Some(culture) = current {
        if let Ok(index) = locale_data::LOCALES.binary_search_by_key(&culture.name(), |row| row.0) {
            return &locale_data::LOCALES[index];
        }
        current = culture.parent();
    }
    &locale_data::LOCALES[locale_data::LOCALES
        .binary_search_by_key(&"en", |row| row.0)
        .expect("CLDR includes en")]
}
/// Immutable region metadata. Explicit profiles let hosts supply application or
/// user overrides without changing culture, process state, or compiled CLDR data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionProfile {
    pub culture: Culture,
    pub two_letter_iso_region_name: String,
    pub english_name: String,
    pub native_name: String,
    pub iso_currency_symbol: String,
    pub currency_symbol: String,
    pub currency_decimal_digits: u8,
    pub is_metric: bool,
}
impl RegionProfile {
    pub fn for_culture(culture: &Culture) -> Result<Self, TranslationError> {
        let code = culture
            .region_code()
            .ok_or_else(|| TranslationError::Region(culture.to_string()))?;
        let index = locale_data::REGIONS
            .binary_search_by_key(&code.as_str(), |row| row.0)
            .map_err(|_| TranslationError::Region(code.clone()))?;
        let (_, english, currency, digits) = locale_data::REGIONS[index];
        let locale = locale_metadata(culture);
        Ok(Self {
            culture: culture.clone(),
            two_letter_iso_region_name: code.clone(),
            english_name: english.into(),
            native_name: if locale.1 == code { locale.2 } else { english }.into(),
            iso_currency_symbol: currency.into(),
            currency_symbol: if locale.1 == code { locale.3 } else { currency }.into(),
            currency_decimal_digits: digits,
            is_metric: !matches!(code.as_str(), "US" | "LR" | "MM"),
        })
    }
    /// Resolve an explicit region with the locale's language. The returned profile
    /// can be customized before passing it to `set_region_override`.
    pub fn for_region(code: &str, culture: &Culture) -> Result<Self, TranslationError> {
        let region: icu_locale::subtags::Region = code
            .parse()
            .map_err(|_| TranslationError::Region(code.into()))?;
        let mut locale = culture.locale();
        locale.id.region = Some(region);
        let mut profile = Self::for_culture(&Culture::parse(&locale.to_string())?)?;
        profile.culture = culture.clone();
        Ok(profile)
    }
}
pub trait RegionProfileProvider {
    fn get_profile(
        &self,
        culture: &Culture,
        region_override: Option<&RegionProfile>,
    ) -> Result<RegionProfile, TranslationError>;
}
#[derive(Default)]
pub struct DefaultRegionProfileProvider;
impl RegionProfileProvider for DefaultRegionProfileProvider {
    fn get_profile(
        &self,
        culture: &Culture,
        region_override: Option<&RegionProfile>,
    ) -> Result<RegionProfile, TranslationError> {
        match region_override {
            Some(profile) => {
                let mut profile = profile.clone();
                profile.culture = culture.clone();
                Ok(profile)
            }
            None => RegionProfile::for_culture(culture),
        }
    }
}
