//! The files of a crate: read from the crate root along the `mod`
//! declarations (`#[path]` included), parsed on file level with `syn`, and
//! searched for the declarations the model is made of. Macros are not
//! expanded, with one exception: an invocation of a macro the crate itself
//! defines with one of the simple forms the framework uses
//! (`ferro_transition_class!`) is expanded by substitution, so that the
//! declarations it writes are read like the others.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use proc_macro2::{Delimiter, Group, TokenTree};
use quote::ToTokens;
use syn::visit::Visit;

use super::constants::{evaluate, Scope};
use super::declarations::{read_declaration, read_property, Accessor, Declaration, DECLARATION_MACROS};
use super::modules::{Glob, Import, ItemKind, Modules};
use super::tokens::{
    calls_of, group_of, ident_of, invocations, is_arrow, is_punct, stated_calls_of, split_types, text_of, tokens_of, with_parenthesised_break_values, Cursor, ParseError, Tokens, TypeEnd,
};
use super::{codes, Diagnostic, InvocationCounts, ScanOptions, ScannedFile, Severity};
use crate::model::VALUE_REGISTRATIONS;

/// Where a declaration is written.
#[derive(Clone)]
pub(crate) struct Site {
    pub module: usize,
    pub file: usize,
    pub line: usize,
    /// The `cfg` conditions around it, outermost first.
    pub cfg: Vec<String>,
    /// The declaration is read from the expansion of a macro of the crate: its tokens are
    /// the ones of the definition, and `line` is the line of the invocation.
    pub expanded: bool,
}

pub(crate) struct Located<T> {
    pub site: Site,
    pub value: T,
}

/// How an invocation of a declaration macro was met.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Category {
    Read,
    Failed,
    MacroDefinition,
    Unread,
    TestCode,
}

fn add_count(counts: &mut BTreeMap<String, InvocationCounts>, name: &str, category: Category) {
    let counts = counts.entry(name.to_string()).or_default();
    match category {
        Category::Read => counts.read += 1,
        Category::Failed => counts.failed += 1,
        Category::MacroDefinition => counts.in_macro_definitions += 1,
        Category::Unread => counts.unread += 1,
        Category::TestCode => counts.in_test_code += 1,
    }
}

/// A text value of a constant expression: a literal, a path to a constant, or anything else.
pub(crate) enum Text {
    Literal(String),
    Path(Vec<String>),
    Other(String),
}

/// `static X: MarkupAssembly = MarkupAssembly { .. }`, as written.
pub(crate) struct RawAssembly {
    pub name: Option<Text>,
    pub crate_name: Option<Text>,
    pub xmlns_definitions: Vec<(Text, Text)>,
    pub xmlns_prefixes: Vec<(Text, Text)>,
    pub metadata: Vec<(Text, Text)>,
    pub site: Site,
}

/// A function of an inherent `impl` block, as written.
pub(crate) struct RawFunction {
    pub owner: String,
    pub name: String,
    pub visibility: String,
    pub receiver: bool,
    pub parameters: Vec<String>,
    pub return_type: Option<String>,
    /// The macro that declares the function (`ferro_routed_event`), when one does.
    pub declared_by: Option<String>,
    pub site: Site,
}

/// A type whose runtime type is implemented by hand (`impl StaticType for X`).
pub(crate) struct HandWritten {
    pub name: String,
    /// `impl ObjectType for X`: a class of the object model.
    pub class: bool,
    pub site: Site,
}

/// A `macro_rules!` definition of the crate.
pub(crate) struct LocalMacro {
    /// The rules: matcher and transcriber.
    pub rules: Vec<(Tokens, Tokens)>,
    /// The definition contains an invocation of a declaration macro.
    pub declaring: bool,
    pub site: Site,
}

/// An invocation, in item position or among the members of an inherent `impl` block, of a
/// macro the scanner does not read by itself.
pub(crate) struct Invocation {
    pub name: String,
    pub tokens: Tokens,
    pub site: Site,
    /// The type of the `impl` block the invocation is a member of.
    pub impl_owner: Option<String>,
}

/// The names a function imports for itself (`use std::rc::Rc;` in its body): each name
/// with the path it stands for. The types a statement of the function names are resolved
/// with them first.
pub(crate) type LocalImports = Vec<(String, Vec<String>)>;

/// `MarkupType::register_handle::<Handle>(<Type as MarkupTyped>::MARKUP)`: one more Rust
/// type that holds a value of a type with markup metadata.
pub(crate) struct RawHandle {
    pub handle: Tokens,
    /// The type the metadata is declared for, as the declaration writes it (`dyn Trait`).
    pub type_: Tokens,
    pub imports: LocalImports,
}

/// `ValueTypes::register_cast::<From, To>(..)`.
pub(crate) struct RawCast {
    pub from: Tokens,
    pub to: Tokens,
    pub imports: LocalImports,
}

/// `ValueTypes::register_<registration>::<Types..>(..)`, for the registrations but
/// `register_cast` ([`VALUE_REGISTRATIONS`](crate::model::VALUE_REGISTRATIONS)).
pub(crate) struct RawValueType {
    pub registration: String,
    pub types: Vec<Tokens>,
    pub imports: LocalImports,
}

/// The macros a function invokes in its body, for what their expansions register with
/// the untyped value conversions.
pub(crate) struct InvokedMacros {
    /// The `macro_rules!` definitions the body itself has, by name.
    pub definitions: BTreeMap<String, Vec<(Tokens, Tokens)>>,
    /// The invocations: the name, the tokens and the line.
    pub invoked: Vec<(String, Tokens, usize)>,
    pub imports: LocalImports,
    pub site: Site,
}

/// What the files of a crate state, before anything is resolved.
pub(crate) struct Source {
    pub modules: Modules,
    pub files: Vec<ScannedFile>,
    pub diagnostics: Vec<Diagnostic>,
    pub declarations: Vec<Located<Declaration>>,
    /// The variants of the plain enumerations, by module and name, with their values.
    pub enums: BTreeMap<(usize, String), Vec<(String, Option<i64>)>>,
    /// The constants of the `bitflags!` types, by module and name, with their expressions.
    pub flags: BTreeMap<(usize, String), Vec<(String, Tokens)>>,
    /// The associated constants of the inherent `impl` blocks, by the module of the block
    /// and the name of the type, with their expressions.
    pub associated: BTreeMap<(usize, String), Vec<(String, Tokens)>>,
    /// The type aliases without parameters (`type Name = Type;`): the name and the type.
    pub aliases: Vec<Located<(String, Tokens)>>,
    /// The lists of the classes the crate registers (`const TYPES: &[&TypeInfo]`): the
    /// types of each list, or nothing for a list with an entry the scanner does not read.
    pub class_lists: Vec<Located<Option<Vec<Tokens>>>>,
    /// The handles the crate registers for types with markup metadata.
    pub handles: Vec<Located<RawHandle>>,
    /// The casts the crate registers.
    pub casts: Vec<Located<RawCast>>,
    /// The other registrations with the untyped value conversions.
    pub value_types: Vec<Located<RawValueType>>,
    /// The calls of each registration function in the text of the files, and the ones read.
    pub value_calls: BTreeMap<String, (usize, usize)>,
    /// The macros the functions invoke in their bodies.
    pub invoked_macros: Vec<InvokedMacros>,
    /// The text constants of the modules, by module and name.
    pub constants: BTreeMap<(usize, String), String>,
    /// The associated text constants, by the name of the type and of the constant.
    pub associated_constants: BTreeMap<(String, String), String>,
    pub functions: Vec<RawFunction>,
    pub hand_written: Vec<HandWritten>,
    pub assemblies: Vec<RawAssembly>,
    pub namespaces: Vec<Located<Vec<(Text, Text)>>>,
    /// The entries of `ferro_rust_paths!`: the segments of each path.
    pub rust_paths: Vec<Located<Vec<String>>>,
    /// The entries of the lists of generic types of `ferro_rust_paths!`: the type, the
    /// path the crate states for it, and whether the type is a contract.
    pub generic_paths: Vec<Located<(Tokens, String, bool)>>,
    pub local_macros: BTreeMap<String, Vec<LocalMacro>>,
    pub invocations: Vec<Invocation>,
    /// The declarations read from expansions of local macros, by the name of the macro.
    pub expanded: BTreeMap<String, usize>,
    read_files: BTreeSet<PathBuf>,
}

/// The place of the items being read.
struct Context<'a> {
    module: usize,
    file: usize,
    /// The directory of the file.
    file_directory: &'a Path,
    /// The directory the files of the modules declared here are in.
    directory: PathBuf,
    /// Inside an inline module of the file.
    inline: bool,
    cfg: Vec<String>,
}

