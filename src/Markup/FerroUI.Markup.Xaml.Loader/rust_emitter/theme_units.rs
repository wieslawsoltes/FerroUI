//! The runs of the units of a compiled theme (#568, step 2 of docs/porting/theme-trimming.md):
//! the entries of a unit, taken out of the function that builds their dictionary into functions
//! that only the table of the units refers to.
//!
//! The units and their documents are planned before the documents are emitted
//! ([`super::super::theme_unit_graph`]). While a function of a document is written, every entry
//! of a dictionary that a merge inlined from a document of a unit ([`UnitEmission::unit_of`]) is
//! written between two marker comments: [`begin_marker`] with the index of the unit, and
//! [`END_MARKER`]. Only the entries of the body of the function are marked, never those of the
//! body of deferred content, which belong to the entry the content is of. The runs of rows of
//! theme resources end where the unit changes ([`super::theme_rows`]).
//!
//! When the body is complete, [`extract_runs`] takes each run (consecutive marked entries of one
//! unit added to one dictionary) out of it into a function of its own,
//! `<function>_unit_<n>(context, dictionary)`, and writes a call of `rt::apply_theme_unit` with
//! the active table of the class in its place ([`ACTIVE`]: the table of all units for the full
//! theme, the parts of an application for `populate_with_parts`, step 3). The entries are added where they were, in the same
//! order, with the same context, so the dictionaries are the same. An entry stays in the body,
//! unmarked, when it is not one or more complete statements at the top level of the body, when
//! it names a local of the body other than the context and one dictionary, or when a local it
//! declares is named after it. The markers never reach the generated file.
//!
//! **The plan of the runs (#853), an experiment that is off by default.** [`ThemeUnitPlan`]
//! has two switches. With `keys_in_runs`, the key that a merged theme dictionary assigns
//! again to the theme dictionary it was merged into (`<ResourceDictionary x:Key="Default">` of
//! every `*_themeresources.xaml`) is an entry of the unit of its document, when the build has
//! assigned the dictionary a key outside every unit before: the merge joins only dictionaries
//! whose keys are equal, so the assignment writes the value the dictionary has, and a unit that
//! is trimmed takes it with it instead of leaving it in the function of the theme. With
//! `regions`, consecutive calls of runs that add to one dictionary, with only comments between
//! them, are one region: one call of `rt::apply_theme_region` with the units and runs of the
//! region in their order, instead of one call of `rt::apply_theme_unit` each. A region applies
//! the same runs in the same order and stops at the same failure. [`DEFAULT_THEME_UNIT_PLAN`]
//! is what the build and the reference tests of the themes use; [`with_theme_unit_plan`]
//! chooses another one for the emission of a closure.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use ferroui_build_scan::xaml_metadata::{ThemeEntryModel, ThemeUnitModel};
use xamlx::ast::IXamlAstNode;

use crate::compiler_extensions::group_transformers::MergedEntryOrigins;

#[path = "theme_run_program.rs"]
mod bytecode;

use super::{brace_balance, declared_local, identifiers};

/// The start of the marker that opens an entry of a unit; the index of the unit follows. A
/// comment, so the runs of rows step over it, with a control character no position marker has.
const BEGIN: &str = "//\u{1}unit ";

/// The marker that closes an entry of a unit.
pub(crate) const END_MARKER: &str = "//\u{1}end";

/// The name the generated file gives the static table of all units.
pub(crate) const TABLE: &str = "THEME_UNITS";

/// The function of the generated file that gives the table the builds apply the units through:
/// the active table of the class (#568, step 3).
pub(crate) const ACTIVE: &str = "active_theme_units";

/// The function of the runtime that applies a region of runs (#853).
const APPLY_REGION: &str = "rt::apply_theme_region";

/// The experiment of #853: how the runs of the units of a theme are applied. Both switches are
/// off by default ([`DEFAULT_THEME_UNIT_PLAN`]), and then the generated files are what they were
/// before the experiment, byte for byte.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ThemeUnitPlan {
    regions: bool,
    keys_in_runs: bool,
    entries: bool,
}

