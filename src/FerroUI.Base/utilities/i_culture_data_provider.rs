//! The source of culture data.
//!
//! The base library has the conventions of the invariant culture built in
//! and nothing else: the data of every other culture comes from a provider
//! registered with the service locator (by a platform layer, an application
//! or a test). Without a provider, or for a culture the provider has no data
//! for, a culture uses the data of its parent and finally of the invariant
//! culture.

use super::{CultureInfo, CultureTypes, DateTimeFormatInfo, NumberFormatInfo, TextInfo};
use crate::{FerroLocator, LocatorExtensions};
use std::rc::Rc;

/// Supplies the conventions of cultures by name. Register an implementation
/// with the locator:
///
/// ```ignore
/// FerroLocator::current_mutable().bind::<dyn ICultureDataProvider>().to_constant(provider);
/// ```
///
/// A provider is asked on every use of a culture, so it should hand out
/// shared instances it keeps.
pub trait ICultureDataProvider: 'static {
    /// The date and time conventions of the culture named exactly
    /// `culture_name` (a language tag such as `en-US`; never empty), or
    /// `None` when the provider has no data for it (the parent culture is
    /// asked next).
    fn get_date_time_format(&self, culture_name: &str) -> Option<Rc<DateTimeFormatInfo>> {
        let _ = culture_name;
        None
    }

    // number format: owned by numeric agent
    /// The number conventions of the culture named exactly `culture_name`,
    /// or `None` when the provider has no data for it (the parent culture is
    /// asked next).
    fn get_number_format(&self, culture_name: &str) -> Option<Rc<NumberFormatInfo>> {
        let _ = culture_name;
        None
    }

    // string comparison: added with the auto-complete box (see `compare_info.rs`)
    /// The culture-sensitive string comparison rules of the culture named
    /// exactly `culture_name`, or `None` when the provider has no rules for
    /// it (the parent culture is asked next, and the built-in invariant
    /// rules are used last).
    fn get_compare_rules(&self, culture_name: &str) -> Option<Rc<dyn super::ICompareRules>> {
        let _ = culture_name;
        None
    }

    // text conventions: added with the automation peer of the calendar (see `text_info.rs`)
    /// The text conventions (the list separator) of the culture named
    /// exactly `culture_name`, or `None` when the provider has no data for
    /// it (the parent culture is asked next).
    fn get_text_info(&self, culture_name: &str) -> Option<Rc<TextInfo>> {
        let _ = culture_name;
        None
    }

    // enumeration: added with the numeric up-down page of the control catalog (see `CultureInfo::get_cultures`)
    /// The names of the cultures of the given types the provider has data
    /// for, in the order they are listed in (the invariant culture, whose
    /// name is empty, is not among them). A provider that does not enumerate
    /// its cultures lists none.
    fn get_culture_names(&self, types: CultureTypes) -> Vec<String> {
        let _ = types;
        Vec::new()
    }
}

/// Finds the data of `culture`: asks the registered provider for the culture
/// and then for each of its parents; `None` when there is no provider or it
/// has nothing for any of them (the caller then uses the invariant data).
pub(crate) fn find_culture_data<T>(
    culture: &CultureInfo,
    get: impl Fn(&dyn ICultureDataProvider, &str) -> Option<Rc<T>>,
) -> Option<Rc<T>> {
    if culture.is_invariant() {
        return None;
    }
    let provider = FerroLocator::current().get_service::<dyn ICultureDataProvider>()?;
    let mut current = culture.clone();
    while !current.is_invariant() {
        if let Some(data) = get(&*provider, current.name()) {
            return Some(data);
        }
        current = current.parent();
    }
    None
}
