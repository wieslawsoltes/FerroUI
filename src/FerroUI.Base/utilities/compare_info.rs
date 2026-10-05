//! String comparison: the counterparts of .NET's `StringComparison`,
//! `CompareOptions` and `CultureInfo.CompareInfo`.
//!
//! The ordinal comparisons are complete and built in. The culture-sensitive
//! comparisons belong to the culture: [`CultureInfo::compare_info`] gives the
//! comparer of a culture, whose rules come from the culture data provider
//! registered with the locator ([`ICultureDataProvider::get_compare_rules`]).
//! Without a provider, or for a culture the provider has no rules for, the
//! built-in invariant rules are used. They are the ones the reference
//! runtime uses in its invariant globalization mode: a culture-sensitive
//! comparison is an ordinal one, and a culture-sensitive comparison that
//! ignores case is an ordinal one that ignores case. They know no ignorable
//! characters, expansions, contractions or canonical equivalence; a provider
//! that has collation data supplies them.
//!
//! Positions and lengths are in UTF-16 code units, as in the reference.

use super::{CultureInfo, ICultureDataProvider};
use crate::{FerroLocator, LocatorExtensions};
use bitflags::bitflags;
use std::cmp::Ordering;
use std::rc::Rc;

bitflags! {
    /// The string comparison options to use with [`CompareInfo`].
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct CompareOptions: i32 {
        /// The default option settings for string comparisons.
        const NONE = 0;
        /// The string comparison must ignore case.
        const IGNORE_CASE = 0x1;
        /// The string comparison must use successive UTF-16 code units,
        /// upper-cased with the invariant simple case mapping.
        const ORDINAL_IGNORE_CASE = 0x1000_0000;
        /// The string comparison must use successive UTF-16 code units.
        const ORDINAL = 0x4000_0000;
    }
}

/// Specifies the culture, case, and sort rules to be used by the comparisons
/// of strings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum StringComparison {
    /// Compare strings using culture-sensitive sort rules and the current
    /// culture.
    CurrentCulture = 0,
    /// Compare strings using culture-sensitive sort rules, the current
    /// culture, and ignoring the case of the strings being compared.
    CurrentCultureIgnoreCase = 1,
    /// Compare strings using culture-sensitive sort rules and the invariant
    /// culture.
    InvariantCulture = 2,
    /// Compare strings using culture-sensitive sort rules, the invariant
    /// culture, and ignoring the case of the strings being compared.
    InvariantCultureIgnoreCase = 3,
    /// Compare strings using ordinal (binary) sort rules.
    Ordinal = 4,
    /// Compare strings using ordinal (binary) sort rules and ignoring the
    /// case of the strings being compared.
    OrdinalIgnoreCase = 5,
}

impl StringComparison {
    /// The comparer and the options this comparison stands for.
    fn resolve(self) -> (CompareInfo, CompareOptions) {
        match self {
            StringComparison::CurrentCulture => (CultureInfo::current_culture().compare_info(), CompareOptions::NONE),
            StringComparison::CurrentCultureIgnoreCase => {
                (CultureInfo::current_culture().compare_info(), CompareOptions::IGNORE_CASE)
            }
            StringComparison::InvariantCulture => (CompareInfo::invariant(), CompareOptions::NONE),
            StringComparison::InvariantCultureIgnoreCase => (CompareInfo::invariant(), CompareOptions::IGNORE_CASE),
            StringComparison::Ordinal => (CompareInfo::invariant(), CompareOptions::ORDINAL),
            StringComparison::OrdinalIgnoreCase => (CompareInfo::invariant(), CompareOptions::ORDINAL_IGNORE_CASE),
        }
    }

    /// Compares two strings (`string.Compare(a, b, comparison)`).
    pub fn compare(self, a: &str, b: &str) -> Ordering {
        let (compare_info, options) = self.resolve();
        compare_info.compare(a, b, options)
    }

    /// Whether two strings, either of which may be null, are equal
    /// (`string.Equals(a, b, comparison)`).
    pub fn equals(self, a: Option<&str>, b: Option<&str>) -> bool {
        match (a, b) {
            (None, None) => true,
            (Some(a), Some(b)) => self.compare(a, b) == Ordering::Equal,
            _ => false,
        }
    }

