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
use ferroui_build_scan::scanner::{InherentFunction, SourceReferences};
use ferroui_build_scan::xaml_metadata::ThemeUnitsModel;
use ferroui_markup_xaml_loader::rust_emitter::{allocate_names, PARTS_CONSTRUCTOR};

/// The key of the manifest table the trimming is read from.
pub const MANIFEST_TABLE: &str = "package.metadata.ferroui";

/// The switch of the manifest (`theme-trimming = false` turns the trimming off; it is on by
/// default).
pub const TRIMMING_KEY: &str = "theme-trimming";

/// The explicit roots of the manifest (`theme-roots = ["Name", ..]` or `"all"`).
pub const ROOTS_KEY: &str = "theme-roots";

/// The switch of the manifest for compiled bindings (`compile-bindings = false` makes `{Binding}`
/// a reflection binding in the documents of the crate; compiled bindings are the default, #603).
pub const COMPILE_BINDINGS_KEY: &str = "compile-bindings";

/// What the manifest `text` states for compiled bindings in its table
/// `[package.metadata.ferroui]` (`compile-bindings = true` or `false`, #603); `None` when it says
/// nothing. An error names the key when its value is not a boolean.
pub fn compile_bindings_from_manifest(text: &str) -> Result<Option<bool>, String> {
    let manifest = parse_manifest(text)?;
    match ferroui_table(&manifest)?.and_then(|table| table.get(COMPILE_BINDINGS_KEY)) {
        None => Ok(None),
        Some(toml::Value::Boolean(value)) => Ok(Some(*value)),
        Some(_) => Err(format!("[{MANIFEST_TABLE}] {COMPILE_BINDINGS_KEY} must be true or false")),
    }
}

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
        let manifest = parse_manifest(text)?;
        let Some(table) = ferroui_table(&manifest)? else { return Ok(trimming) };
        match table.get(TRIMMING_KEY) {
            None => {}
            Some(toml::Value::Boolean(enabled)) => trimming.enabled = *enabled,
            Some(_) => return Err(format!("[{MANIFEST_TABLE}] {TRIMMING_KEY} must be true or false")),
        }
        let roots_error = || format!("[{MANIFEST_TABLE}] {ROOTS_KEY} must be \"all\" or an array of names");
        match table.get(ROOTS_KEY) {
            None => {}
            Some(toml::Value::String(text)) if text == "all" => trimming.all = true,
            Some(toml::Value::Array(names)) => {
                for name in names {
                    trimming.roots.push(name.as_str().ok_or_else(roots_error)?.to_string());
                }
            }
            Some(_) => return Err(roots_error()),
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
/// not a letter or a digit (`THEME_PARTS_FERROUI_THEMES_WINUI_WINUITHEME`). It is the established
/// name: two classes may have one (`A.B_C` and `A_B.C`), and [`parts_static_names`] gives the
/// names a build writes.
pub fn parts_static_name(class: &str) -> String {
    let name: String = class.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_uppercase() } else { '_' }).collect();
    format!("THEME_PARTS_{name}")
}

/// The names of the statics of the parts tables of the classes `classes` (their full names) in
/// `compiled_markup`, in order (#849): the established name ([`parts_static_name`]) of a class
/// whose established name no other class has, and for the classes that share one, that name
/// disambiguated from the full name of each class (`allocate_names` of the emitter:
/// `THEME_PARTS_A_B_C_1F0E2D3C`).
pub fn parts_static_names(classes: &[&str]) -> Vec<String> {
    let established: Vec<String> = classes.iter().map(|class| parts_static_name(class)).collect();
    let requests: Vec<(&str, &str)> =
        established.iter().zip(classes).map(|(name, class)| (name.as_str(), *class)).collect();
    allocate_names(&requests, |a, b| a == b)
}

/// The path of the function another crate constructs the class `class` (its full name) with from
/// selected units of its compiled markup (#849), as the type models of the crates state it: the
/// class has units (`theme_units` of the model of its crate), and its type has a public associated
/// function [`PARTS_CONSTRUCTOR`] of two parameters (the `functions` of the model), which the build
/// of its crate verified to be `fn(Option<Rc<dyn IServiceProvider>>, rt::ThemeParts) -> Ref<Self>`
/// ([`verify_parts_constructor`]). `None` for any other class, whose markup may still be
/// partitioned into units: it is constructed with `new()`.
pub fn parts_constructor(models: &[AssemblyModel], class: &str) -> Option<String> {
    let model = models.iter().find(|model| model.theme_units.iter().any(|units| units.class == class))?;
    let type_ = model.types.iter().find(|type_| type_.full_name() == class)?;
    let functions = model.functions.iter().find(|functions| {
        functions.owner == type_.rust_path.text || type_.public_path.as_deref() == Some(functions.public_path.as_str())
    })?;
    let found = functions.functions.iter().any(|function| {
        function.name == PARTS_CONSTRUCTOR
            && !function.receiver
            && function.parameters == 2
            && function.declared_by.is_none()
    });
    found.then(|| format!("{}::{PARTS_CONSTRUCTOR}", functions.public_path))
}