impl Source {
    /// Reads the crate from its root file.
    pub fn read(options: &ScanOptions) -> Source {
        let mut source = Source {
            modules: Modules::new(&options.crate_name, &options.extern_crates),
            files: Vec::new(),
            diagnostics: Vec::new(),
            declarations: Vec::new(),
            enums: BTreeMap::new(),
            flags: BTreeMap::new(),
            associated: BTreeMap::new(),
            aliases: Vec::new(),
            class_lists: Vec::new(),
            handles: Vec::new(),
            casts: Vec::new(),
            value_types: Vec::new(),
            value_calls: BTreeMap::new(),
            invoked_macros: Vec::new(),
            constants: BTreeMap::new(),
            associated_constants: BTreeMap::new(),
            functions: Vec::new(),
            hand_written: Vec::new(),
            assemblies: Vec::new(),
            namespaces: Vec::new(),
            rust_paths: Vec::new(),
            generic_paths: Vec::new(),
            local_macros: BTreeMap::new(),
            invocations: Vec::new(),
            expanded: BTreeMap::new(),
            read_files: BTreeSet::new(),
        };
        // The export tables of the crates the crate is built on: what a glob import of one
        // of their modules brings.
        for dependency in &options.dependencies {
            source.modules.extern_crates.insert(dependency.crate_name.clone());
            source.modules.external_crates.insert(dependency.crate_name.clone());
            for export in &dependency.exports {
                source.modules.externals.entry(export.path.clone()).or_insert_with(|| export.declared.clone());
            }
        }
        source.read_file(&options.root, 0, true, &[], None);
        source.expand_local_macros();
        source.expand_registration_macros();
        source.modules.finish();
        source
    }

    pub fn diagnostic(&mut self, severity: Severity, code: &'static str, site: &Site, message: String) {
        self.diagnostic_at(severity, code, site.file, site.line, message);
    }

    fn diagnostic_at(&mut self, severity: Severity, code: &'static str, file: usize, line: usize, message: String) {
        let file = self.files[file].path.clone();
        self.diagnostics.push(Diagnostic { severity, code, file, line, message });
    }

