//! Language validation follows WinUI's ValidateXmlLanguage in
//! dxaml/xcp/core/text/common/text.cpp at the pinned rich-text reference.
//! Copyright (c) Microsoft Corporation. Licensed under the MIT License.
//! See NOTICE.md and docs/porting/winui-rich-text.

use ferroui_base::platform::IPlatformSettings;
use ferroui_base::utilities::CultureInfo;
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

/// Use the framework's platform language service in place of Windows
/// ApplicationLanguages. WinUI's fallback outside an application is en-US.
pub(crate) fn default_language() -> CultureInfo {
    let name = FerroLocator::current()
        .get_service::<dyn IPlatformSettings>()
        .map(|settings| settings.preferred_application_language())
        .filter(|name| !name.is_empty() && is_valid_language_tag(name))
        .unwrap_or_else(|| "en-US".to_owned());
    CultureInfo::get_culture_info(&name)
}

/// An empty assigned language requests the application's language. Nonempty
/// tags stay intact for FerroUI's culture-aware font fallback and shaping.
/// Windows ResolveLocaleName and its ordered fallback-language list are not
/// available through the portable framework contract.
pub(crate) fn resolved_language(language: &CultureInfo) -> CultureInfo {
    if language.is_invariant() {
        default_language()
    } else {
        language.clone()
    }
}
