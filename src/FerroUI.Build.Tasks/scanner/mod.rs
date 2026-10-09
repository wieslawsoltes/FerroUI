//! The source scanner: reads the sources of a crate and fills the build-time
//! type model from its declarations (docs/porting/xaml.md, 9.5.1 and 9.5.2).
//!
//! # What is read
//!
//! The files of the crate, from its root along the `mod` declarations
//! (`#[path]` included; modules and items under `#[cfg(test)]` are left
//! out), and in them:
//!
//! | Declaration | Gives |
//! |---|---|
//! | `ferro_class!(X: Base ..)` | a class, its base class, its Rust path |
//! | `ferro_static_type!(X)` | the owner type of attached properties |
//! | `ferro_properties! { impl X .. { .. } }`, `ferro_property!` | the registered properties: accessor, kind and value type from the type of the accessor; name, owner and host from the registration in its body; added owners |
//! | `ferro_class_info!(X { new, interfaces, markup })` | the default constructor, the interfaces and the markup metadata of a class |
//! | `ferro_markup_type!`, `ferro_markup_enum!` | the other types, with handles, members and values |
//! | `const NAMESPACES`, `static _: MarkupAssembly` | the namespace table and the assembly |
//! | `const TYPES: &[&TypeInfo]` | the classes the crate registers: a class that is not in the list is marked ([`TypeModel::unregistered`]) |
//! | `MarkupType::register_handle::<H>(..)`, `ValueTypes::register_cast::<A, B>(..)` in a function | one more handle of a type with markup metadata ([`AssemblyModel::handles`]); the casts between Rust types ([`AssemblyModel::casts`]) |
//! | `struct`, `enum`, `trait`, `type`, `use`, `mod`, `bitflags!` | the names of each module, for the resolution of paths; the values of enumerations; the type aliases ([`AssemblyModel::aliases`]) |
//! | `impl X { fn .., const .. }` | the functions of each type ([`Scan::functions`]); the constants an enumeration or a set of flags names as members |
//! | `ferro_rust_paths!` | the public paths the crate states, compared with the ones the scanner finds |
//!
//! Macros are not expanded, except the macros the crate itself defines with
//! a single form of identifiers and types (`ferro_transition_class!`): an
//! invocation of one, as an item or among the members of an `impl` block, is
//! expanded by substitution and the declarations it writes are read.
//!
//! # What is not guessed
//!
//! A declaration whose form a reader does not know is a diagnostic with the
//! file and the line, and is not in the model. A path of a type that the
//! `use` items and the modules of the crate do not resolve stays as written
//! and is listed in [`RustType::unresolved`]. The name of a property whose
//! accessor adds an owner to a property of another crate is nothing: the
//! model of that crate has it.
//!
//! # Seams for the next stages
//!
//! - [`MemberModel::call`](crate::model::MemberModel::call) and
//!   [`AccessorModel::call`](crate::model::AccessorModel::call) are chosen at
//!   the end of the scan ([`crate::call_forms`], 9.5.3), from the resolved path
//!   of the callable, [`Scan::functions`] and the functions the models of the
//!   dependencies list.
//! - Without the models of the crates the crate is built on, paths into them
//!   are absolute as the file spells them. With them
//!   ([`ScanOptions::dependencies`]) a path into such a crate is the path of
//!   the declaring module, a name behind a glob import of such a crate is
//!   resolved, and an owner added to a property of such a crate has the name
//!   of the property.
//! - [`Scan::normalise`] resolves any further type text against the modules
//!   of the scan (the signatures of the handlers of an `x:Class` type).

mod constants;
mod declarations;
mod modules;
mod source;
mod tokens;

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::PathBuf;

use proc_macro2::TokenTree;

use crate::model::{
    AccessorModel, AliasModel, AssemblyModel, AttributeModel, AttributeValueModel, CallableModel, CastModel, EnumMemberModel, ExportModel, GenericModel,
    HandleModel, MemberModel, ParameterModel, PropertyModel, RegisteredKind, RegisteredModel, RegistrationModel, RustType, TypeKind, TypeModel,
    XmlnsDefinitionModel, XmlnsPrefixModel,
};
use crate::model_set::ModelSet;
use declarations::{
    first_line, property_type, read_registration, Accessor, Declaration, EnumMember, MarkupBody, RawAccessor, RawAttribute, RawMember, RawParameter,
    RawProperty, RawValue,
};
use modules::{Modules, Target};
use source::{Located, Site, Source, Text};
use tokens::{pieces, plain_path, text_of, text_of_pieces, tokens_of, Tokens};

/// The names of the declaration macros: the macros whose invocations are counted
/// ([`ScannedFile::invocations`]) and read.
pub const DECLARATION_MACRO_NAMES: &[&str] = declarations::DECLARATION_MACROS;

/// The codes of the diagnostics of the scanner (the range `FRN9xxx` of xaml.md 9.6.5).
pub mod codes {
    /// A file cannot be read, is not Rust the reader parses, or is not found.
    pub const FILE: &str = "FRN9001";
    /// A declaration has a form the scanner does not know.
    pub const FORM: &str = "FRN9010";
    /// A declaration macro is invoked where the scanner does not read.
    pub const POSITION: &str = "FRN9011";
    /// A macro of the crate declares through a declaration macro and is not expanded.
    pub const LOCAL_MACRO: &str = "FRN9012";
    /// A runtime type or markup metadata is implemented by hand.
    pub const HAND_WRITTEN: &str = "FRN9013";
    /// A path of a type is not resolved.
    pub const UNRESOLVED: &str = "FRN9020";
    /// The accessor of a registered property is not read, or its registration is not.
    pub const REGISTRATION: &str = "FRN9021";
    /// Two declarations state the same thing.
    pub const DUPLICATE: &str = "FRN9022";
    /// A declaration names a type the scanner has no declaration of.
    pub const OWNER: &str = "FRN9023";
    /// The public path the crate states for a type is not the one the scanner finds.
    pub const PUBLIC_PATH: &str = "FRN9024";
    /// A class the crate declares is not in its list of registered classes.
    pub const UNREGISTERED: &str = "FRN9025";
    /// The assembly, the namespace table or the list of registered classes is not read.
    pub const ASSEMBLY: &str = "FRN9030";
}

/// What to scan.
#[derive(Clone, Debug)]
pub struct ScanOptions {
    /// The name of the crate, as Rust paths spell it (`ferroui_controls`).
    pub crate_name: String,
    /// The root file of the crate (`lib.rs`).
    pub root: PathBuf,
    /// The assembly name, for a crate that has no `MarkupAssembly`.
    pub assembly_name: Option<String>,
    /// The names of the crates the crate is built on. They are needed only where a file
    /// imports a glob of another crate; elsewhere the `use` items tell.
    pub extern_crates: Vec<String>,
    /// The models of the crates the crate is built on (the ones it depends on directly
    /// and theirs), as their scans wrote them. With them the scan resolves what lives in
    /// those crates: the names behind a glob import of one of their modules, the declaring
    /// module of every type of theirs the crate names (the type texts of the model are
    /// then canonical, [`ModelSet::canonical`]), and the names of their properties the
    /// crate adds owners to.
    pub dependencies: Vec<AssemblyModel>,
}

