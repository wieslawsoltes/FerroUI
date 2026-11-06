//! A counterpart of .NET's `System.Globalization.CultureInfo`, on ICU4X: a
//! culture is made from a [`Locale`](icu_locale_core::Locale)
//! ([`CultureInfo::from_locale`]) and gives it ([`CultureInfo::locale`]); its
//! name and its Windows LCID derive from the locale. It is the formatting
//! parameter of the converters, the bindings and `StringFormat` (decision O3
//! of docs/idiomatic-rust-signatures.md): beside its locale it carries the
//! number and date conventions the .NET format strings need, the ones the
//! registered [`ICultureDataProvider`] has for its locale or conventions
//! attached to it ([`CultureInfo::with_number_format`],
//! [`CultureInfo::with_date_time_format`]), which a locale cannot hold.
//!
//! Date and time conventions are reached through
//! [`CultureInfo::date_time_format`]; see
//! [`DateTimeFormatInfo`](super::DateTimeFormatInfo) for the cultures that
//! have data of their own.
//!
//! The invariant culture is the unknown locale `und`. A name that is not a
//! BCP-47 language tag (a .NET sort-order name such as `es-ES_tradnl`, a
//! private name such as `x-custom`) still names a culture, which keeps the
//! name as it was given and has no locale of its own (its
//! [`locale`](CultureInfo::locale) is `und`, and `Locale::try_from` refuses
//! it). The identity of text (its language, font matching, shaping) is a
//! `Locale`. The ambient values of a thread are in
//! [`current_locale`](super::current_locale()): its locale, and the culture
//! it formats with ([`CultureInfo::current_formatting`]).

use super::{DateTimeFormatInfo, GregorianCalendar, ICultureDataProvider, NumberFormatInfo};
use crate::{FerroLocator, LocatorExtensions};
use bitflags::bitflags;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use icu_locale_core::Locale;

/// LCID of the invariant culture.
const INVARIANT_LCID: i32 = 0x007F;

/// LCID reported for cultures without a Windows LCID (`LOCALE_CUSTOM_UNSPECIFIED`).
const CUSTOM_UNSPECIFIED_LCID: i32 = 0x1000;

/// Windows language identifiers of the cultures fonts commonly carry localized names for.
static KNOWN_CULTURES: &[(i32, &str)] = &[
    (0x0401, "ar-SA"), (0x0402, "bg-BG"), (0x0403, "ca-ES"), (0x0404, "zh-TW"), (0x0405, "cs-CZ"),
    (0x0406, "da-DK"), (0x0407, "de-DE"), (0x0408, "el-GR"), (0x0409, "en-US"), (0x040A, "es-ES_tradnl"),
    (0x040B, "fi-FI"), (0x040C, "fr-FR"), (0x040D, "he-IL"), (0x040E, "hu-HU"), (0x040F, "is-IS"),
    (0x0410, "it-IT"), (0x0411, "ja-JP"), (0x0412, "ko-KR"), (0x0413, "nl-NL"), (0x0414, "nb-NO"),
    (0x0415, "pl-PL"), (0x0416, "pt-BR"), (0x0418, "ro-RO"), (0x0419, "ru-RU"), (0x041A, "hr-HR"),
    (0x041B, "sk-SK"), (0x041C, "sq-AL"), (0x041D, "sv-SE"), (0x041E, "th-TH"), (0x041F, "tr-TR"),
    (0x0420, "ur-PK"), (0x0421, "id-ID"), (0x0422, "uk-UA"), (0x0423, "be-BY"), (0x0424, "sl-SI"),
    (0x0425, "et-EE"), (0x0426, "lv-LV"), (0x0427, "lt-LT"), (0x0429, "fa-IR"), (0x042A, "vi-VN"),
    (0x042B, "hy-AM"), (0x042D, "eu-ES"), (0x042F, "mk-MK"), (0x0436, "af-ZA"), (0x0437, "ka-GE"),
    (0x0439, "hi-IN"), (0x043E, "ms-MY"), (0x043F, "kk-KZ"), (0x0445, "bn-IN"), (0x0446, "pa-IN"),
    (0x0447, "gu-IN"), (0x0449, "ta-IN"), (0x044A, "te-IN"), (0x044B, "kn-IN"), (0x044C, "ml-IN"),
    (0x044E, "mr-IN"), (0x0456, "gl-ES"), (0x0804, "zh-CN"), (0x0807, "de-CH"), (0x0809, "en-GB"),
    (0x080A, "es-MX"), (0x080C, "fr-BE"), (0x0810, "it-CH"), (0x0813, "nl-BE"), (0x0814, "nn-NO"),
    (0x0816, "pt-PT"), (0x081D, "sv-FI"), (0x0C04, "zh-HK"), (0x0C07, "de-AT"), (0x0C09, "en-AU"),
    (0x0C0A, "es-ES"), (0x0C0C, "fr-CA"), (0x1004, "zh-SG"), (0x1009, "en-CA"), (0x100C, "fr-CH"),
    (0x1404, "zh-MO"),
];