impl ThemeUnitPlan {
    /// Both switches off: one call of `rt::apply_theme_unit` for each run, and the key
    /// assignments of merged theme dictionaries in the function of the document.
    pub const OFF: Self = Self { regions: false, keys_in_runs: false, entries: false };

    /// The two switches of #853 on.
    pub const ON: Self = Self { regions: true, keys_in_runs: true, entries: false };

    /// The deferred resources of a run written as entries an application selects by key (#852,
    /// step 2): statics of their own (`THEME_ENTRY_<n>`), which the unit names, instead of
    /// statements of the run.
    pub const fn with_entries(self, on: bool) -> Self {
        Self { entries: on, ..self }
    }

    /// Whether the deferred resources of the runs are entries selected by key.
    pub const fn entries(self) -> bool {
        self.entries
    }

    /// Consecutive calls of runs that add to one dictionary written as one call of
    /// `rt::apply_theme_region`.
    pub const fn with_regions(self, on: bool) -> Self {
        Self { regions: on, ..self }
    }

    /// The key a merged theme dictionary assigns again to the dictionary it was merged into
    /// written as an entry of the unit of its document.
    pub const fn with_keys_in_runs(self, on: bool) -> Self {
        Self { keys_in_runs: on, ..self }
    }

    /// Whether regions are written.
    pub const fn regions(self) -> bool {
        self.regions
    }

    /// Whether the key assignments of merged theme dictionaries are entries of their units.
    pub const fn keys_in_runs(self) -> bool {
        self.keys_in_runs
    }

    /// The plan of the emission on this thread: the one [`with_theme_unit_plan`] chose, else
    /// [`DEFAULT_THEME_UNIT_PLAN`].
    pub fn current() -> Self {
        PLAN.with(|plan| plan.get()).unwrap_or(DEFAULT_THEME_UNIT_PLAN)
    }
}

/// The plan the build and the reference tests of the themes emit with. The experiment of #853
/// is turned on here (`ThemeUnitPlan::ON`); the checked-in generated files of the themes and of
/// the emitter tests are then regenerated.
pub const DEFAULT_THEME_UNIT_PLAN: ThemeUnitPlan = ThemeUnitPlan::OFF;

ferroui_base::ferro_thread_local! {
    /// The plan [`with_theme_unit_plan`] chose for the emission on this thread.
    static PLAN: Cell<Option<ThemeUnitPlan>> = const { Cell::new(None) };
}

/// Runs `emit` with `plan` as the plan of the runs of theme units (#853) on this thread, and
/// restores the plan before it afterwards, also when `emit` panics.
pub fn with_theme_unit_plan<R>(plan: ThemeUnitPlan, emit: impl FnOnce() -> R) -> R {
    struct Restore(Option<ThemeUnitPlan>);
    impl Drop for Restore {
        fn drop(&mut self) {
            let before = self.0;
            PLAN.with(|plan| plan.set(before));
        }
    }
    let _restore = Restore(PLAN.with(|current| current.replace(Some(plan))));
    emit()
}

/// The marker that opens an entry of the unit `unit`, indented as a statement of a body.
pub(crate) fn begin_marker(unit: u32) -> String {
    format!("    {BEGIN}{unit}")
}

/// What the line is, if it is a marker: `Some(Some(unit))` for the opening marker of an entry
/// of `unit`, `Some(None)` for the closing marker.
pub(crate) fn unit_marker(line: &str) -> Option<Option<u32>> {
    let text = line.trim_start();
    if text == END_MARKER {
        return Some(None);
    }
    text.strip_prefix(BEGIN).and_then(|unit| unit.parse::<u32>().ok()).map(Some)
}

/// The units of the class file being emitted: where its entries come from, the unit of each
/// merged document, and the runs written so far.
pub(crate) struct UnitEmission {
    origins: Rc<MergedEntryOrigins>,
    unit_of_uri: HashMap<String, u32>,
    names: Vec<String>,
    /// The functions of the runs of each unit, by its index, in the order they are applied.
    runs: RefCell<Vec<Vec<String>>>,
    /// The deferred resources written as entries selected by key (#852, step 2): the index of an
    /// entry is the number of its static, `THEME_ENTRY_<n>`.
    entries: RefCell<Vec<UnitEntry>>,
}

