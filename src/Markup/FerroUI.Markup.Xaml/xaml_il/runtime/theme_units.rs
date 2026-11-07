//! The units of a compiled theme (#568, step 2 of docs/porting/theme-trimming.md): what
//! generated code calls, through `rt`, to apply the entries of one document of a theme to the
//! dictionary they belong to.
//!
//! Not a port of an upstream file. The emitter partitions the compiled markup of a class whose
//! documents merge other documents (`MergeResourceInclude`) into units: one unit per merged
//! document, and the ambient unit (index 0) for the content that is always kept. The entries a
//! unit adds to one dictionary at one place of the build are a run: a function of their own
//! ([`ThemeUnitRun`]) that only the [`ThemeUnit`] of the unit refers to. The function that builds
//! the dictionary calls [`apply_theme_unit`] where the entries were, with the table of the units
//! the build applies, so the entries are added at the same place and in the same order as before.
//!
//! The generated file holds one static `ThemeUnit` per unit and the static table of all units
//! (`THEME_UNITS`), in canonical order. Applying every unit through that table builds the same
//! dictionaries as the theme built before it was partitioned. A table that leaves a unit out
//! leaves out its entries, and the linker then removes the run functions of the unit and
//! everything only they reach.
//!
//! Step 3 chooses the table at run time. The builds apply the units of the table that the last
//! populate of the class made active ([`set_active_theme_units`], [`active_theme_units`]): the
//! whole table for the constructors of the full theme, the parts an application passes to
//! `with_parts`. Only the constructors of the full theme name the table of all units, so an
//! application that constructs the theme from its parts links only those. A load by URI and the
//! constructors of the type information apply the parts the application registered
//! ([`register_theme_parts`], [`registered_theme_parts`]).

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::RwLock;

use ferroui_base::controls::{ResourceDictionary, ResourceKey};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::Ref;

use super::XamlIlContext;
use crate::XamlLoadError;

/// A run of a unit: adds the entries of the unit at one place of the build to `dictionary`, in
/// order, with the context of the build at that place; the first failure is the error.
pub type ThemeUnitRun = fn(&Rc<XamlIlContext>, &Ref<ResourceDictionary>) -> Result<(), XamlLoadError>;

/// A unit of a compiled theme: its name (the path of its document below the root of the
/// assembly, or `ambient`), its canonical index and its runs, in the order the build applies
/// them.
#[derive(Clone, Copy)]
pub struct ThemeUnit {
    pub name: &'static str,
    pub index: u32,
    pub runs: &'static [ThemeUnitRun],
}

impl std::fmt::Debug for ThemeUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ThemeUnit").field("name", &self.name).field("index", &self.index).field("runs", &self.runs.len()).finish()
    }
}

/// Applies the run `run` of the unit with the canonical index `index` to `dictionary`, when the
/// table `units` has the unit; a table without it adds nothing. `units` is in canonical order
/// (ascending indices).
#[inline(never)]
pub fn apply_theme_unit(
    units: &[&ThemeUnit],
    index: u32,
    run: usize,
    context: &Rc<XamlIlContext>,
    dictionary: &Ref<ResourceDictionary>,
) -> Result<(), XamlLoadError> {
    let Ok(found) = units.binary_search_by_key(&index, |unit| unit.index) else {
        return Ok(());
    };
    match units[found].runs.get(run) {
        Some(apply) => apply(context, dictionary),
        None => Ok(()),
    }
}

/// The unit named `name` of the table `units`.
pub fn find_theme_unit<'u>(units: &[&'u ThemeUnit], name: &str) -> Option<&'u ThemeUnit> {
    units.iter().copied().find(|unit| unit.name == name)
}

/// A table of units: the units of one theme, in canonical order.
pub type ThemeParts = &'static [&'static ThemeUnit];

