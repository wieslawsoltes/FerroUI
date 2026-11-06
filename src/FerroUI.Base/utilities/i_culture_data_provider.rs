//! The source of culture data.
//!
//! The base library has the conventions of the invariant culture built in
//! and nothing else: the data of every other culture comes from a provider
//! registered with the service locator (by a platform layer, an application
//! or a test). Without a provider, or for a culture the provider has no data
//! for, a culture uses the data of its parent and finally of the invariant
//! culture.
//!
//! The provider is keyed by [`Locale`] (C5 of #423, docs/idiomatic-rust-signatures.md
//! section 3.4): the methods `..._by_locale` take the locale of a culture.
//! The methods keyed by the name of a culture remain until the release after
//! 0.5, deprecated: the default of each method keyed by locale asks the one
//! keyed by name with the BCP-47 text of the locale, so a provider that
//! implements the old methods keeps working.

use super::{CultureInfo, CultureTypes, DateTimeFormatInfo, Locale, NumberFormatInfo, TextInfo};
use crate::{FerroLocator, LocatorExtensions};
use std::rc::Rc;

/// Supplies the conventions of cultures by locale. Register an
/// implementation with the locator:
///
/// ```ignore
/// FerroLocator::current_mutable().bind::<dyn ICultureDataProvider>().to_constant(provider);
/// ```
///
/// A provider is asked on every use of a culture, so it should hand out
/// shared instances it keeps. The locale it is asked for is never the
/// unknown locale `und` (the invariant culture, whose data is built in);
/// a culture without data of its own is asked for the parents of its
/// locale next (`en-GB` after `en-GB-u-hc-h12`, `en` after `en-GB`).
pub trait ICultureDataProvider: 'static {
    /// The date and time conventions of `locale`, or `None` when the
    /// provider has no data for it (the parent locale is asked next).
    ///
    /// The default asks [`get_date_time_format`](Self::get_date_time_format)
    /// with the BCP-47 text of the locale.
    #[allow(deprecated)]
    fn get_date_time_format_by_locale(&self, locale: &Locale) -> Option<Rc<DateTimeFormatInfo>> {
        self.get_date_time_format(&locale.to_string())
    }

    // number format: owned by numeric agent
    /// The number conventions of `locale`, or `None` when the provider has
    /// no data for it (the parent locale is asked next).
    ///
    /// The default asks [`get_number_format`](Self::get_number_format) with
    /// the BCP-47 text of the locale.
    #[allow(deprecated)]
    fn get_number_format_by_locale(&self, locale: &Locale) -> Option<Rc<NumberFormatInfo>> {
        self.get_number_format(&locale.to_string())
    }

    // string comparison: added with the auto-complete box (see `compare_info.rs`)
    /// The culture-sensitive string comparison rules of `locale`, or `None`
    /// when the provider has no rules for it (the parent locale is asked
    /// next, and the built-in invariant rules are used last).
    ///
    /// The default asks [`get_compare_rules`](Self::get_compare_rules) with
    /// the BCP-47 text of the locale.
    #[allow(deprecated)]
    fn get_compare_rules_by_locale(&self, locale: &Locale) -> Option<Rc<dyn super::ICompareRules>> {
        self.get_compare_rules(&locale.to_string())
    }

    // text conventions: added with the automation peer of the calendar (see `text_info.rs`)
    /// The text conventions (the list separator) of `locale`, or `None` when
    /// the provider has no data for it (the parent locale is asked next).
    ///
    /// The default asks [`get_text_info`](Self::get_text_info) with the
    /// BCP-47 text of the locale.
    #[allow(deprecated)]
    fn get_text_info_by_locale(&self, locale: &Locale) -> Option<Rc<TextInfo>> {
        self.get_text_info(&locale.to_string())
    }

    // enumeration: added with the numeric up-down page of the control catalog (see `CultureInfo::get_cultures`)
    /// The locales of the cultures of the given types the provider has data
    /// for, in the order they are listed in (the unknown locale `und`, the
    /// invariant culture, is not among them). A provider that does not
    /// enumerate its cultures lists none.
    ///
    /// The default lists the locales of the names
    /// [`get_culture_names`](Self::get_culture_names) lists; a name that is
    /// not a BCP-47 language tag names no locale and is left out.
    #[allow(deprecated)]
    fn get_locales(&self, types: CultureTypes) -> Vec<Locale> {
        self.get_culture_names(types).iter().filter_map(|name| Locale::try_from_str(name).ok()).collect()
    }

    /// The date and time conventions of the culture named exactly
    /// `culture_name` (a language tag such as `en-US`; never empty), or
    /// `None` when the provider has no data for it (the parent culture is
    /// asked next).
    #[deprecated(since = "0.5.0", note = "implement `get_date_time_format_by_locale`, keyed by the locale of the culture")]
    fn get_date_time_format(&self, culture_name: &str) -> Option<Rc<DateTimeFormatInfo>> {
        let _ = culture_name;
        None
    }

    /// The number conventions of the culture named exactly `culture_name`,
    /// or `None` when the provider has no data for it (the parent culture is
    /// asked next).
    #[deprecated(since = "0.5.0", note = "implement `get_number_format_by_locale`, keyed by the locale of the culture")]
    fn get_number_format(&self, culture_name: &str) -> Option<Rc<NumberFormatInfo>> {
        let _ = culture_name;
        None
    }

    /// The culture-sensitive string comparison rules of the culture named
    /// exactly `culture_name`, or `None` when the provider has no rules for
    /// it (the parent culture is asked next, and the built-in invariant
    /// rules are used last).
    #[deprecated(since = "0.5.0", note = "implement `get_compare_rules_by_locale`, keyed by the locale of the culture")]
    fn get_compare_rules(&self, culture_name: &str) -> Option<Rc<dyn super::ICompareRules>> {
        let _ = culture_name;
        None
    }

    /// The text conventions (the list separator) of the culture named
    /// exactly `culture_name`, or `None` when the provider has no data for
    /// it (the parent culture is asked next).
    #[deprecated(since = "0.5.0", note = "implement `get_text_info_by_locale`, keyed by the locale of the culture")]
    fn get_text_info(&self, culture_name: &str) -> Option<Rc<TextInfo>> {
        let _ = culture_name;
        None
    }

    /// The names of the cultures of the given types the provider has data
    /// for, in the order they are listed in (the invariant culture, whose
    /// name is empty, is not among them). A provider that does not enumerate
    /// its cultures lists none.
    #[deprecated(since = "0.5.0", note = "implement `get_locales`, which lists the locales of the cultures")]
    fn get_culture_names(&self, types: CultureTypes) -> Vec<String> {
        let _ = types;
        Vec::new()
    }
}