    /// Reads the file `path` as the module `module`. `own_directory`: the files of the
    /// modules it declares are in its directory (`lib.rs`, `mod.rs`, a file named by
    /// `#[path]`), not in the directory named after it. `declared`: the file and the line
    /// of the `mod` item, for the diagnostic of a file that cannot be read.
    fn read_file(&mut self, path: &Path, module: usize, own_directory: bool, cfg: &[String], declared: Option<(usize, usize)>) {
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) => {
                let message = format!("the file {} cannot be read: {error}", path.display());
                match declared {
                    Some((file, line)) => self.diagnostic_at(Severity::Error, codes::FILE, file, line, message),
                    None => self.diagnostics.push(Diagnostic { severity: Severity::Error, code: codes::FILE, file: path.to_path_buf(), line: 0, message }),
                }
                return;
            }
        };
        let identity = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        if !self.read_files.insert(identity) {
            if let Some((file, line)) = declared {
                let message = format!("the file {} is the file of another module too: it is read once", path.display());
                self.diagnostic_at(Severity::Note, codes::FILE, file, line, message);
            }
            return;
        }
        let file = self.files.len();
        self.files.push(ScannedFile { path: path.to_path_buf(), module: self.modules.module_path(module), invocations: BTreeMap::new() });
        let parsed = match syn::parse_file(&text) {
            Ok(parsed) => parsed,
            Err(error) => {
                // A form of the language the parser refuses and a rewrite of the tokens
                // makes readable (`with_parenthesised_break_values`).
                let rewritten = text.parse::<proc_macro2::TokenStream>().ok().map(with_parenthesised_break_values);
                match rewritten.filter(|(_, changed)| *changed).and_then(|(tokens, _)| syn::parse2::<syn::File>(tokens).ok()) {
                    Some(parsed) => parsed,
                    None => {
                        let line = error.span().start().line;
                        self.diagnostic_at(Severity::Error, codes::FILE, file, line, format!("the file is not read: {error}"));
                        return;
                    }
                }
            }
        };
        // Every call of a registration function of the untyped value conversions in the text
        // of the file, wherever it stands: what the functions read is counted against it.
        if text.contains("ValueTypes") {
            if let Ok(tokens) = text.parse::<proc_macro2::TokenStream>() {
                let tokens = tokens_of(tokens);
                // What the definition of a declaration macro registers, it registers for the
                // declarations, which are read where the macro is invoked.
                calls_of(&tokens, "ValueTypes", DECLARATION_MACROS, &mut |function| {
                    if let Some(registration) = function.strip_prefix("register_").filter(|name| VALUE_REGISTRATIONS.iter().any(|(known, _)| known == name)) {
                        self.value_calls.entry(registration.to_string()).or_default().0 += 1;
                    }
                });
            }
        }
        let file_directory = path.parent().unwrap_or(Path::new("")).to_path_buf();
        let directory = if own_directory {
            file_directory.clone()
        } else {
            file_directory.join(path.file_stem().map(|stem| stem.to_string_lossy().into_owned()).unwrap_or_default())
        };
        let context = Context { module, file, file_directory: &file_directory, directory, inline: false, cfg: cfg.to_vec() };
        self.read_items(&parsed.items, &context);
    }

    fn read_items(&mut self, items: &[syn::Item], context: &Context) {
        for item in items {
            let (test, conditions) = conditions_of(attributes_of(item));
            if test {
                self.count_only(item, context.file, Category::TestCode);
                continue;
            }
            let mut cfg = context.cfg.clone();
            cfg.extend(conditions);
            match item {
                syn::Item::Mod(module) => self.read_module(module, context, cfg),
                syn::Item::Use(item) => {
                    let mut prefix = Vec::new();
                    if item.leading_colon.is_some() {
                        prefix.push(String::new());
                    }
                    self.read_use(&item.tree, &mut prefix, is_public(&item.vis), context.module);
                }
                syn::Item::Macro(item) => self.read_item_macro(item, context, cfg),
                syn::Item::Impl(item) => self.read_impl(item, context, cfg),
                syn::Item::ExternCrate(item) => {
                    if item.ident != "self" {
                        self.modules.extern_crates.insert(item.ident.to_string());
                    }
                }
                other => {
                    self.read_plain_item(other, context, cfg);
                    self.count_only(other, context.file, Category::Unread);
                }
            }
        }
    }

    /// Counts the invocations of declaration macros inside an item the scanner does not
    /// read into: each is reported, unless the item is test code.
    fn count_only(&mut self, item: &syn::Item, file: usize, category: Category) {
        let mut found = Vec::new();
        Counter { category, counts: &mut self.files[file].invocations, found: &mut found }.visit_item(item);
        self.report_unread(found, file, category);
    }

    fn report_unread(&mut self, found: Vec<(String, usize)>, file: usize, category: Category) {
        if category != Category::Unread {
            return;
        }
        for (name, line) in found {
            let message = format!("`{name}!` is written where the scanner does not read (inside a function, another item or the arguments of a macro): what it declares is not in the model");
            self.diagnostic_at(Severity::Note, codes::POSITION, file, line, message);
        }
    }

    fn read_module(&mut self, item: &syn::ItemMod, context: &Context, cfg: Vec<String>) {
        let name = item.ident.to_string();
        let name = name.trim_start_matches("r#");
        let line = item.ident.span().start().line;
        let child = self.modules.add_module(context.module, name, is_public(&item.vis));
        if let Some((_, items)) = &item.content {
            let inner = Context {
                module: child,
                file: context.file,
                file_directory: context.file_directory,
                directory: context.directory.join(name),
                inline: true,
                cfg,
            };
            self.read_items(items, &inner);
            return;
        }
        let (file, own_directory) = match path_attribute(&item.attrs) {
            Some(path) => {
                let base = if context.inline { context.directory.as_path() } else { context.file_directory };
                (base.join(path), true)
            }
            None => {
                let flat = context.directory.join(format!("{name}.rs"));
                let nested = context.directory.join(name).join("mod.rs");
                if flat.is_file() {
                    (flat, false)
                } else if nested.is_file() {
                    (nested, true)
                } else {
                    let message = format!("the file of the module `{name}` is not found: neither {} nor {}", flat.display(), nested.display());
                    self.diagnostic_at(Severity::Error, codes::FILE, context.file, line, message);
                    return;
                }
            }
        };
        self.read_file(&file, child, own_directory, &cfg, Some((context.file, line)));
    }

    fn read_use(&mut self, tree: &syn::UseTree, prefix: &mut Vec<String>, public: bool, module: usize) {
        match tree {
            syn::UseTree::Path(path) => {
                prefix.push(path.ident.to_string());
                self.read_use(&path.tree, prefix, public, module);
                prefix.pop();
            }
            syn::UseTree::Name(name) => {
                let name = name.ident.to_string();
                let mut path = prefix.clone();
                let name = if name == "self" {
                    match prefix.last() {
                        Some(last) => last.clone(),
                        None => return,
                    }
                } else {
                    path.push(name.clone());
                    name
                };
                self.modules.modules[module].imports.push(Import { name, path, public });
            }
            syn::UseTree::Rename(rename) => {
                if rename.rename == "_" {
                    return;
                }
                let mut path = prefix.clone();
                if rename.ident != "self" {
                    path.push(rename.ident.to_string());
                }
                if !path.is_empty() {
                    self.modules.modules[module].imports.push(Import { name: rename.rename.to_string(), path, public });
                }
            }
            syn::UseTree::Glob(_) => {
                if !prefix.is_empty() {
                    self.modules.modules[module].globs.push(Glob { path: prefix.clone(), public });
                }
            }
            syn::UseTree::Group(group) => {
                for tree in &group.items {
                    self.read_use(tree, prefix, public, module);
                }
            }
        }
    }

    /// An item that is not a module, a `use`, a macro or an `impl`: the name it declares,
    /// and what the model reads from it (the values of an enumeration, a text constant,
    /// the namespace table, the assembly).
    fn read_plain_item(&mut self, item: &syn::Item, context: &Context, cfg: Vec<String>) {
        let module = context.module;
        let site = |ident: &syn::Ident| Site { module, file: context.file, line: ident.span().start().line, cfg: cfg.clone(), expanded: false };
        match item {
            syn::Item::Struct(item) => self.modules.add_item(module, &item.ident.to_string(), ItemKind::Type, is_public(&item.vis)),
            syn::Item::Union(item) => self.modules.add_item(module, &item.ident.to_string(), ItemKind::Type, is_public(&item.vis)),
            syn::Item::Trait(item) => self.modules.add_item(module, &item.ident.to_string(), ItemKind::Type, is_public(&item.vis)),
            syn::Item::TraitAlias(item) => self.modules.add_item(module, &item.ident.to_string(), ItemKind::Type, is_public(&item.vis)),
            syn::Item::Type(item) => {
                let name = item.ident.to_string();
                self.modules.add_item(module, &name, ItemKind::Type, is_public(&item.vis));
                if item.generics.params.is_empty() {
                    self.aliases.push(Located { site: site(&item.ident), value: (name, tokens_of(item.ty.to_token_stream())) });
                }
            }
            syn::Item::Fn(item) => {
                self.modules.add_item(module, &item.sig.ident.to_string(), ItemKind::Value, is_public(&item.vis));
                self.read_registrations(&item.block, context, &cfg);
            }
            syn::Item::Enum(item) => {
                let name = item.ident.to_string();
                self.modules.add_item(module, &name, ItemKind::Type, is_public(&item.vis));
                let mut next = Some(0i64);
                let mut variants: Vec<(String, Option<i64>)> = Vec::new();
                for variant in &item.variants {
                    if let Some((_, expression)) = &variant.discriminant {
                        // A discriminant is a constant expression over literals and the
                        // variants before it.
                        let earlier = |variant: &str| variants.iter().find(|(known, _)| known == variant).and_then(|(_, value)| *value);
                        next = evaluate(&tokens_of(expression.to_token_stream()), &Scope { type_name: &name, member: &earlier, all: &|| None });
                    }
                    variants.push((variant.ident.to_string(), next));
                    next = next.and_then(|value| value.checked_add(1));
                }
                self.enums.insert((module, name), variants);
            }
            syn::Item::Const(item) => {
                let name = item.ident.to_string();
                self.modules.add_item(module, &name, ItemKind::Value, is_public(&item.vis));
                self.read_registrations_of_value(&item.expr, context, &cfg);
                if let Text::Literal(text) = text_of_expression(&item.expr) {
                    self.constants.insert((module, name.clone()), text);
                }
                if is_class_list(&item.ty) {
                    let list = class_list_of(&item.expr);
                    if list.is_none() {
                        let message = format!("the list of registered classes `{name}` has an entry that is not `Type::TYPE` or `<Type as StaticType>::TYPE` (or a type, in a macro that writes the list): the classes the crate registers are not read, and no class is marked as not registered");
                        self.diagnostic(Severity::Warning, codes::ASSEMBLY, &site(&item.ident), message);
                    }
                    self.class_lists.push(Located { site: site(&item.ident), value: list });
                }
                if name == "NAMESPACES" {
                    match pairs_of(&item.expr) {
                        Some(pairs) => self.namespaces.push(Located { site: site(&item.ident), value: pairs }),
                        None => {
                            let message = "`NAMESPACES` is not a list of pairs (`&[(\"module\", \"Namespace\"), ..]`): the namespace table is not read".to_string();
                            self.diagnostic(Severity::Error, codes::ASSEMBLY, &site(&item.ident), message);
                        }
                    }
                }
            }
            syn::Item::Static(item) => {
                self.modules.add_item(module, &item.ident.to_string(), ItemKind::Value, is_public(&item.vis));
                self.read_registrations_of_value(&item.expr, context, &cfg);
                if last_segment_of_type(&item.ty).as_deref() == Some("MarkupAssembly") {
                    match assembly_of(&item.expr, site(&item.ident)) {
                        Some(assembly) => self.assemblies.push(assembly),
                        None => {
                            let message = "the value of the `MarkupAssembly` is not a struct expression: the assembly is not read".to_string();
                            self.diagnostic(Severity::Error, codes::ASSEMBLY, &site(&item.ident), message);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// A macro in item position: a declaration macro, a definition, or another macro.
    fn read_item_macro(&mut self, item: &syn::ItemMacro, context: &Context, cfg: Vec<String>) {
        let (name, line) = name_of_macro(&item.mac);
        let tokens = tokens_of(item.mac.tokens.clone());
        let site = Site { module: context.module, file: context.file, line, cfg, expanded: false };
        let file = context.file;
        if name == "macro_rules" {
            let mut declaring = false;
            let counts = &mut self.files[file].invocations;
            invocations(&tokens, DECLARATION_MACROS, &mut |name, _| {
                declaring = true;
                add_count(counts, name, Category::MacroDefinition);
            });
            if let Some(ident) = &item.ident {
                self.local_macros.entry(ident.to_string()).or_default().push(LocalMacro { rules: rules_of(&tokens), declaring, site });
            }
            return;
        }
        if DECLARATION_MACROS.contains(&name.as_str()) {
            self.read_registrations_of_tokens(&tokens, &site);
            let result = read_declaration(&name, &tokens, line);
            let category = if result.is_ok() { Category::Read } else { Category::Failed };
            let counts = &mut self.files[file].invocations;
            add_count(counts, &name, category);
            invocations(&tokens, DECLARATION_MACROS, &mut |name, _| add_count(counts, name, category));
            match result {
                Ok(declaration) => {
                    if let Declaration::Class { name, virtual_trait: Some(virtual_trait), .. } = &declaration {
                        // What the class macro writes for the virtual members of the class.
                        for item in [format!("{name}VTable"), virtual_trait.clone(), format!("{virtual_trait}Ext")] {
                            self.modules.add_item(site.module, &item, ItemKind::Type, true);
                        }
                    }
                    self.declarations.push(Located { site, value: declaration });
                }
                Err(error) => self.unreadable(&name, &site, error),
            }
            return;
        }
        let mut found = Vec::new();
        let counts = &mut self.files[file].invocations;
        invocations(&tokens, DECLARATION_MACROS, &mut |name, line| {
            add_count(counts, name, Category::Unread);
            found.push((name.to_string(), line));
        });
        self.report_unread(found, file, Category::Unread);
        match name.as_str() {
            "ferro_rust_paths" => self.read_rust_paths(&tokens, &site),
            "bitflags" => {
                if let Err(error) = self.read_bitflags(&tokens, &site) {
                    let message = format!("`bitflags!`: {}: the values of its constants are not read", error.message);
                    self.diagnostic_at(Severity::Note, codes::FORM, file, error.line, message);
                }
            }
            _ => self.invocations.push(Invocation { name, tokens, site, impl_owner: None }),
        }
    }

    /// The diagnostic of a declaration whose form the scanner does not know.
    fn unreadable(&mut self, name: &str, site: &Site, error: ParseError) {
        let message = format!("`{name}!`: {}: the declaration is not read", error.message);
        self.diagnostic_at(Severity::Error, codes::FORM, site.file, if error.line == 0 { site.line } else { error.line }, message);
    }

    /// An `impl` block: the functions and the property accessors of an inherent one, and
    /// the runtime type a type implements by hand.
    fn read_impl(&mut self, item: &syn::ItemImpl, context: &Context, cfg: Vec<String>) {
        let owner = last_segment_of_type(&item.self_ty);
        let trait_ = item.trait_.as_ref().and_then(|(_, path, _)| path.segments.last()).map(|segment| segment.ident.to_string());
        let file = context.file;
        let site_at = |line: usize, cfg: &[String]| Site { module: context.module, file, line, cfg: cfg.to_vec(), expanded: false };
        let line = item.impl_token.span.start().line;
        if let (Some(owner), Some(trait_)) = (&owner, &trait_) {
            match trait_.as_str() {
                "ObjectType" | "StaticType" => {
                    self.hand_written.push(HandWritten { name: owner.clone(), class: trait_ == "ObjectType", site: site_at(line, &cfg) });
                }
                "MarkupTyped" => {
                    let message = format!("the markup metadata of `{owner}` is written by hand (`impl MarkupTyped`): it is not in the model");
                    self.diagnostic_at(Severity::Warning, codes::HAND_WRITTEN, file, line, message);
                }
                _ => {}
            }
        }
        for member in &item.items {
            let (test, conditions) = conditions_of(attributes_of_member(member));
            let mut found = Vec::new();
            if test {
                Counter { category: Category::TestCode, counts: &mut self.files[file].invocations, found: &mut found }.visit_impl_item(member);
                continue;
            }
            let mut member_cfg = cfg.clone();
            member_cfg.extend(conditions);
            match (member, &owner, &trait_) {
                (syn::ImplItem::Macro(member), Some(owner), None) => {
                    let (name, line) = name_of_macro(&member.mac);
                    let tokens = tokens_of(member.mac.tokens.clone());
                    let site = site_at(line, &member_cfg);
                    match name.as_str() {
                        "ferro_property" => {
                            self.read_registrations_of_tokens(&tokens, &site);
                            let result = read_property(&mut Cursor::new(&tokens, line));
                            let category = if result.is_ok() { Category::Read } else { Category::Failed };
                            let counts = &mut self.files[file].invocations;
                            add_count(counts, &name, category);
                            invocations(&tokens, DECLARATION_MACROS, &mut |name, _| add_count(counts, name, category));
                            match result {
                                Ok((stated, accessor)) => {
                                    self.declarations.push(Located { site, value: property_of(stated, owner, accessor) });
                                }
                                Err(error) => self.unreadable(&name, &site, error),
                            }
                            continue;
                        }
                        "ferro_routed_event" => {
                            if let Some((function, visibility, return_type)) = accessor_signature(&tokens, line) {
                                self.functions.push(RawFunction {
                                    owner: owner.clone(),
                                    name: function,
                                    visibility,
                                    receiver: false,
                                    parameters: Vec::new(),
                                    return_type: Some(return_type),
                                    declared_by: Some(name.clone()),
                                    site,
                                });
                            }
                        }
                        // A macro of the crate that writes members of the block.
                        _ if !DECLARATION_MACROS.contains(&name.as_str()) => {
                            self.invocations.push(Invocation { name: name.clone(), tokens, site, impl_owner: Some(owner.clone()) });
                        }
                        _ => {}
                    }
                }
                (syn::ImplItem::Fn(member), Some(owner), None) => {
                    self.read_registrations(&member.block, context, &member_cfg);
                    let parameters = member
                        .sig
                        .inputs
                        .iter()
                        .filter_map(|input| match input {
                            syn::FnArg::Typed(typed) => Some(text_of(&tokens_of(typed.ty.to_token_stream()))),
                            syn::FnArg::Receiver(_) => None,
                        })
                        .collect();
                    let return_type = match &member.sig.output {
                        syn::ReturnType::Type(_, type_) => Some(text_of(&tokens_of(type_.to_token_stream()))),
                        syn::ReturnType::Default => None,
                    };
                    self.functions.push(RawFunction {
                        owner: owner.clone(),
                        name: member.sig.ident.to_string(),
                        visibility: visibility_of(&member.vis),
                        receiver: member.sig.receiver().is_some(),
                        parameters,
                        return_type,
                        declared_by: None,
                        site: site_at(member.sig.ident.span().start().line, &member_cfg),
                    });
                }
                (syn::ImplItem::Const(member), Some(owner), None) => {
                    if let Text::Literal(text) = text_of_expression(&member.expr) {
                        self.associated_constants.insert((owner.clone(), member.ident.to_string()), text);
                    }
                    let constants = self.associated.entry((context.module, owner.clone())).or_default();
                    constants.push((member.ident.to_string(), tokens_of(member.expr.to_token_stream())));
                }
                _ => {}
            }
            Counter { category: Category::Unread, counts: &mut self.files[file].invocations, found: &mut found }.visit_impl_item(member);
            self.report_unread(found, file, Category::Unread);
        }
    }

    /// `ferro_rust_paths! { classes: [crate::A, ..], types: [..], contracts: [..], .. }`:
    /// the paths of the three plain lists.
    fn read_rust_paths(&mut self, tokens: &[TokenTree], site: &Site) {
        let mut cursor = Cursor::new(tokens, site.line);
        while !cursor.is_end() {
            let Ok(list) = cursor.take_ident("the name of a list") else { return };
            if cursor.expect_punct(':').is_err() {
                return;
            }
            let Ok((entries, line)) = cursor.take_group(Delimiter::Bracket, "a list in brackets") else { return };
            cursor.eat_punct(',');
            // `(Type<Arguments>, "crate::Type<::path::Argument>")`: the path of an instantiation
            // of a generic type, which the crate states.
            if matches!(list.as_str(), "generics" | "generic_contracts") {
                let mut entries = Cursor::new(&entries, line);
                while !entries.is_end() {
                    let Ok((entry, line)) = entries.take_group(Delimiter::Parenthesis, "an entry in parentheses") else { return };
                    entries.eat_punct(',');
                    let mut entry = Cursor::new(&entry, line);
                    let Ok(type_) = entry.take_type(TypeEnd::default(), "a type") else { return };
                    entry.eat_punct(',');
                    let Some(TokenTree::Literal(literal)) = entry.peek() else { return };
                    let syn::Lit::Str(path) = syn::Lit::new(literal.clone()) else { return };
                    self.generic_paths.push(Located { site: Site { line, ..site.clone() }, value: (type_.to_vec(), path.value(), list == "generic_contracts") });
                }
                continue;
            }
            if !matches!(list.as_str(), "classes" | "types" | "contracts") {
                continue;
            }
            let mut entries = Cursor::new(&entries, line);
            while !entries.is_end() {
                let line = entries.line();
                let Some(path) = entries.take_plain_path() else { return };
                let segments = path.iter().filter_map(ident_of).collect();
                self.rust_paths.push(Located { site: Site { line, ..site.clone() }, value: segments });
                entries.eat_punct(',');
            }
        }
    }

    /// `bitflags! { #[..] pub struct Name: u32 { const A = 1; const B = 1 << 1; } .. }`:
    /// the types it declares and the expressions of their constants.
    fn read_bitflags(&mut self, tokens: &[TokenTree], site: &Site) -> Result<(), ParseError> {
        let mut cursor = Cursor::new(tokens, site.line);
        while !cursor.is_end() {
            cursor.skip_attributes();
            let public = cursor.take_visibility() == "pub";
            if !cursor.eat_ident("struct") {
                return cursor.error("`struct`");
            }
            let name = cursor.take_ident("the name of the flags")?;
            self.modules.add_item(site.module, &name, ItemKind::Type, public);
            cursor.expect_punct(':')?;
            cursor.take_type(TypeEnd { brace: true, ..TypeEnd::default() }, "the type of the bits")?;
            let (body, line) = cursor.take_group(Delimiter::Brace, "the constants in braces")?;
            let mut body = Cursor::new(&body, line);
            let mut constants = Vec::new();
            while !body.is_end() {
                body.skip_attributes();
                if !body.eat_ident("const") {
                    return body.error("`const`");
                }
                let constant = body.take_ident("the name of a constant")?;
                body.expect_punct('=')?;
                let start = body.position();
                while !body.is_end() && !body.is_punct(';') {
                    body.next();
                }
                constants.push((constant, body.since(start).to_vec()));
                body.expect_punct(';')?;
            }
            self.flags.insert((site.module, name), constants);
            cursor.eat_punct(';');
        }
        Ok(())
    }

    /// Expands the invocations of the macros the crate defines, where a rule of the
    /// definition is one the scanner can apply, and reads what the expansion declares:
    /// the types (`pub struct $name`) and the declaration macros at its top level.
    fn expand_local_macros(&mut self) {
        let pending = std::mem::take(&mut self.invocations);
        for invocation in pending {
            let Some(definitions) = self.local_macros.get(&invocation.name) else { continue };
            let declaring = definitions.iter().any(|definition| definition.declaring);
            let expansion = match definitions.as_slice() {
                [definition] => definition
                    .rules
                    .iter()
                    .find_map(|(matcher, transcriber)| bind(matcher, &invocation.tokens).map(|bindings| substitute(transcriber, &bindings)))
                    .ok_or("no rule of the definition has a form the scanner applies (identifiers, types, a visibility, attributes)")
                    .and_then(|expansion| expansion.ok_or("the rule repeats a part of its input, which the scanner does not expand")),
                _ => Err("the crate defines more than one macro of that name"),
            };
            let name = invocation.name.clone();
            let site = Site { expanded: true, ..invocation.site.clone() };
            let expansion = match expansion {
                Ok(expansion) => expansion,
                Err(reason) => {
                    if declaring {
                        let message = format!("`{name}!` declares through a declaration macro, and its invocation is not expanded: {reason}; what it declares is not in the model");
                        self.diagnostic(Severity::Warning, codes::LOCAL_MACRO, &site, message);
                    }
                    continue;
                }
            };
            for (index, token) in expansion.iter().enumerate() {
                match token {
                    TokenTree::Ident(ident) if matches!(ident.to_string().as_str(), "struct" | "enum" | "trait" | "union") => {
                        if let Some(declared) = expansion.get(index + 1).and_then(ident_of) {
                            let public = index > 0 && ident_of(&expansion[index - 1]).as_deref() == Some("pub");
                            self.modules.add_item(site.module, &declared, ItemKind::Type, public);
                        }
                    }
                    TokenTree::Ident(ident) if DECLARATION_MACROS.contains(&ident.to_string().as_str()) => {
                        let is_invocation = expansion.get(index + 1).is_some_and(|next| is_punct(next, '!'));
                        let Some(group) = expansion.get(index + 2).and_then(|next| match next {
                            TokenTree::Group(group) if is_invocation => Some(group),
                            _ => None,
                        }) else {
                            continue;
                        };
                        let declaration_macro = ident.to_string();
                        let tokens = tokens_of(group.stream());
                        // Among the members of an `impl` block the only declaration is the
                        // accessor of a property, of the type of the block unless it states
                        // its owner.
                        let declaration = match (&invocation.impl_owner, declaration_macro.as_str()) {
                            (Some(owner), "ferro_property") => {
                                read_property(&mut Cursor::new(&tokens, site.line)).map(|(stated, accessor)| property_of(stated, owner, accessor))
                            }
                            (Some(_), _) => Err(ParseError { line: site.line, message: "the declaration is written among the members of an `impl` block".to_string() }),
                            (None, _) => read_declaration(&declaration_macro, &tokens, site.line),
                        };
                        match declaration {
                            Ok(declaration) => {
                                *self.expanded.entry(name.clone()).or_default() += 1;
                                self.declarations.push(Located { site: site.clone(), value: declaration });
                            }
                            Err(error) => {
                                let message = format!("`{declaration_macro}!` in the expansion of `{name}!`: {}: the declaration is not read", error.message);
                                self.diagnostic(Severity::Error, codes::LOCAL_MACRO, &site, message);
                            }
                        }
                    }
                    TokenTree::Group(group) => {
                        let mut nested = 0usize;
                        invocations(&tokens_of(group.stream()), DECLARATION_MACROS, &mut |_, _| nested += 1);
                        let after_macro = index >= 2 && is_punct(&expansion[index - 1], '!');
                        if nested > 0 && !after_macro {
                            let message = format!("the expansion of `{name}!` invokes a declaration macro inside a block ({nested}): what it declares there is not in the model");
                            self.diagnostic(Severity::Warning, codes::LOCAL_MACRO, &site, message);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Finds the invocations of declaration macros in syntax the scanner does not read into.
struct Counter<'a> {
    category: Category,
    counts: &'a mut BTreeMap<String, InvocationCounts>,
    found: &'a mut Vec<(String, usize)>,
}

impl<'ast> Visit<'ast> for Counter<'_> {
    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        let (name, line) = name_of_macro(node);
        let nested = if name == "macro_rules" { Category::MacroDefinition } else { self.category };
        if DECLARATION_MACROS.contains(&name.as_str()) {
            add_count(self.counts, &name, self.category);
            self.found.push((name, line));
        }
        let counts = &mut *self.counts;
        let found = &mut *self.found;
        invocations(&tokens_of(node.tokens.clone()), DECLARATION_MACROS, &mut |name, line| {
            add_count(counts, name, nested);
            if nested != Category::MacroDefinition {
                found.push((name.to_string(), line));
            }
        });
    }
}

/// The last segment of the path of a macro and its line.
fn name_of_macro(node: &syn::Macro) -> (String, usize) {
    match node.path.segments.last() {
        Some(segment) => (segment.ident.to_string(), segment.ident.span().start().line),
        None => (String::new(), 0),
    }
}

fn attributes_of(item: &syn::Item) -> &[syn::Attribute] {
    match item {
        syn::Item::Const(item) => &item.attrs,
        syn::Item::Enum(item) => &item.attrs,
        syn::Item::ExternCrate(item) => &item.attrs,
        syn::Item::Fn(item) => &item.attrs,
        syn::Item::ForeignMod(item) => &item.attrs,
        syn::Item::Impl(item) => &item.attrs,
        syn::Item::Macro(item) => &item.attrs,
        syn::Item::Mod(item) => &item.attrs,
        syn::Item::Static(item) => &item.attrs,
        syn::Item::Struct(item) => &item.attrs,
        syn::Item::Trait(item) => &item.attrs,
        syn::Item::TraitAlias(item) => &item.attrs,
        syn::Item::Type(item) => &item.attrs,
        syn::Item::Union(item) => &item.attrs,
        syn::Item::Use(item) => &item.attrs,
        _ => &[],
    }
}

fn attributes_of_member(member: &syn::ImplItem) -> &[syn::Attribute] {
    match member {
        syn::ImplItem::Const(member) => &member.attrs,
        syn::ImplItem::Fn(member) => &member.attrs,
        syn::ImplItem::Type(member) => &member.attrs,
        syn::ImplItem::Macro(member) => &member.attrs,
        _ => &[],
    }
}

/// The `cfg` conditions of attributes: whether one of them makes the item test code
/// (`test`, or `all(..)` with `test` in it), and the other conditions as written.
fn conditions_of(attributes: &[syn::Attribute]) -> (bool, Vec<String>) {
    let mut test = false;
    let mut conditions = Vec::new();
    for attribute in attributes {
        if !attribute.path().is_ident("cfg") {
            continue;
        }
        let syn::Meta::List(list) = &attribute.meta else { continue };
        let tokens = tokens_of(list.tokens.clone());
        let in_all = tokens.len() == 2
            && ident_of(&tokens[0]).as_deref() == Some("all")
            && group_of(&tokens[1], Delimiter::Parenthesis).is_some_and(|group| names_test(&tokens_of(group.stream())));
        if (tokens.len() == 1 && names_test(&tokens)) || in_all {
            test = true;
        } else {
            conditions.push(text_of(&tokens));
        }
    }
    (test, conditions)
}

/// Whether one of `tokens` is the word `test`.
fn names_test(tokens: &[TokenTree]) -> bool {
    tokens.iter().any(|token| ident_of(token).as_deref() == Some("test"))
}

/// The value of `#[path = ".."]`.
fn path_attribute(attributes: &[syn::Attribute]) -> Option<String> {
    attributes.iter().find(|attribute| attribute.path().is_ident("path")).and_then(|attribute| match &attribute.meta {
        syn::Meta::NameValue(pair) => match &pair.value {
            syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(text), .. }) => Some(text.value()),
            _ => None,
        },
        _ => None,
    })
}

fn is_public(visibility: &syn::Visibility) -> bool {
    matches!(visibility, syn::Visibility::Public(_))
}

fn visibility_of(visibility: &syn::Visibility) -> String {
    text_of(&tokens_of(visibility.to_token_stream()))
}

/// The name of a type that is a path, without its arguments (`Border`, `a::Border<T>`).
fn last_segment_of_type(type_: &syn::Type) -> Option<String> {
    match type_ {
        syn::Type::Path(path) if path.qself.is_none() => path.path.segments.last().map(|segment| segment.ident.to_string()),
        _ => None,
    }
}

fn text_of_expression(expression: &syn::Expr) -> Text {
    match expression {
        syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(text), .. }) => Text::Literal(text.value()),
        syn::Expr::Path(path) if path.qself.is_none() => {
            let mut segments = Vec::new();
            if path.path.leading_colon.is_some() {
                segments.push(String::new());
            }
            segments.extend(path.path.segments.iter().map(|segment| segment.ident.to_string()));
            Text::Path(segments)
        }
        syn::Expr::Paren(inner) => text_of_expression(&inner.expr),
        syn::Expr::Group(inner) => text_of_expression(&inner.expr),
        other => Text::Other(text_of(&tokens_of(other.to_token_stream()))),
    }
}

/// The items of `&[a, b]` or `[a, b]`.
fn items_of(expression: &syn::Expr) -> Option<Vec<&syn::Expr>> {
    match expression {
        syn::Expr::Reference(reference) => items_of(&reference.expr),
        syn::Expr::Paren(inner) => items_of(&inner.expr),
        syn::Expr::Group(inner) => items_of(&inner.expr),
        syn::Expr::Array(array) => Some(array.elems.iter().collect()),
        _ => None,
    }
}

/// The pairs of `&[(a, b), ..]`.
fn pairs_of(expression: &syn::Expr) -> Option<Vec<(Text, Text)>> {
    items_of(expression)?
        .into_iter()
        .map(|item| match item {
            syn::Expr::Tuple(tuple) if tuple.elems.len() == 2 => Some((text_of_expression(&tuple.elems[0]), text_of_expression(&tuple.elems[1]))),
            _ => None,
        })
        .collect()
}

/// The pairs of `&[Name { first: a, second: b }, ..]`.
fn struct_pairs_of(expression: &syn::Expr, first: &str, second: &str) -> Option<Vec<(Text, Text)>> {
    items_of(expression)?
        .into_iter()
        .map(|item| match item {
            syn::Expr::Struct(item) => Some((text_of_expression(field_of(item, first)?), text_of_expression(field_of(item, second)?))),
            _ => None,
        })
        .collect()
}

fn field_of<'a>(expression: &'a syn::ExprStruct, name: &str) -> Option<&'a syn::Expr> {
    expression.fields.iter().find(|field| matches!(&field.member, syn::Member::Named(ident) if ident == name)).map(|field| &field.expr)
}

/// `MarkupAssembly { name: .., crate_name: .., xmlns_definitions: &[..], .. }`.
fn assembly_of(expression: &syn::Expr, site: Site) -> Option<RawAssembly> {
    let syn::Expr::Struct(expression) = expression else { return None };
    Some(RawAssembly {
        name: field_of(expression, "name").map(text_of_expression),
        crate_name: field_of(expression, "crate_name").map(text_of_expression),
        xmlns_definitions: field_of(expression, "xmlns_definitions").and_then(|list| struct_pairs_of(list, "xml_namespace", "namespace")).unwrap_or_default(),
        xmlns_prefixes: field_of(expression, "xmlns_prefixes").and_then(|list| struct_pairs_of(list, "xml_namespace", "prefix")).unwrap_or_default(),
        metadata: field_of(expression, "metadata").and_then(pairs_of).unwrap_or_default(),
        site,
    })
}

/// `#[..]* vis fn name() -> Type { .. }`: the name, the visibility and the type as text.
fn accessor_signature(tokens: &[TokenTree], line: usize) -> Option<(String, String, String)> {
    let mut cursor = Cursor::new(tokens, line);
    cursor.skip_attributes();
    let visibility = cursor.take_visibility();
    if !cursor.eat_ident("fn") {
        return None;
    }
    let name = cursor.take_ident("a name").ok()?;
    cursor.take_group(Delimiter::Parenthesis, "`()`").ok()?;
    if !cursor.eat_arrow('-') {
        return None;
    }
    let type_ = cursor.take_type(TypeEnd { brace: true, ..TypeEnd::default() }, "a type").ok()?;
    Some((name, visibility, text_of(type_)))
}

/// The rules of a `macro_rules!` definition: `(matcher) => { transcriber };`.
fn rules_of(tokens: &[TokenTree]) -> Vec<(Tokens, Tokens)> {
    let mut rules = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        let (Some(TokenTree::Group(matcher)), true, Some(TokenTree::Group(transcriber))) = (tokens.get(index), is_arrow(tokens, index + 1, '='), tokens.get(index + 3)) else {
            return Vec::new();
        };
        rules.push((tokens_of(matcher.stream()), tokens_of(transcriber.stream())));
        index += 4;
        if tokens.get(index).is_some_and(|token| is_punct(token, ';')) {
            index += 1;
        }
    }
    rules
}

/// Whether `tokens` are the repetition of attributes of a matcher or a transcriber
/// (`#[$meta:meta]`, `#[$meta]`).
fn is_attribute_repetition(tokens: &[TokenTree]) -> bool {
    tokens.len() == 2 && is_punct(&tokens[0], '#') && group_of(&tokens[1], Delimiter::Bracket).is_some()
}

/// Matches the tokens of an invocation against the matcher of a rule: the tokens each
/// `$name:fragment` stands for. The fragments are `ident`, `vis`, `ty`, `path`, `expr`,
/// `literal` and `tt`; the only repetition is the one of attributes, which are skipped.
/// Nothing for any other matcher, or when the invocation does not have the form.
fn bind(matcher: &[TokenTree], input: &[TokenTree]) -> Option<BTreeMap<String, Tokens>> {
    let mut bindings = BTreeMap::new();
    let mut cursor = Cursor::new(input, 0);
    let mut index = 0;
    while index < matcher.len() {
        let token = &matcher[index];
        if !is_punct(token, '$') {
            let found = cursor.next()?;
            match (token, found) {
                (TokenTree::Group(expected), TokenTree::Group(actual)) if expected.delimiter() == actual.delimiter() => {
                    bindings.extend(bind(&tokens_of(expected.stream()), &tokens_of(actual.stream()))?);
                }
                (TokenTree::Group(_), _) | (_, TokenTree::Group(_)) => return None,
                _ => {
                    if token.to_string() != found.to_string() {
                        return None;
                    }
                }
            }
            index += 1;
            continue;
        }
        match matcher.get(index + 1)? {
            TokenTree::Group(group) => {
                let repeated = matcher.get(index + 2).is_some_and(|operator| is_punct(operator, '*'));
                if !repeated || !is_attribute_repetition(&tokens_of(group.stream())) {
                    return None;
                }
                cursor.skip_attributes();
                index += 3;
            }
            TokenTree::Ident(name) => {
                if !matcher.get(index + 2).is_some_and(|colon| is_punct(colon, ':')) {
                    return None;
                }
                let start = cursor.position();
                match ident_of(matcher.get(index + 3)?)?.as_str() {
                    "ident" => {
                        if !matches!(cursor.next()?, TokenTree::Ident(_)) {
                            return None;
                        }
                    }
                    "vis" => {
                        cursor.take_visibility();
                    }
                    "ty" | "path" => {
                        cursor.take_type(TypeEnd::default(), "a type").ok()?;
                    }
                    "expr" => {
                        cursor.take_expression("an expression").ok()?;
                    }
                    "literal" => {
                        cursor.eat_punct('-');
                        if !matches!(cursor.next()?, TokenTree::Literal(_)) {
                            return None;
                        }
                    }
                    "tt" => {
                        cursor.next()?;
                    }
                    _ => return None,
                }
                bindings.insert(name.to_string(), cursor.since(start).to_vec());
                index += 4;
            }
            _ => return None,
        }
    }
    cursor.is_end().then_some(bindings)
}

/// The transcriber of a rule with every `$name` replaced by what it stands for and
/// `$crate` by `crate`; the repetition of attributes is left out. Nothing when the
/// transcriber repeats anything else or names a variable the matcher did not bind.
fn substitute(tokens: &[TokenTree], bindings: &BTreeMap<String, Tokens>) -> Option<Tokens> {
    let mut result = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        let token = &tokens[index];
        if is_punct(token, '$') {
            match tokens.get(index + 1)? {
                TokenTree::Ident(name) if name == "crate" => {
                    result.push(TokenTree::Ident(name.clone()));
                    index += 2;
                }
                TokenTree::Ident(name) => {
                    result.extend(bindings.get(&name.to_string())?.iter().cloned());
                    index += 2;
                }
                TokenTree::Group(group) => {
                    let repeated = tokens.get(index + 2).is_some_and(|operator| is_punct(operator, '*'));
                    let inner = tokens_of(group.stream());
                    if !repeated || !is_attribute_repetition(&inner) {
                        return None;
                    }
                    index += 3;
                }
                _ => return None,
            }
        } else if let TokenTree::Group(group) = token {
            let inner = substitute(&tokens_of(group.stream()), bindings)?;
            let mut replaced = Group::new(group.delimiter(), inner.into_iter().collect());
            replaced.set_span(group.span());
            result.push(TokenTree::Group(replaced));
            index += 1;
        } else {
            result.push(token.clone());
            index += 1;
        }
    }
    Some(result)
}

/// The accessor `accessor` that `ferro_property!` declares among the members of the `impl`
/// block of `impl_owner`: a property of the owner it states (`for Owner;`), else of the type
/// of the block. An accessor of the property of another type is a function of the type of
/// the block.
fn property_of(stated: Option<String>, impl_owner: &str, accessor: Accessor) -> Declaration {
    let function_of = stated.as_deref().filter(|owner| *owner != impl_owner).map(|_| impl_owner.to_string());
    Declaration::Properties { owner: stated.unwrap_or_else(|| impl_owner.to_string()), accessors: vec![accessor], function_of }
}

/// Whether `type_` is `&[&TypeInfo]`: the type of a list of classes a crate registers.
fn is_class_list(type_: &syn::Type) -> bool {
    let syn::Type::Reference(list) = type_ else { return false };
    let syn::Type::Slice(slice) = &*list.elem else { return false };
    let syn::Type::Reference(entry) = &*slice.elem else { return false };
    last_segment_of_type(&entry.elem).as_deref() == Some("TypeInfo")
}

/// The types of a list of classes: `&[Type::TYPE, <Type as StaticType>::TYPE]`, or the
/// arguments of a macro that writes such a list from types (`types![Type, ..]`). Nothing
/// when an entry has another form.
fn class_list_of(expression: &syn::Expr) -> Option<Vec<Tokens>> {
    match expression {
        syn::Expr::Macro(invocation) => split_types(&tokens_of(invocation.mac.tokens.clone()), 0).ok(),
        syn::Expr::Paren(inner) => class_list_of(&inner.expr),
        syn::Expr::Group(inner) => class_list_of(&inner.expr),
        _ => items_of(expression)?
            .into_iter()
            .map(|entry| match entry {
                syn::Expr::Path(path) if path.path.segments.last().is_some_and(|last| last.ident == "TYPE") => match &path.qself {
                    Some(qualified) => Some(tokens_of(qualified.ty.to_token_stream())),
                    None => {
                        // The path without `::TYPE`.
                        let tokens = tokens_of(path.path.to_token_stream());
                        (tokens.len() > 3).then(|| tokens[..tokens.len() - 3].to_vec())
                    }
                },
                _ => None,
            })
            .collect(),
    }
}

/// `<Type as MarkupTyped>::MARKUP`: the type.
fn markup_of(expression: &syn::Expr) -> Option<&syn::Type> {
    match expression {
        syn::Expr::Path(path) if path.path.segments.last().is_some_and(|last| last.ident == "MARKUP") => path.qself.as_ref().map(|qualified| &*qualified.ty),
        syn::Expr::Paren(inner) => markup_of(&inner.expr),
        syn::Expr::Group(inner) => markup_of(&inner.expr),
        _ => None,
    }
}

/// The type arguments of the last segment of a path (`register_cast::<A, B>`), when all
/// of them are types the call states (`_` is left to inference).
fn type_arguments_of(segment: &syn::PathSegment) -> Option<Vec<Tokens>> {
    match &segment.arguments {
        syn::PathArguments::AngleBracketed(arguments) => arguments
            .args
            .iter()
            .map(|argument| match argument {
                syn::GenericArgument::Type(type_) if !matches!(type_, syn::Type::Infer(_)) => Some(tokens_of(type_.to_token_stream())),
                _ => None,
            })
            .collect(),
        _ => None,
    }
}

/// Finds what a function registers for markup next to the declarations: the handles
/// (`MarkupType::register_handle::<Handle>(type_)`, where `type_` is
/// `<Type as MarkupTyped>::MARKUP` or a variable of the function bound to it) and the
/// casts (`ValueTypes::register_cast::<From, To>(..)`).
#[derive(Default)]
struct RegistrationFinder {
    /// `let name = <Type as MarkupTyped>::MARKUP;`
    bindings: Vec<(String, Tokens)>,
    /// The `use` items of the function.
    imports: LocalImports,
    /// The handle and the type of each registration of a handle, with its line.
    found: Vec<((Tokens, Tokens), usize)>,
    casts: Vec<((Tokens, Tokens), usize)>,
    /// The other registrations with the untyped value conversions whose types the call
    /// states: the registration and the types, with the line.
    value_types: Vec<((String, Vec<Tokens>), usize)>,
    /// The lines of the registrations of handles whose type is not read.
    unread: Vec<usize>,
    /// The `macro_rules!` definitions of the body, and the macros it invokes.
    definitions: BTreeMap<String, Vec<(Tokens, Tokens)>>,
    invoked: Vec<(String, Tokens, usize)>,
}

/// The names the `use` tree `tree` below `prefix` imports, each with its path.
fn local_imports(tree: &syn::UseTree, prefix: &mut Vec<String>, imports: &mut LocalImports) {
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            local_imports(&path.tree, prefix, imports);
            prefix.pop();
        }
        syn::UseTree::Name(name) if name.ident == "self" => imports.extend(prefix.last().map(|last| (last.clone(), prefix.clone()))),
        syn::UseTree::Name(name) => {
            let mut path = prefix.clone();
            path.push(name.ident.to_string());
            imports.push((name.ident.to_string(), path));
        }
        syn::UseTree::Rename(rename) if rename.rename != "_" => {
            let mut path = prefix.clone();
            if rename.ident != "self" {
                path.push(rename.ident.to_string());
            }
            imports.push((rename.rename.to_string(), path));
        }
        syn::UseTree::Group(group) => group.items.iter().for_each(|tree| local_imports(tree, prefix, imports)),
        // What a glob brings is not known here, and a name imported as `_` is no name.
        syn::UseTree::Rename(_) | syn::UseTree::Glob(_) => {}
    }
}

