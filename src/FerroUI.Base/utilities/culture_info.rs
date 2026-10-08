//! A minimal counterpart of .NET's `System.Globalization.CultureInfo`: a
//! BCP-47 language tag plus its Windows LCID where known. It carries no
//! formatting data; the text subsystem only needs culture identity (font
//! matching, shaping language, localized font names).
//!
//! Date and time conventions are reached through
//! [`CultureInfo::date_time_format`]; see
//! [`DateTimeFormatInfo`](super::DateTimeFormatInfo) for the cultures that
//! have data of their own.

use super::{DateTimeFormatInfo, GregorianCalendar, ICultureDataProvider, NumberFormatInfo};
use crate::{FerroLocator, LocatorExtensions};
use bitflags::bitflags;
use std::cell::RefCell;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

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

thread_local! {
    static CURRENT_CULTURE: RefCell<CultureInfo> = RefCell::new(CultureInfo::invariant_culture());
    static CURRENT_UI_CULTURE: RefCell<CultureInfo> = RefCell::new(CultureInfo::invariant_culture());
}

/// Identifies a culture by its language tag.
///
/// Cloning is cheap (one reference count). Equality compares names ignoring
/// ASCII case.
#[derive(Clone)]
pub struct CultureInfo {
    name: Rc<str>,
    lcid: i32,
    /// Date and time conventions attached to this instance; they replace
    /// the ones the culture data provider has for the name.
    date_time_format: Option<Rc<DateTimeFormatInfo>>,
    /// Number conventions attached to this instance.
    number_format: Option<Rc<NumberFormatInfo>>,
}

impl CultureInfo {
    /// The culture-independent (invariant) culture; its name is empty.
    pub fn invariant_culture() -> CultureInfo {
        thread_local! {
            static INVARIANT: CultureInfo = CultureInfo { name: Rc::from(""), lcid: INVARIANT_LCID, date_time_format: None, number_format: None };
        }
        INVARIANT.with(Clone::clone)
    }

    /// The culture used by the current thread. Defaults to the invariant
    /// culture until the platform layer sets it.
    pub fn current_culture() -> CultureInfo {
        CURRENT_CULTURE.with(|culture| culture.borrow().clone())
    }

    /// Sets the culture used by the current thread.
    pub fn set_current_culture(culture: CultureInfo) {
        CURRENT_CULTURE.with(|current| *current.borrow_mut() = culture);
    }

    /// The UI culture used by the current thread. Defaults to the invariant
    /// culture until the platform layer sets it.
    pub fn current_ui_culture() -> CultureInfo {
        CURRENT_UI_CULTURE.with(|culture| culture.borrow().clone())
    }

    /// Sets the UI culture used by the current thread.
    pub fn set_current_ui_culture(culture: CultureInfo) {
        CURRENT_UI_CULTURE.with(|current| *current.borrow_mut() = culture);
    }

    /// The culture for a language tag such as `en-US` (C# `GetCultureInfo(string)`).
    /// An empty name gives the invariant culture.
    pub fn get_culture_info(name: &str) -> CultureInfo {
        if name.is_empty() {
            return Self::invariant_culture();
        }

        let lcid = KNOWN_CULTURES
            .iter()
            .find(|(_, known)| known.eq_ignore_ascii_case(name))
            .map_or(CUSTOM_UNSPECIFIED_LCID, |(lcid, _)| *lcid);

        CultureInfo { name: Rc::from(name), lcid, date_time_format: None, number_format: None }
    }

    /// The culture for an IETF language tag (C# `GetCultureInfoByIetfLanguageTag`),
    /// or `None` for a name that is not an IETF tag (C# throws
    /// `CultureNotFoundException`): the old names `zh-CHT` and `zh-CHS`, an
    /// alternate sort order and the traditional sort of Spanish.
    pub fn get_culture_info_by_ietf_language_tag(name: &str) -> Option<CultureInfo> {
        // Disallow old zh-CHT/zh-CHS names
        if name == "zh-CHT" || name == "zh-CHS" {
            return None;
        }

        let ci = Self::get_culture_info(name);

        // Disallow alt sorts and es-es_TS
        if ci.lcid > 0xffff || ci.lcid == 0x040a {
            return None;
        }

        Some(ci)
    }

    /// The culture for a Windows language identifier (C# `GetCultureInfo(int)`),
    /// or `None` when the identifier is not known (C# throws `CultureNotFoundException`).
    pub fn get_culture_info_by_lcid(lcid: i32) -> Option<CultureInfo> {
        if lcid == INVARIANT_LCID {
            return Some(Self::invariant_culture());
        }

        KNOWN_CULTURES
            .iter()
            .find(|(known, _)| *known == lcid)
            .map(|(lcid, name)| CultureInfo { name: Rc::from(*name), lcid: *lcid, date_time_format: None, number_format: None })
    }

    /// The cultures of the given types (C# `GetCultures`).
    ///
    /// The managed runtime lists the cultures of its globalization library.
    /// The base library has the data of the invariant culture only, so the
    /// list is the one the registered
    /// [`ICultureDataProvider`](super::ICultureDataProvider) gives
    /// ([`get_culture_names`](super::ICultureDataProvider::get_culture_names)),
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
        cultures.extend(provider.get_culture_names(types).iter().map(|name| Self::get_culture_info(name)));
        cultures
    }

    /// The language tag (C# `Name`); empty for the invariant culture.
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The Windows language identifier (C# `LCID`).
    #[inline]
    pub fn lcid(&self) -> i32 {
        self.lcid
    }

    /// The primary language subtag (C# `TwoLetterISOLanguageName`); `iv` for
    /// the invariant culture.
    pub fn two_letter_iso_language_name(&self) -> &str {
        if self.name.is_empty() {
            return "iv";
        }
        let end = self.name.find(['-', '_']).unwrap_or(self.name.len());
        &self.name[..end]
    }

    /// The culture this one specializes (C# `Parent`): the tag without its
    /// last subtag; the parent of a language-only culture is the invariant
    /// culture. Chinese regional cultures have their script culture as parent
    /// (`zh-CN` -> `zh-Hans`, `zh-TW` -> `zh-Hant`), as in .NET.
    pub fn parent(&self) -> CultureInfo {
        let Some(separator) = self.name.rfind(['-', '_']) else {
            return Self::invariant_culture();
        };

        let (language, last) = (&self.name[..separator], &self.name[separator + 1..]);

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
        self.name.is_empty()
    }

    /// The date and time conventions of the culture (C# `DateTimeFormat`):
    /// the ones attached with [`with_date_time_format`](Self::with_date_time_format),
    /// else the data the registered
    /// [`ICultureDataProvider`](super::ICultureDataProvider) has for the
    /// culture or the nearest of its parents, else the invariant conventions.
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
    /// for the culture or the nearest of its parents, else the invariant
    /// conventions.
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
        // number format: owned by numeric agent (the lookup of the data of a culture name
        // belongs in `NumberFormatInfo::for_culture`, through `ICultureDataProvider::get_number_format`).
        match &self.number_format {
            Some(info) => Rc::clone(info),
            None => super::NumberFormatInfo::for_culture(self),
        }
    }
}

impl PartialEq for CultureInfo {
    fn eq(&self, other: &Self) -> bool {
        self.name.eq_ignore_ascii_case(&other.name)
    }
}

impl Eq for CultureInfo {}

impl Hash for CultureInfo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for byte in self.name.bytes() {
            state.write_u8(byte.to_ascii_lowercase());
        }
    }
}

impl fmt::Display for CultureInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

impl fmt::Debug for CultureInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CultureInfo({:?})", &*self.name)
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
}
