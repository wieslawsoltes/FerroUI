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

use super::compiled::{add_deferred_resource, DeferredBuild};
use super::XamlIlContext;
use crate::XamlLoadError;
use ferroui_base::data::core::ValueType;

/// A run of a unit: adds the entries of the unit at one place of the build to `dictionary`, in
/// order, with the context of the build at that place; the first failure is the error.
pub type ThemeUnitRun = fn(&Rc<XamlIlContext>, &Ref<ResourceDictionary>) -> Result<(), XamlLoadError>;

/// A deferred resource of a unit that an application selects by its key (#852, step 2): a static
/// of its own, which only the units that keep the key name, so that the linker removes the build
/// of a key no application table names. Added to the dictionary of the run `run` of the unit, after
/// the statements of that run.
#[derive(Clone, Copy)]
pub struct ThemeEntry {
    /// The index of the run among the runs of the unit.
    pub run: u16,
    pub key: &'static str,
    pub build: DeferredBuild,
    pub line: i32,
    pub position: i32,
}

/// A unit of a compiled theme: its name (the path of its document below the root of the
/// assembly, or `ambient`), its canonical index, its runs, in the order the build applies them,
/// and the entries selected by key (#852, step 2), ordered by run.
#[derive(Clone, Copy)]
pub struct ThemeUnit {
    pub name: &'static str,
    pub index: u32,
    pub runs: &'static [ThemeUnitRun],
    pub entries: &'static [&'static ThemeEntry],
}

impl ThemeUnit {
    /// Applies the run `run` to `dictionary`: its function, then the entries of the unit of that
    /// run. A run the unit does not have adds nothing.
    fn apply(&self, run: usize, context: &Rc<XamlIlContext>, dictionary: &Ref<ResourceDictionary>) -> Result<(), XamlLoadError> {
        let Some(apply) = self.runs.get(run) else { return Ok(()) };
        apply(context, dictionary)?;
        let start = self.entries.partition_point(|entry| usize::from(entry.run) < run);
        for entry in self.entries[start..].iter().take_while(|entry| usize::from(entry.run) == run) {
            add_deferred_resource(dictionary, entry.key, ValueType::object(), context, entry.build, entry.line, entry.position)?;
        }
        Ok(())
    }
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
    units[found].apply(run, context, dictionary)
}