impl<'ast> Visit<'ast> for RegistrationFinder {
    /// A registration function of the untyped value conversions named with its types
    /// (`ValueTypes::register_nullable::<T>`): called, or handed to a function that calls
    /// it (`ValueTypes::register_deferred(ValueTypes::register_object::<T>)`). One whose
    /// types the path leaves to inference is not read: nothing states them.
    fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
        syn::visit::visit_expr_path(self, node);
        let segments: Vec<&syn::PathSegment> = node.path.segments.iter().collect();
        let [.., owner, last] = segments.as_slice() else { return };
        if owner.ident != "ValueTypes" {
            return;
        }
        let line = last.ident.span().start().line;
        let name = last.ident.to_string();
        let registration = name.strip_prefix("register_").and_then(|name| VALUE_REGISTRATIONS.iter().find(|(known, _)| *known == name));
        let (Some((registration, count)), Some(types)) = (registration, type_arguments_of(last)) else { return };
        if types.len() != *count {
            return;
        }
        match (*registration, types.as_slice()) {
            ("cast", [from, to]) => self.casts.push(((from.clone(), to.clone()), line)),
            _ => self.value_types.push(((registration.to_string(), types), line)),
        }
    }

    /// A macro the body defines for itself, or one it invokes in item position.
    fn visit_item_macro(&mut self, node: &'ast syn::ItemMacro) {
        let (name, _) = name_of_macro(&node.mac);
        match &node.ident {
            Some(defined) if name == "macro_rules" => {
                self.definitions.entry(defined.to_string()).or_default().extend(rules_of(&tokens_of(node.mac.tokens.clone())));
            }
            _ => self.visit_macro(&node.mac),
        }
    }

    /// A macro the body invokes: what it registers is in its expansion.
    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        let (name, line) = name_of_macro(node);
        self.invoked.push((name, tokens_of(node.tokens.clone()), line));
    }

    fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
        let mut prefix = Vec::new();
        if node.leading_colon.is_some() {
            prefix.push(String::new());
        }
        local_imports(&node.tree, &mut prefix, &mut self.imports);
    }

    fn visit_local(&mut self, node: &'ast syn::Local) {
        if let (syn::Pat::Ident(name), Some(init)) = (&node.pat, &node.init) {
            if let Some(type_) = markup_of(&init.expr) {
                self.bindings.push((name.ident.to_string(), tokens_of(type_.to_token_stream())));
            }
        }
        syn::visit::visit_local(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        syn::visit::visit_expr_call(self, node);
        let syn::Expr::Path(function) = &*node.func else { return };
        let segments: Vec<&syn::PathSegment> = function.path.segments.iter().collect();
        let [.., owner, last] = segments.as_slice() else { return };
        let line = last.ident.span().start().line;
        if owner.ident != "MarkupType" || last.ident != "register_handle" {
            return;
        }
        let handle = type_arguments_of(last).filter(|arguments| arguments.len() == 1).and_then(|mut arguments| arguments.pop());
        let type_ = node.args.first().and_then(|argument| match markup_of(argument) {
            Some(type_) => Some(tokens_of(type_.to_token_stream())),
            None => match argument {
                syn::Expr::Path(variable) => {
                    let name = variable.path.get_ident()?;
                    self.bindings.iter().rev().find(|(known, _)| name == known).map(|(_, type_)| type_.clone())
                }
                _ => None,
            },
        });
        match (handle, type_) {
            (Some(handle), Some(type_)) if node.args.len() == 1 => self.found.push(((handle, type_), line)),
            _ => self.unread.push(line),
        }
    }
}