    /// Whether `source` starts with `prefix` (`source.StartsWith(prefix,
    /// comparison)`).
    pub fn starts_with(self, source: &str, prefix: &str) -> bool {
        let (compare_info, options) = self.resolve();
        compare_info.is_prefix(source, prefix, options)
    }

    /// The position of the first occurrence of `value` in `source` in
    /// UTF-16 code units, or -1 (`source.IndexOf(value, comparison)`).
    pub fn index_of(self, source: &str, value: &str) -> i32 {
        let (compare_info, options) = self.resolve();
        compare_info.index_of(source, value, options)
    }

    /// Whether `value` occurs in `source` (`source.Contains(value,
    /// comparison)`).
    pub fn contains(self, source: &str, value: &str) -> bool {
        self.index_of(source, value) >= 0
    }
}

/// The culture-sensitive comparison rules of a culture, supplied by a
/// culture data provider. `options` never has an ordinal flag: the ordinal
/// comparisons do not reach the rules of a culture.
pub trait ICompareRules: 'static {
    /// Compares two strings.
    fn compare(&self, a: &str, b: &str, options: CompareOptions) -> Ordering;

    /// Whether `source` starts with `prefix`.
    fn is_prefix(&self, source: &str, prefix: &str, options: CompareOptions) -> bool;

    /// The position of the first occurrence of `value` in `source` in
    /// UTF-16 code units, or -1.
    fn index_of(&self, source: &str, value: &str, options: CompareOptions) -> i32;
}

/// Implements a set of methods for culture-sensitive string comparisons:
/// the comparer of a culture.
#[derive(Clone)]
pub struct CompareInfo {
    name: Rc<str>,
    /// The rules of the culture; the built-in invariant rules if `None`.
    rules: Option<Rc<dyn ICompareRules>>,
}

impl CompareInfo {
    /// The comparer of the invariant culture.
    pub fn invariant() -> CompareInfo {
        CompareInfo { name: Rc::from(""), rules: None }
    }

    /// The name of the culture used for the comparisons.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Compares two strings using the specified options.
    pub fn compare(&self, a: &str, b: &str, options: CompareOptions) -> Ordering {
        match self.culture_rules(options) {
            Some(rules) => rules.compare(a, b, options),
            None => {
                let ignore_case = Self::built_in_ignores_case(options);
                OrdinalUnits::new(a, ignore_case).cmp(OrdinalUnits::new(b, ignore_case))
            }
        }
    }

    /// Determines whether `source` starts with `prefix` using the specified
    /// options.
    pub fn is_prefix(&self, source: &str, prefix: &str, options: CompareOptions) -> bool {
        match self.culture_rules(options) {
            Some(rules) => rules.is_prefix(source, prefix, options),
            None => {
                let ignore_case = Self::built_in_ignores_case(options);
                let mut source = OrdinalUnits::new(source, ignore_case);
                OrdinalUnits::new(prefix, ignore_case).all(|unit| source.next() == Some(unit))
            }
        }
    }

    /// Searches for `value` in `source` using the specified options and
    /// returns the position of its first occurrence in UTF-16 code units,
    /// or -1. An empty value is found at 0.
    pub fn index_of(&self, source: &str, value: &str, options: CompareOptions) -> i32 {
        match self.culture_rules(options) {
            Some(rules) => rules.index_of(source, value, options),
            None => {
                let ignore_case = Self::built_in_ignores_case(options);
                let value: Vec<u16> = OrdinalUnits::new(value, ignore_case).collect();
                if value.is_empty() {
                    return 0;
                }
                let source: Vec<u16> = OrdinalUnits::new(source, ignore_case).collect();
                source.windows(value.len()).position(|window| window == value.as_slice()).map_or(-1, |index| index as i32)
            }
        }
    }

    /// The rules of the culture for a culture-sensitive comparison; `None`
    /// for an ordinal comparison and for a culture without rules.
    fn culture_rules(&self, options: CompareOptions) -> Option<&Rc<dyn ICompareRules>> {
        if options.intersects(CompareOptions::ORDINAL | CompareOptions::ORDINAL_IGNORE_CASE) {
            return None;
        }
        self.rules.as_ref()
    }