bitflags! {
    /// The types of culture lists [`CultureInfo::get_cultures`] retrieves (C#
    /// `CultureTypes`). The obsolete members of the managed enumeration are
    /// not ported.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct CultureTypes: i32 {
        /// Cultures that are associated with a language but are not specific
        /// to a country or region. The invariant culture is listed with them.
        const NEUTRAL_CULTURES = 0x1;
        /// Cultures that are specific to a country or region.
        const SPECIFIC_CULTURES = 0x2;
        /// The cultures installed in the operating system.
        const INSTALLED_WIN32_CULTURES = 0x4;
        /// All cultures.
        const ALL_CULTURES = 0x7;
    }
}

/// The identity of a culture: its locale, and the name and the Windows
/// language identifier that derive from it. Shared by the clones of a
/// culture and by the cultures made from it with attached conventions.
struct CultureIdentity {
    /// The locale; the unknown locale `und` for the invariant culture and
    /// for a name that is not a BCP-47 tag.
    locale: Locale,
    /// The BCP-47 text of the locale (empty for the invariant culture), or
    /// the name as it was given when it is not a BCP-47 tag.
    name: Box<str>,
    lcid: i32,
}

/// A culture: a locale, with the number and date conventions it formats
/// with.
///
/// Cloning is cheap (one reference count, and one more per attached
/// conventions). Equality compares the names ignoring ASCII case: two
/// cultures of the same locale are equal, the conventions attached to them
/// are not compared.
#[derive(Clone)]
pub struct CultureInfo {
    identity: Rc<CultureIdentity>,
    /// Date and time conventions attached to this instance; they replace
    /// the ones the culture data provider has for the locale.
    date_time_format: Option<Rc<DateTimeFormatInfo>>,
    /// Number conventions attached to this instance.
    number_format: Option<Rc<NumberFormatInfo>>,
}

impl CultureInfo {
    /// A culture of the identity, without attached conventions.
    fn of_identity(locale: Locale, name: Box<str>, lcid: i32) -> CultureInfo {
        CultureInfo { identity: Rc::new(CultureIdentity { locale, name, lcid }), date_time_format: None, number_format: None }
    }

    /// The culture-independent (invariant) culture; its name is empty and its
    /// locale is the unknown locale `und`.
    pub fn invariant_culture() -> CultureInfo {
        crate::ferro_thread_local! {
            static INVARIANT: CultureInfo = CultureInfo::of_identity(Locale::UNKNOWN, Box::from(""), INVARIANT_LCID);
        }
        INVARIANT.with(Clone::clone)
    }

    /// The culture of a locale: its name is the BCP-47 text of the locale
    /// (`en-US`, `zh-Hant-TW`, `en-US-u-hc-h12`), its Windows language
    /// identifier the one of the locale where one is known
    /// ([`lcid`](Self::lcid)), and its conventions the ones the registered
    /// [`ICultureDataProvider`] has for the locale or the nearest of its
    /// parents. The unknown locale `und` is the invariant culture.
    pub fn from_locale(locale: Locale) -> CultureInfo {
        if locale == Locale::UNKNOWN {
            return Self::invariant_culture();
        }
        let lcid = KNOWN_CULTURES
            .iter()
            .find(|(_, known)| locale.normalizing_eq(known))
            .map_or(CUSTOM_UNSPECIFIED_LCID, |(lcid, _)| *lcid);
        let name = locale.to_string().into_boxed_str();
        Self::of_identity(locale, name, lcid)
    }

