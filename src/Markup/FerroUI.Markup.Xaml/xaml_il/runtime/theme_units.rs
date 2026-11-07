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

use std::rc::Rc;

use ferroui_base::controls::ResourceDictionary;
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

#[cfg(test)]
#[path = "theme_units_tests.rs"]
mod tests;