/// A deferred resource of a run written as an entry selected by key (#852, step 2).
struct UnitEntry {
    unit: u32,
    run: usize,
    key: String,
    /// The key as the Rust literal the statement had.
    literal: String,
    build: String,
    position: (i32, i32),
}

/// `lines`, the statements of a run that adds to `dictionary`, without the deferred resources that
/// can be selected by key, and those resources as `(key, literal, build, position)`. A resource is
/// selectable when its statement is the only one of the run that names its key, so that adding it
/// after the other statements of the run adds the same entries. The comment of its position
/// goes with it.
fn selectable_entries(lines: Vec<String>, dictionary: &str) -> (Vec<String>, Vec<(String, String, String, (i32, i32))>) {
    let code: Vec<&str> = lines.iter().map(|line| line.trim_start()).filter(|text| !text.starts_with("//")).collect();
    let single = |literal: &str| code.iter().filter(|text| text.contains(literal)).count() == 1;
    let mut found = Vec::new();
    let mut kept: Vec<String> = Vec::with_capacity(lines.len());
    for line in &lines {
        let selectable = super::theme_rows::deferred_addition(line.trim_start())
            .filter(|addition| addition.dictionary == dictionary && single(addition.key))
            .and_then(|addition| {
                let key = super::theme_rows::string_value(addition.key)?;
                Some((key, addition.key.to_string(), addition.build.to_string(), addition.position))
            });
        match selectable {
            Some(entry) => {
                while kept.last().is_some_and(|last| last.trim_start().starts_with("//")) {
                    kept.pop();
                }
                found.push(entry);
            }
            None => kept.push(line.clone()),
        }
    }
    (kept, found)
}

impl UnitEmission {
    /// `names` are the names of the units, by their indices.
    pub(crate) fn new(origins: Rc<MergedEntryOrigins>, unit_of_uri: HashMap<String, u32>, names: Vec<String>) -> Self {
        let runs = RefCell::new(vec![Vec::new(); names.len()]);
        Self { origins, unit_of_uri, names, runs, entries: RefCell::new(Vec::new()) }
    }

    /// States the number of the static of each entry on the entries of the graph of its unit
    /// (#852, step 2), which an application selects by. An entry of a key the graph of its unit
    /// does not list is added to it with everything the unit names, so selecting it keeps what
    /// it may need.
    pub(crate) fn record_slots(&self, units: &mut [ThemeUnitModel]) {
        for (slot, entry) in self.entries.borrow().iter().enumerate() {
            let Some(unit) = units.get_mut(entry.unit as usize) else { continue };
            let position = match unit.entries.binary_search_by(|known| known.key.as_str().cmp(&entry.key)) {
                Ok(position) => position,
                Err(position) => {
                    let added = ThemeEntryModel {
                        key: entry.key.clone(),
                        references: unit.references.clone(),
                        types: unit.types.clone(),
                        slots: Vec::new(),
                    };
                    unit.entries.insert(position, added);
                    position
                }
            };
            unit.entries[position].slots.push(slot as u32);
        }
    }

    /// The unit of the entry `node`: the unit of the document a merge inlined it from.
    pub(crate) fn unit_of(&self, node: &Rc<dyn IXamlAstNode>) -> Option<u32> {
        let uri = self.origins.origin_of(node)?;
        self.unit_of_uri.get(&uri).copied()
    }

    /// Records the run `function` of `unit`; returns its index among the runs of the unit.
    fn push_run(&self, unit: u32, function: String) -> usize {
        let mut runs = self.runs.borrow_mut();
        let of_unit = &mut runs[unit as usize];
        of_unit.push(function);
        of_unit.len() - 1
    }

    /// The functions of the runs of each unit, by its index.
    pub(crate) fn runs(&self) -> Vec<Vec<String>> {
        self.runs.borrow().clone()
    }