impl Source {
    /// The handles and the casts the function with the body `block` registers.
    fn read_registrations(&mut self, block: &syn::Block, context: &Context, cfg: &[String]) {
        let mut finder = RegistrationFinder::default();
        finder.visit_block(block);
        self.take_registrations(finder, context, cfg);
    }

    /// What the body of a declaration registers with the untyped value conversions: the
    /// accessor of a property that registers the element reference of its value type when
    /// it first runs (`ValueTypes::register_element_ref::<Control>();`). The body is tokens,
    /// so the calls are found in the tokens, with their types stated.
    fn read_registrations_of_tokens(&mut self, tokens: &[TokenTree], site: &Site) {
        let mut stated: Vec<(String, Vec<Tokens>, usize)> = Vec::new();
        stated_calls_of(tokens, "ValueTypes", &mut |function, types, line| stated.push((function.to_string(), types, line)));
        for (function, types, line) in stated {
            let Some((registration, count)) = function.strip_prefix("register_").and_then(|name| VALUE_REGISTRATIONS.iter().find(|(known, _)| *known == name)) else {
                continue;
            };
            if types.len() != *count {
                continue;
            }
            let site = Site { line, ..site.clone() };
            self.value_calls.entry(registration.to_string()).or_default().1 += 1;
            match (*registration, types.as_slice()) {
                ("cast", [from, to]) => self.casts.push(Located { site, value: RawCast { from: from.clone(), to: to.clone(), imports: Vec::new() } }),
                _ => self.value_types.push(Located { site, value: RawValueType { registration: registration.to_string(), types, imports: Vec::new() } }),
            }
        }
    }