    /// The culture for a name such as `en-US` (C# `GetCultureInfo(string)`).
    /// An empty name gives the invariant culture.
    ///
    /// A name that is a BCP-47 language tag gives the culture of its locale
    /// ([`from_locale`](Self::from_locale)), so the case of its subtags is
    /// normalized as .NET normalizes it (`EN-us` is `en-US`) and `und` is the
    /// invariant culture. A name that is not one (`es-ES_tradnl`, `x-custom`)
    /// gives a culture that keeps the name as it is, with the Windows language
    /// identifier of the name where one is known, and no locale.
    pub fn get_culture_info(name: &str) -> CultureInfo {
        if name.is_empty() {
            return Self::invariant_culture();
        }
        match Locale::try_from_str(name) {
            Ok(locale) => Self::from_locale(locale),
            Err(_) => {
                let lcid = KNOWN_CULTURES
                    .iter()
                    .find(|(_, known)| known.eq_ignore_ascii_case(name))
                    .map_or(CUSTOM_UNSPECIFIED_LCID, |(lcid, _)| *lcid);
                Self::of_identity(Locale::UNKNOWN, Box::from(name), lcid)
            }
        }
    }

    /// The culture for an IETF language tag (C# `GetCultureInfoByIetfLanguageTag`):
    /// the culture of the locale the tag is parsed into, the invariant culture
    /// for the empty tag, or `None` for a text that is not a BCP-47 language
    /// tag (C# throws `CultureNotFoundException`): the old names `zh-CHT` and
    /// `zh-CHS`, the traditional sort of Spanish `es-ES_tradnl`, a private name
    /// such as `x-custom`, a text with `_` as separator.
    pub fn get_culture_info_by_ietf_language_tag(name: &str) -> Option<CultureInfo> {
        if name.is_empty() {
            return Some(Self::invariant_culture());
        }
        Locale::try_from_str(name).ok().map(Self::from_locale)
    }

    /// The culture for a Windows language identifier (C# `GetCultureInfo(int)`),
    /// or `None` when the identifier is not known (C# throws `CultureNotFoundException`).
    pub fn get_culture_info_by_lcid(lcid: i32) -> Option<CultureInfo> {
        if lcid == INVARIANT_LCID {
            return Some(Self::invariant_culture());
        }

        KNOWN_CULTURES.iter().find(|(known, _)| *known == lcid).map(|(_, name)| Self::get_culture_info(name))
    }

    /// The cultures of the given types (C# `GetCultures`).
    ///
    /// The managed runtime lists the cultures of its globalization library.
    /// The base library has the data of the invariant culture only, so the
    /// list is the one the registered
    /// [`ICultureDataProvider`](super::ICultureDataProvider) gives, the
    /// cultures of its locales
    /// ([`get_locales`](super::ICultureDataProvider::get_locales)),
    /// preceded by the invariant culture when the neutral cultures are asked
    /// for. Without a provider the list is the invariant culture alone,
    /// whatever the types: what the managed runtime returns in its invariant
    /// globalization mode.
    pub fn get_cultures(types: CultureTypes) -> Vec<CultureInfo> {
        let Some(provider) = FerroLocator::current().get_service::<dyn ICultureDataProvider>() else {
            return vec![Self::invariant_culture()];
        };

        let mut cultures = Vec::new();
        if types.contains(CultureTypes::NEUTRAL_CULTURES) {
            cultures.push(Self::invariant_culture());
        }
        cultures.extend(provider.get_locales(types).into_iter().map(Self::from_locale));
        cultures
    }

    /// The locale of the culture: the unknown locale `und` for the invariant
    /// culture, and for a culture whose name is not a BCP-47 language tag
    /// (`es-ES_tradnl`), which has no locale of its own (the conversion
    /// `Locale::try_from(&culture)` refuses such a culture instead).
    #[inline]
    pub fn locale(&self) -> &Locale {
        &self.identity.locale
    }

    /// Whether the culture is the culture of its [`locale`](Self::locale): the
    /// invariant culture, or a culture of a BCP-47 language tag.
    #[inline]
    pub(crate) fn is_locale(&self) -> bool {
        self.is_invariant() || self.identity.locale != Locale::UNKNOWN
    }

