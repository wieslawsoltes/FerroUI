//! Language validation follows WinUI's ValidateXmlLanguage in
//! dxaml/xcp/core/text/common/text.cpp at the pinned rich-text reference.
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License.
//! See NOTICE.md and docs/porting/winui-rich-text.

use ferroui_base::platform::IPlatformSettings;
use ferroui_base::utilities::{locale, Locale};
use ferroui_base::{FerroLocator, LocatorExtensions};

/// WinUI's XML-language contract deliberately accepts more than registered
/// BCP-47 tags: empty, or ASCII subtags of one to eight characters, with letters
/// in the primary subtag and letters/digits in subsequent subtags.
pub(crate) fn is_valid_language_tag(tag: &str) -> bool {
    tag.is_empty()
        || tag.split('-').enumerate().all(|(index, subtag)| {
            (1..=8).contains(&subtag.len())
                && subtag
                    .bytes()
                    .all(|byte| byte.is_ascii_alphabetic() || (index != 0 && byte.is_ascii_digit()))
        })
}

/// The language of a tag: the empty tag is the invariant culture (the unknown
/// locale `und`); any other tag has to be a valid XML language tag and a BCP-47
/// language tag, which a locale holds (C5 of #423: the language is a locale, so
/// the XML language tags that are no BCP-47 tags, such as `x-private`, are
/// refused).
pub(crate) fn parse_language_tag(tag: &str) -> Result<Locale, super::DocumentError> {
    if tag.is_empty() {
        return Ok(Locale::UNKNOWN);
    }
    if !is_valid_language_tag(tag) {
        return Err(super::DocumentError::InvalidFormat(
            "Language must be a valid XML language tag",
        ));
    }
    Locale::try_from_str(tag)
        .map_err(|_| super::DocumentError::InvalidFormat("Language must be a BCP-47 language tag"))
}

/// The tag of a language: the BCP-47 tag of the locale, empty for the invariant
/// culture (the unknown locale `und`).
pub(crate) fn language_tag(language: &Locale) -> String {
    if *language == Locale::UNKNOWN {
        String::new()
    } else {
        language.to_string()
    }
}

/// Use the framework's platform language service in place of Windows
/// ApplicationLanguages. WinUI's fallback outside an application is en-US.
pub(crate) fn default_language() -> Locale {
    FerroLocator::current()
        .get_service::<dyn IPlatformSettings>()
        .map(|settings| settings.preferred_application_language())
        .filter(|name| !name.is_empty() && is_valid_language_tag(name))
        .and_then(|name| Locale::try_from_str(&name).ok())
        .unwrap_or(locale!("en-US"))
}

/// An empty assigned language (the invariant culture, the unknown locale `und`)
/// requests the application's language. Other locales stay intact for
/// FerroUI's culture-aware font fallback and shaping. Windows ResolveLocaleName
/// and its ordered fallback-language list are not available through the
/// portable framework contract.
pub(crate) fn resolved_language(language: &Locale) -> Locale {
    if *language == Locale::UNKNOWN {
        default_language()
    } else {
        language.clone()
    }
}