    /// What the value of a constant or of a static registers: a function written as a
    /// closure in it (`value_types: || { ValueTypes::register_reference::<T>(); }`).
    fn read_registrations_of_value(&mut self, value: &syn::Expr, context: &Context, cfg: &[String]) {
        let mut finder = RegistrationFinder::default();
        finder.visit_expr(value);
        self.take_registrations(finder, context, cfg);
    }

    fn take_registrations(&mut self, finder: RegistrationFinder, context: &Context, cfg: &[String]) {
        if !finder.invoked.is_empty() {
            let line = finder.invoked[0].2;
            self.invoked_macros.push(InvokedMacros {
                definitions: finder.definitions.clone(),
                invoked: finder.invoked.clone(),
                imports: finder.imports.clone(),
                site: Site { module: context.module, file: context.file, line, cfg: cfg.to_vec(), expanded: true },
            });
        }
        self.take_found(finder, context.module, context.file, cfg, true);
    }

    /// Expands the macros the functions invoke in their bodies, where the macro is one of
    /// the crate (or of the body) and a rule of it has a form the scanner applies, and
    /// reads what the expansions register with the untyped value conversions: a table of
    /// casts written as invocations of a macro (`assignable!(A => Rc<dyn B>);`), also when
    /// the invocations are written by another macro that is handed the name of the first
    /// (`for_each!(assignable)`).
    ///
    /// The calls in the text of a definition count as read when every invocation of the
    /// macro that was met is expanded; a definition with an invocation that is not expanded
    /// (a rule that repeats a part of its input) keeps its calls as not read.
    fn expand_registration_macros(&mut self) {
        const MAX_DEPTH: usize = 8;
        let pending = std::mem::take(&mut self.invoked_macros);
        // By definition: the calls of the registration functions in its text, whether an
        // invocation was expanded, and whether one was not.
        let mut definitions_met: BTreeMap<String, (BTreeMap<String, usize>, bool, bool)> = BTreeMap::new();
        for (body_index, body) in pending.into_iter().enumerate() {
            let mut definitions = body.definitions;
            let mut queue: Vec<(String, Tokens, usize, usize)> = body.invoked.into_iter().map(|(name, tokens, line)| (name, tokens, line, 0)).collect();
            while let Some((name, tokens, line, depth)) = queue.pop() {
                // A definition of the body before one of the crate.
                let (key, rules) = match definitions.get(&name) {
                    Some(rules) => (format!("{body_index}:{name}"), rules.clone()),
                    None => match self.local_macros.get(&name).map(Vec::as_slice) {
                        Some([definition]) => (name.clone(), definition.rules.clone()),
                        _ => continue,
                    },
                };
                let met = definitions_met.entry(key).or_insert_with(|| {
                    let mut calls: BTreeMap<String, usize> = BTreeMap::new();
                    for (_, transcriber) in &rules {
                        calls_of(transcriber, "ValueTypes", &[], &mut |function| {
                            if let Some(registration) = function.strip_prefix("register_").filter(|name| VALUE_REGISTRATIONS.iter().any(|(known, _)| known == name)) {
                                *calls.entry(registration.to_string()).or_default() += 1;
                            }
                        });
                    }
                    (calls, false, false)
                });
                let expansion = match depth < MAX_DEPTH {
                    true => rules.iter().find_map(|(matcher, transcriber)| bind(matcher, &tokens).map(|bindings| substitute(transcriber, &bindings))).flatten(),
                    false => None,
                };
                let block = expansion.and_then(|expansion| {
                    let braced = TokenTree::Group(Group::new(Delimiter::Brace, expansion.into_iter().collect()));
                    syn::parse2::<syn::Block>(std::iter::once(braced).collect()).ok()
                });
                let Some(block) = block else {
                    met.2 = true;
                    continue;
                };
                met.1 = true;
                let mut finder = RegistrationFinder::default();
                finder.visit_block(&block);
                for (defined, rules) in std::mem::take(&mut finder.definitions) {
                    definitions.entry(defined).or_default().extend(rules);
                }
                queue.extend(std::mem::take(&mut finder.invoked).into_iter().map(|(name, tokens, _)| (name, tokens, line, depth + 1)));
                finder.imports = body.imports.clone();
                // The tokens of an expansion have the lines of the definition: what it
                // registers is at the invocation.
                for entry in finder.found.iter_mut() {
                    entry.1 = line;
                }
                for entry in finder.casts.iter_mut() {
                    entry.1 = line;
                }
                for entry in finder.value_types.iter_mut() {
                    entry.1 = line;
                }
                finder.unread.clear();
                self.take_found(finder, body.site.module, body.site.file, &body.site.cfg, false);
            }
        }
        for (calls, expanded, not_expanded) in definitions_met.into_values() {
            if expanded && !not_expanded {
                for (registration, count) in calls {
                    self.value_calls.entry(registration).or_default().1 += count;
                }
            }
        }
    }