impl ScanOptions {
    pub fn new(crate_name: &str, root: impl Into<PathBuf>) -> Self {
        Self { crate_name: crate_name.to_string(), root: root.into(), assembly_name: None, extern_crates: Vec::new(), dependencies: Vec::new() }
    }

    /// The scan with the models of the crates the crate is built on.
    pub fn with_dependencies(mut self, dependencies: Vec<AssemblyModel>) -> Self {
        self.dependencies = dependencies;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Something of the crate is not in the model.
    Error,
    /// Something of the model is incomplete.
    Warning,
    /// Something the reader of the model should know.
    Note,
}

/// A diagnostic of the scanner, with the place in the Rust source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    /// One of [`codes`].
    pub code: &'static str,
    pub file: PathBuf,
    /// The line, from 1; 0 when the diagnostic is about the file.
    pub line: usize,
    pub message: String,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let severity = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Note => "note",
        };
        write!(formatter, "{}({}): {severity} {}: {}", self.file.display(), self.line, self.code, self.message)
    }
}

/// How the invocations of one declaration macro in a file were met. Their sum is the
/// number of invocations written in the file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InvocationCounts {
    /// Read into the model.
    pub read: usize,
    /// In a form the scanner does not know (a diagnostic says which).
    pub failed: usize,
    /// Inside a `macro_rules!` definition.
    pub in_macro_definitions: usize,
    /// Where the scanner does not read: inside a function, another item or the arguments
    /// of another macro (a diagnostic says where).
    pub unread: usize,
    /// Inside an item under `#[cfg(test)]`.
    pub in_test_code: usize,
}

impl InvocationCounts {
    pub fn total(&self) -> usize {
        self.read + self.failed + self.in_macro_definitions + self.unread + self.in_test_code
    }

    fn add(&mut self, other: &InvocationCounts) {
        self.read += other.read;
        self.failed += other.failed;
        self.in_macro_definitions += other.in_macro_definitions;
        self.unread += other.unread;
        self.in_test_code += other.in_test_code;
    }
}

/// A file the scanner read.
#[derive(Clone, Debug)]
pub struct ScannedFile {
    pub path: PathBuf,
    /// The module the file is (`ferroui_controls::border`).
    pub module: String,
    /// The invocations of the declaration macros in the file, by the name of the macro.
    pub invocations: BTreeMap<String, InvocationCounts>,
}

/// A function of an inherent `impl` block of the crate, as written: what the choice of a
/// call form (9.5.3) and the handlers of an `x:Class` type (9.4.4) are checked against.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InherentFunction {
    /// The Rust path of the type (`::ferroui_controls::border::Border`), or its name as
    /// written when the scanner did not resolve it.
    pub owner: String,
    pub name: String,
    /// The visibility as written; empty when private.
    pub visibility: String,
    /// Whether the function takes `self`.
    pub receiver: bool,
    /// The types of the parameters after `self`, as written.
    pub parameters: Vec<String>,
    pub return_type: Option<String>,
    /// The macro that declares the function (`ferro_routed_event`), when one does.
    pub declared_by: Option<String>,
    /// The module of the `impl` block.
    pub module: String,
    pub file: PathBuf,
    pub line: usize,
}

/// What a scan found, in numbers.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Statistics {
    pub files: usize,
    pub modules: usize,
    /// The invocations of each declaration macro, over all files.
    pub invocations: BTreeMap<String, InvocationCounts>,
    /// The declarations read from expansions of macros of the crate, by macro.
    pub expanded: BTreeMap<String, usize>,
    pub classes: usize,
    pub static_types: usize,
    pub markup_types: usize,
    pub enums: usize,
    /// The classes and static types that are not in the list of registered classes of
    /// the crate.
    pub unregistered: usize,
    pub types_without_namespace: usize,
    pub types_without_public_path: usize,
    pub registered: usize,
    pub styled: usize,
    pub direct: usize,
    pub attached: usize,
    pub added_owners: usize,
    pub aliases: usize,
    /// Accessors whose body states no registration the scanner reads.
    pub unknown_registrations: usize,
    /// Registered properties whose name is not in the model.
    pub registered_without_name: usize,
    pub constructors: usize,
    pub properties: usize,
    pub indexers: usize,
    pub methods: usize,
    pub fields: usize,
    pub events: usize,
    /// The callables of the members, and how many are paths, and of those how many are
    /// resolved.
    pub callables: usize,
    pub callable_paths: usize,
    pub callable_paths_resolved: usize,
    /// The call forms chosen for the callables (9.5.3): every callable but `new:` of a
    /// class and the source of an added owner, which are form A by their declaration.
    pub call_forms: crate::call_forms::CallFormStatistics,
    pub enum_members: usize,
    pub enum_members_without_value: usize,
    /// The type texts of the model, and how many have an unresolved path.
    pub type_texts: usize,
    pub type_texts_unresolved: usize,
    /// The unresolved paths, as written, with the number of type texts each is in.
    pub unresolved_paths: BTreeMap<String, usize>,
    /// The entries of `ferro_rust_paths!`, and how many are the public path the scanner
    /// finds for the type.
    pub rust_paths_listed: usize,
    pub rust_paths_agreeing: usize,
}

/// The result of a scan.
pub struct Scan {
    pub model: AssemblyModel,
    pub diagnostics: Vec<Diagnostic>,
    pub files: Vec<ScannedFile>,
    pub functions: Vec<InherentFunction>,
    pub statistics: Statistics,
    modules: Modules,
}

impl Scan {
    /// The type text `text`, written in the module `module` of the scanned crate
    /// (`ferroui_controls::border`), normalised as the type texts of the model are.
    /// Nothing when the module is not one of the scan or the text is not tokens.
    pub fn normalise(&self, module: &str, text: &str) -> Option<RustType> {
        let module = self.modules.find_module(module)?;
        let tokens = tokens_of(text.parse().ok()?);
        let (text, unresolved) = normalise(&self.modules, module, &tokens, None, &[]);
        Some(RustType { text, unresolved })
    }