    /// The statics of the units and the tables of the theme, as items of the generated file:
    /// `THEME_UNIT_<index>` for each unit (public, so that the parts table of an application
    /// names them), `THEME_UNITS`, `AMBIENT_THEME_UNITS`, `THEME_CLASS`, the active table
    /// ([`ACTIVE`]), `registered_theme_units` and, in debug builds, `THEME_UNIT_TYPE_KEYS`.
    /// `class` is the full name of the class; `type_keys` are the type keys of each unit, by
    /// its index.
    pub(crate) fn table(&self, class: &str, type_keys: &[Vec<String>], text_keys: &[Vec<String>]) -> String {
        let runs = self.runs.borrow();
        let entries = self.entries.borrow();
        let mut text = String::new();
        for (index, name) in self.names.iter().enumerate() {
            let name_text = name.replace('`', "'").replace(['\r', '\n'], " ");
            let of_unit: Vec<(usize, &UnitEntry)> =
                entries.iter().enumerate().filter(|(_, entry)| entry.unit as usize == index).collect();
            for (slot, entry) in &of_unit {
                text.push_str(&format!(
                    "/// A resource of the unit `{name_text}` an application selects by its key (#852).\n\
                     pub static THEME_ENTRY_{slot}: rt::ThemeEntry = rt::ThemeEntry {{ run: {}, key: {}, build: {}, line: {}, position: {} }};\n\n",
                    entry.run, entry.literal, entry.build, entry.position.0, entry.position.1
                ));
            }
            let (runs_operand, entries_operand) = match of_unit.is_empty() {
                true => (format!("&[{}]", runs[index].join(", ")), "&[]".to_string()),
                false => {
                    text.push_str(&format!(
                        "/// The runs of the unit `{name_text}`, for an application that keeps some of its entries (#852).\n\
                         pub static THEME_UNIT_{index}_RUNS: [rt::ThemeUnitRun; {}] = [{}];\n\n",
                        runs[index].len(),
                        runs[index].join(", ")
                    ));
                    let named: Vec<String> = of_unit.iter().map(|(slot, _)| format!("&THEME_ENTRY_{slot}")).collect();
                    (format!("&THEME_UNIT_{index}_RUNS"), format!("&[{}]", named.join(", ")))
                }
            };
            text.push_str(&format!("/// The unit `{name_text}` of the theme (#568): its runs, in the order the builds apply them.\n"));
            text.push_str(&format!(
                "pub static THEME_UNIT_{index}: rt::ThemeUnit = rt::ThemeUnit {{ name: {}, index: {index}, runs: {runs_operand}, entries: {entries_operand} }};\n\n",
                super::rust_string_literal(name),
            ));
        }
        text.push_str("/// Every unit of the theme, in canonical order: the full theme (#568). Only the constructors\n");
        text.push_str("/// of the full theme name it, so that an application that keeps some units links only those.\n");
        let all: Vec<String> = (0..self.names.len()).map(|index| format!("&THEME_UNIT_{index}")).collect();
        text.push_str(&format!("pub static {TABLE}: &[&rt::ThemeUnit] = &[{}];\n", all.join(", ")));
        text.push('\n');
        text.push_str("/// The ambient unit alone: what a load by URI applies when the application registered no\n");
        text.push_str("/// parts and constructed no full theme (#568, step 3).\n");
        text.push_str("pub static AMBIENT_THEME_UNITS: &[&rt::ThemeUnit] = &[&THEME_UNIT_0];\n");
        text.push('\n');
        text.push_str("/// The full name of the class of the theme: the name its parts are registered and made\n");
        text.push_str("/// active under (#568, step 3).\n");
        text.push_str(&format!("pub const THEME_CLASS: &str = {};\n", super::rust_string_literal(class)));
        text.push('\n');
        text.push_str("/// The table the builds of the dictionaries apply the units through: the one the last\n");
        text.push_str("/// populate of the class made active (#568, step 3).\n");
        text.push_str(&format!("fn {ACTIVE}() -> rt::ThemeParts {{\n"));
        text.push_str("    rt::active_theme_units(THEME_CLASS)\n");
        text.push_str("}\n");
        text.push('\n');
        text.push_str("/// The units a load by URI and the constructors of the type information apply (#568, step 3):\n");
        text.push_str("/// every unit, since the build links what can show any control (`ferroui-base/full-themes`:\n");
        text.push_str("/// the run-time loader, hot reload, the designer, DevTools, Inspect).\n");
        text.push_str("#[cfg(ferroui_full_themes)]\n");
        text.push_str("pub fn registered_theme_units() -> rt::ThemeParts {\n");
        text.push_str(&format!("    {TABLE}\n"));
        text.push_str("}\n");
        text.push('\n');
        text.push_str("/// The units a load by URI and the constructors of the type information apply (#568, step 3):\n");
        text.push_str("/// the parts the application registered, else the ambient unit.\n");
        text.push_str("#[cfg(not(ferroui_full_themes))]\n");
        text.push_str("pub fn registered_theme_units() -> rt::ThemeParts {\n");
        text.push_str("    rt::registered_theme_parts(THEME_CLASS).unwrap_or(AMBIENT_THEME_UNITS)\n");
        text.push_str("}\n");
        text.push('\n');
        // What a lookup that misses reports in a debug build (#852, step 2).
        let table_of = |of_unit: &dyn Fn(usize) -> Vec<String>| -> String {
            let units: Vec<String> = (0..self.names.len())
                .map(|index| {
                    let names: Vec<String> = of_unit(index).iter().map(|key| super::rust_string_literal(key)).collect();
                    format!("&[{}]", names.join(", "))
                })
                .collect();
            units.join(", ")
        };
        let of_text = |index: usize| text_keys.get(index).cloned().unwrap_or_default();
        let of_entries = |index: usize| -> Vec<String> {
            let mut keys: Vec<String> =
                entries.iter().filter(|entry| entry.unit as usize == index).map(|entry| entry.key.clone()).collect();
            keys.sort();
            keys.dedup();
            keys
        };
        text.push_str("/// The text keys of each unit, by its index: the names of the diagnostic of a trimmed resource\n");
        text.push_str("/// (#852). Debug builds only: release builds carry no names.\n");
        text.push_str("#[cfg(debug_assertions)]\n");
        text.push_str(&format!("pub static THEME_UNIT_TEXT_KEYS: &[&[&str]] = &[{}];\n\n", table_of(&of_text)));
        text.push_str("/// The keys of each unit an application selects by key (#852). Debug builds only.\n");
        text.push_str("#[cfg(debug_assertions)]\n");
        text.push_str(&format!("pub static THEME_UNIT_ENTRY_KEYS: &[&[&str]] = &[{}];\n\n", table_of(&of_entries)));
        text.push_str("/// What the table `parts` leaves out of the theme, for the diagnostic of a lookup that misses\n");
        text.push_str("/// it (#852). Debug builds only.\n");
        text.push_str("#[cfg(debug_assertions)]\n");
        text.push_str("pub fn trimmed_theme_keys(parts: rt::ThemeParts) -> rt::TrimmedKeys {\n");
        text.push_str("    rt::trimmed_keys(THEME_UNIT_TYPE_KEYS, THEME_UNIT_TEXT_KEYS, THEME_UNIT_ENTRY_KEYS, parts)\n");
        text.push_str("}\n\n");
        text.push_str("/// The type keys of each unit, by its index: the names of the diagnostic of a trimmed control\n");
        text.push_str("/// theme (#568, step 3). Debug builds only: release builds carry no names.\n");
        text.push_str("#[cfg(debug_assertions)]\n");
        let keys: Vec<String> = (0..self.names.len())
            .map(|index| {
                let names: Vec<String> = type_keys.get(index).map(Vec::as_slice).unwrap_or_default().iter().map(|key| super::rust_string_literal(key)).collect();
                format!("&[{}]", names.join(", "))
            })
            .collect();
        text.push_str(&format!("pub static THEME_UNIT_TYPE_KEYS: &[&[&str]] = &[{}];\n", keys.join(", ")));
        text
    }
}