    /// The name (C# `Name`): the BCP-47 text of the locale, empty for the
    /// invariant culture, or the name as it was given when it is not a BCP-47
    /// tag.
    #[inline]
    pub fn name(&self) -> &str {
        &self.identity.name
    }

    /// The Windows language identifier (C# `LCID`): the one of the locale
    /// where one is known, `0x007F` for the invariant culture, else
    /// `0x1000` (`LOCALE_CUSTOM_UNSPECIFIED`).
    #[inline]
    pub fn lcid(&self) -> i32 {
        self.identity.lcid
    }

    /// The primary language subtag (C# `TwoLetterISOLanguageName`); `iv` for
    /// the invariant culture.
    pub fn two_letter_iso_language_name(&self) -> &str {
        if self.is_invariant() {
            return "iv";
        }
        if self.is_locale() {
            return self.identity.locale.id.language.as_str();
        }
        let name = self.name();
        let end = name.find(['-', '_']).unwrap_or(name.len());
        &name[..end]
    }

    /// The culture this one specializes (C# `Parent`): the culture of the
    /// parent of its locale, the locale without its last variant, else
    /// without its region, else without its script; the parent of a
    /// language-only culture is the invariant culture. Chinese regional
    /// cultures have their script culture as parent (`zh-CN` -> `zh-Hans`,
    /// `zh-TW` -> `zh-Hant`), as in .NET. A culture whose name is not a
    /// BCP-47 tag has the culture of its name without its last subtag as
    /// parent (`es-ES_tradnl` -> `es-ES`).
    pub fn parent(&self) -> CultureInfo {
        if self.is_invariant() {
            return Self::invariant_culture();
        }
        if self.is_locale() {
            return Self::from_locale(parent_locale(&self.identity.locale));
        }

        let name = self.name();
        let Some(separator) = name.rfind(['-', '_']) else {
            return Self::invariant_culture();
        };

        let (language, last) = (&name[..separator], &name[separator + 1..]);

        if language.eq_ignore_ascii_case("zh") {
            const SIMPLIFIED: [&str; 3] = ["CN", "SG", "MY"];
            const TRADITIONAL: [&str; 3] = ["TW", "HK", "MO"];

            if SIMPLIFIED.iter().any(|region| region.eq_ignore_ascii_case(last)) {
                return Self::get_culture_info("zh-Hans");
            }

            if TRADITIONAL.iter().any(|region| region.eq_ignore_ascii_case(last)) {
                return Self::get_culture_info("zh-Hant");
            }
        }

        Self::get_culture_info(language)
    }

    /// Whether this is the invariant culture.
    #[inline]
    pub fn is_invariant(&self) -> bool {
        self.identity.name.is_empty()
    }

    /// The date and time conventions of the culture (C# `DateTimeFormat`):
    /// the ones attached with [`with_date_time_format`](Self::with_date_time_format),
    /// else the data the registered
    /// [`ICultureDataProvider`](super::ICultureDataProvider) has for the
    /// locale of the culture or the nearest of its parents, else the
    /// invariant conventions.
    ///
    /// Unlike .NET, the library itself carries the data of the invariant
    /// culture only.
    pub fn date_time_format(&self) -> Rc<DateTimeFormatInfo> {
        match &self.date_time_format {
            Some(info) => Rc::clone(info),
            None => DateTimeFormatInfo::for_culture(self),
        }
    }

    /// A culture with the same identity and the given date and time
    /// conventions (the C# `new CultureInfo(name) { DateTimeFormat = .. }`).
    pub fn with_date_time_format(&self, date_time_format: DateTimeFormatInfo) -> CultureInfo {
        CultureInfo { date_time_format: Some(Rc::new(date_time_format)), ..self.clone() }
    }

    /// A culture with the same identity and the given number conventions
    /// (the C# `new CultureInfo(name) { NumberFormat = .. }`).
    pub fn with_number_format(&self, number_format: NumberFormatInfo) -> CultureInfo {
        CultureInfo { number_format: Some(Rc::new(number_format)), ..self.clone() }
    }

