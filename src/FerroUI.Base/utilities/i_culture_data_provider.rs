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
//! The methods keyed by the name of a culture, deprecated in 0.5.0, were
//! removed in 0.7.0 (#625).

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
    /// The default has no data.
    fn get_date_time_format_by_locale(&self, locale: &Locale) -> Option<Rc<DateTimeFormatInfo>> {
        let _ = locale;
        None
    }

    // number format: owned by numeric agent
    /// The number conventions of `locale`, or `None` when the provider has
    /// no data for it (the parent locale is asked next).
    /// The default has no data.
    fn get_number_format_by_locale(&self, locale: &Locale) -> Option<Rc<NumberFormatInfo>> {
        let _ = locale;
        None
    }

    // string comparison: added with the auto-complete box (see `compare_info.rs`)
    /// The culture-sensitive string comparison rules of `locale`, or `None`
    /// when the provider has no rules for it (the parent locale is asked
    /// next, and the built-in invariant rules are used last).
    /// The default has no rules.
    fn get_compare_rules_by_locale(&self, locale: &Locale) -> Option<Rc<dyn super::ICompareRules>> {
        let _ = locale;
        None
    }

    // text conventions: added with the automation peer of the calendar (see `text_info.rs`)
    /// The text conventions (the list separator) of `locale`, or `None` when
    /// the provider has no data for it (the parent locale is asked next).
    /// The default has no data.
    fn get_text_info_by_locale(&self, locale: &Locale) -> Option<Rc<TextInfo>> {
        let _ = locale;
        None
    }

    // enumeration: added with the numeric up-down page of the control catalog (see `CultureInfo::get_cultures`)
    /// The locales of the cultures of the given types the provider has data
    /// for, in the order they are listed in (the unknown locale `und`, the
    /// invariant culture, is not among them). A provider that does not
    /// enumerate its cultures lists none, the default.
    fn get_locales(&self, types: CultureTypes) -> Vec<Locale> {
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
/// locale: the locale of its first parent that is the culture of a locale
/// (`es-ES`) and the parents of that locale are asked for.
pub(crate) fn find_culture_data<T: ?Sized>(
    culture: &CultureInfo,
    by_locale: impl Fn(&dyn ICultureDataProvider, &Locale) -> Option<Rc<T>>,
) -> Option<Rc<T>> {
    if culture.is_invariant() {
        return None;
    }
    let provider = FerroLocator::current().get_service::<dyn ICultureDataProvider>()?;
    let mut current = culture.clone();
    while !current.is_locale() {
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
        // A culture whose name is no BCP-47 tag is asked for the locale of its first parent that
        // is one.
        assert_eq!(asked("de-DE_phoneb"), (vec!["de-DE".to_owned(), "de".to_owned()], ",".to_owned()));
        // The conventions are shared, and attached ones are not asked for.
        let culture = CultureInfo::from_locale(locale!("de-CH"));
        assert!(Rc::ptr_eq(&culture.number_format(), &provider.german));
        provider.asked.borrow_mut().clear();
        let attached = culture.with_number_format(NumberFormatInfo::new());
        assert_eq!(attached.number_format().number_decimal_separator(), ".");
        assert!(provider.asked.borrow().is_empty());
    }
}
