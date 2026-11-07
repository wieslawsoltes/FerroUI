//! The units of the themes an application keeps (#568, steps 3 and 4 of
//! docs/porting/theme-trimming.md): the opt-in of the application, its roots, and the closure of
//! the roots over the unit graph of a theme.
//!
//! Not a port of an upstream file. A theme whose compiled markup is partitioned into units states
//! its unit graph in its `.xamlmeta` (`theme_units`: for each unit the keys and type keys it
//! defines, the keys it references and the types it constructs or selects). An application (unless
//! `[package.metadata.ferroui] theme-trimming = false` in its manifest turns it off) constructs the
//! theme from the units its build computes here:
//!
//! - **Roots.** The types the compiled documents of the application construct, select or name in
//!   `{x:Type}`, with their base classes (from the type models); every token of the documents that
//!   may name a key; the names of `theme-roots = ["Name", ..]` (a unit, by the path of its
//!   document, or a type or a key); the names and string literals of the Rust sources of the
//!   application ([`source_roots`]: a name that is a type key or a class, a string that is a key);
//!   and the roots a build script adds ([`Roots::extend`]). `theme-roots = "all"` and a build with
//!   `ferroui-base/full-themes` (`DEP_FERROUI_BASE_XAML_FULL_THEMES`: the run-time loader, hot
//!   reload, the designer, DevTools, Inspect) keep every unit. What the compiled documents of the
//!   dependencies name is a root too (the `references` of their `.xamlmeta`).
//! - **Live types.** A live type makes its base classes live, and the classes its code creates
//!   (the creates edges of the type models, `TypeModel::creates`: `ListBox` creates
//!   `ListBoxItem`), transitively ([`TypeGraph::live`]).
//! - **Closure.** The ambient unit (index 0) is always kept. A unit is kept when it defines a live
//!   key or the type key of a live type, or is named. A kept unit makes its references live keys
//!   (`StaticResource`, `DynamicResource`, `BasedOn`, `Theme` and every other mention of a key) and
//!   its types live types: the types its templates construct root their own control themes.
//!
//! The result is the canonical indices of the kept units, ascending: the order of the table the
//! build writes and `with_parts` takes.

use std::collections::{BTreeMap, BTreeSet};

use ferroui_build_scan::model::{AssemblyModel, TypeKind};
use ferroui_build_scan::scanner::SourceReferences;
use ferroui_build_scan::xaml_metadata::ThemeUnitsModel;

/// The key of the manifest table the trimming is read from.
pub const MANIFEST_TABLE: &str = "package.metadata.ferroui";

/// The switch of the manifest (`theme-trimming = false` turns the trimming off; it is on by
/// default).
pub const TRIMMING_KEY: &str = "theme-trimming";

/// The explicit roots of the manifest (`theme-roots = ["Name", ..]` or `"all"`).
pub const ROOTS_KEY: &str = "theme-roots";

/// What the manifest of an application states about the trimming of its themes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThemeTrimming {
    /// The application constructs its themes from the units it needs: the default, and
    /// `theme-trimming = true`; `theme-trimming = false` turns it off.
    pub enabled: bool,
    /// `theme-roots = "all"`: every unit is kept (the trimmed construction, with the full table).
    pub all: bool,
    /// `theme-roots = ["Name", ..]`: units, by the path of their document, or types or keys.
    pub roots: Vec<String>,
}

impl Default for ThemeTrimming {
    /// Trimming on, with the roots the build computes.
    fn default() -> Self {
        Self { enabled: true, all: false, roots: Vec::new() }
    }
}

impl ThemeTrimming {
    /// The trimming the manifest `text` states in its table `[package.metadata.ferroui]`; the
    /// default (trimming on) when it has none. An error names the key whose value is not one of
    /// the forms above.
    pub fn from_manifest(text: &str) -> Result<Self, String> {
        let mut trimming = ThemeTrimming::default();
        for (key, value) in manifest_table(text, MANIFEST_TABLE)? {
            match key.as_str() {
                TRIMMING_KEY => match value {
                    TomlValue::Bool(enabled) => trimming.enabled = enabled,
                    _ => return Err(format!("[{MANIFEST_TABLE}] {TRIMMING_KEY} must be true or false")),
                },
                ROOTS_KEY => match value {
                    TomlValue::String(text) if text == "all" => trimming.all = true,
                    TomlValue::Array(names) => trimming.roots.extend(names),
                    _ => return Err(format!("[{MANIFEST_TABLE}] {ROOTS_KEY} must be \"all\" or an array of names")),
                },
                _ => {}
            }
        }
        Ok(trimming)
    }
}