    /// The calendar of the culture (C# `Calendar`); always the Gregorian one.
    pub fn calendar(&self) -> GregorianCalendar {
        GregorianCalendar
    }

    /// The text conventions of the culture (C# `TextInfo`): the data the
    /// registered [`ICultureDataProvider`](super::ICultureDataProvider) has
    /// for the locale of the culture or the nearest of its parents, else the
    /// invariant conventions.
    pub fn text_info(&self) -> Rc<super::TextInfo> {
        super::TextInfo::for_culture(self)
    }
}

impl CultureInfo {
    /// The number conventions of the culture (C# `NumberFormat`).
    ///
    /// Only the invariant conventions are built in: those of other cultures
    /// come from the provider registered with the locator (see
    /// [`ICultureDataProvider`](super::ICultureDataProvider)); a culture
    /// without data gets the conventions of its nearest parent that has
    /// data and finally the invariant ones.
    pub fn number_format(&self) -> Rc<super::NumberFormatInfo> {
        // number format: owned by numeric agent (the lookup of the data of a locale belongs in
        // `NumberFormatInfo::for_culture`, through `ICultureDataProvider::get_number_format_by_locale`).
        match &self.number_format {
            Some(info) => Rc::clone(info),
            None => super::NumberFormatInfo::for_culture(self),
        }
    }
}

/// The locale `locale` specializes, the locale of the parent of its culture
/// ([`CultureInfo::parent`]): the locale without its last variant, else without its region, else without
/// its script; the parent of a language alone is the unknown locale `und` (the invariant
/// culture), and so is the parent of `und`. Chinese regional locales without a script have
/// their script locale as parent (`zh-CN` -> `zh-Hans`, `zh-TW` -> `zh-Hant`), as in .NET.
/// A locale with extensions (`en-US-u-hc-h12`) has the locale without them as its parent.
pub(crate) fn parent_locale(locale: &Locale) -> Locale {
    if !locale.extensions.is_empty() {
        return Locale::from(locale.id.clone());
    }
    let mut id = locale.id.clone();
    if !id.variants.is_empty() {
        let mut variants: Vec<_> = id.variants.iter().copied().collect();
        variants.pop();
        id.variants = icu_locale_core::subtags::Variants::from_vec_unchecked(variants);
        return Locale::from(id);
    }
    if let Some(region) = id.region.take() {
        if id.script.is_none() && id.language.as_str() == "zh" {
            const SIMPLIFIED: [&str; 3] = ["CN", "SG", "MY"];
            const TRADITIONAL: [&str; 3] = ["TW", "HK", "MO"];
            if SIMPLIFIED.contains(&region.as_str()) {
                return icu_locale_core::locale!("zh-Hans");
            }
            if TRADITIONAL.contains(&region.as_str()) {
                return icu_locale_core::locale!("zh-Hant");
            }
        }
        return Locale::from(id);
    }
    if id.script.take().is_some() {
        return Locale::from(id);
    }
    Locale::UNKNOWN
}

impl PartialEq for CultureInfo {
    fn eq(&self, other: &Self) -> bool {
        self.name().eq_ignore_ascii_case(other.name())
    }
}

impl Eq for CultureInfo {}

impl Hash for CultureInfo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for byte in self.name().bytes() {
            state.write_u8(byte.to_ascii_lowercase());
        }
    }
}

impl fmt::Display for CultureInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl fmt::Debug for CultureInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CultureInfo({:?})", self.name())
    }
}

impl TryFrom<&CultureInfo> for icu_locale_core::Locale {
    type Error = icu_locale_core::ParseError;

    /// The locale of the culture ([`CultureInfo::locale`]). The invariant
    /// culture, whose name is empty, is the unknown locale `und`
    /// ([`Locale::UNKNOWN`](icu_locale_core::Locale::UNKNOWN)), the locale
    /// ICU4X formats with its root data.
    ///
    /// [`CultureInfo::get_culture_info`] takes any name, so a culture whose
    /// name is not a well-formed BCP-47 tag is an error, the error of the
    /// parser for the name: a .NET sort-order name such as `es-ES_tradnl`
    /// (ICU4X does not take `_` as a separator) or a private-use name such as
    /// `x-custom`. The case of the subtags is normalized (`EN-us` is `en-US`).
    fn try_from(value: &CultureInfo) -> Result<Self, Self::Error> {
        if value.is_locale() {
            return Ok(value.locale().clone());
        }
        icu_locale_core::Locale::try_from_str(value.name())
    }
}

