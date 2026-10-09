//! The module tree of a scanned crate and the resolution of paths against
//! it: what a name means in a module, from the items the module declares, its
//! `use` items and its glob imports (xaml.md 9.5.2, step 1).
//!
//! This is a reader of the names the declarations of a crate use, not the
//! name resolution of the language: there is one namespace (a type and a
//! function of one name in one module are the type), visibility is not
//! checked (the compiler checked it), and the names a macro declares are
//! known only for the macros the scanner reads.

use std::collections::{BTreeMap, BTreeSet};

/// A name a `use` item brings into a module.
pub(crate) struct Import {
    /// The name in the module (the last segment, or the name after `as`).
    pub name: String,
    /// The segments of the path; a leading `::` is an empty first segment.
    pub path: Vec<String>,
    /// Declared with `pub` (another crate can name it).
    pub public: bool,
}

/// A glob import of a module (`use a::b::*`).
pub(crate) struct Glob {
    pub path: Vec<String>,
    pub public: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ItemKind {
    Module(usize),
    /// A struct, an enumeration, a trait, a type alias, a class.
    Type,
    /// A function, a constant, a static.
    Value,
}

pub(crate) struct Item {
    pub kind: ItemKind,
    pub public: bool,
}

pub(crate) struct Module {
    /// The segments of the path of the module, the name of the crate first.
    pub path: Vec<String>,
    pub parent: Option<usize>,
    pub items: BTreeMap<String, Item>,
    pub imports: Vec<Import>,
    pub globs: Vec<Glob>,
    /// The modules of the crate the glob imports name, and whether the import is public
    /// ([`Modules::finish`]).
    glob_sources: Vec<(usize, bool)>,
    /// The module has a glob import that names no module of the crate: a name that is
    /// not found in it may come from there.
    external_glob: bool,
    /// The modules of other crates the glob imports name, of the crates whose export
    /// table is known ([`Modules::externals`]), as absolute paths (`::ferroui_base`).
    external_globs: Vec<String>,
}

/// What a path names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Target {
    Module(usize),
    /// An item of a module of the crate, and the segments after it (`Type::function`).
    Item { module: usize, name: String, rest: Vec<String> },
    /// A path into another crate, from its name.
    External(Vec<String>),
    /// A name of the language prelude or a primitive type.
    Builtin,
}

/// The names every module has without a `use` item: the primitive types and the types of
/// the prelude a declaration can name.
const BUILTIN: &[&str] = &[
    "bool", "char", "str", "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize", "f32", "f64", "String", "Option",
    "Vec", "Box", "Result",
];

/// The crates every crate can name.
const ALWAYS_EXTERN: &[&str] = &["std", "core", "alloc"];

/// Imports are followed this deep; a cycle of `use` items ends there.
const MAX_DEPTH: usize = 32;

/// Modules are exported this deep ([`Modules::export_table`]).
const MAX_EXPORT_DEPTH: usize = 12;