    /// The diagnostics of one severity.
    pub fn diagnostics_of(&self, severity: Severity) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics.iter().filter(move |diagnostic| diagnostic.severity == severity)
    }

    /// The numbers of the scan as text, one per line: what a reader judges the coverage by.
    pub fn summary(&self) -> String {
        let statistics = &self.statistics;
        let mut lines = vec![
            format!("crate {} (assembly `{}`): {} files, {} modules", self.model.crate_name, self.model.name, statistics.files, statistics.modules),
            format!(
                "types: {} ({} classes, {} static types, {} markup types, {} enumerations); {} without a namespace, {} without a public path",
                self.model.types.len(),
                statistics.classes,
                statistics.static_types,
                statistics.markup_types,
                statistics.enums,
                statistics.types_without_namespace,
                statistics.types_without_public_path
            ),
            format!(
                "registered properties: {} ({} styled, {} direct, {} attached); {} added owners, {} aliases, {} registrations not read, {} without a name",
                statistics.registered,
                statistics.styled,
                statistics.direct,
                statistics.attached,
                statistics.added_owners,
                statistics.aliases,
                statistics.unknown_registrations,
                statistics.registered_without_name
            ),
            format!(
                "members: {} constructors, {} plain properties, {} indexers, {} methods, {} fields, {} events",
                statistics.constructors, statistics.properties, statistics.indexers, statistics.methods, statistics.fields, statistics.events
            ),
            format!(
                "callables: {} ({} paths, {} of them resolved; the others are closures)",
                statistics.callables, statistics.callable_paths, statistics.callable_paths_resolved
            ),
            statistics.call_forms.summary(),
            format!("enumeration members: {} ({} without a value)", statistics.enum_members, statistics.enum_members_without_value),
            format!(
                "registration: {} classes are not in the list of registered classes; {} handles registered for types with markup metadata; {} casts; {} type aliases",
                statistics.unregistered,
                self.model.handles.len(),
                self.model.casts.len(),
                self.model.aliases.len()
            ),
            format!(
                "type texts: {} ({} with an unresolved path, {} distinct unresolved paths)",
                statistics.type_texts,
                statistics.type_texts_unresolved,
                statistics.unresolved_paths.len()
            ),
            format!("public paths stated by the crate (ferro_rust_paths!): {} ({} are the path the scanner finds)", statistics.rust_paths_listed, statistics.rust_paths_agreeing),
            format!("inherent functions: {}", self.functions.len()),
        ];
        for (name, counts) in &statistics.invocations {
            lines.push(format!(
                "{name}!: {} invocations ({} read, {} not read: form, {} not read: position, {} in macro definitions, {} in test code)",
                counts.total(),
                counts.read,
                counts.failed,
                counts.unread,
                counts.in_macro_definitions,
                counts.in_test_code
            ));
        }
        for (name, count) in &statistics.expanded {
            lines.push(format!("{name}! (a macro of the crate): {count} declarations read from its expansions"));
        }
        let mut by_code: BTreeMap<(Severity, &str), usize> = BTreeMap::new();
        for diagnostic in &self.diagnostics {
            *by_code.entry((diagnostic.severity, diagnostic.code)).or_default() += 1;
        }
        for ((severity, code), count) in by_code {
            lines.push(format!("diagnostics: {count} {severity:?} {code}"));
        }
        let mut unresolved: Vec<(&String, &usize)> = statistics.unresolved_paths.iter().collect();
        unresolved.sort_by(|left, right| right.1.cmp(left.1).then(left.0.cmp(right.0)));
        for (path, count) in unresolved.into_iter().take(40) {
            lines.push(format!("unresolved: `{path}` in {count} type texts"));
        }
        lines.join("\n")
    }
}

/// The path of the function a closure without parameters dereferences the result of:
/// `|| *Type::function()`, as it is or in parentheses. Nothing for any other expression.
fn dereferenced_call(tokens: &[TokenTree]) -> Option<Vec<String>> {
    let is = |token: Option<&TokenTree>, character: char| matches!(token, Some(TokenTree::Punct(punct)) if punct.as_char() == character);
    match tokens {
        [TokenTree::Group(group)] if group.delimiter() == proc_macro2::Delimiter::Parenthesis => {
            dereferenced_call(&group.stream().into_iter().collect::<Vec<_>>())
        }
        [.., TokenTree::Group(arguments)] if tokens.len() > 4 && arguments.delimiter() == proc_macro2::Delimiter::Parenthesis && arguments.stream().is_empty() => {
            if !(is(tokens.first(), '|') && is(tokens.get(1), '|') && is(tokens.get(2), '*')) {
                return None;
            }
            plain_path(&tokens[3..tokens.len() - 1])
        }
        _ => None,
    }
}

/// Scans the crate `options` describes. The scan does not fail: what it cannot read is in
/// the diagnostics of the result.
pub fn scan_crate(options: &ScanOptions) -> Scan {
    let mut source = Source::read(options);
    let declarations = std::mem::take(&mut source.declarations);
    let mut builder = Builder {
        source,
        model: AssemblyModel::new(options.assembly_name.as_deref().unwrap_or(""), &options.crate_name),
        index: BTreeMap::new(),
        reported: BTreeSet::new(),
        statistics: Statistics::default(),
    };
    builder.assembly(options);
    // The types the crate declares come first: the other declarations name them.
    for declaration in &declarations {
        match &declaration.value {
            Declaration::Class { name, base, .. } => builder.class(name, Some(base), TypeKind::Class, &declaration.site),
            Declaration::StaticType { name } => builder.class(name, None, TypeKind::Static, &declaration.site),
            _ => {}
        }
    }
    builder.hand_written();
    builder.class_lists();
    for declaration in &declarations {
        match &declaration.value {
            Declaration::MarkupType { kind, type_, is_dyn, name, body } => builder.markup_type(*kind, type_, *is_dyn, name.as_deref(), body, &declaration.site),
            Declaration::MarkupEnum { name, flags, members, body } => builder.markup_enum(name, *flags, members, body, &declaration.site),
            _ => {}
        }
    }
    for declaration in &declarations {
        match &declaration.value {
            Declaration::ClassInfo { name, new, interfaces, markup } => builder.class_info(name, new.as_deref(), interfaces, markup.as_ref(), &declaration.site),
            Declaration::Properties { owner, accessors, function_of } => builder.properties(owner, accessors, function_of.as_deref(), &declaration.site),
            _ => {}
        }
    }
    builder.link_properties();
    builder.finish(&options.dependencies)
}

/// The text of the type `tokens` written in `module`: every path the modules resolve made
/// absolute, the others as written and listed. `self_type` is what `Self` stands for;
/// `imports` are the names the function the type is written in imports for itself.
fn normalise(modules: &Modules, module: usize, tokens: &[TokenTree], self_type: Option<&str>, imports: &[(String, Vec<String>)]) -> (String, Vec<String>) {
    let mut unresolved: Vec<String> = Vec::new();
    let text = text_of_pieces(&pieces(tokens, &mut |segments: &[String]| {
        let written = segments.join("::");
        if let (Some(self_type), Some("Self")) = (self_type, segments.first().map(String::as_str)) {
            let mut text = self_type.to_string();
            for segment in &segments[1..] {
                text.push_str("::");
                text.push_str(segment);
            }
            return text;
        }
        let imported: Option<Vec<String>> = imports
            .iter()
            .find(|(name, _)| Some(name) == segments.first())
            .map(|(_, path)| path.iter().chain(&segments[1..]).cloned().collect());
        match modules.resolve(module, imported.as_deref().unwrap_or(segments)) {
            Some(target) => modules.absolute(&target).unwrap_or(written),
            None => {
                if !unresolved.contains(&written) {
                    unresolved.push(written.clone());
                }
                written
            }
        }
    }));
    (text, unresolved)
}

/// Builds the model from what the files state.
struct Builder {
    source: Source,
    model: AssemblyModel,
    /// The types of the model by the text of their Rust path.
    index: BTreeMap<String, usize>,
    /// The unresolved paths already reported, by file, line and path.
    reported: BTreeSet<(usize, usize, String)>,
    statistics: Statistics,
}

impl Builder {
    /// The site of `tokens` inside the declaration at `site`: their own line, unless the
    /// declaration is read from an expansion.
    fn site_of(&self, tokens: &[TokenTree], site: &Site) -> Site {
        if site.expanded {
            site.clone()
        } else {
            Site { line: first_line(tokens, site.line), ..site.clone() }
        }
    }

    fn rust_type(&mut self, tokens: &[TokenTree], site: &Site, self_type: Option<&str>) -> RustType {
        self.rust_type_in(tokens, site, self_type, &[])
    }