/// A part of a body between markers: the lines of one entry of `unit`.
struct Segment {
    unit: u32,
    lines: Vec<String>,
}

/// A run being collected: consecutive entries of one unit added to one dictionary.
struct Run {
    unit: u32,
    dictionary: String,
    lines: Vec<String>,
}

/// Only row tables actually emitted for this body can become schedule imports.
pub(super) struct Schedule<'a> {
    pub(super) image: &'a super::BytecodeImage,
    pub(super) document: &'a str,
    pub(super) rows: &'a HashMap<String, super::theme_rows::RowKind>,
}

/// `lines`, the complete body of the function `function_name` (of `document_text`), with each
/// run of marked entries taken out into a function of its own (see the module documentation).
/// `local_types` are the Rust types of the locals of the body the emitter states;
/// `dictionary_type` is the Rust type of a local that holds a `ResourceDictionary`. Returns the
/// body, without markers, and the functions of the runs.
pub(super) fn extract_runs(
    function_name: &str,
    document_text: &str,
    lines: Vec<String>,
    local_types: &HashMap<String, String>,
    dictionary_type: &str,
    units: &UnitEmission,
    bytecode: Option<Schedule<'_>>,
) -> (Vec<String>, Vec<String>) {
    if !lines.iter().any(|line| unit_marker(line).is_some()) {
        return (lines, Vec::new());
    }
    // The last line that names each identifier.
    let mut last_use: HashMap<String, usize> = HashMap::new();
    for (index, line) in lines.iter().enumerate() {
        let text = line.trim_start();
        if !text.starts_with("//") {
            for name in identifiers(text) {
                last_use.insert(name, index);
            }
        }
    }

    let mut extractor = Extractor {
        function_name,
        document_text,
        dictionary_type,
        bytecode,
        local_types,
        units,
        declared: ["context", "name_scope", "root", "service_provider"].iter().map(|name| name.to_string()).collect(),
        body: Vec::with_capacity(lines.len()),
        functions: Vec::new(),
        run: None,
        pending_comments: Vec::new(),
        calls: Vec::new(),
    };
    let mut depth: i64 = 0;
    let mut index = 0;
    while index < lines.len() {
        let line = &lines[index];
        match unit_marker(line) {
            Some(Some(unit)) if depth == 0 => {
                // The entry: the lines up to its closing marker.
                let end = (index + 1..lines.len()).find(|&next| unit_marker(&lines[next]).is_some());
                match end {
                    Some(end) if unit_marker(&lines[end]) == Some(None) => {
                        let segment = Segment { unit, lines: lines[index + 1..end].to_vec() };
                        // An entry that stays in the body keeps the blocks it opens or closes.
                        depth += segment
                            .lines
                            .iter()
                            .map(|line| line.trim_start())
                            .filter(|text| !text.starts_with("//"))
                            .map(brace_balance)
                            .sum::<i64>();
                        extractor.segment(segment, |name| last_use.get(name).is_some_and(|last| *last >= end));
                        index = end + 1;
                    }
                    // Not closed where expected: the lines stay in the body.
                    _ => index += 1,
                }
            }
            // A marker inside a block, or a closing marker of an entry that stays: dropped.
            Some(_) => index += 1,
            None => {
                let text = line.trim_start();
                if !text.starts_with("//") {
                    depth += brace_balance(text);
                }
                extractor.host_line(line.clone());
                index += 1;
            }
        }
    }
    extractor.finish()
}