    /// Takes what `finder` found. `counted`: the calls are calls in the text of a function
    /// (the ones of an expansion are counted with the definition of the macro).
    fn take_found(&mut self, finder: RegistrationFinder, module: usize, file: usize, cfg: &[String], counted: bool) {
        let context = (module, file);
        let site = |line: usize| Site { module: context.0, file: context.1, line, cfg: cfg.to_vec(), expanded: false };
        for ((handle, type_), line) in finder.found {
            self.handles.push(Located { site: site(line), value: RawHandle { handle, type_, imports: finder.imports.clone() } });
        }
        for ((from, to), line) in finder.casts {
            if counted {
                self.value_calls.entry("cast".to_string()).or_default().1 += 1;
            }
            self.casts.push(Located { site: site(line), value: RawCast { from, to, imports: finder.imports.clone() } });
        }
        for ((registration, types), line) in finder.value_types {
            if counted {
                self.value_calls.entry(registration.clone()).or_default().1 += 1;
            }
            self.value_types.push(Located { site: site(line), value: RawValueType { registration, types, imports: finder.imports.clone() } });
        }
        for line in finder.unread {
            let message = "`MarkupType::register_handle` is called with a type that is not `<Type as MarkupTyped>::MARKUP` or a variable of the function bound to it: the handle is not in the model".to_string();
            self.diagnostic_at(Severity::Warning, codes::FORM, context.1, line, message);
        }
    }

