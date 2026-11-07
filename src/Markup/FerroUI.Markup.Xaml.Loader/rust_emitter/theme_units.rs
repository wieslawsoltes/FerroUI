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
//! the table of all units in its place. The entries are added where they were, in the same
//! order, with the same context, so the dictionaries are the same. An entry stays in the body,
//! unmarked, when it is not one or more complete statements at the top level of the body, when
//! it names a local of the body other than the context and one dictionary, or when a local it
//! declares is named after it. The markers never reach the generated file.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use xamlx::ast::IXamlAstNode;

use crate::compiler_extensions::group_transformers::MergedEntryOrigins;

use super::{brace_balance, declared_local, identifiers};

/// The start of the marker that opens an entry of a unit; the index of the unit follows. A
/// comment, so the runs of rows step over it, with a control character no position marker has.
const BEGIN: &str = "//\u{1}unit ";

/// The marker that closes an entry of a unit.
pub(crate) const END_MARKER: &str = "//\u{1}end";

/// The name the generated file gives the static table of all units.
pub(crate) const TABLE: &str = "THEME_UNITS";

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
}

impl UnitEmission {
    /// `names` are the names of the units, by their indices.
    pub(crate) fn new(origins: Rc<MergedEntryOrigins>, unit_of_uri: HashMap<String, u32>, names: Vec<String>) -> Self {
        let runs = RefCell::new(vec![Vec::new(); names.len()]);
        Self { origins, unit_of_uri, names, runs }
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

    /// The statics of the units and the table of all units, as items of the generated file:
    /// `THEME_UNIT_<index>` for each unit, and `THEME_UNITS`.
    pub(crate) fn table(&self) -> String {
        let runs = self.runs.borrow();
        let mut text = String::new();
        for (index, name) in self.names.iter().enumerate() {
            let name_text = name.replace('`', "'").replace(['\r', '\n'], " ");
            text.push_str(&format!("/// The unit `{name_text}` of the theme (#568): its runs, in the order the builds apply them.\n"));
            text.push_str(&format!(
                "static THEME_UNIT_{index}: rt::ThemeUnit = rt::ThemeUnit {{ name: {}, index: {index}, runs: &[{}] }};\n\n",
                super::rust_string_literal(name),
                runs[index].join(", ")
            ));
        }
        text.push_str("/// Every unit of the theme, in canonical order: the table the builds of its dictionaries\n");
        text.push_str("/// apply the units through (#568).\n");
        let all: Vec<String> = (0..self.names.len()).map(|index| format!("&THEME_UNIT_{index}")).collect();
        text.push_str(&format!("pub static {TABLE}: &[&rt::ThemeUnit] = &[{}];\n", all.join(", ")));
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
        local_types,
        units,
        declared: ["context", "name_scope", "root", "service_provider"].iter().map(|name| name.to_string()).collect(),
        body: Vec::with_capacity(lines.len()),
        functions: Vec::new(),
        run: None,
        pending_comments: Vec::new(),
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
    local_types: &'e HashMap<String, String>,
    units: &'e UnitEmission,
    /// The locals declared in the body so far.
    declared: HashSet<String>,
    body: Vec<String>,
    functions: Vec<String>,
    run: Option<Run>,
    /// The comments of the body since the last run, which belong to what follows them.
    pending_comments: Vec<String>,
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
        let uses_context = run
            .lines
            .iter()
            .map(|line| line.trim_start())
            .filter(|text| !text.starts_with("//"))
            .any(|text| identifiers(text).iter().any(|name| name == "context"));
        let unit_name = self.units.names.get(run.unit as usize).map_or("", String::as_str).replace('`', "'").replace(['\r', '\n'], " ");
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
        text.push_str(&format!("    dictionary: &{},\n", self.dictionary_type));
        text.push_str(") -> ::core::result::Result<(), ::ferroui_markup_xaml::XamlLoadError> {\n");
        if uses_context {
            text.push_str("    let context = ::core::clone::Clone::clone(context);\n");
        }
        text.push_str(&format!("    let {} = ::core::clone::Clone::clone(dictionary);\n", run.dictionary));
        for line in &run.lines {
            text.push_str(line);
            text.push('\n');
        }
        text.push_str("    ::core::result::Result::Ok(())\n");
        text.push_str("}\n");
        self.functions.push(text);
        let index = self.units.push_run(run.unit, function);
        self.body.push(format!("    rt::apply_theme_unit({TABLE}, {}, {index}, &context, &{})?;", run.unit, run.dictionary));
    }

    fn finish(mut self) -> (Vec<String>, Vec<String>) {
        self.flush();
        self.body.append(&mut self.pending_comments);
        (self.body, self.functions)
    }
}

#[cfg(test)]
#[path = "theme_units_tests.rs"]
mod tests;