impl From<&icu_locale_core::Locale> for CultureInfo {
    /// The culture of the locale ([`CultureInfo::from_locale`]); the unknown
    /// locale `und` is the invariant culture. Extensions stay in the name
    /// (`en-US-u-hc-h12`), and the culture has the Windows language
    /// identifier of the locale where one is known.
    fn from(value: &icu_locale_core::Locale) -> Self {
        CultureInfo::from_locale(value.clone())
    }
}

impl From<icu_locale_core::Locale> for CultureInfo {
    /// The culture of the locale ([`CultureInfo::from_locale`]).
    fn from(value: icu_locale_core::Locale) -> Self {
        CultureInfo::from_locale(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_by_name_and_lcid() {
        assert!(CultureInfo::invariant_culture().is_invariant());
        assert_eq!(CultureInfo::invariant_culture().lcid(), 0x7F);
        let culture = CultureInfo::get_culture_info("ja-JP");
        assert_eq!(culture.lcid(), 0x0411);
        assert_eq!(culture.two_letter_iso_language_name(), "ja");
        assert_eq!(CultureInfo::get_culture_info_by_lcid(0x0409).unwrap().name(), "en-US");
        assert!(CultureInfo::get_culture_info_by_lcid(0x7777).is_none());
        assert_eq!(CultureInfo::get_culture_info("EN-us"), CultureInfo::get_culture_info("en-US"));
    }

    #[test]
    fn the_cultures_are_the_ones_of_the_provider() {
        use crate::utilities::TestCultureDataProvider;

        let _scope = FerroLocator::enter_scope();
        let names = |types| CultureInfo::get_cultures(types).iter().map(|c| c.name().to_string()).collect::<Vec<_>>();

        // Without a provider: the invariant culture, whatever the types.
        assert_eq!(names(CultureTypes::SPECIFIC_CULTURES), [""]);
        assert_eq!(names(CultureTypes::ALL_CULTURES), [""]);

        TestCultureDataProvider::register();
        assert_eq!(names(CultureTypes::SPECIFIC_CULTURES), ["en-GB", "en-US"]);
        // The invariant culture is listed with the neutral ones.
        assert_eq!(names(CultureTypes::NEUTRAL_CULTURES), ["", "en"]);
        assert_eq!(names(CultureTypes::ALL_CULTURES), ["", "en", "en-GB", "en-US"]);
        assert!(names(CultureTypes::INSTALLED_WIN32_CULTURES).is_empty());
    }

    #[test]
    fn parent_drops_the_last_subtag() {
        assert_eq!(CultureInfo::get_culture_info("ja-JP").parent().name(), "ja");
        assert!(CultureInfo::get_culture_info("ja").parent().is_invariant());
        assert!(CultureInfo::invariant_culture().parent().is_invariant());
        assert_eq!(CultureInfo::get_culture_info("zh-Hans-CN").parent().name(), "zh-Hans");
        assert_eq!(CultureInfo::get_culture_info("zh-CN").parent().name(), "zh-Hans");
        assert_eq!(CultureInfo::get_culture_info("zh-TW").parent().name(), "zh-Hant");
        assert_eq!(CultureInfo::get_culture_info("zh-Hant").parent().name(), "zh");
    }

    /// Not from upstream (C5 of #423): the parent of a locale is the locale of the parent
    /// of its culture.
    #[test]
    fn the_parent_of_a_locale_is_the_one_of_its_culture() {
        for name in ["ja-JP", "ja", "zh-Hans-CN", "zh-CN", "zh-TW", "zh-Hant", "sr-Cyrl-RS", "de-DE-1996", "und"] {
            let locale = Locale::try_from_str(name).unwrap();
            let culture = CultureInfo::from(&locale);
            assert_eq!(parent_locale(&locale), Locale::try_from(&culture.parent()).unwrap(), "{name}");
        }
        assert_eq!(parent_locale(&Locale::try_from_str("en-US-u-hc-h12").unwrap()), Locale::try_from_str("en-US").unwrap());
        assert_eq!(parent_locale(&Locale::UNKNOWN), Locale::UNKNOWN);
    }

    #[test]
    fn converts_to_and_from_icu_locales() {
        use icu_locale_core::Locale;

        for name in ["en-US", "de-DE", "zh-Hant-TW", "ja", "en-US-u-hc-h12"] {
            let culture = CultureInfo::get_culture_info(name);
            let locale = Locale::try_from(&culture).unwrap();
            assert_eq!(locale.to_string(), name);
            let back = CultureInfo::from(&locale);
            assert_eq!(back.name(), name);
            assert_eq!(back, culture);
            assert_eq!(back.lcid(), culture.lcid());
        }

        let locale = Locale::try_from(&CultureInfo::get_culture_info("zh-Hant-TW")).unwrap();
        assert_eq!(locale.id.language.as_str(), "zh");
        assert_eq!(locale.id.script.as_ref().map(|script| script.as_str()), Some("Hant"));
        assert_eq!(locale.id.region.as_ref().map(|region| region.as_str()), Some("TW"));
        assert_eq!(CultureInfo::from(&Locale::try_from_str("de-DE").unwrap()).lcid(), 0x0407);

        // The case of the subtags is normalized.
        let locale = Locale::try_from(&CultureInfo::get_culture_info("EN-us")).unwrap();
        assert_eq!(locale.to_string(), "en-US");
    }

    #[test]
    fn the_invariant_culture_is_the_unknown_locale() {
        use icu_locale_core::Locale;

        assert_eq!(Locale::try_from(&CultureInfo::invariant_culture()), Ok(Locale::UNKNOWN));
        let culture = CultureInfo::from(&Locale::UNKNOWN);
        assert!(culture.is_invariant());
        assert_eq!(culture.lcid(), INVARIANT_LCID);
        assert!(CultureInfo::from(&Locale::try_from_str("und").unwrap()).is_invariant());
        // Only `und` alone: with a region it names a culture.
        assert_eq!(CultureInfo::from(&Locale::try_from_str("und-US").unwrap()).name(), "und-US");
    }

    /// Not from upstream (C5 of #423, L2): a culture is made from a locale and gives it; the
    /// name and the Windows language identifier derive from the locale.
    #[test]
    fn a_culture_is_made_from_and_gives_its_locale() {
        use icu_locale_core::locale;

        for (locale, name, lcid) in [
            (locale!("ja-JP"), "ja-JP", 0x0411),
            (locale!("zh-Hant-TW"), "zh-Hant-TW", 0x1000),
            (locale!("pt-BR"), "pt-BR", 0x0416),
            (locale!("en-US-u-hc-h12"), "en-US-u-hc-h12", 0x1000),
            (locale!("und-US"), "und-US", 0x1000),
        ] {
            let culture = CultureInfo::from_locale(locale.clone());
            assert_eq!(culture.locale(), &locale);
            assert_eq!(culture.name(), name);
            assert_eq!(culture.lcid(), lcid, "{name}");
            assert!(!culture.is_invariant());
            assert_eq!(Locale::try_from(&culture), Ok(locale.clone()));
            assert_eq!(CultureInfo::from(locale.clone()), culture);
            assert_eq!(CultureInfo::from(&locale), culture);
            assert_eq!(CultureInfo::get_culture_info(name), culture);
        }

        let invariant = CultureInfo::from_locale(Locale::UNKNOWN);
        assert!(invariant.is_invariant());
        assert_eq!(invariant.locale(), &Locale::UNKNOWN);
        assert_eq!(invariant.name(), "");
        assert_eq!(invariant.lcid(), INVARIANT_LCID);
        assert_eq!(CultureInfo::invariant_culture().locale(), &Locale::UNKNOWN);

        // Attached conventions keep the locale.
        let culture = CultureInfo::from_locale(locale!("de-DE")).with_number_format(NumberFormatInfo::new());
        assert_eq!(culture.locale(), &locale!("de-DE"));
        assert_eq!(culture, CultureInfo::get_culture_info("de-DE"));
    }

    /// Not from upstream (C5 of #423, L2): the name of a culture of a language tag is the text
    /// of its locale, normalized as .NET normalizes the name (`EN-us` is `en-US`; the variants
    /// of ICU4X are sorted), and `und` is the invariant culture.
    #[test]
    fn the_name_of_a_language_tag_is_the_text_of_its_locale() {
        use icu_locale_core::locale;

        let culture = CultureInfo::get_culture_info("EN-us");
        assert_eq!(culture.name(), "en-US");
        assert_eq!(culture.locale(), &locale!("en-US"));
        assert_eq!(culture.lcid(), 0x0409);
        assert_eq!(CultureInfo::get_culture_info("zh-hant-tw").name(), "zh-Hant-TW");
        assert_eq!(CultureInfo::get_culture_info("sl-rozaj-biske").name(), "sl-biske-rozaj");
        assert_eq!(CultureInfo::get_culture_info("de-DE-U-CO-PHONEBK").name(), "de-DE-u-co-phonebk");
        assert!(CultureInfo::get_culture_info("und").is_invariant());
        assert_eq!(CultureInfo::get_culture_info_by_lcid(0x0804).map(|culture| culture.locale().clone()), Some(locale!("zh-CN")));
        assert_eq!(CultureInfo::get_culture_info("ZH-cn").two_letter_iso_language_name(), "zh");
        // The parent of a culture is the culture of the parent of its locale.
        assert_eq!(CultureInfo::get_culture_info("de-DE-u-co-phonebk").parent().name(), "de-DE");
        assert_eq!(CultureInfo::get_culture_info("sl-rozaj-biske").parent().name(), "sl-biske");
        assert_eq!(CultureInfo::get_culture_info("und-US").parent(), CultureInfo::invariant_culture());
    }

    /// Not from upstream (C5 of #423, L2): a name that is no BCP-47 tag still names a culture,
    /// which keeps the name as it was given and has no locale of its own.
    #[test]
    fn a_name_that_is_no_language_tag_keeps_its_name_and_has_no_locale() {
        use icu_locale_core::locale;

        let culture = CultureInfo::get_culture_info("es-ES_tradnl");
        assert_eq!(culture.name(), "es-ES_tradnl");
        assert_eq!(culture.lcid(), 0x040A);
        assert_eq!(culture.locale(), &Locale::UNKNOWN);
        assert!(!culture.is_invariant());
        assert_eq!(culture.two_letter_iso_language_name(), "es");
        assert_eq!(culture.parent(), CultureInfo::from_locale(locale!("es-ES")));
        assert_eq!(CultureInfo::get_culture_info_by_lcid(0x040A), Some(culture.clone()));
        assert_eq!(CultureInfo::get_culture_info("ES-es_TRADNL"), culture);

        let custom = CultureInfo::get_culture_info("x-custom");
        assert_eq!((custom.name(), custom.lcid(), custom.locale()), ("x-custom", CUSTOM_UNSPECIFIED_LCID, &Locale::UNKNOWN));
        assert_eq!(custom.parent().name(), "x");
        assert!(custom.parent().parent().is_invariant());
    }

    /// Not from upstream (C5 of #423, L2): an IETF language tag is parsed into a locale.
    #[test]
    fn an_ietf_language_tag_is_parsed_into_a_locale() {
        use icu_locale_core::locale;

        let culture = CultureInfo::get_culture_info_by_ietf_language_tag("ar-sa").expect("a culture");
        assert_eq!((culture.name(), culture.locale()), ("ar-SA", &locale!("ar-SA")));
        assert!(CultureInfo::get_culture_info_by_ietf_language_tag("").expect("the invariant culture").is_invariant());
        assert!(CultureInfo::get_culture_info_by_ietf_language_tag("und").expect("the invariant culture").is_invariant());
        for name in ["zh-CHT", "zh-CHS", "es-ES_tradnl", "x-custom", "en_US", "en-"] {
            assert!(CultureInfo::get_culture_info_by_ietf_language_tag(name).is_none(), "{name}");
        }
    }

    #[test]
    fn names_that_are_not_language_tags_are_not_locales() {
        use icu_locale_core::Locale;

        for name in ["es-ES_tradnl", "x-custom", "zh-CHT"] {
            assert!(Locale::try_from(&CultureInfo::get_culture_info(name)).is_err(), "{name}");
        }
    }
}
