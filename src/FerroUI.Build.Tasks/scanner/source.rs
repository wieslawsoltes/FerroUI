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

use super::declarations::{read_declaration, read_property, Declaration, DECLARATION_MACROS};
use super::modules::{Glob, Import, ItemKind, Modules};
use super::tokens::{group_of, ident_of, invocations, is_arrow, is_punct, text_of, tokens_of, Cursor, ParseError, Tokens, TypeEnd};
use super::{codes, Diagnostic, InvocationCounts, ScanOptions, ScannedFile, Severity};

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

/// An invocation, in item position, of a macro the scanner does not read by itself.
pub(crate) struct Invocation {
    pub name: String,
    pub tokens: Tokens,
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
            constants: BTreeMap::new(),
            associated_constants: BTreeMap::new(),
            functions: Vec::new(),
            hand_written: Vec::new(),
            assemblies: Vec::new(),
            namespaces: Vec::new(),
            rust_paths: Vec::new(),
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
                let line = error.span().start().line;
                self.diagnostic_at(Severity::Error, codes::FILE, file, line, format!("the file is not read: {error}"));
                return;
            }
        };
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
            syn::Item::Type(item) => self.modules.add_item(module, &item.ident.to_string(), ItemKind::Type, is_public(&item.vis)),
            syn::Item::Fn(item) => self.modules.add_item(module, &item.sig.ident.to_string(), ItemKind::Value, is_public(&item.vis)),
            syn::Item::Enum(item) => {
                let name = item.ident.to_string();
                self.modules.add_item(module, &name, ItemKind::Type, is_public(&item.vis));
                let mut next = Some(0i64);
                let mut variants = Vec::new();
                for variant in &item.variants {
                    if let Some((_, expression)) = &variant.discriminant {
                        next = integer_of(expression);
                    }
                    variants.push((variant.ident.to_string(), next));
                    next = next.and_then(|value| value.checked_add(1));
                }
                self.enums.insert((module, name), variants);
            }
            syn::Item::Const(item) => {
                let name = item.ident.to_string();
                self.modules.add_item(module, &name, ItemKind::Value, is_public(&item.vis));
                if let Text::Literal(text) = text_of_expression(&item.expr) {
                    self.constants.insert((module, name.clone()), text);
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
            _ => self.invocations.push(Invocation { name, tokens, site }),
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
                            let result = read_property(&mut Cursor::new(&tokens, line));
                            let category = if result.is_ok() { Category::Read } else { Category::Failed };
                            let counts = &mut self.files[file].invocations;
                            add_count(counts, &name, category);
                            invocations(&tokens, DECLARATION_MACROS, &mut |name, _| add_count(counts, name, category));
                            match result {
                                Ok((stated, accessor)) => {
                                    let declaration = Declaration::Properties { owner: stated.unwrap_or_else(|| owner.clone()), accessors: vec![accessor] };
                                    self.declarations.push(Located { site, value: declaration });
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
                        _ => {}
                    }
                }
                (syn::ImplItem::Fn(member), Some(owner), None) => {
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
            let site = Site { expanded: true, ..invocation.site };
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
                        match read_declaration(&declaration_macro, &tokens_of(group.stream()), site.line) {
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

/// The value of an integer literal, negated or not.
fn integer_of(expression: &syn::Expr) -> Option<i64> {
    match expression {
        syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Int(integer), .. }) => integer.base10_parse::<i64>().ok(),
        syn::Expr::Unary(syn::ExprUnary { op: syn::UnOp::Neg(_), expr, .. }) => integer_of(expr).and_then(i64::checked_neg),
        syn::Expr::Paren(inner) => integer_of(&inner.expr),
        syn::Expr::Group(inner) => integer_of(&inner.expr),
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

/// The value of the expression of a constant of a `bitflags!` type: integer literals,
/// `<<`, `|`, parentheses and the other constants of the type (`Self::A.bits()`).
pub(crate) fn flags_value(constants: &[(String, Tokens)], name: &str, depth: usize) -> Option<i64> {
    let (_, expression) = constants.iter().find(|(constant, _)| constant == name)?;
    if depth > 16 {
        return None;
    }
    let mut cursor = Cursor::new(expression, 0);
    let value = flags_or(constants, &mut cursor, depth)?;
    cursor.is_end().then_some(value)
}

fn flags_or(constants: &[(String, Tokens)], cursor: &mut Cursor, depth: usize) -> Option<i64> {
    let mut value = flags_shift(constants, cursor, depth)?;
    while cursor.eat_punct('|') {
        value |= flags_shift(constants, cursor, depth)?;
    }
    Some(value)
}

fn flags_shift(constants: &[(String, Tokens)], cursor: &mut Cursor, depth: usize) -> Option<i64> {
    let mut value = flags_primary(constants, cursor, depth)?;
    while cursor.is_punct('<') && cursor.peek_at(1).is_some_and(|token| is_punct(token, '<')) {
        cursor.next();
        cursor.next();
        let shift = flags_primary(constants, cursor, depth)?;
        value = value.checked_shl(u32::try_from(shift).ok()?)?;
    }
    Some(value)
}

fn flags_primary(constants: &[(String, Tokens)], cursor: &mut Cursor, depth: usize) -> Option<i64> {
    match cursor.next()? {
        TokenTree::Literal(literal) => match syn::Lit::new(literal.clone()) {
            syn::Lit::Int(integer) => integer.base10_parse::<i64>().ok(),
            _ => None,
        },
        TokenTree::Group(group) if group.delimiter() == Delimiter::Parenthesis || group.delimiter() == Delimiter::None => {
            let inner = tokens_of(group.stream());
            let mut inner = Cursor::new(&inner, 0);
            let value = flags_or(constants, &mut inner, depth)?;
            inner.is_end().then_some(value)
        }
        TokenTree::Ident(_) => {
            // `Self::A.bits()` or `Name::A.bits()`: a constant of the same type.
            if !cursor.eat_path_separator() {
                return None;
            }
            let constant = cursor.take_ident("a constant").ok()?;
            if !cursor.eat_punct('.') || !cursor.eat_ident("bits") {
                return None;
            }
            cursor.take_group(Delimiter::Parenthesis, "`()`").ok()?;
            flags_value(constants, &constant, depth + 1)
        }
        _ => None,
    }
}