/// Finds the data of `culture`: asks the registered provider for the locale
/// of the culture (`by_locale`) and then for each of its parents; `None` when
/// there is no provider or it has nothing for any of them (the caller then
/// uses the invariant data).
///
/// A culture whose name is not a BCP-47 language tag (`es-ES_tradnl`) has no
/// locale: it and its parents are asked for by name (`by_name`, the
/// deprecated methods) until a parent is the culture of a locale
/// (`es-ES`), whose locale and its parents are asked for next.
pub(crate) fn find_culture_data<T: ?Sized>(
    culture: &CultureInfo,
    by_locale: impl Fn(&dyn ICultureDataProvider, &Locale) -> Option<Rc<T>>,
    by_name: impl Fn(&dyn ICultureDataProvider, &str) -> Option<Rc<T>>,
) -> Option<Rc<T>> {
    if culture.is_invariant() {
        return None;
    }
    let provider = FerroLocator::current().get_service::<dyn ICultureDataProvider>()?;
    let mut current = culture.clone();
    while !current.is_locale() {
        if let Some(data) = by_name(&*provider, current.name()) {
            return Some(data);
        }
        current = current.parent();
    }
    let mut locale = current.locale().clone();
    while locale != Locale::UNKNOWN {
        if let Some(data) = by_locale(&*provider, &locale) {
            return Some(data);
        }
        locale = super::parent_locale(&locale);
    }
    None
}

#[cfg(test)]
mod tests {
    // Not from upstream (C5 of #423, L2): the provider is keyed by locale.
    use super::*;
    use crate::utilities::locale;
    use std::cell::RefCell;

    /// Records the locales it is asked for; has the number conventions of `de` alone.
    struct Recording {
        asked: RefCell<Vec<String>>,
        german: Rc<NumberFormatInfo>,
    }