/// Whether `name` can be the name of a crate: an identifier without capitals (a head in
/// capitals is a type: `Self`, a type parameter, a type a macro declares).
fn is_crate_name(name: &str) -> bool {
    !name.is_empty()
        && !matches!(name, "crate" | "self" | "super")
        && name.chars().all(|character| character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_')
}

pub(crate) struct Modules {
    pub modules: Vec<Module>,
    /// The names of other crates: the ones the caller stated and the first segments of
    /// the `use` paths of the crate that name nothing in their module.
    pub extern_crates: BTreeSet<String>,
    /// The export tables of the crates the crate is built on: every public path of a type
    /// of such a crate (`::ferroui_base::Border`), with the path of the module that
    /// declares it. A name a glob import of a module of such a crate brings is found here.
    pub externals: BTreeMap<String, String>,
    /// The crates whose export table is in [`externals`](Self::externals).
    pub external_crates: BTreeSet<String>,
}

impl Modules {
    /// The tree with the root module of the crate `crate_name`.
    pub fn new(crate_name: &str, extern_crates: &[String]) -> Self {
        let mut modules = Self {
            modules: Vec::new(),
            extern_crates: extern_crates.iter().cloned().collect(),
            externals: BTreeMap::new(),
            external_crates: BTreeSet::new(),
        };
        modules.modules.push(Module {
            path: vec![crate_name.to_string()],
            parent: None,
            items: BTreeMap::new(),
            imports: Vec::new(),
            globs: Vec::new(),
            glob_sources: Vec::new(),
            external_glob: false,
            external_globs: Vec::new(),
        });
        modules
    }

    /// Adds the module `name` of `parent` and returns its index; the index of the module
    /// when the parent has it already (two `cfg` variants of one module).
    pub fn add_module(&mut self, parent: usize, name: &str, public: bool) -> usize {
        if let Some(Item { kind: ItemKind::Module(index), .. }) = self.modules[parent].items.get(name) {
            return *index;
        }
        let index = self.modules.len();
        let mut path = self.modules[parent].path.clone();
        path.push(name.to_string());
        self.modules.push(Module {
            path,
            parent: Some(parent),
            items: BTreeMap::new(),
            imports: Vec::new(),
            globs: Vec::new(),
            glob_sources: Vec::new(),
            external_glob: false,
            external_globs: Vec::new(),
        });
        self.modules[parent].items.insert(name.to_string(), Item { kind: ItemKind::Module(index), public });
        index
    }

    /// Adds an item of a module. A type replaces a value of the same name; nothing
    /// replaces a type or a module.
    pub fn add_item(&mut self, module: usize, name: &str, kind: ItemKind, public: bool) {
        let items = &mut self.modules[module].items;
        match items.get(name) {
            Some(Item { kind: ItemKind::Value, .. }) | None => {
                items.insert(name.to_string(), Item { kind, public });
            }
            Some(_) => {}
        }
    }

    /// The path of a module as `module_path!()` gives it (`ferroui_controls::primitives`).
    pub fn module_path(&self, module: usize) -> String {
        self.modules[module].path.join("::")
    }

    /// The module with the path `path` (`ferroui_controls::primitives`).
    pub fn find_module(&self, path: &str) -> Option<usize> {
        self.modules.iter().position(|module| module.path.join("::") == path)
    }

    /// Completes the tree once every file is read: the modules the glob imports name,
    /// which modules import a glob of another crate, and which names are crates.
    pub fn finish(&mut self) {
        // The path of a glob import may itself go through a glob import, so the sources
        // are found in rounds: the first one sees no glob at all.
        for _ in 0..4 {
            let mut changed = false;
            for index in 0..self.modules.len() {
                let mut sources = Vec::new();
                for glob in &self.modules[index].globs {
                    if let Some(Target::Module(source)) = self.resolve(index, &glob.path) {
                        if source != index {
                            sources.push((source, glob.public));
                        }
                    }
                }
                if sources != self.modules[index].glob_sources {
                    self.modules[index].glob_sources = sources;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        for index in 0..self.modules.len() {
            let external = self.modules[index].globs.iter().any(|glob| !matches!(self.resolve(index, &glob.path), Some(Target::Module(_))));
            // The globs of modules of crates whose export table is known: their names are
            // looked up there. The module still counts as one with a glob of another crate,
            // so that a name no table has is not guessed.
            let mut known = Vec::new();
            for glob in &self.modules[index].globs {
                let segments = match self.resolve(index, &glob.path) {
                    Some(Target::Module(_)) => continue,
                    Some(Target::External(segments)) => segments,
                    _ => glob.path.iter().filter(|segment| !segment.is_empty()).cloned().collect(),
                };
                if segments.first().is_some_and(|name| self.external_crates.contains(name)) {
                    known.push(format!("::{}", segments.join("::")));
                }
            }
            self.modules[index].external_glob = external;
            self.modules[index].external_globs = known;
        }
        // Without a glob of another crate, the first segment of a `use` path that names
        // nothing of the module names a crate (the language leaves nothing else).
        let mut crates = Vec::new();
        for (index, module) in self.modules.iter().enumerate() {
            if module.external_glob {
                continue;
            }
            for path in module.imports.iter().map(|import| &import.path).chain(module.globs.iter().map(|glob| &glob.path)) {
                let head = path[0].as_str();
                if !is_crate_name(head) {
                    continue;
                }
                let names_a_crate = match self.lookup(index, head, None, 0) {
                    None => true,
                    Some(Target::External(segments)) => segments[0] == head,
                    Some(_) => false,
                };
                if names_a_crate {
                    crates.push(head.to_string());
                }
            }
        }
        self.extern_crates.extend(crates);
    }

    /// Whether the unknown head `name` of a path with more segments is the name of a crate:
    /// a crate the scanner knows, or, in a module without a glob of another crate, any
    /// name a crate can have.
    fn is_extern(&self, module: usize, name: &str) -> bool {
        is_crate_name(name) && (ALWAYS_EXTERN.contains(&name) || self.extern_crates.contains(name) || !self.modules[module].external_glob)
    }

    /// What `path` names in `module`; nothing when a segment is not known.
    pub fn resolve(&self, module: usize, path: &[String]) -> Option<Target> {
        self.resolve_at(module, path, None, 0)
    }

    /// `skip` is the import (of `module`) whose path is being resolved: its own name does
    /// not resolve its first segment (`use bitflags::bitflags;`).
    fn resolve_at(&self, module: usize, path: &[String], skip: Option<usize>, depth: usize) -> Option<Target> {
        if depth > MAX_DEPTH || path.is_empty() {
            return None;
        }
        let (mut current, mut index) = match path[0].as_str() {
            "" => return (path.len() > 1).then(|| Target::External(path[1..].to_vec())),
            "crate" => (0, 1),
            "self" => (module, 1),
            "super" => {
                let mut current = module;
                let mut index = 0;
                while index < path.len() && path[index] == "super" {
                    current = self.modules[current].parent?;
                    index += 1;
                }
                (current, index)
            }
            first => match self.lookup(module, first, skip, depth) {
                Some(Target::Module(found)) => (found, 1),
                Some(Target::Item { module, name, mut rest }) => {
                    rest.extend(path[1..].iter().cloned());
                    return Some(Target::Item { module, name, rest });
                }
                Some(Target::External(mut segments)) => {
                    segments.extend(path[1..].iter().cloned());
                    return Some(Target::External(segments));
                }
                Some(Target::Builtin) | None => {
                    if path.len() == 1 {
                        return BUILTIN.contains(&first).then_some(Target::Builtin);
                    }
                    // A path below a module a glob import of another crate brings
                    // (`media::Brush` next to `use ferroui_base::*`).
                    if let Some(found) = self.external_item(module, &path.join("::")) {
                        return Some(found);
                    }
                    return self.is_extern(module, first).then(|| Target::External(path.to_vec()));
                }
            },
        };
        while index < path.len() {
            match self.lookup(current, &path[index], None, depth + 1) {
                Some(Target::Module(found)) => {
                    current = found;
                    index += 1;
                }
                Some(Target::Item { module, name, mut rest }) => {
                    rest.extend(path[index + 1..].iter().cloned());
                    return Some(Target::Item { module, name, rest });
                }
                Some(Target::External(mut segments)) => {
                    segments.extend(path[index + 1..].iter().cloned());
                    return Some(Target::External(segments));
                }
                // The module is known and the name is not one the scanner read (a macro
                // declares it): the path is absolute as it is written.
                Some(Target::Builtin) | None => {
                    return Some(Target::Item { module: current, name: path[index].clone(), rest: path[index + 1..].to_vec() });
                }
            }
        }
        Some(Target::Module(current))
    }

    /// What the name `name` is in `module`: an item of the module, a name one of its `use`
    /// items imports (an explicit import before a glob, as in the language), or a name of
    /// a glob import of a module of the crate.
    fn lookup(&self, module: usize, name: &str, skip: Option<usize>, depth: usize) -> Option<Target> {
        self.lookup_visiting(module, name, skip, depth, &mut Vec::new())
    }

    /// `visited` holds the modules already asked for `name` through glob imports: glob
    /// imports of modules of one crate form cycles (`pub use child::*` and `use super::*`).
    fn lookup_visiting(&self, module: usize, name: &str, skip: Option<usize>, depth: usize, visited: &mut Vec<usize>) -> Option<Target> {
        if depth > MAX_DEPTH || visited.contains(&module) {
            return None;
        }
        visited.push(module);
        let declared = &self.modules[module];
        if let Some(item) = declared.items.get(name) {
            return Some(match item.kind {
                ItemKind::Module(index) => Target::Module(index),
                ItemKind::Type | ItemKind::Value => Target::Item { module, name: name.to_string(), rest: Vec::new() },
            });
        }
        for (index, import) in declared.imports.iter().enumerate() {
            if import.name != name || skip == Some(index) {
                continue;
            }
            return match self.resolve_at(module, &import.path, Some(index), depth + 1) {
                Some(Target::Builtin) | None if import.path.len() == 1 && is_crate_name(&import.path[0]) => Some(Target::External(import.path.clone())),
                Some(Target::Builtin) => None,
                found => found,
            };
        }
        for (source, _) in &declared.glob_sources {
            if let Some(found) = self.lookup_visiting(*source, name, None, depth + 1, visited) {
                return Some(found);
            }
        }
        self.external_item(module, name)
    }

    /// The type of another crate that `path` (a name, or a path below a module) names
    /// through a glob import of `module`, by the export table of that crate: the path of
    /// its declaring module.
    fn external_item(&self, module: usize, path: &str) -> Option<Target> {
        self.modules[module].external_globs.iter().find_map(|glob| {
            let declared = self.externals.get(&format!("{glob}::{path}"))?;
            Some(Target::External(declared.split("::").filter(|segment| !segment.is_empty()).map(str::to_string).collect()))
        })
    }

    /// The export table of the crate: every path another crate can name a type of the
    /// crate by, with the absolute path of the item in its declaring module (a path into
    /// another crate for a type the crate exports again), in the order of the paths.
    pub fn export_table(&self) -> Vec<(String, String)> {
        let mut table = BTreeMap::new();
        let path = self.modules[0].path.clone();
        self.export_module(0, &path, &mut vec![0], &mut table);
        table.into_iter().collect()
    }

    /// Adds what `module`, named by `path` from outside, exports. `stack` holds the
    /// modules on the way: a module that exports itself again is entered once.
    fn export_module(&self, module: usize, path: &[String], stack: &mut Vec<usize>, table: &mut BTreeMap<String, String>) {
        for (name, target) in self.exports(module, &mut vec![module]) {
            let mut named = path.to_vec();
            named.push(name);
            match target {
                Target::Module(inner) => {
                    if !stack.contains(&inner) && stack.len() < MAX_EXPORT_DEPTH {
                        stack.push(inner);
                        self.export_module(inner, &named, stack, table);
                        stack.pop();
                    }
                }
                Target::Builtin => {}
                Target::Item { ref rest, .. } if !rest.is_empty() => {}
                item => {
                    // A function, a constant or a static is not a type.
                    if let Target::Item { module: declaring, name: declared_name, .. } = &item {
                        let is_value = self.modules[*declaring].items.get(declared_name).is_some_and(|declared| declared.kind == ItemKind::Value);
                        if is_value {
                            continue;
                        }
                    }
                    if let Some(declared) = self.absolute(&item) {
                        table.insert(format!("::{}", named.join("::")), declared);
                    }
                }
            }
        }
    }

    /// The absolute text of a target (`::ferroui_controls::border::Border`); nothing for a
    /// builtin name, which has no path.
    pub fn absolute(&self, target: &Target) -> Option<String> {
        match target {
            Target::Module(module) => Some(format!("::{}", self.module_path(*module))),
            Target::Item { module, name, rest } => {
                let mut text = format!("::{}::{name}", self.module_path(*module));
                for segment in rest {
                    text.push_str("::");
                    text.push_str(segment);
                }
                Some(text)
            }
            Target::External(segments) => Some(format!("::{}", segments.join("::"))),
            Target::Builtin => None,
        }
    }

    /// The names a module gives another crate: its public items, the names its public
    /// `use` items import, and the names of its public glob imports of modules of the
    /// crate (an explicit name before a glob).
    fn exports(&self, module: usize, visiting: &mut Vec<usize>) -> BTreeMap<String, Target> {
        let mut names = BTreeMap::new();
        let declared = &self.modules[module];
        for (name, item) in &declared.items {
            if item.public {
                let target = match item.kind {
                    ItemKind::Module(index) => Target::Module(index),
                    ItemKind::Type | ItemKind::Value => Target::Item { module, name: name.clone(), rest: Vec::new() },
                };
                names.insert(name.clone(), target);
            }
        }
        for (index, import) in declared.imports.iter().enumerate() {
            if import.public && !names.contains_key(&import.name) {
                if let Some(target) = self.resolve_at(module, &import.path, Some(index), 0) {
                    names.insert(import.name.clone(), target);
                }
            }
        }
        for (source, public) in &declared.glob_sources {
            if *public && !visiting.contains(source) {
                visiting.push(*source);
                for (name, target) in self.exports(*source, visiting) {
                    names.entry(name).or_insert(target);
                }
                visiting.pop();
            }
        }
        names
    }

    /// The shortest public path of every item of the crate another crate can name, by the
    /// absolute path of the item in its declaring module; ties go to the path that is
    /// first by its segments (the rule of `scripts/rust_paths.py`).
    pub fn public_paths(&self) -> BTreeMap<String, String> {
        let mut reached: BTreeSet<usize> = BTreeSet::new();
        let mut pending: BTreeSet<(usize, Vec<String>, usize)> = BTreeSet::new();
        let mut best: BTreeMap<String, Vec<String>> = BTreeMap::new();
        pending.insert((1, self.modules[0].path.clone(), 0));
        while let Some((_, path, module)) = pending.pop_first() {
            if !reached.insert(module) {
                continue;
            }
            for (name, target) in self.exports(module, &mut vec![module]) {
                let mut named = path.clone();
                named.push(name);
                match target {
                    Target::Module(inner) => {
                        if !reached.contains(&inner) {
                            pending.insert((named.len(), named, inner));
                        }
                    }
                    Target::Item { rest, .. } if !rest.is_empty() => {}
                    item @ Target::Item { .. } => {
                        let Some(declared) = self.absolute(&item) else { continue };
                        let shorter = best.get(&declared).is_none_or(|known| (named.len(), &named) < (known.len(), known));
                        if shorter {
                            best.insert(declared, named);
                        }
                    }
                    Target::External(_) | Target::Builtin => {}
                }
            }
        }
        best.into_iter().map(|(declared, path)| (declared, format!("::{}", path.join("::")))).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(text: &str) -> Vec<String> {
        text.split("::").map(str::to_string).collect()
    }

    fn import(modules: &mut Modules, module: usize, name: &str, target: &str, public: bool) {
        modules.modules[module].imports.push(Import { name: name.to_string(), path: path(target), public });
    }

    fn glob(modules: &mut Modules, module: usize, target: &str, public: bool) {
        modules.modules[module].globs.push(Glob { path: path(target), public });
    }

    /// `fixture`: `mod border` (private, `Border`), `pub mod primitives` (`Popup`, private
    /// `Hidden`), `mod prelude` (imports of another crate); the root re-exports `Border`
    /// by name and everything of `primitives` by a glob.
    fn fixture() -> (Modules, usize, usize, usize) {
        let mut modules = Modules::new("fixture", &[]);
        let border = modules.add_module(0, "border", false);
        let primitives = modules.add_module(0, "primitives", true);
        let prelude = modules.add_module(0, "prelude", false);
        modules.add_item(border, "Border", ItemKind::Type, true);
        modules.add_item(primitives, "Popup", ItemKind::Type, true);
        modules.add_item(primitives, "Hidden", ItemKind::Type, false);
        import(&mut modules, 0, "Border", "border::Border", true);
        glob(&mut modules, 0, "primitives", true);
        import(&mut modules, border, "Ref", "base::Ref", false);
        import(&mut modules, border, "Rc", "std::rc::Rc", false);
        import(&mut modules, border, "Popup", "crate::Popup", false);
        import(&mut modules, border, "flags", "flags::flags", false);
        glob(&mut modules, prelude, "base", false);
        import(&mut modules, prelude, "Border", "super::Border", false);
        modules.finish();
        (modules, border, primitives, prelude)
    }

    fn resolved(modules: &Modules, module: usize, text: &str) -> Option<String> {
        modules.resolve(module, &path(text)).map(|target| modules.absolute(&target).unwrap_or_else(|| text.to_string()))
    }

    /// Not from upstream: a path is resolved through `use` items, re-exports and globs of
    /// the crate to the module that declares the item; a path into another crate is
    /// absolute as the `use` item spells it.
    #[test]
    fn paths_resolve_to_the_declaring_module() {
        let (modules, border, primitives, _) = fixture();
        assert_eq!(resolved(&modules, border, "Border"), Some("::fixture::border::Border".to_string()));
        assert_eq!(resolved(&modules, primitives, "crate::Border"), Some("::fixture::border::Border".to_string()));
        assert_eq!(resolved(&modules, border, "Popup"), Some("::fixture::primitives::Popup".to_string()));
        assert_eq!(resolved(&modules, border, "super::primitives::Popup::new"), Some("::fixture::primitives::Popup::new".to_string()));
        assert_eq!(resolved(&modules, border, "self::Border"), Some("::fixture::border::Border".to_string()));
        assert_eq!(resolved(&modules, border, "Ref"), Some("::base::Ref".to_string()));
        assert_eq!(resolved(&modules, border, "Rc"), Some("::std::rc::Rc".to_string()));
        assert_eq!(resolved(&modules, border, "flags"), Some("::flags::flags".to_string()));
        assert_eq!(resolved(&modules, border, "base::media::Brush"), Some("::base::media::Brush".to_string()));
        assert_eq!(resolved(&modules, border, "::other::Thing"), Some("::other::Thing".to_string()));
        assert_eq!(resolved(&modules, border, "Option"), Some("Option".to_string()));
        assert_eq!(modules.resolve(border, &path("Option")), Some(Target::Builtin));
        assert_eq!(modules.resolve(border, &path("crate::primitives")), Some(Target::Module(primitives)));
        // A name of a known module that the scanner did not read is absolute as written.
        assert_eq!(resolved(&modules, border, "crate::primitives::Generated"), Some("::fixture::primitives::Generated".to_string()));
        assert!(modules.extern_crates.contains("base") && modules.extern_crates.contains("flags") && modules.extern_crates.contains("std"));
    }

    /// Not from upstream: a name that nothing of the module declares or imports is not
    /// resolved, and next to a glob of another crate a path with an unknown head is a
    /// path into a crate only when the head is known to be one.
    #[test]
    fn unknown_names_are_not_guessed() {
        let (modules, border, _, prelude) = fixture();
        assert_eq!(modules.resolve(border, &path("Mystery")), None);
        assert_eq!(modules.resolve(border, &path("super::super::Border")), None);
        // `prelude` imports `base::*`: `Brush` may come from there, and so may `media`.
        assert_eq!(modules.resolve(prelude, &path("Brush")), None);
        assert_eq!(modules.resolve(prelude, &path("media::Brush")), None);
        assert_eq!(resolved(&modules, prelude, "std::rc::Rc"), Some("::std::rc::Rc".to_string()));
        assert_eq!(resolved(&modules, prelude, "base::Brush"), Some("::base::Brush".to_string()));
        assert_eq!(resolved(&modules, prelude, "Border"), Some("::fixture::border::Border".to_string()));
    }

    /// Not from upstream: the public path of an item is the shortest one another crate can
    /// write; an item no public path leads to has none.
    #[test]
    fn public_paths_are_the_shortest() {
        let (modules, _, _, _) = fixture();
        let paths = modules.public_paths();
        assert_eq!(paths.get("::fixture::border::Border"), Some(&"::fixture::Border".to_string()));
        assert_eq!(paths.get("::fixture::primitives::Popup"), Some(&"::fixture::Popup".to_string()));
        assert_eq!(paths.get("::fixture::primitives::Hidden"), None);
        assert_eq!(paths.len(), 2, "{paths:?}");
    }

    /// Not from upstream: the export table has every path another crate can write for a
    /// type, with the module that declares it.
    #[test]
    fn export_table_has_every_public_path() {
        let (mut modules, _, primitives, _) = fixture();
        modules.add_item(primitives, "helper", ItemKind::Value, true);
        let pair = |path: &str, declared: &str| (path.to_string(), declared.to_string());
        assert_eq!(
            modules.export_table(),
            vec![
                pair("::fixture::Border", "::fixture::border::Border"),
                pair("::fixture::Popup", "::fixture::primitives::Popup"),
                pair("::fixture::primitives::Popup", "::fixture::primitives::Popup"),
            ]
        );
    }

    /// Not from upstream: with the export table of another crate, a name its glob import
    /// brings is the type the table has, by the module that declares it; a name the table
    /// does not have stays unknown.
    #[test]
    fn glob_of_another_crate_is_resolved_by_its_export_table() {
        let mut modules = Modules::new("fixture", &[]);
        let prelude = modules.add_module(0, "prelude", false);
        let nested = modules.add_module(0, "nested", false);
        glob(&mut modules, prelude, "base", false);
        glob(&mut modules, nested, "base::media", false);
        modules.external_crates.insert("base".to_string());
        modules.externals.insert("::base::Brush".to_string(), "::base::media::brush::Brush".to_string());
        modules.externals.insert("::base::media::Brush".to_string(), "::base::media::brush::Brush".to_string());
        modules.externals.insert("::base::media::Color".to_string(), "::base::media::color::Color".to_string());
        modules.finish();
        assert_eq!(resolved(&modules, prelude, "Brush"), Some("::base::media::brush::Brush".to_string()));
        assert_eq!(resolved(&modules, prelude, "media::Color"), Some("::base::media::color::Color".to_string()));
        assert_eq!(resolved(&modules, prelude, "Brush::parse"), Some("::base::media::brush::Brush::parse".to_string()));
        assert_eq!(resolved(&modules, nested, "Color"), Some("::base::media::color::Color".to_string()));
        assert_eq!(modules.resolve(prelude, &path("Color")), None);
        assert_eq!(modules.resolve(prelude, &path("media::Missing")), None);
        assert_eq!(modules.resolve(0, &path("Brush")), None);
    }
}
