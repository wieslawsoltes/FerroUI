//! Culture context and provider lookup over shared System comparison objects.
use super::{CultureInfo,ICultureDataProvider};
#[cfg(test)] use crate::{FerroLocator,LocatorExtensions};
use std::{cmp::Ordering,rc::Rc};
pub use ferroui_system::globalization::{CompareInfo,CompareOptions,ICompareRules};
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

impl CultureInfo {
    /// The comparer of the culture (C# `CompareInfo`): the rules the culture
    /// data provider has for the culture or for the nearest of its parents,
    /// and otherwise the built-in invariant rules.
    pub(crate) fn base_compare_info(&self, provider:Option<Rc<dyn ICultureDataProvider>>) -> CompareInfo {
        let mut rules = None;
        if !self.is_invariant() {
            if let Some(provider) = provider {
                let mut current = self.clone();
                while rules.is_none() && !current.is_invariant() {
                    rules = provider.get_compare_rules(current.data_name());
                    current = current.parent();
                }
            }
        }
        CompareInfo::for_name(self.data_name(),rules)
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