    impl ICultureDataProvider for Recording {
        fn get_number_format_by_locale(&self, locale: &Locale) -> Option<Rc<NumberFormatInfo>> {
            self.asked.borrow_mut().push(locale.to_string());
            (*locale == locale!("de")).then(|| Rc::clone(&self.german))
        }
    }

    fn register_recording() -> Rc<Recording> {
        let provider = Rc::new(Recording {
            asked: RefCell::new(Vec::new()),
            german: Rc::new(NumberFormatInfo::new().with_number_decimal_separator(",")),
        });
        FerroLocator::current_mutable().bind::<dyn ICultureDataProvider>().to_constant(provider.clone());
        provider
    }

    #[test]
    fn the_provider_is_asked_for_the_locale_and_its_parents() {
        let _scope = FerroLocator::enter_scope();
        let provider = register_recording();
        let asked = |name: &str| {
            provider.asked.borrow_mut().clear();
            let format = CultureInfo::get_culture_info(name).number_format();
            (provider.asked.borrow().clone(), format.number_decimal_separator().to_owned())
        };

        // The locale, then its parents; the invariant culture is never asked for.
        assert_eq!(asked("de-AT"), (vec!["de-AT".to_owned(), "de".to_owned()], ",".to_owned()));
        assert_eq!(asked("DE-at-u-nu-latn"), (vec!["de-AT-u-nu-latn".to_owned(), "de-AT".to_owned(), "de".to_owned()], ",".to_owned()));
        assert_eq!(asked("zh-TW"), (vec!["zh-TW".to_owned(), "zh-Hant".to_owned(), "zh".to_owned()], ".".to_owned()));
        assert_eq!(asked(""), (Vec::new(), ".".to_owned()));
        // A culture whose name is no BCP-47 tag is asked for by name (the deprecated methods,
        // which this provider does not have), then the locale of its first parent that is one.
        assert_eq!(asked("de-DE_phoneb"), (vec!["de-DE".to_owned(), "de".to_owned()], ",".to_owned()));
        // The conventions are shared, and attached ones are not asked for.
        let culture = CultureInfo::from_locale(locale!("de-CH"));
        assert!(Rc::ptr_eq(&culture.number_format(), &provider.german));
        provider.asked.borrow_mut().clear();
        let attached = culture.with_number_format(NumberFormatInfo::new());
        assert_eq!(attached.number_format().number_decimal_separator(), ".");
        assert!(provider.asked.borrow().is_empty());
    }

    /// A provider written against the names: the deprecated methods.
    struct ByName;

    #[allow(deprecated)]
    impl ICultureDataProvider for ByName {
        fn get_number_format(&self, culture_name: &str) -> Option<Rc<NumberFormatInfo>> {
            matches!(culture_name, "pl-PL" | "es-ES_tradnl")
                .then(|| Rc::new(NumberFormatInfo::new().with_number_decimal_separator(culture_name)))
        }

        fn get_culture_names(&self, types: CultureTypes) -> Vec<String> {
            let _ = types;
            vec!["pl-PL".to_owned(), "es-ES_tradnl".to_owned()]
        }
    }

    #[test]
    fn a_provider_keyed_by_name_keeps_working() {
        let _scope = FerroLocator::enter_scope();
        FerroLocator::current_mutable().bind::<dyn ICultureDataProvider>().to_constant(Rc::new(ByName));

        // The methods keyed by locale ask the ones keyed by name with the text of the locale.
        assert_eq!(CultureInfo::get_culture_info("PL-pl").number_format().number_decimal_separator(), "pl-PL");
        assert_eq!(CultureInfo::from_locale(locale!("pl-PL-u-nu-latn")).number_format().number_decimal_separator(), "pl-PL");
        // A culture whose name is no BCP-47 tag is asked for by its name.
        assert_eq!(CultureInfo::get_culture_info("es-ES_tradnl").number_format().number_decimal_separator(), "es-ES_tradnl");
        assert_eq!(CultureInfo::get_culture_info("pl").number_format().number_decimal_separator(), ".");
        // The listed names that are no BCP-47 tags name no locale.
        let names: Vec<_> = CultureInfo::get_cultures(CultureTypes::SPECIFIC_CULTURES).iter().map(|c| c.name().to_owned()).collect();
        assert_eq!(names, ["pl-PL"]);
    }
}