/// The roots of an application.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Roots {
    /// The local names of the types the application constructs or selects.
    pub types: BTreeSet<String>,
    /// What may name a key: keys, and tokens of the documents that may spell one.
    pub keys: BTreeSet<String>,
    /// Explicit names: a unit by the path of its document, else a type or a key.
    pub names: BTreeSet<String>,
    /// Every unit is kept.
    pub all: bool,
}

impl Roots {
    /// Adds the roots of `other`: what a build script states beside the documents, and the
    /// roots of the sources ([`source_roots`]).
    pub fn extend(&mut self, other: Roots) {
        self.types.extend(other.types);
        self.keys.extend(other.keys);
        self.names.extend(other.names);
        self.all |= other.all;
    }
}

/// The edges between types that make a type live with another, by local name: the base class
/// of a class (what keeps the theme of a base class for a class that does not override its
/// style key) and the classes the code of a class creates (#568, step 4: the containers a
/// `ListBox` creates, the selection handles of a `TextBox`).
#[derive(Clone, Debug, Default)]
pub struct TypeGraph {
    /// The local name of the base class of each class, by its local name.
    base_of: BTreeMap<String, String>,
    /// The local names of the classes the code of each class creates, by its local name.
    creates: BTreeMap<String, BTreeSet<String>>,
    /// The local names of the classes of the object model.
    classes: BTreeSet<String>,
}

/// The last segment of a Rust path, without generic arguments (`::a::b::Border<T>` is `Border`).
fn rust_local_name(text: &str) -> &str {
    let text = text.split('<').next().unwrap_or(text).trim();
    text.rsplit("::").next().unwrap_or(text).trim()
}

/// The last segment of a namespace-qualified name (`FerroUI.Controls.ListBoxItem` is
/// `ListBoxItem`).
fn markup_local_name(full_name: &str) -> &str {
    full_name.rsplit('.').next().unwrap_or(full_name)
}

impl TypeGraph {
    /// The edges the models state: for each class of the object model with a base class, the
    /// markup name of the base, found by its Rust name; for each type that creates classes, their
    /// local names.
    pub fn from_models<'m>(models: impl IntoIterator<Item = &'m AssemblyModel>) -> Self {
        let models: Vec<&AssemblyModel> = models.into_iter().collect();
        // The markup name of each type, by the local name of its Rust path.
        let mut markup_name: BTreeMap<String, String> = BTreeMap::new();
        for model in &models {
            for type_ in &model.types {
                markup_name.entry(rust_local_name(&type_.rust_path.text).to_string()).or_insert_with(|| type_.name.clone());
            }
        }
        let mut graph = Self::default();
        for model in &models {
            for type_ in &model.types {
                if type_.object_model && type_.kind == TypeKind::Class {
                    graph.classes.insert(type_.name.clone());
                }
                if !type_.creates.is_empty() {
                    let created = graph.creates.entry(type_.name.clone()).or_default();
                    created.extend(type_.creates.iter().map(|full_name| markup_local_name(full_name).to_string()).filter(|name| *name != type_.name));
                }
                let Some(base) = &type_.base else { continue };
                let local = rust_local_name(&base.text);
                let base = markup_name.get(local).cloned().unwrap_or_else(|| local.to_string());
                if !base.is_empty() && base != type_.name {
                    graph.base_of.entry(type_.name.clone()).or_insert(base);
                }
            }
        }
        graph
    }

    /// The base classes stated as pairs `(class, base)`, for the tests.
    pub fn from_pairs(pairs: &[(&str, &str)]) -> Self {
        Self { base_of: pairs.iter().map(|(class, base)| (class.to_string(), base.to_string())).collect(), ..Self::default() }
    }

    /// The graph with the creates edges stated as pairs `(class, created)`, for the tests.
    pub fn with_creates(mut self, pairs: &[(&str, &str)]) -> Self {
        for (class, created) in pairs {
            self.creates.entry(class.to_string()).or_default().insert(created.to_string());
            self.classes.insert(class.to_string());
            self.classes.insert(created.to_string());
        }
        self
    }

    /// Whether `name` is a class of the object model of the models, or a class of an edge.
    pub fn is_class(&self, name: &str) -> bool {
        self.classes.contains(name)
    }

    /// `name` and its base classes, nearest first.
    pub fn chain(&self, name: &str) -> Vec<String> {
        let mut chain = vec![name.to_string()];
        while let Some(base) = self.base_of.get(chain.last().map(String::as_str).unwrap_or_default()) {
            if chain.contains(base) {
                break;
            }
            chain.push(base.clone());
        }
        chain
    }

    /// The types that are live when `name` is: `name`, its base classes, the classes any of them
    /// creates, and so on, transitively.
    pub fn live(&self, name: &str) -> BTreeSet<String> {
        let mut live = BTreeSet::new();
        let mut pending = vec![name.to_string()];
        while let Some(next) = pending.pop() {
            for type_ in self.chain(&next) {
                if !live.insert(type_.clone()) {
                    continue;
                }
                pending.extend(self.creates.get(&type_).into_iter().flatten().filter(|created| !live.contains(*created)).cloned());
            }
        }
        live
    }
}