    /// Whether the built-in comparison for `options` ignores case.
    fn built_in_ignores_case(options: CompareOptions) -> bool {
        options.intersects(CompareOptions::IGNORE_CASE | CompareOptions::ORDINAL_IGNORE_CASE)
    }

    /// The upper-case form of a character under the invariant simple case
    /// mapping, the mapping of the ordinal comparison that ignores case: one
    /// character maps to one character (no expansions: `ß` stays `ß`), no
    /// culture is involved, and the dotless `ı` (U+0131) and the long `ſ`
    /// (U+017F) keep their identity, as in the reference runtime.
    pub fn to_upper_ordinal(c: char) -> char {
        if c.is_ascii() {
            return c.to_ascii_uppercase();
        }
        match c {
            '\u{0131}' | '\u{017F}' => return c,
            // The Greek letters with ypogegrammeni: their simple upper-case
            // forms are the letters with prosgegrammeni, their full ones are
            // two characters.
            '\u{1F80}'..='\u{1F87}' | '\u{1F90}'..='\u{1F97}' | '\u{1FA0}'..='\u{1FA7}' => {
                return char::from_u32(c as u32 + 8).unwrap_or(c);
            }
            '\u{1FB3}' => return '\u{1FBC}',
            '\u{1FC3}' => return '\u{1FCC}',
            '\u{1FF3}' => return '\u{1FFC}',
            // Case pairs the reference runtime folds that are newer than the
            // case tables of some toolchains.
            '\u{A7CF}' => return '\u{A7CE}',
            '\u{A7D3}' => return '\u{A7D2}',
            '\u{A7D5}' => return '\u{A7D4}',
            _ => {}
        }
        let mut upper = c.to_uppercase();
        match (upper.next(), upper.next()) {
            (Some(upper), None) => upper,
            // A character whose only upper-case form is an expansion has no
            // simple one.
            _ => c,
        }
    }
}

/// The UTF-16 code units of a text for an ordinal comparison, upper-cased
/// character by character when the comparison ignores case.
struct OrdinalUnits<'a> {
    chars: std::str::Chars<'a>,
    ignore_case: bool,
    pending: Option<u16>,
}

impl<'a> OrdinalUnits<'a> {
    fn new(text: &'a str, ignore_case: bool) -> Self {
        Self { chars: text.chars(), ignore_case, pending: None }
    }
}

impl Iterator for OrdinalUnits<'_> {
    type Item = u16;

    fn next(&mut self) -> Option<u16> {
        if let Some(unit) = self.pending.take() {
            return Some(unit);
        }
        let c = self.chars.next()?;
        let c = if self.ignore_case { CompareInfo::to_upper_ordinal(c) } else { c };
        let mut units = [0u16; 2];
        let units = c.encode_utf16(&mut units);
        if units.len() == 2 {
            self.pending = Some(units[1]);
        }
        Some(units[0])
    }
}

