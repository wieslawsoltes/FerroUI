/* CultureInfo equality/hash and Marvin32 derive from .NET runtime revision
60629d14374c56f1cb51819049ad1fa529307f8d. Exact servicing behavior is separately measured.
The MIT License (MIT)

Copyright (c) .NET Foundation and Contributors

All rights reserved.

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/
//! Original reference cultures and the virtual operations consumed by native bindings.
//! Formatting data remains supplied by the selected framework's explicit providers.
use super::{
    CompareInfo, DateTimeFormatInfo, GregorianCalendar, ICultureDataProvider, NumberFormatInfo,
    TextInfo,
};
use crate::data::core::ValueTypes;
use crate::metadata::{MarkupType, MarkupTyped};
use crate::{BoxedValue, FerroLocator, LocatorExtensions, Utf16Text};
use bitflags::bitflags;
use std::{
    any::Any,
    cell::{OnceCell, RefCell},
    collections::HashMap,
    fmt,
    rc::{Rc, Weak},
};
/// LCID of the invariant culture.
const INVARIANT_LCID: i32 = 0x007F;

/// LCID reported for cultures without a Windows LCID (`LOCALE_CUSTOM_UNSPECIFIED`).
const CUSTOM_UNSPECIFIED_LCID: i32 = 0x1000;

/// Windows language identifiers of the cultures fonts commonly carry localized names for.
static KNOWN_CULTURES: &[(i32, &str)] = &[
    (0x0401, "ar-SA"),
    (0x0402, "bg-BG"),
    (0x0403, "ca-ES"),
    (0x0404, "zh-TW"),
    (0x0405, "cs-CZ"),
    (0x0406, "da-DK"),
    (0x0407, "de-DE"),
    (0x0408, "el-GR"),
    (0x0409, "en-US"),
    (0x040A, "es-ES_tradnl"),
    (0x040B, "fi-FI"),
    (0x040C, "fr-FR"),
    (0x040D, "he-IL"),
    (0x040E, "hu-HU"),
    (0x040F, "is-IS"),
    (0x0410, "it-IT"),
    (0x0411, "ja-JP"),
    (0x0412, "ko-KR"),
    (0x0413, "nl-NL"),
    (0x0414, "nb-NO"),
    (0x0415, "pl-PL"),
    (0x0416, "pt-BR"),
    (0x0418, "ro-RO"),
    (0x0419, "ru-RU"),
    (0x041A, "hr-HR"),
    (0x041B, "sk-SK"),
    (0x041C, "sq-AL"),
    (0x041D, "sv-SE"),
    (0x041E, "th-TH"),
    (0x041F, "tr-TR"),
    (0x0420, "ur-PK"),
    (0x0421, "id-ID"),
    (0x0422, "uk-UA"),
    (0x0423, "be-BY"),
    (0x0424, "sl-SI"),
    (0x0425, "et-EE"),
    (0x0426, "lv-LV"),
    (0x0427, "lt-LT"),
    (0x0429, "fa-IR"),
    (0x042A, "vi-VN"),
    (0x042B, "hy-AM"),
    (0x042D, "eu-ES"),
    (0x042F, "mk-MK"),
    (0x0436, "af-ZA"),
    (0x0437, "ka-GE"),
    (0x0439, "hi-IN"),
    (0x043E, "ms-MY"),
    (0x043F, "kk-KZ"),
    (0x0445, "bn-IN"),
    (0x0446, "pa-IN"),
    (0x0447, "gu-IN"),
    (0x0449, "ta-IN"),
    (0x044A, "te-IN"),
    (0x044B, "kn-IN"),
    (0x044C, "ml-IN"),
    (0x044E, "mr-IN"),
    (0x0456, "gl-ES"),
    (0x0804, "zh-CN"),
    (0x0807, "de-CH"),
    (0x0809, "en-GB"),
    (0x080A, "es-MX"),
    (0x080C, "fr-BE"),
    (0x0810, "it-CH"),
    (0x0813, "nl-BE"),
    (0x0814, "nn-NO"),
    (0x0816, "pt-PT"),
    (0x081D, "sv-FI"),
    (0x0C04, "zh-HK"),
    (0x0C07, "de-AT"),
    (0x0C09, "en-AU"),
    (0x0C0A, "es-ES"),
    (0x0C0C, "fr-CA"),
    (0x1004, "zh-SG"),
    (0x1009, "en-CA"),
    (0x100C, "fr-CH"),
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

/// Explicit failures of the native reference/encoding bridge. Callback errors stay errors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CultureInfoError {
    NullName,
    NullCompareInfo,
    NullTextInfo,
    InvalidUtf16,
    Failed(String),
}
impl fmt::Display for CultureInfoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NullName => f.write_str("culture Name is null"),
            Self::NullCompareInfo => f.write_str("culture CompareInfo is null"),
            Self::NullTextInfo => f.write_str("culture TextInfo is null"),
            Self::InvalidUtf16 => {
                f.write_str("culture text cannot be represented by the UTF8 bridge")
            }
            Self::Failed(message) => f.write_str(message),
        }
    }
}
impl std::error::Error for CultureInfoError {}
fn unwrap<T>(value: Result<T, CultureInfoError>) -> T {
    value.unwrap_or_else(|error| std::panic::resume_unwind(Box::new(error)))
}

/// Base instance fields for a CultureInfo subclass. An object's fields are never copied by
/// casting or cloning a CultureInfo handle. This is not complete CLR CultureData.
#[derive(Clone)]
pub struct CultureInfoData {
    name: Rc<str>,
    name_text: Utf16Text,
    lcid: i32,
    date_time_format: Option<Rc<DateTimeFormatInfo>>,
    number_format: Option<Rc<NumberFormatInfo>>,
    fallback_text_info: Rc<OnceCell<Rc<TextInfo>>>,
    fallback_compare_info: Rc<OnceCell<Rc<CompareInfo>>>,
}
impl CultureInfoData {
    pub fn new(name: &str) -> Self {
        let known = KNOWN_CULTURES
            .iter()
            .find(|(_, known)| known.eq_ignore_ascii_case(name));
        let (lcid, name) = known.map(|(id, name)| (*id, *name)).unwrap_or((
            if name.is_empty() {
                INVARIANT_LCID
            } else {
                CUSTOM_UNSPECIFIED_LCID
            },
            name,
        ));
        Self {
            name: Rc::from(name),
            name_text: name.into(),
            lcid,
            date_time_format: None,
            number_format: None,
            fallback_text_info: Rc::new(OnceCell::new()),
            fallback_compare_info: Rc::new(OnceCell::new()),
        }
    }
}
impl PartialEq for CultureInfoData {
    fn eq(&self, other: &Self) -> bool {
        self.name_text == other.name_text
    }
}

/// Virtual culture slots. Implementations are original Rc objects, including native subclasses.
/// Errors represent thrown operations; nullable returns remain distinct from failures.
pub trait CultureInfoImpl: Any {
    fn data(&self) -> &CultureInfoData;
    fn name(&self) -> Result<Option<Utf16Text>, CultureInfoError> {
        Ok(Some(self.data().name_text.clone()))
    }
    fn to_text(&self) -> Result<Option<Utf16Text>, CultureInfoError> {
        Ok(Some(self.data().name_text.clone()))
    }
    fn equals(
        &self,
        this: &CultureInfo,
        other: Option<BoxedValue>,
    ) -> Result<bool, CultureInfoError> {
        this.base_equals_object(other)
    }
    fn text_info(&self, this: &CultureInfo) -> Result<Option<Rc<TextInfo>>, CultureInfoError> {
        if let Some(info) = super::text_info::provided_for_culture(this) {
            return Ok(Some(info));
        }
        Ok(Some(
            self.data()
                .fallback_text_info
                .get_or_init(|| super::text_info::fallback_for_culture(this))
                .clone(),
        ))
    }
    fn compare_info(
        &self,
        this: &CultureInfo,
    ) -> Result<Option<Rc<CompareInfo>>, CultureInfoError> {
        let provider = FerroLocator::current().get_service::<dyn ICultureDataProvider>();
        if provider.is_some() {
            return Ok(Some(Rc::new(this.base_compare_info(provider))));
        }
        Ok(Some(
            self.data()
                .fallback_compare_info
                .get_or_init(|| Rc::new(this.base_compare_info(None)))
                .clone(),
        ))
    }
    fn hash_code(&self, this: &CultureInfo) -> Result<i32, CultureInfoError> {
        this.base_hash_code()
    }
    fn number_format(&self, this: &CultureInfo) -> Rc<NumberFormatInfo> {
        self.data()
            .number_format
            .clone()
            .unwrap_or_else(|| NumberFormatInfo::for_culture(this))
    }
    fn date_time_format(&self, this: &CultureInfo) -> Rc<DateTimeFormatInfo> {
        self.data()
            .date_time_format
            .clone()
            .unwrap_or_else(|| DateTimeFormatInfo::for_culture(this))
    }
}
impl CultureInfoImpl for CultureInfoData {
    fn data(&self) -> &CultureInfoData {
        self
    }
}

/// A class reference retaining the original object allocation and its virtual interface.
#[derive(Clone)]
pub struct CultureInfo {
    object: BoxedValue,
    implementation: Rc<dyn CultureInfoImpl>,
}
pub struct WeakCultureInfo {
    object: Weak<dyn crate::AnyValue>,
    implementation: Weak<dyn CultureInfoImpl>,
}
impl WeakCultureInfo {
    pub fn upgrade(&self) -> Option<CultureInfo> {
        Some(CultureInfo {
            object: self.object.upgrade()?,
            implementation: self.implementation.upgrade()?,
        })
    }
}
crate::ferro_thread_local! {
 static CURRENT_CULTURE:RefCell<CultureInfo>=RefCell::new(CultureInfo::invariant_culture());
 static CURRENT_UI_CULTURE:RefCell<CultureInfo>=RefCell::new(CultureInfo::invariant_culture());
 static CACHED:RefCell<HashMap<String,CultureInfo>>=RefCell::new(HashMap::new());
}
type CultureLookup=fn(&str)->Option<CultureInfo>;
static LOOKUP:std::sync::OnceLock<CultureLookup>=std::sync::OnceLock::new();
impl CultureInfo {
    #[doc(hidden)]
    pub fn from_projection(object:BoxedValue,implementation:Rc<dyn CultureInfoImpl>)->Self {Self {object,implementation}}
    /// Install a shared class-reference source before application culture use. Unknown tags
    /// keep the framework's existing provider/tag behavior; no object is selected by virtual Name.
    pub fn install_lookup(lookup:CultureLookup)->Result<(),&'static str> {
        let old=LOOKUP.get_or_init(||lookup);if std::ptr::fn_addr_eq(*old,lookup){Ok(())}else{Err("culture lookup already installed")}
    }

    /// Register and retain the original class object. Rust PartialEq of a custom plain class
    /// remains its responsibility, as for other native metadata model classes.
    pub fn from_object<T: CultureInfoImpl + PartialEq + 'static>(object: Rc<T>) -> Self {
        Self::register_implementation::<T>();
        Self {
            object: object.clone(),
            implementation: object,
        }
    }
    fn register_implementation<T: CultureInfoImpl + PartialEq + 'static>() {
        ValueTypes::register_reference::<T>();
        MarkupType::register_handle::<T>(Self::MARKUP);
        MarkupType::register_handle::<Rc<T>>(Self::MARKUP);
        ValueTypes::register_boxed_cast_from_id::<CultureInfo>(std::any::TypeId::of::<T>(), |object| {
            let any: Rc<dyn Any> = object.clone();
            let object = any.downcast::<T>().ok()?;
            Some(CultureInfo {
                object: object.clone(),
                implementation: object,
            })
        });
        ValueTypes::register_boxed_cast_from_id::<Option<CultureInfo>>(std::any::TypeId::of::<T>(), |object| {
            let any: Rc<dyn Any> = object.clone();
            let object = any.downcast::<T>().ok()?;
            Some(Some(CultureInfo {
                object: object.clone(),
                implementation: object,
            }))
        });
        ValueTypes::register_object_text::<T>(|value| {
            value
                .to_text()
                .map_err(|e| crate::metadata::MarkupInvokeError::Failed(e.to_string()))
        });
    }
    pub fn object(&self) -> BoxedValue {
        self.object.clone()
    }
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.object.downcast_ref()
    }
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.object, &other.object)
    }
    pub fn downgrade(&self) -> WeakCultureInfo {
        WeakCultureInfo {
            object: Rc::downgrade(&self.object),
            implementation: Rc::downgrade(&self.implementation),
        }
    }
    /// An uncached tag-backed framework object. This does not implement the validating CLR
    /// CultureInfo constructor. Source constructors remain explicitly missing from metadata.
    pub fn from_language_tag(name: &str) -> Self {
        Self::from_object(Rc::new(CultureInfoData::new(name)))
    }
    pub fn invariant_culture() -> Self {
        Self::get_culture_info("")
    }
    pub fn get_culture_info(name: &str) -> Self {
        if let Some(value)=LOOKUP.get().and_then(|lookup|lookup(name)) {return value;}
        let key = name.to_ascii_lowercase();
        if let Some(value) = CACHED.with_borrow(|values| values.get(&key).cloned()) {
            return value;
        }
        let value = Self::from_language_tag(name);
        CACHED.with_borrow_mut(|values| values.entry(key).or_insert(value).clone())
    }
    pub fn current_culture() -> Self {
        CURRENT_CULTURE.with_borrow(Clone::clone)
    }
    pub fn current_ui_culture() -> Self {
        CURRENT_UI_CULTURE.with_borrow(Clone::clone)
    }
    pub fn set_current_culture(value: Self) {
        let old = CURRENT_CULTURE.with_borrow_mut(|current| std::mem::replace(current, value));
        drop(old);
    }
    pub fn set_current_ui_culture(value: Self) {
        let old = CURRENT_UI_CULTURE.with_borrow_mut(|current| std::mem::replace(current, value));
        drop(old);
    }
    pub fn try_name_text(&self) -> Result<Option<Utf16Text>, CultureInfoError> {
        self.implementation.name()
    }
    pub fn name_text(&self) -> Option<Utf16Text> {
        unwrap(self.try_name_text())
    }
    /// Checked UTF8 bridge for existing font/platform APIs. Native Name metadata uses name_text.
    pub fn try_name_utf8(&self) -> Result<String, CultureInfoError> {
        self.try_name_text()?
            .ok_or(CultureInfoError::NullName)?
            .to_utf8()
            .map_err(|_| CultureInfoError::InvalidUtf16)
    }
    /// The immutable native language tag, borrowed as in the current framework API.
    /// Live/fallible extension names remain available through `try_name_text` and
    /// `try_name_utf8`; they are not substituted for native formatting data.
    pub fn name(&self) -> &str {
        self.implementation.data().name.as_ref()
    }
    pub fn try_to_text(&self) -> Result<Option<Utf16Text>, CultureInfoError> {
        self.implementation.to_text()
    }
    pub fn to_text(&self) -> Option<Utf16Text> {
        unwrap(self.try_to_text())
    }
    pub fn try_equals_object(&self, other: Option<BoxedValue>) -> Result<bool, CultureInfoError> {
        self.implementation.equals(self, other)
    }
    pub fn base_equals_object(&self, other: Option<BoxedValue>) -> Result<bool, CultureInfoError> {
        let Some(other) = crate::metadata::from_markup_value::<Self>(&other) else {
            return Ok(false);
        };
        if self.ptr_eq(&other) {
            return Ok(true);
        }
        let left = self.try_name_text()?;
        let right = other.try_name_text()?;
        let left = left.ok_or(CultureInfoError::NullName)?;
        if Some(left) != right {
            return Ok(false);
        }
        let left = self.try_compare_info()?;
        let right = other.try_compare_info()?;
        let left = left.ok_or(CultureInfoError::NullCompareInfo)?;
        Ok(right.is_some_and(|right| left.name() == right.name()))
    }
    pub fn try_hash_code(&self) -> Result<i32, CultureInfoError> {
        self.implementation.hash_code(self)
    }
    pub fn base_hash_code(&self) -> Result<i32, CultureInfoError> {
        self.base_hash_code_with_seed(process_hash_seed())
    }
    #[doc(hidden)]
    pub fn base_hash_code_with_seed(&self, seed: u64) -> Result<i32, CultureInfoError> {
        let name = self.try_name_text()?.ok_or(CultureInfoError::NullName)?;
        let a = hash_culture_text(&name, seed);
        let compare = self
            .try_compare_info()?
            .ok_or(CultureInfoError::NullCompareInfo)?;
        Ok(a.wrapping_add(hash_culture_text(&compare.name().into(), seed)))
    }
    pub fn try_text_info(&self) -> Result<Option<Rc<TextInfo>>, CultureInfoError> {
        self.implementation.text_info(self)
    }
    pub fn text_info_nullable(&self) -> Option<Rc<TextInfo>> {
        unwrap(self.try_text_info())
    }
    /// Checked legacy bridge for a caller that immediately uses the returned TextInfo.
    pub fn text_info(&self) -> Rc<TextInfo> {
        unwrap(
            self.try_text_info()
                .and_then(|v| v.ok_or(CultureInfoError::NullTextInfo)),
        )
    }
    pub fn try_compare_info(&self) -> Result<Option<Rc<CompareInfo>>, CultureInfoError> {
        self.implementation.compare_info(self)
    }
    pub fn compare_info_nullable(&self) -> Option<Rc<CompareInfo>> {
        unwrap(self.try_compare_info())
    }
    pub fn compare_info(&self) -> CompareInfo {
        (*unwrap(
            self.try_compare_info()
                .and_then(|v| v.ok_or(CultureInfoError::NullCompareInfo)),
        ))
        .clone()
    }
    pub(crate) fn data_name(&self) -> &str {
        &self.implementation.data().name
    }
    pub fn lcid(&self) -> i32 {
        self.implementation.data().lcid
    }
    pub fn is_invariant(&self) -> bool {
        self.implementation.data().name.is_empty()
    }
    pub fn two_letter_iso_language_name(&self) -> &str {
        let name = &self.implementation.data().name;
        if name.is_empty() {
            "iv"
        } else {
            &name[..name.find(['-', '_']).unwrap_or(name.len())]
        }
    }
    pub fn parent(&self) -> Self {
        let name = &self.implementation.data().name;
        let Some(separator) = name.rfind(['-', '_']) else {
            return Self::invariant_culture();
        };
        let (language, last) = (&name[..separator], &name[separator + 1..]);
        if language.eq_ignore_ascii_case("zh") {
            if ["CN", "SG", "MY"]
                .iter()
                .any(|v| v.eq_ignore_ascii_case(last))
            {
                return Self::get_culture_info("zh-Hans");
            }
            if ["TW", "HK", "MO"]
                .iter()
                .any(|v| v.eq_ignore_ascii_case(last))
            {
                return Self::get_culture_info("zh-Hant");
            }
        }
        Self::get_culture_info(language)
    }
    pub fn get_culture_info_by_lcid(lcid: i32) -> Option<Self> {
        if lcid == INVARIANT_LCID {
            return Some(Self::invariant_culture());
        }
        KNOWN_CULTURES
            .iter()
            .find(|(known, _)| *known == lcid)
            .map(|(_, name)| Self::get_culture_info(name))
    }
    pub fn get_culture_info_by_ietf_language_tag(name: &str) -> Option<Self> {
        if name == "zh-CHT" || name == "zh-CHS" {
            return None;
        }
        let culture = Self::get_culture_info(name);
        if culture.lcid() > 0xffff || culture.lcid() == 0x040a {
            None
        } else {
            Some(culture)
        }
    }
    pub fn get_cultures(types: CultureTypes) -> Vec<Self> {
        let Some(provider) = FerroLocator::current().get_service::<dyn ICultureDataProvider>()
        else {
            return vec![Self::invariant_culture()];
        };
        let mut cultures = Vec::new();
        if types.contains(CultureTypes::NEUTRAL_CULTURES) {
            cultures.push(Self::invariant_culture());
        }
        cultures.extend(
            provider
                .get_culture_names(types)
                .iter()
                .map(|name| Self::get_culture_info(name)),
        );
        cultures
    }
    pub fn number_format(&self) -> Rc<NumberFormatInfo> {
        self.implementation.number_format(self)
    }
    pub fn date_time_format(&self) -> Rc<DateTimeFormatInfo> {
        self.implementation.date_time_format(self)
    }
    pub fn with_number_format(&self, info: NumberFormatInfo) -> Self {
        let mut data = self.implementation.data().clone();
        data.number_format = Some(Rc::new(info));
        Self::from_object(Rc::new(data))
    }
    pub fn with_date_time_format(&self, info: DateTimeFormatInfo) -> Self {
        let mut data = self.implementation.data().clone();
        data.date_time_format = Some(Rc::new(info));
        Self::from_object(Rc::new(data))
    }
    pub fn calendar(&self) -> GregorianCalendar {
        GregorianCalendar
    }
}
impl PartialEq for CultureInfo {
    fn eq(&self, other: &Self) -> bool {
        unwrap(self.try_equals_object(Some(other.object())))
    }
}
// Custom Equals need not be reflexive: intentionally no Eq/Hash marker on the reference handle.
impl fmt::Debug for CultureInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("CultureInfo")
            .field(&Rc::as_ptr(&self.object))
            .finish()
    }
}
impl fmt::Display for CultureInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match unwrap(self.try_to_text()) {
            None => Ok(()),
            Some(text) => f.write_str(&text.to_utf8().map_err(|_| fmt::Error)?),
        }
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
        assert_eq!(
            CultureInfo::get_culture_info_by_lcid(0x0409)
                .unwrap()
                .name(),
            "en-US"
        );
        assert!(CultureInfo::get_culture_info_by_lcid(0x7777).is_none());
        assert_eq!(
            CultureInfo::get_culture_info("EN-us"),
            CultureInfo::get_culture_info("en-US")
        );
    }

    #[test]
    fn the_cultures_are_the_ones_of_the_provider() {
        use crate::utilities::TestCultureDataProvider;

        let _scope = FerroLocator::enter_scope();
        let names = |types| {
            CultureInfo::get_cultures(types)
                .iter()
                .map(|c| c.name().to_string())
                .collect::<Vec<_>>()
        };

        // Without a provider: the invariant culture, whatever the types.
        assert_eq!(names(CultureTypes::SPECIFIC_CULTURES), [""]);
        assert_eq!(names(CultureTypes::ALL_CULTURES), [""]);

        TestCultureDataProvider::register();
        assert_eq!(names(CultureTypes::SPECIFIC_CULTURES), ["en-GB", "en-US"]);
        // The invariant culture is listed with the neutral ones.
        assert_eq!(names(CultureTypes::NEUTRAL_CULTURES), ["", "en"]);
        assert_eq!(
            names(CultureTypes::ALL_CULTURES),
            ["", "en", "en-GB", "en-US"]
        );
        assert!(names(CultureTypes::INSTALLED_WIN32_CULTURES).is_empty());
    }

    #[test]
    fn parent_drops_the_last_subtag() {
        assert_eq!(CultureInfo::get_culture_info("ja-JP").parent().name(), "ja");
        assert!(CultureInfo::get_culture_info("ja").parent().is_invariant());
        assert!(CultureInfo::invariant_culture().parent().is_invariant());
        assert_eq!(
            CultureInfo::get_culture_info("zh-Hans-CN").parent().name(),
            "zh-Hans"
        );
        assert_eq!(
            CultureInfo::get_culture_info("zh-CN").parent().name(),
            "zh-Hans"
        );
        assert_eq!(
            CultureInfo::get_culture_info("zh-TW").parent().name(),
            "zh-Hant"
        );
        assert_eq!(
            CultureInfo::get_culture_info("zh-Hant").parent().name(),
            "zh"
        );
    }
}

// Marvin32 over native UTF16 bytes. The source uses a process-randomized seed too;
// exact cross-process hash values are not stable. Fixed-seed reference vectors are testable.
pub use ferroui_system::{process_hash_seed,hash_culture_text};

/// Explicit comparer keys for native culture dictionaries. Unlike Rust Eq, CLR virtual
/// equality need not be reflexive; the dictionary must actually invoke it on each lookup.
pub trait CultureMapKey: Clone {
    fn culture_map_hash(&self) -> u64;
    fn culture_map_equals(&self, other: &Self) -> bool;
}
impl CultureMapKey for CultureInfo {
    fn culture_map_hash(&self) -> u64 {
        unwrap(self.try_hash_code()) as u32 as u64
    }
    fn culture_map_equals(&self, other: &Self) -> bool {
        self == other
    }
}
/// Culture-key storage with explicit hashing/equality, retaining the original stored key.
/// This value container replaces Rust HashMap only where CLR nonreflexive culture keys occur.
#[derive(Clone)]
pub struct CultureMap<K, V> {
    entries: Vec<(K, V)>,
    buckets: HashMap<u64, Vec<usize>>,
}
impl<K, V> Default for CultureMap<K, V> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            buckets: HashMap::new(),
        }
    }
}
impl<K: CultureMapKey, V> CultureMap<K, V> {
    pub fn new() -> Self {
        Self::default()
    }
    fn find(&self, key: &K, hash: u64) -> Option<usize> {
        self.buckets
            .get(&hash)?
            .iter()
            .rev()
            .copied()
            .find(|&i| self.entries[i].0.culture_map_equals(key))
    }
    pub fn get(&self, key: &K) -> Option<&V> {
        let hash = key.culture_map_hash();
        self.find(key, hash).map(|i| &self.entries[i].1)
    }
    pub fn contains_key(&self, key: &K) -> bool {
        self.get(key).is_some()
    }
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let hash = key.culture_map_hash();
        if let Some(i) = self.find(&key, hash) {
            return Some(std::mem::replace(&mut self.entries[i].1, value));
        }
        let i = self.entries.len();
        self.entries.push((key, value));
        self.buckets.entry(hash).or_default().push(i);
        None
    }
    pub fn entry(&mut self, key: K) -> CultureMapEntry<'_, K, V> {
        let hash = key.culture_map_hash();
        let slot = self.find(&key, hash);
        CultureMapEntry {
            map: self,
            key,
            hash,
            slot,
        }
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.entries.iter().map(|(_, v)| v)
    }
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.entries.iter().map(|(k, v)| (k, v))
    }
    pub fn clear(&mut self) {
        self.entries.clear();
        self.buckets.clear();
    }
}
pub struct CultureMapEntry<'a, K, V> {
    map: &'a mut CultureMap<K, V>,
    key: K,
    hash: u64,
    slot: Option<usize>,
}
impl<'a, K: CultureMapKey, V> CultureMapEntry<'a, K, V> {
    pub fn or_insert(self, value: V) -> &'a mut V {
        self.or_insert_with(|| value)
    }
    pub fn or_insert_with(self, create: impl FnOnce() -> V) -> &'a mut V {
        let i = if let Some(i) = self.slot {
            i
        } else {
            let value = create();
            let i = self.map.entries.len();
            self.map.entries.push((self.key, value));
            self.map.buckets.entry(self.hash).or_default().push(i);
            i
        };
        &mut self.map.entries[i].1
    }
}
impl<K: CultureMapKey, V> std::ops::Index<&K> for CultureMap<K, V> {
    type Output = V;
    fn index(&self, key: &K) -> &V {
        self.get(key).expect("culture dictionary key not found")
    }
}