/// The roots the Rust sources of an application state (#568, step 4): its names that are a type
/// key of a unit of `themes` or a class of `graph` (`CalendarView::new()`, `Ref<ListBox>`, a
/// `use` of `CalendarView`), and its string literals that are a key a unit of `themes` defines
/// (`"AccentButtonStyle"`, `"TextFillColorPrimaryBrush"`). Conservative: a name or a string that
/// matches roots its units whatever the code does with it. A name the code builds at run time
/// matches nothing, and needs `theme-roots`.
pub fn source_roots<'t>(references: &SourceReferences, themes: impl IntoIterator<Item = &'t ThemeUnitsModel>, graph: &TypeGraph) -> Roots {
    let mut type_keys: BTreeSet<&str> = BTreeSet::new();
    let mut keys: BTreeSet<&str> = BTreeSet::new();
    for theme in themes {
        for unit in &theme.units {
            type_keys.extend(unit.type_keys.iter().map(String::as_str));
            keys.extend(unit.keys.iter().map(String::as_str));
        }
    }
    Roots {
        types: references.names.iter().filter(|name| type_keys.contains(name.as_str()) || graph.is_class(name)).cloned().collect(),
        keys: references.strings.iter().filter(|text| keys.contains(text.as_str())).cloned().collect(),
        ..Roots::default()
    }
}

/// The canonical indices of the units of `theme` that `roots` keep, ascending (see the module
/// documentation). A live type makes the types of [`TypeGraph::live`] live. `full_themes` keeps
/// every unit.
pub fn kept_units(theme: &ThemeUnitsModel, roots: &Roots, graph: &TypeGraph, full_themes: bool) -> Vec<u32> {
    if roots.all || full_themes {
        let mut all: Vec<u32> = theme.units.iter().map(|unit| unit.index).collect();
        all.sort_unstable();
        all.dedup();
        return all;
    }
    let mut live_keys: BTreeSet<String> = roots.keys.clone();
    let mut live_types: BTreeSet<String> = BTreeSet::new();
    let mut kept = vec![false; theme.units.len()];
    let mut pending: Vec<usize> = Vec::new();
    for type_ in &roots.types {
        live_types.extend(graph.live(type_));
    }
    for name in &roots.names {
        match theme.units.iter().position(|unit| unit.name == *name) {
            Some(position) => pending.push(position),
            None => {
                live_keys.insert(name.clone());
                live_types.extend(graph.live(name));
            }
        }
    }
    pending.extend(theme.units.iter().position(|unit| unit.index == 0));
    loop {
        // Keep what is named or found, and what it references.
        while let Some(position) = pending.pop() {
            if std::mem::replace(&mut kept[position], true) {
                continue;
            }
            let unit = &theme.units[position];
            live_keys.extend(unit.references.iter().cloned());
            for type_ in &unit.types {
                if !live_types.contains(type_) {
                    live_types.extend(graph.live(type_));
                }
            }
        }
        // The units that define a live key or the type key of a live type.
        pending.extend((0..theme.units.len()).filter(|&position| {
            let unit = &theme.units[position];
            !kept[position]
                && (unit.keys.iter().any(|key| live_keys.contains(key)) || unit.type_keys.iter().any(|type_key| live_types.contains(type_key)))
        }));
        if pending.is_empty() {
            break;
        }
    }
    let mut indices: Vec<u32> = theme.units.iter().zip(&kept).filter(|(_, kept)| **kept).map(|(unit, _)| unit.index).collect();
    indices.sort_unstable();
    indices
}

/// The name of the static of the parts table of the class `class` (its full name) in
/// `compiled_markup`: `THEME_PARTS_` and the name in upper case, with `_` for anything that is
/// not a letter or a digit (`THEME_PARTS_FERROUI_THEMES_WINUI_WINUITHEME`).
pub fn parts_static_name(class: &str) -> String {
    let name: String = class.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_uppercase() } else { '_' }).collect();
    format!("THEME_PARTS_{name}")
}