    /// The value of the member `name` of the enumeration or of the set of flags declared
    /// as `declared` (the module and the name): a variant, a constant of the `bitflags!`
    /// type, or an associated constant of the type (`pub const NONE: Flags = Flags::empty();`,
    /// `pub const Enter: Key = Key::Return;`) whose expression is a constant expression
    /// over literals and the other members.
    pub fn member_value(&self, declared: &(usize, String), name: &str, depth: usize) -> Option<i64> {
        if depth > 16 {
            return None;
        }
        if let Some((_, value)) = self.enums.get(declared).and_then(|variants| variants.iter().find(|(variant, _)| variant == name)) {
            return *value;
        }
        let constant = |constants: &BTreeMap<(usize, String), Vec<(String, Tokens)>>| -> Option<Tokens> {
            constants.get(declared).and_then(|constants| constants.iter().find(|(constant, _)| constant == name)).map(|(_, expression)| expression.clone())
        };
        let expression = constant(&self.flags).or_else(|| constant(&self.associated))?;
        let member = |other: &str| self.member_value(declared, other, depth + 1);
        let all = || {
            let constants = self.flags.get(declared)?;
            constants.iter().try_fold(0i64, |union, (constant, _)| Some(union | self.member_value(declared, constant, depth + 1)?))
        };
        evaluate(&expression, &Scope { type_name: &declared.1, member: &member, all: &all })
    }
}
