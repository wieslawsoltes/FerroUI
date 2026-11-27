//! `font-language-override`: OpenType language system tags as BCP 47 language tags.
//!
//! The shapers of the framework take a culture and not an OpenType tag, so the descriptor is
//! applied by giving the run the culture of the tag. Only registered tags with one clear language
//! are listed. A tag that is not listed is ignored by the caller and the text keeps its culture.

/// By tag, in byte order, for the binary search.
const LANGUAGES: &[(&str, &str)] = &[
    ("AFK", "af"), ("ARA", "ar"), ("AZE", "az"), ("BEL", "be"), ("BEN", "bn"), ("BGR", "bg"),
    ("BOS", "bs"), ("BRE", "br"), ("CAT", "ca"), ("CSY", "cs"), ("DAN", "da"), ("DEU", "de"),
    ("ELL", "el"), ("ENG", "en"), ("ESP", "es"), ("ETI", "et"), ("EUQ", "eu"), ("FAR", "fa"),
    ("FIN", "fi"), ("FRA", "fr"), ("GAE", "gd"), ("GAL", "gl"), ("GUJ", "gu"), ("HIN", "hi"),
    ("HRV", "hr"), ("HUN", "hu"), ("HYE", "hy"), ("IND", "id"), ("IRI", "ga"), ("ISL", "is"),
    ("ITA", "it"), ("IWR", "he"), ("JAN", "ja"), ("KAN", "kn"), ("KAT", "ka"), ("KAZ", "kk"),
    ("KHM", "km"), ("KOR", "ko"), ("LAO", "lo"), ("LTH", "lt"), ("LVI", "lv"), ("MAL", "ml"),
    ("MAR", "mr"), ("MKD", "mk"), ("MLY", "ms"), ("MNG", "mn"), ("MTS", "mt"), ("NLD", "nl"),
    ("NOR", "nb"), ("NYN", "nn"), ("ORI", "or"), ("PAN", "pa"), ("PLK", "pl"), ("PTG", "pt"),
    ("ROM", "ro"), ("RUS", "ru"), ("SKY", "sk"), ("SLV", "sl"), ("SQI", "sq"), ("SRB", "sr"),
    ("SVE", "sv"), ("SWK", "sw"), ("TAM", "ta"), ("TEL", "te"), ("THA", "th"), ("TRK", "tr"),
    ("UKR", "uk"), ("URD", "ur"), ("UZB", "uz"), ("VIT", "vi"), ("WEL", "cy"), ("ZHH", "zh-HK"), ("ZHS", "zh-Hans"), ("ZHT", "zh-Hant"),
];

/// The BCP 47 language of a registered OpenType language system tag. Tags are case-sensitive, and
/// the space that pads a three-letter tag to four bytes may be present or not.
pub fn opentype_language_to_bcp47(tag: &str) -> Option<&'static str> {
    let tag = tag.trim_end_matches(' ');
    LANGUAGES.binary_search_by_key(&tag, |entry| entry.0).ok().map(|index| LANGUAGES[index].1)
}

/// Every tag and its language, for tests that compare the table with a shaper.
#[cfg(any(test, feature = "testing"))]
pub fn language_table() -> &'static [(&'static str, &'static str)] {
    LANGUAGES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_is_sorted_and_has_no_duplicates() {
        assert!(LANGUAGES.windows(2).all(|pair| pair[0].0 < pair[1].0));
    }

    #[test]
    fn registered_tags_map_with_or_without_the_padding_space() {
        assert_eq!(opentype_language_to_bcp47("TRK"), Some("tr"));
        assert_eq!(opentype_language_to_bcp47("TRK "), Some("tr"));
        assert_eq!(opentype_language_to_bcp47("ZHS"), Some("zh-Hans"));
        assert_eq!(opentype_language_to_bcp47("ZHT"), Some("zh-Hant"));
        assert_eq!(opentype_language_to_bcp47("ZHH"), Some("zh-HK"));
    }

    #[test]
    fn other_tags_are_not_mapped() {
        for tag in ["trk", "XXX", "", " ", "TR", "TRKX"] {
            assert_eq!(opentype_language_to_bcp47(tag), None, "{tag:?}");
        }
    }
}