    /// The type `tokens` written in a function with the imports `imports`.
    fn rust_type_in(&mut self, tokens: &[TokenTree], site: &Site, self_type: Option<&str>, imports: &[(String, Vec<String>)]) -> RustType {
        let (text, unresolved) = normalise(&self.source.modules, site.module, tokens, self_type, imports);
        self.statistics.type_texts += 1;
        if !unresolved.is_empty() {
            self.statistics.type_texts_unresolved += 1;
            let site = self.site_of(tokens, site);
            for path in &unresolved {
                *self.statistics.unresolved_paths.entry(path.clone()).or_default() += 1;
                if self.reported.insert((site.file, site.line, path.clone())) {
                    let message = format!("the path `{path}` of the type `{text}` is not resolved by the `use` items and the modules of the crate: it stays as written");
                    self.source.diagnostic(Severity::Warning, codes::UNRESOLVED, &site, message);
                }
            }
        }
        RustType { text, unresolved }
    }

    fn rust_types(&mut self, types: &[Tokens], site: &Site, self_type: Option<&str>) -> Vec<RustType> {
        types.iter().map(|tokens| self.rust_type(tokens, site, self_type)).collect()
    }

    /// The absolute text of a path of an expression, when its head is resolved.
    fn resolve_path(&self, module: usize, segments: &[String], self_type: Option<&str>) -> Option<String> {
        if let (Some(self_type), Some("Self")) = (self_type, segments.first().map(String::as_str)) {
            return Some(std::iter::once(self_type.to_string()).chain(segments[1..].iter().cloned()).collect::<Vec<_>>().join("::"));
        }
        let target = self.source.modules.resolve(module, segments)?;
        self.source.modules.absolute(&target)
    }

    fn callable(&mut self, tokens: &[TokenTree], site: &Site, self_type: Option<&str>) -> CallableModel {
        self.statistics.callables += 1;
        match plain_path(tokens) {
            Some(segments) => {
                let resolved = self.resolve_path(site.module, &segments, self_type);
                self.statistics.callable_paths += 1;
                if resolved.is_some() {
                    self.statistics.callable_paths_resolved += 1;
                }
                CallableModel { path: Some(text_of(tokens)), resolved, dereferenced: None }
            }
            None => {
                let dereferenced = dereferenced_call(tokens).and_then(|segments| self.resolve_path(site.module, &segments, self_type));
                CallableModel { path: None, resolved: None, dereferenced }
            }
        }
    }

    /// The type of the model the name `name` stands for in the module of `site`.
    fn find_declared(&self, site: &Site, name: &str) -> Option<usize> {
        let target = self.source.modules.resolve(site.module, &[name.to_string()])?;
        match &target {
            Target::Item { rest, .. } if rest.is_empty() => self.index.get(&self.source.modules.absolute(&target)?).copied(),
            _ => None,
        }
    }

    /// Adds a type to the model. A second type with the Rust type of a known one is added
    /// and reported; the declarations that name the type go to the first.
    fn add_type(&mut self, type_: TypeModel, site: &Site) {
        let key = type_.rust_path.text.clone();
        if self.index.contains_key(&key) {
            let message = format!("`{key}` is declared twice: both declarations are in the model, and the declarations that name the type go to the first");
            self.source.diagnostic(Severity::Warning, codes::DUPLICATE, site, message);
        } else {
            self.index.insert(key, self.model.types.len());
        }
        self.model.types.push(type_);
    }

    /// A class (`ferro_class!`) or a static type (`ferro_static_type!`): declared in the
    /// module of the invocation.
    fn class(&mut self, name: &str, base: Option<&Tokens>, kind: TypeKind, site: &Site) {
        let module = self.source.modules.module_path(site.module);
        let mut type_ = TypeModel::new(name, kind, RustType::resolved(&format!("::{module}::{name}")), &module);
        type_.object_model = true;
        type_.cfg = site.cfg.clone();
        type_.base = base.map(|base| self.rust_type(base, site, None));
        self.add_type(type_, site);
    }

    /// The types whose runtime type is implemented by hand: a class when the type
    /// implements `ObjectType`, else a static type. Their base is not read.
    fn hand_written(&mut self) {
        let hand_written = std::mem::take(&mut self.source.hand_written);
        for type_ in &hand_written {
            let module = self.source.modules.module_path(type_.site.module);
            let key = format!("::{module}::{}", type_.name);
            let kind = if type_.class { TypeKind::Class } else { TypeKind::Static };
            match self.index.get(&key) {
                Some(index) => {
                    self.model.types[*index].object_model = true;
                    if type_.class {
                        self.model.types[*index].kind = TypeKind::Class;
                    }
                }
                None => {
                    let mut model = TypeModel::new(&type_.name, kind, RustType::resolved(&key), &module);
                    model.object_model = true;
                    model.cfg = type_.site.cfg.clone();
                    self.add_type(model, &type_.site);
                    let message = format!("the runtime type of `{}` is implemented by hand (`impl StaticType`): the model has the type, and not its base", type_.name);
                    self.source.diagnostic(Severity::Note, codes::HAND_WRITTEN, &type_.site, message);
                }
            }
        }
    }

    /// Marks the classes and static types that are not in the lists of registered classes
    /// of the crate. A crate without such a list, or with a list the scanner did not
    /// read, has no marked type: nothing says which of its types it registers.
    fn class_lists(&mut self) {
        let lists = std::mem::take(&mut self.source.class_lists);
        let (Some(first), true) = (lists.first(), lists.iter().all(|list| list.value.is_some())) else { return };
        let mut listed: BTreeSet<usize> = BTreeSet::new();
        for Located { site, value } in &lists {
            for entry in value.iter().flatten() {
                let path = plain_path(entry).and_then(|segments| self.resolve_path(site.module, &segments, None));
                match path.and_then(|path| self.index.get(&path).copied()) {
                    Some(index) => {
                        listed.insert(index);
                    }
                    None => {
                        let message = format!("the list of registered classes names `{}`, which is no class or static type of the crate the scanner read", text_of(entry));
                        self.source.diagnostic(Severity::Warning, codes::OWNER, site, message);
                    }
                }
            }
        }
        for index in 0..self.model.types.len() {
            if self.model.types[index].object_model && !listed.contains(&index) {
                self.model.types[index].unregistered = true;
                let message = format!(
                    "`{}` is declared and is not in the list of registered classes: the type is not known by its name or by its handle until an instance of it is created",
                    self.model.types[index].rust_path.text
                );
                self.source.diagnostic(Severity::Note, codes::UNREGISTERED, &first.site, message);
            }
        }
    }

    fn markup_type(&mut self, kind: TypeKind, type_: &Tokens, is_dyn: bool, name: Option<&str>, body: &MarkupBody, site: &Site) {
        // The metadata of a static type that owns attached properties belongs to its
        // runtime type (`type_info: X`).
        if let Some(type_info) = &body.type_info {
            if let Some(index) = plain_path(type_info).filter(|path| path.len() == 1).and_then(|path| self.find_declared(site, &path[0])) {
                let self_type = self.model.types[index].rust_path.text.clone();
                self.apply_markup(index, body, site, &self_type);
                return;
            }
        }
        let mut rust_path = self.rust_type(type_, site, None);
        if is_dyn {
            rust_path.text = format!("dyn {}", rust_path.text);
        }
        let name = name.map(str::to_string).unwrap_or_else(|| text_of(type_));
        let module = self.source.modules.module_path(site.module);
        let self_type = rust_path.text.clone();
        let mut model = TypeModel::new(&name, kind, rust_path, &module);
        model.cfg = site.cfg.clone();
        let index = self.model.types.len();
        self.add_type(model, site);
        self.apply_markup(index, body, site, &self_type);
    }