ferroui_base::ferro_thread_local! {
    /// The table the builds of each class apply, by the full name of the class: the table of the
    /// last populate of the class.
    static ACTIVE: RefCell<Vec<(&'static str, ThemeParts)>> = const { RefCell::new(Vec::new()) };
}

/// The parts each application registered, by the full name of the class. Process-wide: an
/// application registers them once, at start-up, from the thread it starts on.
static REGISTERED: RwLock<Vec<(&'static str, ThemeParts)>> = RwLock::new(Vec::new());

/// Makes `parts` the table the builds of the class `class` (its full name) apply, until the next
/// populate of the class. Called by the populate of the class before it builds anything; the
/// table stays active afterwards, for content that is built later.
pub fn set_active_theme_units(class: &'static str, parts: ThemeParts) {
    ACTIVE.with(|active| {
        let mut active = active.borrow_mut();
        match active.iter_mut().find(|(known, _)| *known == class) {
            Some(entry) => entry.1 = parts,
            None => active.push((class, parts)),
        }
    });
}

/// The table the builds of the class `class` apply: the one the last populate of the class made
/// active, or no unit before the first populate.
pub fn active_theme_units(class: &str) -> ThemeParts {
    ACTIVE.with(|active| active.borrow().iter().find(|(known, _)| *known == class).map(|(_, parts)| *parts).unwrap_or(&[]))
}

/// Registers `parts` as the units of the theme `class` (its full name) that the application
/// keeps: the table a load of the theme by URI and the constructors of the type information
/// apply. Replaces an earlier registration. Called by the generated `register()` of an
/// application that trims its themes (the default; `theme-trimming = false` in
/// `[package.metadata.ferroui]` turns it off).
pub fn register_theme_parts(class: &'static str, parts: ThemeParts) {
    let mut registered = REGISTERED.write().unwrap_or_else(|error| error.into_inner());
    match registered.iter_mut().find(|(known, _)| *known == class) {
        Some(entry) => entry.1 = parts,
        None => registered.push((class, parts)),
    }
}

/// [`register_theme_parts`] when nothing is registered for the class yet. The constructors of
/// the full theme call it with the table of all units, so that a later load by URI gets the
/// full theme, as before the units existed.
pub fn register_theme_parts_if_absent(class: &'static str, parts: ThemeParts) {
    let mut registered = REGISTERED.write().unwrap_or_else(|error| error.into_inner());
    if !registered.iter().any(|(known, _)| *known == class) {
        registered.push((class, parts));
    }
}

/// The parts registered for the theme `class`, if any.
pub fn registered_theme_parts(class: &str) -> Option<ThemeParts> {
    let registered = REGISTERED.read().unwrap_or_else(|error| error.into_inner());
    registered.iter().find(|(known, _)| *known == class).map(|(_, parts)| *parts)
}

/// The type keys that `parts` leaves out: the type keys of each unit whose canonical index is
/// not in `parts`. `type_keys` holds the type keys of every unit, by its index (the static the
/// generated file has in debug builds). For the diagnostic of a trimmed control theme.
pub fn trimmed_type_keys(type_keys: &'static [&'static [&'static str]], parts: &[&ThemeUnit]) -> Vec<&'static str> {
    let mut trimmed = Vec::new();
    for (index, keys) in type_keys.iter().enumerate() {
        if parts.binary_search_by_key(&(index as u32), |unit| unit.index).is_err() {
            trimmed.extend(keys.iter().copied());
        }
    }
    trimmed.sort_unstable();
    trimmed.dedup();
    trimmed
}

ferroui_base::ferro_thread_local! {
    /// The trimmed control themes reported so far.
    static REPORTED: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
}

/// The diagnostic of a lookup that missed: when `key` is the type key of a control whose theme
/// is in `trimmed` (the type keys the application left out), logs once, as a warning of the
/// framework logger, `the <theme> control theme of <Type> was trimmed: add it to theme-roots`.
/// Returns whether it is such a key. The themes call it in debug builds only; release builds
/// carry no names.
pub fn report_trimmed_theme(theme: &str, key: &ResourceKey, trimmed: &[&'static str]) -> bool {
    let ResourceKey::Type(type_) = key else {
        return false;
    };
    let name = type_.name();
    let Some(found) = trimmed.iter().copied().find(|trimmed| *trimmed == name) else {
        return false;
    };
    let first = REPORTED.with(|reported| {
        let mut reported = reported.borrow_mut();
        let first = !reported.contains(&found);
        if first {
            reported.push(found);
        }
        first
    });
    if first {
        if let Some(log) = Logger::try_get(LogEventLevel::Warning, LogArea::CONTROL) {
            log.log_with_values(None, "the {Theme} control theme of {Type} was trimmed: add it to theme-roots", &[&theme, &name]);
        }
    }
    true
}

#[cfg(test)]
#[path = "theme_units_tests.rs"]
mod tests;