impl CultureInfo {
    /// The comparer of the culture (C# `CompareInfo`): the rules the culture
    /// data provider has for the culture or for the nearest of its parents,
    /// and otherwise the built-in invariant rules.
    pub fn compare_info(&self) -> CompareInfo {
        let mut rules = None;
        if !self.is_invariant() {
            if let Some(provider) = FerroLocator::current().get_service::<dyn ICultureDataProvider>() {
                let mut current = self.clone();
                while rules.is_none() && !current.is_invariant() {
                    rules = provider.get_compare_rules(current.name());
                    current = current.parent();
                }
            }
        }
        CompareInfo { name: Rc::from(self.name()), rules }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinal_compares_code_units() {
        assert!(StringComparison::Ordinal.equals(Some("name"), Some("name")));
        assert!(!StringComparison::Ordinal.equals(Some("name"), Some("NAME")));
        assert!(StringComparison::Ordinal.equals(None, None));
        assert!(!StringComparison::Ordinal.equals(None, Some("")));
        assert!(StringComparison::Ordinal.starts_with("name", "na"));
        assert!(StringComparison::Ordinal.starts_with("name", ""));
        assert!(!StringComparison::Ordinal.starts_with("name", "NA"));
        assert_eq!(1, StringComparison::Ordinal.index_of("name", "am"));
        assert_eq!(0, StringComparison::Ordinal.index_of("name", ""));
        assert_eq!(-1, StringComparison::Ordinal.index_of("name", "AM"));
        // A code unit above the surrogates sorts after a surrogate pair.
        assert_eq!(Ordering::Greater, StringComparison::Ordinal.compare("\u{FF5E}", "\u{10400}"));
    }

    #[test]
    fn ordinal_ignore_case_uses_the_simple_upper_case_mapping() {
        let comparison = StringComparison::OrdinalIgnoreCase;
        assert!(comparison.equals(Some("name"), Some("NAME")));
        assert!(comparison.equals(Some("\u{00E9}t\u{00E9}"), Some("\u{00C9}T\u{00C9}")));
        assert!(comparison.starts_with("name", "NAM"));
        assert_eq!(1, comparison.index_of("name", "AME"));

        // No expansions: the sharp s is not "SS".
        assert!(!comparison.equals(Some("stra\u{00DF}e"), Some("STRASSE")));
        assert!(comparison.equals(Some("\u{00DF}"), Some("\u{00DF}")));
        assert!(!comparison.contains("stra\u{00DF}e", "SS"));

        // No culture: the Turkish letters keep their identity.
        assert!(!comparison.equals(Some("\u{0131}"), Some("I")));
        assert!(!comparison.equals(Some("\u{0131}"), Some("i")));
        assert!(!comparison.equals(Some("\u{0130}"), Some("i")));
        assert!(!comparison.equals(Some("\u{0130}"), Some("I")));
        assert!(comparison.equals(Some("i"), Some("I")));

        // The simple mapping where the full one is an expansion.
        assert!(comparison.equals(Some("\u{1F80}"), Some("\u{1F88}")));
        assert!(!comparison.equals(Some("\u{1F80}"), Some("\u{1F08}\u{0399}")));
    }

    #[test]
    fn ordinal_ignore_case_handles_surrogate_pairs() {
        let comparison = StringComparison::OrdinalIgnoreCase;
        // Two characters that share their high surrogate.
        assert!(!comparison.equals(Some("\u{1F600}"), Some("\u{1F601}")));
        assert!(comparison.equals(Some("a\u{1F600}"), Some("A\u{1F600}")));
        // A pair is folded as the character it encodes.
        assert!(comparison.equals(Some("\u{10428}"), Some("\u{10400}")));
        // Positions count code units.
        assert_eq!(3, comparison.index_of("a\u{1F600}b", "B"));
        assert_eq!(3, StringComparison::Ordinal.index_of("a\u{1F600}b", "b"));
    }

    /// Pairs checked against the reference runtime (version 10.0.5), whose
    /// ordinal comparison that ignores case was also compared with
    /// [`CompareInfo::to_upper_ordinal`] for every character that has an
    /// upper-case or a lower-case form there.
    #[test]
    fn ordinal_ignore_case_matches_the_reference_runtime_for_edge_characters() {
        let comparison = StringComparison::OrdinalIgnoreCase;
        let rows: &[(&str, &str, bool)] = &[
            // No expansions and no culture.
            ("\u{00DF}", "SS", false),
            ("\u{00DF}", "\u{1E9E}", false),
            ("\u{FB01}", "FI", false),
            ("\u{0149}", "\u{02BC}N", false),
            ("\u{0131}", "I", false),
            ("\u{0131}", "i", false),
            ("\u{0130}", "i", false),
            ("\u{0130}", "I", false),
            ("\u{017F}", "S", false),
            ("\u{017F}", "s", false),
            ("\u{212A}", "k", false),
            ("\u{2126}", "\u{03C9}", false),
            // The simple upper-case mapping.
            ("\u{00B5}", "\u{039C}", true),
            ("\u{00B5}", "\u{03BC}", true),
            ("\u{03C2}", "\u{03A3}", true),
            ("\u{03C2}", "\u{03C3}", true),
            ("\u{01C5}", "\u{01C4}", true),
            ("\u{01C5}", "\u{01C6}", true),
            ("\u{1F80}", "\u{1F88}", true),
            ("\u{1F80}", "\u{1F08}\u{0399}", false),
            ("\u{10D0}", "\u{1C90}", true),
            ("\u{1C80}", "\u{0412}", true),
            ("\u{1C88}", "\u{A64A}", true),
            ("\u{24D0}", "\u{24B6}", true),
            ("\u{FF41}", "\u{FF21}", true),
            ("\u{A7CF}", "\u{A7CE}", true),
            ("\u{A7D3}", "\u{A7D2}", true),
            ("\u{A7D5}", "\u{A7D4}", true),
            // A surrogate pair is folded as the character it encodes.
            ("\u{10428}", "\u{10400}", true),
            ("\u{1E922}", "\u{1E900}", true),
            ("\u{10CC0}", "\u{10C80}", true),
            ("\u{1F600}", "\u{1F601}", false),
        ];

        for (a, b, equal) in rows {
            assert_eq!(*equal, comparison.equals(Some(a), Some(b)), "{a:?} {b:?}");
            assert_eq!(*equal, comparison.equals(Some(b), Some(a)), "{b:?} {a:?}");
            assert_eq!(*equal, comparison.starts_with(a, b), "{a:?} starts with {b:?}");
            let source = format!("a{a}b");
            assert_eq!(if *equal { 1 } else { -1 }, comparison.index_of(&source, b), "{source:?} index of {b:?}");
        }
    }

    #[test]
    fn culture_comparisons_use_the_built_in_invariant_rules_without_a_provider() {
        assert!(StringComparison::InvariantCultureIgnoreCase.equals(Some("na"), Some("NA")));
        assert!(!StringComparison::InvariantCulture.equals(Some("na"), Some("NA")));
        assert!(StringComparison::CurrentCultureIgnoreCase.starts_with("name", "NAM"));
        assert!(!StringComparison::CurrentCulture.starts_with("name", "NAM"));
        assert!(StringComparison::CurrentCultureIgnoreCase.contains("name", "AME"));
        assert_eq!("", CultureInfo::invariant_culture().compare_info().name());
    }

    /// Rules that treat every text as equal to every other.
    struct EverythingIsEqual;

    impl ICompareRules for EverythingIsEqual {
        fn compare(&self, _a: &str, _b: &str, _options: CompareOptions) -> Ordering {
            Ordering::Equal
        }

        fn is_prefix(&self, _source: &str, _prefix: &str, _options: CompareOptions) -> bool {
            true
        }

        fn index_of(&self, _source: &str, _value: &str, _options: CompareOptions) -> i32 {
            0
        }
    }

    struct Provider;

    impl ICultureDataProvider for Provider {
        fn get_compare_rules(&self, culture_name: &str) -> Option<Rc<dyn ICompareRules>> {
            (culture_name == "xx").then(|| Rc::new(EverythingIsEqual) as Rc<dyn ICompareRules>)
        }
    }

    #[test]
    fn the_rules_of_a_culture_come_from_the_provider() {
        let scope = FerroLocator::enter_scope();
        FerroLocator::current_mutable().bind::<dyn ICultureDataProvider>().to_constant(Rc::new(Provider));

        // The rules of the parent culture are used for a culture without.
        let compare_info = CultureInfo::get_culture_info("xx-YY").compare_info();
        assert_eq!("xx-YY", compare_info.name());
        assert_eq!(Ordering::Equal, compare_info.compare("a", "b", CompareOptions::NONE));
        assert!(compare_info.is_prefix("a", "b", CompareOptions::IGNORE_CASE));
        // The ordinal comparisons do not reach the rules of the culture.
        assert_ne!(Ordering::Equal, compare_info.compare("a", "b", CompareOptions::ORDINAL));
        assert_eq!(-1, compare_info.index_of("a", "b", CompareOptions::ORDINAL_IGNORE_CASE));

        // A culture the provider knows nothing about, and the invariant one.
        let other = CultureInfo::get_culture_info("zz").compare_info();
        assert_ne!(Ordering::Equal, other.compare("a", "b", CompareOptions::NONE));
        assert_ne!(Ordering::Equal, CompareInfo::invariant().compare("a", "b", CompareOptions::NONE));

        scope.dispose();
    }
}