    fn markup_enum(&mut self, name: &str, flags: bool, members: &[EnumMember], body: &MarkupBody, site: &Site) {
        let segments = [name.to_string()];
        let target = self.source.modules.resolve(site.module, &segments);
        // The name is read again as tokens, which have no line of the file: the line of a
        // diagnostic is the one of the declaration.
        let rust_path = self.rust_type(&tokens_of(name.parse().unwrap_or_default()), &Site { expanded: true, ..site.clone() }, None);
        let declared = match &target {
            Some(Target::Item { module, name, rest }) if rest.is_empty() => Some((*module, name.clone())),
            _ => None,
        };
        let module = self.source.modules.module_path(site.module);
        let mut model = TypeModel::new(name, TypeKind::Enum, rust_path, &module);
        model.cfg = site.cfg.clone();
        model.is_flags = flags;
        for member in members {
            let mut value = None;
            let mut rust_value = None;
            let mut rust_variant = None;
            if flags {
                let tokens = member.value.clone().unwrap_or_default();
                rust_value = Some(text_of(&tokens));
                // `Type::CONSTANT` of the `bitflags!` type the enumeration is: a constant of
                // the `bitflags!` invocation or an associated constant of the type.
                if let (Some(path), Some(declared)) = (plain_path(&tokens), &declared) {
                    let same_type = path.len() == 2 && (path[0] == "Self" || self.source.modules.resolve(site.module, &path[..1]) == target);
                    if same_type {
                        value = self.source.member_value(declared, &path[1], 0);
                    }
                }
            } else {
                let variant = member.variant.clone().unwrap_or_else(|| member.name.clone());
                // A variant, or an associated constant of the enumeration that names one
                // (`pub const Enter: Key = Key::Return;`).
                if let Some(declared) = &declared {
                    value = self.source.member_value(declared, &variant, 0);
                }
                rust_variant = Some(variant);
            }
            model.enum_members.push(EnumMemberModel { name: member.name.clone(), rust_variant, rust_value, value });
        }
        let self_type = model.rust_path.text.clone();
        let index = self.model.types.len();
        self.add_type(model, site);
        self.apply_markup(index, body, site, &self_type);
    }

    fn class_info(&mut self, name: &str, new: Option<&[TokenTree]>, interfaces: &[Tokens], markup: Option<&MarkupBody>, site: &Site) {
        let Some(index) = self.find_declared(site, name) else {
            let message = format!("`ferro_class_info!` of `{name}`: no class `{name}` is read (its `ferro_class!` is not in a file the scanner read, or the name is not resolved here): the declaration is not in the model");
            self.source.diagnostic(Severity::Error, codes::OWNER, site, message);
            return;
        };
        let self_type = self.model.types[index].rust_path.text.clone();
        if let Some(new) = new {
            let callable = self.callable(new, site, Some(&self_type));
            self.model.types[index].default_constructor = Some(callable);
        }
        let interfaces = self.rust_types(interfaces, site, Some(&self_type));
        self.model.types[index].interfaces.extend(interfaces);
        if let Some(markup) = markup {
            self.apply_markup(index, markup, site, &self_type);
        }
    }

    /// Adds the parts of markup metadata to the type `index`.
    fn apply_markup(&mut self, index: usize, body: &MarkupBody, site: &Site, self_type: &str) {
        let self_type = Some(self_type);
        let handles = self.rust_types(&body.handles, site, self_type);
        let interfaces = self.rust_types(&body.interfaces, site, self_type);
        let this = body.this.as_ref().map(|tokens| self.rust_type(tokens, site, self_type));
        let base = body.base.as_ref().map(|tokens| self.rust_type(tokens, site, self_type));
        let type_info = body.type_info.as_ref().map(|tokens| self.rust_type(tokens, site, self_type));
        let notify = body.notify_property_changed.as_ref().map(|tokens| self.rust_type(tokens, site, self_type));
        let generic = body.generic.as_ref().map(|(definition, arguments)| GenericModel { definition: definition.clone(), arguments: self.rust_types(arguments, site, self_type) });
        let parse = body.parse.as_ref().map(|tokens| self.callable(tokens, site, self_type));
        let constructors: Vec<MemberModel> =
            body.constructors.iter().enumerate().map(|(position, raw)| self.member(raw, site, self_type, Some(format!("__markup_new_{position}")))).collect();
        let methods: Vec<MemberModel> = body
            .methods
            .iter()
            .enumerate()
            .map(|(position, raw)| self.member(raw, site, self_type, Some(format!("__markup_{}_{position}", raw.name))))
            .collect();
        let fields: Vec<MemberModel> = body.fields.iter().map(|raw| self.member(raw, site, self_type, Some(format!("__markup_field_{}", raw.name)))).collect();
        let events: Vec<MemberModel> = body.events.iter().map(|raw| self.member(raw, site, self_type, None)).collect();
        let properties: Vec<PropertyModel> = body.properties.iter().map(|raw| self.property(raw, site, self_type, Some("__markup_"))).collect();
        let static_properties: Vec<PropertyModel> = body.static_properties.iter().map(|raw| self.property(raw, site, self_type, Some("__markup_static_"))).collect();
        let indexers: Vec<PropertyModel> = body.indexers.iter().map(|raw| self.property(raw, site, self_type, None)).collect();
        let attributes = self.attributes(&body.attributes, site, self_type);
        let property_attributes: Vec<(String, Vec<AttributeModel>)> =
            body.property_attributes.iter().map(|(name, attributes)| (name.clone(), self.attributes(attributes, site, self_type))).collect();

        let type_ = &mut self.model.types[index];
        if body.namespace.is_some() {
            type_.explicit_namespace = body.namespace.clone();
        }
        type_.handles.extend(handles);
        type_.interfaces.extend(interfaces);
        type_.this = this.or(type_.this.take());
        // The base of a class of the object model is its base class; `base:` is for the others.
        type_.base = type_.base.take().or(base);
        type_.type_info = type_info.or(type_.type_info.take());
        type_.notify_property_changed = notify.or(type_.notify_property_changed.take());
        type_.generic = generic.or(type_.generic.take());
        type_.parse = parse.or(type_.parse.take());
        if body.content.is_some() {
            type_.content_property = body.content.clone();
        }
        type_.constructors.extend(constructors);
        type_.methods.extend(methods);
        type_.fields.extend(fields);
        type_.events.extend(events);
        type_.properties.extend(properties);
        type_.static_properties.extend(static_properties);
        type_.indexers.extend(indexers);
        type_.attributes.extend(attributes);
        type_.property_attributes.extend(property_attributes);
    }

    fn member(&mut self, raw: &RawMember, site: &Site, self_type: Option<&str>, typed_function: Option<String>) -> MemberModel {
        MemberModel {
            name: raw.name.clone(),
            parameters: raw.parameters.iter().map(|parameter| self.parameter(parameter, site, self_type)).collect(),
            return_type: raw.return_type.as_ref().map(|tokens| self.rust_type(tokens, site, self_type)),
            is_static: raw.is_static,
            fallible: raw.fallible,
            attributes: self.attributes(&raw.attributes, site, self_type),
            callable: self.callable(&raw.callable, site, self_type),
            typed_function,
            call: None,
        }
    }