struct Extractor<'e> {
    function_name: &'e str,
    document_text: &'e str,
    dictionary_type: &'e str,
    bytecode: Option<Schedule<'e>>,
    local_types: &'e HashMap<String, String>,
    units: &'e UnitEmission,
    /// The locals declared in the body so far.
    declared: HashSet<String>,
    body: Vec<String>,
    functions: Vec<String>,
    run: Option<Run>,
    /// The comments of the body since the last run, which belong to what follows them.
    pending_comments: Vec<String>,
    /// The calls of runs written into the body: their lines, units, runs and dictionaries.
    calls: Vec<Call>,
}

/// A call of a run the body has: the index of its line in the body, the unit, the index of the
/// run among the runs of the unit, and the local of the dictionary it adds to.
struct Call {
    line: usize,
    unit: u32,
    run: usize,
    dictionary: String,
}

impl Extractor<'_> {
    /// A line of the body that is not part of an entry of a unit.
    fn host_line(&mut self, line: String) {
        let text = line.trim_start();
        if text.starts_with("//") {
            self.pending_comments.push(line);
            return;
        }
        self.flush();
        if let Some((name, _)) = declared_local(text) {
            self.declared.insert(name);
        }
        self.body.append(&mut self.pending_comments);
        self.body.push(line);
    }

    /// An entry of a unit: added to the current run, or the start of a new one, or kept in the
    /// body when it cannot be a run. `used_later` says whether a local is named after the entry.
    fn segment(&mut self, segment: Segment, used_later: impl Fn(&str) -> bool) {
        let Some(dictionary) = self.dictionary_of(&segment, used_later) else {
            // Kept in the body, as it was written.
            self.flush();
            self.body.append(&mut self.pending_comments);
            for line in segment.lines {
                let text = line.trim_start();
                if !text.starts_with("//") {
                    if let Some((name, _)) = declared_local(text) {
                        self.declared.insert(name);
                    }
                }
                self.body.push(line);
            }
            return;
        };
        let continues = self.run.as_ref().is_some_and(|run| run.unit == segment.unit && run.dictionary == dictionary);
        if !continues {
            self.flush();
            self.body.append(&mut self.pending_comments);
            self.run = Some(Run { unit: segment.unit, dictionary, lines: Vec::new() });
        }
        let run = self.run.as_mut().expect("a run");
        run.lines.append(&mut self.pending_comments);
        run.lines.extend(segment.lines);
    }

    /// The dictionary the entry adds to, when it can be a run: complete statements at the top
    /// level, naming no local of the body but the context and that dictionary, and declaring no
    /// local that is named after it.
    fn dictionary_of(&self, segment: &Segment, used_later: impl Fn(&str) -> bool) -> Option<String> {
        let mut depth: i64 = 0;
        let mut inner: HashSet<String> = HashSet::new();
        let mut dictionary: Option<String> = None;
        let mut has_code = false;
        for line in &segment.lines {
            let text = line.trim_start();
            if text.starts_with("//") {
                continue;
            }
            has_code = true;
            if depth == 0 {
                if let Some((name, _)) = declared_local(text) {
                    if used_later(&name) {
                        return None;
                    }
                    inner.insert(name);
                }
            }
            depth += brace_balance(text);
            if depth < 0 {
                return None;
            }
            for name in identifiers(text) {
                if inner.contains(&name) || !self.declared.contains(&name) || name == "context" {
                    continue;
                }
                if self.local_types.get(&name).map(String::as_str) != Some(self.dictionary_type) {
                    return None;
                }
                match &dictionary {
                    Some(known) if *known != name => return None,
                    Some(_) => {}
                    None => dictionary = Some(name),
                }
            }
        }
        (has_code && depth == 0).then_some(dictionary).flatten()
    }

    /// Ends the current run: its function, and the call of it in the body.
    fn flush(&mut self) {
        let Some(run) = self.run.take() else { return };
        let function = format!("{}_unit_{}", self.function_name, self.functions.len());
        let schedule = self.bytecode.as_ref().and_then(|config| bytecode::compile(&run.lines, &run.dictionary, config));
        let index = self.units.push_run(run.unit, function.clone());
        // The deferred resources an application selects by key (#852, step 2) leave the run.
        let Run { unit, dictionary, lines } = run;
        let lines = match schedule.is_none() && self.bytecode.is_none() && ThemeUnitPlan::current().entries() {
            true => {
                let (kept, found) = selectable_entries(lines, &dictionary);
                let mut entries = self.units.entries.borrow_mut();
                entries.extend(found.into_iter().map(|(key, literal, build, position)| UnitEntry {
                    unit,
                    run: index,
                    key,
                    literal,
                    build,
                    position,
                }));
                kept
            }
            false => lines,
        };
        let code = || lines.iter().map(|line| line.trim_start()).filter(|text| !text.starts_with("//"));
        let uses_context = schedule.is_some() || code().any(|text| identifiers(text).iter().any(|name| name == "context"));
        let uses_dictionary = schedule.is_some() || code().next().is_some();
        let unit_name = self.units.names.get(unit as usize).map_or("", String::as_str).replace('`', "'").replace(['\r', '\n'], " ");
        let mut text = String::new();
        text.push_str(&format!(
            "/// Run {} of `{}` (`{}`): entries of the unit `{unit_name}` (#568).\n",
            self.functions.len(),
            self.function_name,
            self.document_text
        ));
        text.push_str(&format!("fn {function}(\n"));
        text.push_str(&format!(
            "    {}: &::std::rc::Rc<::ferroui_markup_xaml::xaml_il::runtime::XamlIlContext>,\n",
            if uses_context { "context" } else { "_context" }
        ));
        text.push_str(&format!("    {}: &{},\n", if uses_dictionary { "dictionary" } else { "_dictionary" }, self.dictionary_type));
        text.push_str(") -> ::core::result::Result<(), ::ferroui_markup_xaml::XamlLoadError> {\n");
        if let Some(schedule) = schedule {
            text.push_str(&schedule);
        } else {
            if uses_context {
                text.push_str("    let context = ::core::clone::Clone::clone(context);\n");
            }
            if uses_dictionary {
                text.push_str(&format!("    let {dictionary} = ::core::clone::Clone::clone(dictionary);\n"));
            }
            for line in &lines {
                text.push_str(line);
                text.push('\n');
            }
            text.push_str("    ::core::result::Result::Ok(())\n");
        }
        text.push_str("}\n");
        self.functions.push(text);
        self.calls.push(Call { line: self.body.len(), unit, run: index, dictionary: dictionary.clone() });
        self.body.push(format!("    rt::apply_theme_unit({ACTIVE}(), {unit}, {index}, &context, &{dictionary})?;"));
    }

    fn finish(mut self) -> (Vec<String>, Vec<String>) {
        self.flush();
        self.body.append(&mut self.pending_comments);
        let body = match ThemeUnitPlan::current().regions() {
            true => regions(self.body, &self.calls),
            false => self.body,
        };
        (body, self.functions)
    }
}