/// The path of the static of the unit `index` of the theme whose table of all units is
/// `table_path` (`::crate::compiled_xaml::THEME_UNITS` gives `::crate::compiled_xaml::THEME_UNIT_<index>`).
pub fn unit_static_path(table_path: &str, index: u32) -> String {
    let module = table_path.rsplit_once("::").map_or("", |(module, _)| module);
    format!("{module}::THEME_UNIT_{index}")
}

/// A value of the manifest, of the forms the table of the opt-in has.
#[derive(Clone, Debug, PartialEq, Eq)]
enum TomlValue {
    Bool(bool),
    String(String),
    Array(Vec<String>),
    /// Any other value: a number, a date, an inline table, an array of other values.
    Other,
}

/// The text of `line` before a comment (`#` outside a string).
fn without_comment(line: &str) -> &str {
    let mut in_string: Option<char> = None;
    let mut escaped = false;
    for (index, c) in line.char_indices() {
        match in_string {
            Some(quote) => {
                if escaped {
                    escaped = false;
                } else if c == '\\' && quote == '"' {
                    escaped = true;
                } else if c == quote {
                    in_string = None;
                }
            }
            None if c == '"' || c == '\'' => in_string = Some(c),
            None if c == '#' => return &line[..index],
            None => {}
        }
    }
    line
}

/// The strings of `text` (`"a"` or `'a'`), in order; `None` when `text` has anything else than
/// strings, commas and white space between `[` and `]`.
fn string_items(text: &str) -> Option<Vec<String>> {
    let inner = text.trim().strip_prefix('[')?.strip_suffix(']')?;
    let mut items = Vec::new();
    let mut rest = inner.trim_start();
    while !rest.is_empty() {
        let (item, after) = string_value(rest)?;
        items.push(item);
        rest = after.trim_start();
        match rest.strip_prefix(',') {
            Some(after) => rest = after.trim_start(),
            None if rest.is_empty() => {}
            None => return None,
        }
    }
    Some(items)
}

/// The string at the start of `text` and what follows it.
fn string_value(text: &str) -> Option<(String, &str)> {
    let quote = text.chars().next().filter(|c| *c == '"' || *c == '\'')?;
    let mut value = String::new();
    let mut escaped = false;
    for (index, c) in text.char_indices().skip(1) {
        if escaped {
            value.push(match c {
                'n' => '\n',
                't' => '\t',
                other => other,
            });
            escaped = false;
        } else if c == '\\' && quote == '"' {
            escaped = true;
        } else if c == quote {
            return Some((value, &text[index + c.len_utf8()..]));
        } else {
            value.push(c);
        }
    }
    None
}

fn parse_value(text: &str) -> TomlValue {
    let text = text.trim();
    match text {
        "true" => return TomlValue::Bool(true),
        "false" => return TomlValue::Bool(false),
        _ => {}
    }
    if text.starts_with('[') {
        return string_items(text).map_or(TomlValue::Other, TomlValue::Array);
    }
    match string_value(text) {
        Some((value, rest)) if rest.trim().is_empty() => TomlValue::String(value),
        _ => TomlValue::Other,
    }
}

/// The keys and values of the table `[name]` of the manifest `text`, in order. Only what the
/// opt-in needs is read: `key = value` lines, an array may span lines. The names of the headers
/// are compared without white space and quotes.
fn manifest_table(text: &str, name: &str) -> Result<Vec<(String, TomlValue)>, String> {
    let normalised = |header: &str| header.chars().filter(|c| !c.is_whitespace() && *c != '"' && *c != '\'').collect::<String>();
    let mut entries = Vec::new();
    let mut inside = false;
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        let line = without_comment(line).trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            let header = line.trim_start_matches('[').trim_end_matches(']');
            inside = !line.starts_with("[[") && normalised(header) == name;
            continue;
        }
        if !inside {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        let key = normalised(key);
        let mut value = value.trim().to_string();
        // An array that continues on the next lines.
        if value.starts_with('[') {
            while value.matches('[').count() > value.matches(']').count() {
                let Some(next) = lines.next() else {
                    return Err(format!("[{name}] {key}: the array is not closed"));
                };
                value.push(' ');
                value.push_str(without_comment(next).trim());
            }
        }
        entries.push((key, parse_value(&value)));
    }
    Ok(entries)
}

#[cfg(test)]
#[path = "theme_roots_tests.rs"]
mod tests;