    fn parameter(&mut self, raw: &RawParameter, site: &Site, self_type: Option<&str>) -> ParameterModel {
        ParameterModel { name: raw.name.clone(), type_: self.rust_type(&raw.type_, site, self_type), attributes: self.attributes(&raw.attributes, site, self_type) }
    }

    /// `prefix` is the start of the names of the typed functions of the accessors
    /// (`__markup_` for `__markup_get_Name`); nothing for an indexer, which has none.
    fn property(&mut self, raw: &RawProperty, site: &Site, self_type: Option<&str>, prefix: Option<&str>) -> PropertyModel {
        let accessor = |builder: &mut Self, raw_accessor: &Option<RawAccessor>, which: &str| {
            raw_accessor.as_ref().map(|raw_accessor| AccessorModel {
                fallible: raw_accessor.fallible,
                callable: builder.callable(&raw_accessor.callable, site, self_type),
                typed_function: prefix.map(|prefix| format!("{prefix}{which}_{}", raw.name)),
                call: None,
            })
        };
        let getter = accessor(self, &raw.getter, "get");
        let setter = accessor(self, &raw.setter, "set");
        PropertyModel {
            name: raw.name.clone(),
            parameters: raw.parameters.iter().map(|parameter| self.parameter(parameter, site, self_type)).collect(),
            value_type: self.rust_type(&raw.type_, site, self_type),
            getter,
            setter,
            attributes: self.attributes(&raw.attributes, site, self_type),
        }
    }

    fn attributes(&mut self, raw: &[RawAttribute], site: &Site, self_type: Option<&str>) -> Vec<AttributeModel> {
        raw.iter()
            .map(|attribute| AttributeModel {
                name: attribute.name.clone(),
                arguments: attribute.arguments.iter().map(|value| self.value(value, site, self_type)).collect(),
                properties: attribute.properties.iter().map(|(name, value)| (name.clone(), self.value(value, site, self_type))).collect(),
            })
            .collect()
    }

    fn value(&mut self, raw: &RawValue, site: &Site, self_type: Option<&str>) -> AttributeValueModel {
        match raw {
            RawValue::Null => AttributeValueModel::Null,
            RawValue::Bool(value) => AttributeValueModel::Bool(*value),
            RawValue::Int(value) => AttributeValueModel::Int(*value),
            RawValue::Float(text) => AttributeValueModel::Float(text.clone()),
            RawValue::Str(text) => AttributeValueModel::Str(text.clone()),
            RawValue::Type(tokens) => AttributeValueModel::Type(self.rust_type(tokens, site, self_type)),
            RawValue::Array(items) => AttributeValueModel::Array(items.iter().map(|item| self.value(item, site, self_type)).collect()),
        }
    }

    /// The accessors of the registered properties of `owner`. `function_of` is the name of
    /// the type whose `impl` block has them, when it is not the owner.
    fn properties(&mut self, owner: &str, accessors: &[Accessor], function_of: Option<&str>, site: &Site) {
        let Some(index) = self.find_declared(site, owner) else {
            let message = format!("the properties of `{owner}`: no class or static type `{owner}` is read (no `ferro_class!` or `ferro_static_type!` of it in a file the scanner read): the properties are not in the model");
            self.source.diagnostic(Severity::Error, codes::OWNER, site, message);
            return;
        };
        let self_type = self.model.types[index].rust_path.text.clone();
        let function_of = function_of.map(|name| self.resolve_path(site.module, &[name.to_string()], None).unwrap_or_else(|| name.to_string()));
        for accessor in accessors {
            let site = if site.expanded { site.clone() } else { Site { line: accessor.line, ..site.clone() } };
            let Some((kind, value_type)) = property_type(&accessor.return_type) else {
                let message = format!(
                    "the accessor `{}` returns `{}`, which is not a property definition (`StyledProperty<T>`, `AttachedProperty<T>`, `DirectProperty<Owner, T>`): it is not in the model",
                    accessor.name,
                    text_of(&accessor.return_type)
                );
                self.source.diagnostic(Severity::Warning, codes::REGISTRATION, &site, message);
                continue;
            };
            let value_type = self.rust_type(&value_type, &site, Some(&self_type));
            let registration = read_registration(&accessor.body);
            let mut model = RegisteredModel {
                name: registration.name.clone(),
                kind,
                value_type,
                owner: None,
                host: None,
                accessor: accessor.name.clone(),
                function_of: function_of.clone(),
                visibility: accessor.visibility.clone(),
                registration: registration.kind,
                source: None,
                assign_binding: registration.assign_binding,
                inherits: registration.inherits,
                read_only: kind == RegisteredKind::Direct && registration.read_only,
                added_owners: Vec::new(),
            };
            match registration.kind {
                RegistrationModel::Declared => {
                    // `register::<Owner, T>`, `register_attached::<Owner, Host, T>`.
                    model.owner = registration.type_arguments.first().map(|tokens| self.rust_type(tokens, &site, Some(&self_type)));
                    if kind == RegisteredKind::Attached && registration.type_arguments.len() == 3 {
                        model.host = Some(self.rust_type(&registration.type_arguments[1], &site, Some(&self_type)));
                    }
                    if model.name.is_none() {
                        let message = format!("the accessor `{}` registers a property whose name is not a string literal: the name is not in the model", accessor.name);
                        self.source.diagnostic(Severity::Warning, codes::REGISTRATION, &site, message);
                    }
                }
                RegistrationModel::AddedOwner | RegistrationModel::Alias => {
                    model.source = registration.source.as_ref().map(|tokens| self.callable(tokens, &site, Some(&self_type)));
                    // `.add_owner::<Owner>(..)`.
                    model.owner = registration.added_owner.as_ref().map(|tokens| self.rust_type(tokens, &site, Some(&self_type)));
                }
                RegistrationModel::Unknown => {
                    let message = format!(
                        "the body of the accessor `{}` states no registration the scanner reads (`FerroProperty::register*::<..>(\"Name\", ..)`, `Other::accessor().add_owner*::<..>(..)`, `Other::accessor()`): the name of the property is not in the model",
                        accessor.name
                    );
                    self.source.diagnostic(Severity::Warning, codes::REGISTRATION, &site, message);
                }
            }
            self.model.types[index].registered.push(model);
        }
    }

    /// Gives an added owner and an alias the name of the property its source declares,
    /// and the declaration its added owners, where the source is in the scanned crate.
    fn link_properties(&mut self) {
        let mut names: Vec<(usize, usize, Option<String>)> = Vec::new();
        let mut owners: Vec<(usize, usize, String)> = Vec::new();
        // The accessors that are functions of another type than the one they are listed
        // under, by their path.
        let mut functions: BTreeMap<String, (usize, usize)> = BTreeMap::new();
        for (type_index, type_) in self.model.types.iter().enumerate() {
            for (position, registered) in type_.registered.iter().enumerate() {
                if let Some(function_of) = &registered.function_of {
                    functions.entry(format!("{function_of}::{}", registered.accessor)).or_insert((type_index, position));
                }
            }
        }
        for (type_index, type_) in self.model.types.iter().enumerate() {
            for (position, registered) in type_.registered.iter().enumerate() {
                if !matches!(registered.registration, RegistrationModel::AddedOwner | RegistrationModel::Alias) {
                    continue;
                }
                let mut current = registered;
                for _ in 0..8 {
                    let Some((source_type, source_position)) = current.source.as_ref().and_then(|source| source.resolved.as_deref()).and_then(|path| self.locate(path, &functions)) else {
                        break;
                    };
                    current = &self.model.types[source_type].registered[source_position];
                    if current.registration == RegistrationModel::Declared {
                        names.push((type_index, position, current.name.clone()));
                        if registered.registration == RegistrationModel::AddedOwner {
                            owners.push((source_type, source_position, type_.rust_path.text.clone()));
                        }
                        break;
                    }
                }
            }
        }
        for (type_index, position, name) in names {
            self.model.types[type_index].registered[position].name = name;
        }
        for (type_index, position, owner) in owners {
            let added = &mut self.model.types[type_index].registered[position].added_owners;
            if !added.contains(&owner) {
                added.push(owner);
            }
        }
    }