/// Whether the class `class` (its full name) of the crate whose scan is `model` and `functions`
/// can be constructed from selected units of its compiled markup (#849): its type has a public
/// associated function [`PARTS_CONSTRUCTOR`] (without `self`) of the signature
/// `fn(Option<Rc<dyn IServiceProvider>>, rt::ThemeParts) -> Ref<Self>`, compared by the last
/// segment of each path. `Err` when it has a public one of two parameters and another signature:
/// the type models of the crate state the function to other crates by its name and its number of
/// parameters ([`parts_constructor`]), so a build of another crate would call it.
pub fn verify_parts_constructor(
    model: &AssemblyModel,
    functions: &[InherentFunction],
    class: &str,
) -> Result<bool, String> {
    let Some(type_) = model.types.iter().find(|type_| type_.full_name() == class) else {
        return Ok(false);
    };
    let local = rust_local_name(&type_.rust_path.text);
    let owned = |function: &&InherentFunction| {
        function.owner == type_.rust_path.text || type_.public_path.as_deref() == Some(function.owner.as_str())
    };
    let Some(function) = functions.iter().filter(owned).find(|function| {
        function.name == PARTS_CONSTRUCTOR
            && function.visibility == "pub"
            && !function.receiver
            && function.parameters.len() == 2
    }) else {
        return Ok(false);
    };
    let parameters: Vec<String> = function.parameters.iter().map(|parameter| last_segments(parameter)).collect();
    let returned = function.return_type.as_deref().map(last_segments).unwrap_or_default();
    let fits = parameters[0] == "Option<Rc<dynIServiceProvider>>"
        && parameters[1] == "ThemeParts"
        && (returned == "Ref<Self>" || returned == format!("Ref<{local}>"));
    match fits {
        true => Ok(true),
        false => Err(format!(
            "`{class}` has `pub fn {PARTS_CONSTRUCTOR}({}) -> {}`, which other crates construct the class \
             with from selected units of its compiled markup (#568): its signature must be \
             `fn(Option<Rc<dyn IServiceProvider>>, rt::ThemeParts) -> Ref<Self>`; give the function \
             another name if it is not that constructor",
            function.parameters.join(", "),
            function.return_type.as_deref().unwrap_or("()")
        )),
    }
}

/// `text` (a Rust type as written) without white space and with every path reduced to its last
/// segment (`::core::option::Option<Rc<dyn crate::IServiceProvider>>` is
/// `Option<Rc<dynIServiceProvider>>`).
fn last_segments(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut segment = String::new();
    let mut rest = text;
    while let Some(character) = rest.chars().next() {
        if let Some(after) = rest.strip_prefix("::") {
            // A qualifier: the segment before it is not the last one of its path.
            segment.clear();
            rest = after;
            continue;
        }
        rest = &rest[character.len_utf8()..];
        if character.is_alphanumeric() || character == '_' {
            segment.push(character);
            continue;
        }
        result.push_str(&segment);
        segment.clear();
        if !character.is_whitespace() {
            result.push(character);
        }
    }
    result.push_str(&segment);
    result
}

/// The path of the static of the unit `index` of the theme whose table of all units is
/// `table_path` (`::crate::compiled_xaml::THEME_UNITS` gives `::crate::compiled_xaml::THEME_UNIT_<index>`).
pub fn unit_static_path(table_path: &str, index: u32) -> String {
    let module = table_path.rsplit_once("::").map_or("", |(module, _)| module);
    format!("{module}::THEME_UNIT_{index}")
}

/// The manifest `text` as TOML (#850): the `toml` crate, a dependency of this build crate only, so
/// the parser runs in build scripts and is linked into no application. A manifest Cargo read is
/// valid TOML; anything else is an error of the build.
fn parse_manifest(text: &str) -> Result<toml::Table, String> {
    text.parse::<toml::Table>().map_err(|error| format!("the manifest is not TOML: {}", error.message()))
}

/// The table `package.metadata.ferroui` of the manifest, in whatever form the manifest writes it (a
/// header, dotted keys, an inline table); `None` when it has none. A value of that path that is
/// not a table is an error that names it.
fn ferroui_table(manifest: &toml::Table) -> Result<Option<&toml::Table>, String> {
    let mut table = manifest;
    for key in MANIFEST_TABLE.split('.') {
        match table.get(key) {
            None => return Ok(None),
            Some(toml::Value::Table(inner)) => table = inner,
            Some(_) => return Err(format!("`{MANIFEST_TABLE}` of the manifest must be a table")),
        }
    }
    Ok(Some(table))
}

#[cfg(test)]
#[path = "theme_roots_tests.rs"]
mod tests;