/// Applies the runs of a region of a build (#853) to `dictionary`, in the order of `region`:
/// each pair is the canonical index of a unit and the index of one of its runs, and a pair whose
/// unit the table `units` does not have adds nothing. The first failure is the error and the
/// runs after it are not applied, as the calls of [`apply_theme_unit`] the region stands for.
/// `units` is in canonical order (ascending indices); a region whose units ascend, which is what
/// the emitter writes for the documents a theme merges in order, is applied in one pass over
/// both, and any other order is applied too.
#[inline(never)]
pub fn apply_theme_region(
    units: &[&ThemeUnit],
    region: &[(u16, u16)],
    context: &Rc<XamlIlContext>,
    dictionary: &Ref<ResourceDictionary>,
) -> Result<(), XamlLoadError> {
    // The position in `units` of the first unit not below the unit of the last pair.
    let mut position = 0;
    let mut last = 0;
    for &(unit, run) in region {
        let unit = u32::from(unit);
        if unit < last {
            position = 0;
        }
        last = unit;
        position += units[position..].partition_point(|known| known.index < unit);
        let Some(found) = units.get(position).filter(|found| found.index == unit) else {
            continue;
        };
        found.apply(usize::from(run), context, dictionary)?;
    }
    Ok(())
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

/// [`register_theme_parts`] unless the parts registered for the class already keep as much (#846;
/// its units, then its entries): what the generated `register()` of a crate calls. A library that constructs a theme
/// registers the table of its own documents, and the application the table it computed with the
/// references of every dependency; whichever crate registers first, the wider table stays.
pub fn register_wider_theme_parts(class: &'static str, parts: ThemeParts) {
    let mut registered = REGISTERED.write().unwrap_or_else(|error| error.into_inner());
    match registered.iter_mut().find(|(known, _)| *known == class) {
        Some(entry) if parts_width(entry.1) >= parts_width(parts) => {}
        Some(entry) => entry.1 = parts,
        None => registered.push((class, parts)),
    }
}

/// How much of a theme `parts` keeps, to compare two tables of one theme: its units, then the
/// entries selected by key in them (#852, step 2).
fn parts_width(parts: ThemeParts) -> (usize, usize) {
    (parts.len(), parts.iter().map(|unit| unit.entries.len()).sum())
}

/// The parts a compiled construction of the theme `class` applies (#846): the parts registered for
/// the class when they keep more units or entries than `own`, the table of the crate that writes the
/// construction, else `own`. A library compiled with `<WinUITheme />` thus constructs the theme with
/// the table of the application that uses it (which roots what the application's documents and
/// sources name), and with its own table when it is used alone.
pub fn wider_theme_parts(class: &str, own: ThemeParts) -> ThemeParts {
    match registered_theme_parts(class) {
        Some(registered) if parts_width(registered) > parts_width(own) => registered,
        _ => own,
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

/// What a table of units leaves out of a theme, for the diagnostic of a lookup that misses it:
/// the type keys of the control themes, and the text keys the selection by key removed (#852,
/// step 2).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TrimmedKeys {
    /// The type keys, sorted.
    pub types: Vec<&'static str>,
    /// The keys whose entries a kept unit of the table does not name, sorted.
    pub texts: Vec<&'static str>,
}

/// What `parts` leaves out of a theme. `type_keys` holds the type keys of every unit, by its index,
/// and `entry_keys` the keys of each unit an application selects (the statics the generated file
/// has in debug builds). The text keys are only those the selection removed from a unit `parts`
/// keeps. A key of a unit left out whole is not among them: a control may probe a resource of a
/// unit its application does not keep and do without it, and that is no diagnostic.
pub fn trimmed_keys(
    type_keys: &'static [&'static [&'static str]],
    entry_keys: &'static [&'static [&'static str]],
    parts: &[&ThemeUnit],
) -> TrimmedKeys {
    let mut texts: Vec<&'static str> = parts
        .iter()
        .flat_map(|unit| {
            let selectable = entry_keys.get(unit.index as usize).copied().unwrap_or_default();
            selectable.iter().copied().filter(|key| !unit.entries.iter().any(|entry| entry.key == *key))
        })
        .collect();
    texts.sort_unstable();
    texts.dedup();
    TrimmedKeys { types: trimmed_type_keys(type_keys, parts), texts }
}

ferroui_base::ferro_thread_local! {
    /// The trimmed control themes and resources reported so far, each with whether it is a type.
    static REPORTED: RefCell<Vec<(bool, &'static str)>> = const { RefCell::new(Vec::new()) };
}

/// Whether `found` (a type key, or a text key) is reported for the first time.
fn first_report(is_type: bool, found: &'static str) -> bool {
    REPORTED.with(|reported| {
        let mut reported = reported.borrow_mut();
        let first = !reported.contains(&(is_type, found));
        if first {
            reported.push((is_type, found));
        }
        first
    })
}

/// The diagnostic of a lookup that missed: when `key` is the type key of a control whose theme
/// is in `trimmed`, or a text key of a resource in it (#852, step 2), logs once, as a warning of
/// the framework logger, `the <theme> control theme of <Type> was trimmed: add it to theme-roots`
/// or `the <theme> resource <key> was trimmed: add it to theme-roots`. Returns whether it is such
/// a key. The themes call it in debug builds only; release builds carry no names.
pub fn report_trimmed_theme(theme: &str, key: &ResourceKey, trimmed: &TrimmedKeys) -> bool {
    let (is_type, name, list) = match key {
        ResourceKey::Type(type_) => (true, type_.name(), &trimmed.types),
        ResourceKey::String(text) => (false, &**text, &trimmed.texts),
        ResourceKey::Object(_) => return false,
    };
    let Some(found) = list.iter().copied().find(|known| *known == name) else {
        return false;
    };
    if first_report(is_type, found) {
        if let Some(log) = Logger::try_get(LogEventLevel::Warning, LogArea::CONTROL) {
            match is_type {
                true => log.log_with_values(None, "the {Theme} control theme of {Type} was trimmed: add it to theme-roots", &[&theme, &found]),
                false => log.log_with_values(None, "the {Theme} resource {Key} was trimmed: add it to theme-roots", &[&theme, &found]),
            }
        }
    }
    true
}

#[cfg(test)]
#[path = "theme_units_tests.rs"]
mod tests;