    /// The type and the position of the accessor with the absolute path `path`
    /// (`::crate::decorator::Decorator::child_property`): the path of the type the
    /// accessor is a function of, which `functions` has for the accessors listed under
    /// another type.
    fn locate(&self, path: &str, functions: &BTreeMap<String, (usize, usize)>) -> Option<(usize, usize)> {
        if let Some(found) = functions.get(path) {
            return Some(*found);
        }
        let (type_path, accessor) = path.rsplit_once("::")?;
        let type_index = *self.index.get(type_path)?;
        let position = self.model.types[type_index].registered.iter().position(|registered| registered.accessor == accessor && registered.function_of.is_none())?;
        Some((type_index, position))
    }

    /// The type aliases, the registered handles and the registered casts of the crate.
    fn aliases_and_handles(&mut self) {
        // An alias under a `cfg` condition, or declared twice, is left a name: which of
        // its declarations holds is decided by the build.
        let aliases = std::mem::take(&mut self.source.aliases);
        let mut declared: BTreeMap<String, usize> = BTreeMap::new();
        let paths: Vec<String> = aliases.iter().map(|alias| format!("::{}::{}", self.source.modules.module_path(alias.site.module), alias.value.0)).collect();
        for path in &paths {
            *declared.entry(path.clone()).or_default() += 1;
        }
        for (alias, path) in aliases.iter().zip(paths) {
            if !alias.site.cfg.is_empty() || declared.get(&path) != Some(&1) {
                continue;
            }
            // Its type is no type text of a declaration: it is not counted and not reported,
            // and an alias whose type is not resolved is left a name.
            let (text, unresolved) = normalise(&self.source.modules, alias.site.module, &alias.value.1, None, &[]);
            if unresolved.is_empty() {
                self.model.aliases.push(AliasModel { path, target: RustType { text, unresolved } });
            }
        }
        let handles = std::mem::take(&mut self.source.handles);
        for Located { site, value } in &handles {
            let handle = self.rust_type_in(&value.handle, site, None, &value.imports);
            let type_ = self.rust_type_in(&value.type_, site, None, &value.imports);
            self.model.handles.push(HandleModel { handle, type_ });
        }
        let casts = std::mem::take(&mut self.source.casts);
        for Located { site, value } in &casts {
            let from = self.rust_type_in(&value.from, site, None, &value.imports);
            let to = self.rust_type_in(&value.to, site, None, &value.imports);
            self.model.casts.push(CastModel { from, to });
        }
    }

    /// The value of a text of the assembly or of the namespace table: a literal, a text
    /// constant of the crate, or one of the constants of the base crate an assembly names.
    fn text_value(&self, text: &Text, module: usize) -> Result<String, String> {
        match text {
            Text::Literal(text) => Ok(text.clone()),
            Text::Other(written) => Err(written.clone()),
            Text::Path(segments) => {
                let written = segments.join("::");
                let value = match self.source.modules.resolve(module, segments) {
                    Some(Target::Item { module, name, rest }) if rest.is_empty() => self.source.constants.get(&(module, name)).cloned(),
                    Some(Target::Item { name, rest, .. }) if rest.len() == 1 => self.source.associated_constants.get(&(name, rest[0].clone())).cloned(),
                    Some(Target::External(segments)) if segments.first().map(String::as_str) == Some("ferroui_base") => {
                        let last: Vec<&str> = segments.iter().rev().take(2).map(String::as_str).collect();
                        match last.as_slice() {
                            ["FERRO_XML_NAMESPACE", ..] => Some(ferroui_base::metadata::FERRO_XML_NAMESPACE.to_string()),
                            ["CREATE_SOURCE_INFO", "MarkupAssembly"] => Some(ferroui_base::metadata::MarkupAssembly::CREATE_SOURCE_INFO.to_string()),
                            _ => None,
                        }
                    }
                    _ => None,
                };
                value.ok_or(written)
            }
        }
    }

    /// The value of a text that may be absent; nothing when it is absent or not read.
    fn optional_text(&self, text: &Option<Text>, module: usize) -> Option<String> {
        text.as_ref().and_then(|text| self.text_value(text, module).ok())
    }

    fn text_pairs(&mut self, pairs: &[(Text, Text)], site: &Site, what: &str) -> Vec<(String, String)> {
        let mut values = Vec::new();
        for (first, second) in pairs {
            match (self.text_value(first, site.module), self.text_value(second, site.module)) {
                (Ok(first), Ok(second)) => values.push((first, second)),
                (Err(written), _) | (_, Err(written)) => {
                    let message = format!("`{written}` in {what} is not a string literal or a text constant the scanner reads: the entry is left out");
                    self.source.diagnostic(Severity::Error, codes::ASSEMBLY, site, message);
                }
            }
        }
        values
    }

    /// The assembly and the namespace table of the crate.
    fn assembly(&mut self, options: &ScanOptions) {
        let namespaces = std::mem::take(&mut self.source.namespaces);
        for Located { site, value } in &namespaces {
            let pairs = self.text_pairs(value, site, "the namespace table");
            self.model.namespaces.extend(pairs);
        }
        let assemblies = std::mem::take(&mut self.source.assemblies);
        let own = assemblies
            .iter()
            .find(|assembly| self.optional_text(&assembly.crate_name, assembly.site.module).as_deref() == Some(options.crate_name.as_str()))
            .or(if assemblies.len() == 1 { assemblies.first() } else { None });
        let Some(assembly) = own else {
            if let Some(other) = assemblies.first() {
                let message = format!("none of the {} `MarkupAssembly` values of the crate names the crate `{}`: the assembly is not read", assemblies.len(), options.crate_name);
                self.source.diagnostic(Severity::Error, codes::ASSEMBLY, &other.site, message);
            }
            return;
        };
        match self.optional_text(&assembly.name, assembly.site.module) {
            Some(name) => self.model.name = name,
            None => {
                let message = "the name of the `MarkupAssembly` is not a string literal or a text constant the scanner reads".to_string();
                self.source.diagnostic(Severity::Error, codes::ASSEMBLY, &assembly.site, message);
            }
        }
        let definitions = self.text_pairs(&assembly.xmlns_definitions, &assembly.site, "the xmlns definitions of the assembly");
        self.model.xmlns_definitions = definitions.into_iter().map(|(xml_namespace, namespace)| XmlnsDefinitionModel { xml_namespace, namespace }).collect();
        let prefixes = self.text_pairs(&assembly.xmlns_prefixes, &assembly.site, "the xmlns prefixes of the assembly");
        self.model.xmlns_prefixes = prefixes.into_iter().map(|(xml_namespace, prefix)| XmlnsPrefixModel { xml_namespace, prefix }).collect();
        self.model.metadata = self.text_pairs(&assembly.metadata, &assembly.site, "the metadata of the assembly");
    }