/// The calls of a region (#853) as the operand of `rt::apply_theme_region`: the pairs of the
/// unit and the index of its run, in their order, or `None` when an index does not fit the
/// 16 bits of a pair (the calls then stay as they are).
fn region_operand(calls: &[&Call]) -> Option<String> {
    let pairs = calls
        .iter()
        .map(|call| {
            let unit = u16::try_from(call.unit).ok()?;
            let run = u16::try_from(call.run).ok()?;
            Some(format!("({unit}, {run})"))
        })
        .collect::<Option<Vec<_>>>()?;
    Some(format!("&[{}]", pairs.join(", ")))
}

/// `body` with each region of `calls` (two or more calls of runs that add to one dictionary,
/// with only comments between them) written as one call of `rt::apply_theme_region`, after the
/// comments of the region (#853). A call alone stays a call of `rt::apply_theme_unit`.
fn regions(body: Vec<String>, calls: &[Call]) -> Vec<String> {
    let call_at: HashMap<usize, &Call> = calls.iter().map(|call| (call.line, call)).collect();
    let is_comment = |line: &str| line.trim_start().starts_with("//");
    let mut written = Vec::with_capacity(body.len());
    let mut index = 0;
    while index < body.len() {
        let Some(first) = call_at.get(&index) else {
            written.push(body[index].clone());
            index += 1;
            continue;
        };
        // The region: the calls of the dictionary of the first, over comments.
        let mut region = vec![*first];
        let mut comments = Vec::new();
        let mut end = index + 1;
        let mut next = index + 1;
        while next < body.len() {
            if let Some(call) = call_at.get(&next) {
                if call.dictionary != first.dictionary {
                    break;
                }
                region.push(*call);
                end = next + 1;
            } else if !is_comment(&body[next]) {
                break;
            }
            next += 1;
        }
        let operand = (region.len() > 1).then(|| region_operand(&region)).flatten();
        let Some(operand) = operand else {
            written.push(body[index].clone());
            index += 1;
            continue;
        };
        comments.extend((index..end).filter(|line| !call_at.contains_key(line)).map(|line| body[line].clone()));
        written.extend(comments);
        written.push(format!("    {APPLY_REGION}({ACTIVE}(), {operand}, &context, &{})?;", first.dictionary));
        index = end;
    }
    written
}

#[cfg(test)]
#[path = "theme_units_tests.rs"]
mod tests;