    /// What the models of the crates the crate is built on give the model: every type
    /// text and every resolved callable is written with the paths of the declaring
    /// modules, and an owner added to (or an alias of) a property of one of those crates
    /// has the name of the property.
    fn link_dependencies(&mut self, dependencies: &[AssemblyModel]) {
        if dependencies.is_empty() {
            return;
        }
        let set = ModelSet::new(dependencies.to_vec());
        let mut canonical = |rust_type: &mut RustType| {
            if rust_type.text.contains("::") {
                rust_type.text = set.canonical(&rust_type.text);
            }
        };
        self.model.visit_types_mut(&mut canonical);
        for type_ in &mut self.model.types {
            type_.visit_types_mut(&mut canonical);
            type_.visit_callables_mut(&mut |callable: &mut CallableModel| {
                for path in [&mut callable.resolved, &mut callable.dereferenced] {
                    let canonical = path.as_deref().map(|path| set.canonical_path(path));
                    if canonical.is_some() {
                        *path = canonical;
                    }
                }
            });
        }
        // The names: an accessor is followed to the declaration of its property through
        // the accessors of this crate and of the other crates alike.
        let mut models = dependencies.to_vec();
        models.push(self.model.clone());
        let own = models.len() - 1;
        let set = ModelSet::new(models);
        let mut names: Vec<(usize, usize, String)> = Vec::new();
        for (type_index, type_) in set.models()[own].types.iter().enumerate() {
            for (position, registered) in type_.registered.iter().enumerate() {
                if registered.name.is_some() {
                    continue;
                }
                if let Some(name) = set.name_of(registered) {
                    names.push((type_index, position, name.to_string()));
                }
            }
        }
        for (type_index, position, name) in names {
            self.model.types[type_index].registered[position].name = Some(name);
        }
    }

    /// The namespaces and the public paths of the types, the functions, the numbers.
    fn finish(mut self, dependencies: &[AssemblyModel]) -> Scan {
        let public = self.source.modules.public_paths();
        for index in 0..self.model.types.len() {
            let namespace = match &self.model.types[index].explicit_namespace {
                Some(namespace) => namespace.clone(),
                None => self.model.namespace_of_module(&self.model.types[index].module).to_string(),
            };
            let type_ = &mut self.model.types[index];
            type_.namespace = namespace;
            let path = type_.rust_path.text.strip_prefix("dyn ").unwrap_or(&type_.rust_path.text);
            type_.public_path = public.get(path).cloned();
        }
        self.model.exports = self.source.modules.export_table().into_iter().map(|(path, declared)| ExportModel { path, declared }).collect();
        self.aliases_and_handles();
        self.link_dependencies(dependencies);

        // The public paths the crate states for the emitter, against the ones found here.
        let rust_paths = std::mem::take(&mut self.source.rust_paths);
        for Located { site, value } in &rust_paths {
            self.statistics.rust_paths_listed += 1;
            let stated = format!("::{}", std::iter::once(self.model.crate_name.as_str()).chain(value.iter().skip(1).map(String::as_str)).collect::<Vec<_>>().join("::"));
            let declared = self.source.modules.resolve(site.module, value).and_then(|target| self.source.modules.absolute(&target));
            let found = declared.as_ref().and_then(|declared| public.get(declared));
            if found == Some(&stated) {
                self.statistics.rust_paths_agreeing += 1;
            } else {
                let message = match found {
                    Some(found) => format!("`ferro_rust_paths!` states the public path `{stated}`; the scanner finds `{found}`"),
                    None => format!("`ferro_rust_paths!` states the public path `{stated}`; the scanner finds no public path of that item"),
                };
                self.source.diagnostic(Severity::Note, codes::PUBLIC_PATH, site, message);
            }
        }

        let functions = std::mem::take(&mut self.source.functions);
        let functions: Vec<InherentFunction> = functions
            .into_iter()
            .map(|function| InherentFunction {
                owner: self.resolve_path(function.site.module, &[function.owner.clone()], None).unwrap_or(function.owner),
                name: function.name,
                visibility: function.visibility,
                receiver: function.receiver,
                parameters: function.parameters,
                return_type: function.return_type,
                declared_by: function.declared_by,
                module: self.source.modules.module_path(function.site.module),
                file: self.source.files[function.site.file].path.clone(),
                line: function.site.line,
            })
            .collect();

        // The call form of every callable (9.5.3), and the public functions of the crate
        // for the crates built on it.
        let call_forms = crate::call_forms::choose(&mut self.model, &functions, &public, dependencies);

        let mut statistics = self.statistics;
        statistics.call_forms = call_forms;
        statistics.files = self.source.files.len();
        statistics.modules = self.source.modules.modules.len();
        statistics.expanded = std::mem::take(&mut self.source.expanded);
        for file in &self.source.files {
            for (name, counts) in &file.invocations {
                statistics.invocations.entry(name.clone()).or_default().add(counts);
            }
        }
        for type_ in &self.model.types {
            let object_model = type_.handles.is_empty() && type_.rust_path.text == format!("::{}::{}", type_.module, type_.name);
            match type_.kind {
                TypeKind::Enum => statistics.enums += 1,
                TypeKind::Class if object_model => statistics.classes += 1,
                TypeKind::Static if object_model => statistics.static_types += 1,
                _ => statistics.markup_types += 1,
            }
            if type_.unregistered {
                statistics.unregistered += 1;
            }
            if type_.namespace.is_empty() {
                statistics.types_without_namespace += 1;
            }
            if type_.public_path.is_none() {
                statistics.types_without_public_path += 1;
            }
            statistics.constructors += type_.constructors.len();
            statistics.properties += type_.properties.len() + type_.static_properties.len();
            statistics.indexers += type_.indexers.len();
            statistics.methods += type_.methods.len();
            statistics.fields += type_.fields.len();
            statistics.events += type_.events.len();
            statistics.enum_members += type_.enum_members.len();
            statistics.enum_members_without_value += type_.enum_members.iter().filter(|member| member.value.is_none()).count();
            for registered in &type_.registered {
                statistics.registered += 1;
                match registered.kind {
                    RegisteredKind::Styled => statistics.styled += 1,
                    RegisteredKind::Direct => statistics.direct += 1,
                    RegisteredKind::Attached => statistics.attached += 1,
                }
                match registered.registration {
                    RegistrationModel::Declared => {}
                    RegistrationModel::AddedOwner => statistics.added_owners += 1,
                    RegistrationModel::Alias => statistics.aliases += 1,
                    RegistrationModel::Unknown => statistics.unknown_registrations += 1,
                }
                if registered.name.is_none() {
                    statistics.registered_without_name += 1;
                }
            }
        }
        let mut diagnostics = self.source.diagnostics;
        diagnostics.sort_by(|left, right| (&left.file, left.line, left.severity).cmp(&(&right.file, right.line, right.severity)));
        Scan { model: self.model, diagnostics, files: self.source.files, functions, statistics, modules: self.source.modules }
    }
}
