//! The front end of the XAML compiler for a theme written in Rust (#988, a prototype;
//! docs/xaml-themes.md, section 10).
//!
//! A theme in Rust is one module per document over the API of `ferroui_markup_xaml::rust_theme`
//! (`scripts/themes/xaml_to_rust.py` writes it from the documents). This front end reads those
//! modules with `syn` and gives, for each document, the XAML document the module describes: the
//! same resources, control themes, styles, templates, bindings and comments, in the same order.
//! The compiler then compiles those documents as it compiles the documents of the theme, so that
//! everything it does to a theme (units, trimming by roots, packed rows, shared helpers,
//! compiled bindings) applies to the theme in Rust.
//!
//! The front end reads the API of themes, not any Rust: a statement or an expression outside the
//! forms the API has is an error that names the module and the line. Types and properties are
//! resolved through the type models of the build (the models of the crates the theme is built
//! on and the scan of its own sources), the same models the compiler compiles the documents
//! against.

use ferroui_build_scan::model::{AssemblyModel, RegisteredKind, TypeModel};
use ferroui_build_scan::model_set::ModelSet;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use syn::spanned::Spanned;
use syn::{Expr, ExprCall, ExprClosure, ExprMethodCall, ExprPath, Item, ItemFn, Lit, Stmt};

const NS_FERRO: &str = "https://github.com/ferroui";
const NS_X: &str = "http://schemas.microsoft.com/winfx/2006/xaml";

/// The XAML documents of the theme in Rust below `directory` (`<crate>/rust_theme`), by the
/// names of `documents` (`Controls/Button.xaml`): each from the module the converter wrote for
/// it. `units` gives the document of each unit index (`MergeResourceInclude` of a unit).
pub fn documents(
    directory: &Path,
    documents: &[String],
    models: Vec<AssemblyModel>,
    units: &HashMap<u32, String>,
) -> Result<Vec<(String, String)>, String> {
    let models = ModelSet::new(models);
    let by_module: HashMap<String, String> = documents.iter().map(|document| (module_of(document), document.clone())).collect();
    let mut out = Vec::with_capacity(documents.len());
    let mut errors = Vec::new();
    for document in documents {
        let path = directory.join(format!("{}.rs", module_of(document).replace("::", "/")));
        let text = fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        let file = syn::parse_file(&text).map_err(|error| format!("{}: {error}", path.display()))?;
        let mut reader = Reader::new(&models, &file, units, document, &by_module);
        match reader.document() {
            Ok(xaml) => out.push((document.clone(), xaml)),
            Err(error) => errors.push(format!("{document}: {error}")),
        }
    }
    if !errors.is_empty() {
        return Err(errors.join("; "));
    }
    Ok(out)
}

/// The module of a document: `Controls/Button.xaml` is `controls::button` (the rule of the
/// converter).
pub fn module_of(document: &str) -> String {
    let stem = document.strip_suffix(".xaml").unwrap_or(document);
    stem.split('/').map(snake).collect::<Vec<_>>().join("::")
}

fn snake(name: &str) -> String {
    let name = name.replace(['.', '-'], "_");
    let chars: Vec<char> = name.chars().collect();
    let mut out = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if c.is_ascii_uppercase() && i > 0 {
            let previous = chars[i - 1];
            let next = chars.get(i + 1).copied();
            let after_lower = previous.is_ascii_lowercase() || previous.is_ascii_digit();
            let acronym_end = previous.is_ascii_uppercase() && next.is_some_and(|n| n.is_ascii_lowercase());
            if (after_lower || acronym_end) && !out.ends_with('_') {
                out.push('_');
            }
        }
        out.push(c.to_ascii_lowercase());
    }
    let mut collapsed = String::new();
    for c in out.chars() {
        if c == '_' && collapsed.ends_with('_') {
            continue;
        }
        collapsed.push(c);
    }
    let trimmed = collapsed.trim_matches('_').to_string();
    if trimmed.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        format!("_{trimmed}")
    } else {
        trimmed
    }
}

// --- the XAML tree ---------------------------------------------------------------------------

/// An element of the document being written.
#[derive(Default)]
struct Node {
    tag: String,
    attributes: Vec<(String, String)>,
    children: Vec<Child>,
    text: Option<String>,
}

enum Child {
    Node(Node),
    /// A comment of the document (the front end writes none: `syn` keeps no comments).
    #[allow(dead_code)]
    Comment(String),
}

impl Node {
    fn new(tag: impl Into<String>) -> Self {
        Node { tag: tag.into(), ..Node::default() }
    }

    fn attribute(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.attributes.push((name.into(), value.into()));
    }

    fn child(&mut self, node: Node) {
        self.children.push(Child::Node(node));
    }

    fn write(&self, out: &mut String, indent: usize) {
        let pad = "  ".repeat(indent);
        let _ = write!(out, "{pad}<{}", self.tag);
        for (name, value) in &self.attributes {
            let _ = write!(out, " {name}=\"{}\"", escape(value));
        }
        if self.children.is_empty() && self.text.is_none() {
            out.push_str(" />\n");
            return;
        }
        if let Some(text) = &self.text {
            let _ = writeln!(out, ">{}</{}>", escape_text(text), self.tag);
            return;
        }
        out.push_str(">\n");
        for child in &self.children {
            match child {
                Child::Node(node) => node.write(out, indent + 1),
                Child::Comment(text) => {
                    let _ = writeln!(out, "{}<!--{}-->", "  ".repeat(indent + 1), text.replace("--", "- -"));
                }
            }
        }
        let _ = writeln!(out, "{pad}</{}>", self.tag);
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;").replace('>', "&gt;")
}

fn escape_text(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

// --- reading a module ------------------------------------------------------------------------

/// A path whose last segment names a member of the type the rest names (`Type::new`).
struct Owner<'p>(&'p syn::Path);

impl Owner<'_> {
    fn is_empty(&self) -> bool {
        self.0.segments.len() < 2
    }
}

/// A type of the models, as the document names it.
#[derive(Clone)]
struct XamlType {
    /// `Border`, `n0:InfoBarPanel`.
    name: String,
}

struct Reader<'a> {
    models: &'a ModelSet,
    functions: HashMap<String, &'a ItemFn>,
    imports: HashMap<String, String>,
    units: &'a HashMap<u32, String>,
    document: &'a str,
    /// The documents of the build by their module.
    documents: &'a HashMap<String, String>,
    /// The XML namespaces of the CLR namespaces the document names, by prefix.
    prefixes: BTreeMap<String, String>,
    default_namespaces: Vec<String>,
}

type R<T> = Result<T, String>;

fn at(node: &impl Spanned, message: impl Into<String>) -> String {
    let start = node.span().start();
    format!("{}:{}: {}", start.line, start.column + 1, message.into())
}

fn path_text(path: &syn::Path) -> String {
    let mut text = String::new();
    if path.leading_colon.is_some() {
        text.push_str("::");
    }
    let segments: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
    text.push_str(&segments.join("::"));
    text
}

fn string_literal(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Lit(lit) => match &lit.lit {
            Lit::Str(s) => Some(s.value()),
            _ => None,
        },
        _ => None,
    }
}

fn number_text(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Lit(lit) => match &lit.lit {
            Lit::Float(f) => Some(trim_number(f.base10_digits())),
            Lit::Int(i) => Some(i.base10_digits().to_string()),
            _ => None,
        },
        Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Neg(_)) => number_text(&unary.expr).map(|n| format!("-{n}")),
        Expr::Group(group) => number_text(&group.expr),
        Expr::Paren(paren) => number_text(&paren.expr),
        _ => None,
    }
}

fn trim_number(digits: &str) -> String {
    match digits.strip_suffix(".0") {
        Some(integral) => integral.to_string(),
        None => digits.to_string(),
    }
}

/// The receiver, name and arguments of a chain of method calls, innermost first.
fn chain(expr: &Expr) -> (&Expr, Vec<&ExprMethodCall>) {
    let mut calls = Vec::new();
    let mut current = expr;
    loop {
        match current {
            Expr::MethodCall(call) => {
                calls.push(call);
                current = &call.receiver;
            }
            Expr::Try(try_) => current = &try_.expr,
            Expr::Paren(paren) => current = &paren.expr,
            _ => break,
        }
    }
    calls.reverse();
    (current, calls)
}

fn strip_try(expr: &Expr) -> &Expr {
    match expr {
        Expr::Try(try_) => strip_try(&try_.expr),
        Expr::Paren(paren) => strip_try(&paren.expr),
        _ => expr,
    }
}

/// The argument of `Some(..)`, `Ok(..)` or `value(..)`, else the expression.
fn unwrap_call<'e>(expr: &'e Expr, names: &[&str]) -> &'e Expr {
    if let Expr::Call(call) = strip_try(expr) {
        if let Expr::Path(path) = &*call.func {
            let name = path.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
            if names.contains(&name.as_str()) && call.args.len() == 1 {
                return unwrap_call(&call.args[0], names);
            }
        }
    }
    expr
}

fn closure_body(expr: &Expr) -> Option<&Expr> {
    match expr {
        Expr::Closure(ExprClosure { body, .. }) => Some(body),
        _ => None,
    }
}

/// The value of a block that is one expression (`{ Ok(value(..)) }`), else the expression.
fn single(expr: &Expr) -> &Expr {
    if let Expr::Block(block) = expr {
        if let [Stmt::Expr(inner, None)] = block.block.stmts.as_slice() {
            return single(inner);
        }
    }
    expr
}

fn block_statements(expr: &Expr) -> Vec<&Stmt> {
    match expr {
        Expr::Block(block) => block.block.stmts.iter().collect(),
        _ => Vec::new(),
    }
}

impl<'a> Reader<'a> {
    fn new(
        models: &'a ModelSet,
        file: &'a syn::File,
        units: &'a HashMap<u32, String>,
        document: &'a str,
        documents: &'a HashMap<String, String>,
    ) -> Self {
        let mut functions = HashMap::new();
        let mut imports = HashMap::new();
        for item in &file.items {
            match item {
                Item::Fn(function) => {
                    functions.insert(function.sig.ident.to_string(), function);
                }
                Item::Use(use_) => collect_uses(&use_.tree, String::new(), &mut imports),
                _ => {}
            }
        }
        let mut default_namespaces = Vec::new();
        for model in models.models() {
            for definition in &model.xmlns_definitions {
                if definition.xml_namespace == NS_FERRO && !default_namespaces.contains(&definition.namespace) {
                    default_namespaces.push(definition.namespace.clone());
                }
            }
        }
        Reader { models, functions, imports, units, document, documents, prefixes: BTreeMap::new(), default_namespaces }
    }

    /// The document of the module `controls::button`, as the converter named it: the documents
    /// of the theme are found by their module.
    fn document_of_module(&self, module: &str) -> String {
        self.documents.get(module).cloned().unwrap_or_else(|| module.replace("::", "/"))
    }

    fn function(&self, name: &str) -> R<&'a ItemFn> {
        self.functions.get(name).copied().ok_or_else(|| format!("no function `{name}`"))
    }

    /// The absolute path of a path of the module: through its imports.
    fn absolute(&self, path: &syn::Path) -> String {
        let segments: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
        self.absolute_of(path.leading_colon.is_some(), &segments)
    }

    /// The type a path names without its last segment (`Type` of `Type::new`).
    fn type_of_owner(&mut self, owner: &Owner) -> R<(XamlType, &'a TypeModel)> {
        let segments: Vec<String> = owner.0.segments.iter().map(|s| s.ident.to_string()).collect();
        let segments = &segments[..segments.len().saturating_sub(1)];
        let absolute = self.absolute_of(owner.0.leading_colon.is_some(), segments);
        let model = self.type_model(&absolute)?;
        Ok((self.xaml_type(model), model))
    }

    fn absolute_of(&self, leading: bool, segments: &[String]) -> String {
        let text = format!("{}{}", if leading { "::" } else { "" }, segments.join("::"));
        let path_segments = segments;
        if text.starts_with("::") {
            return text;
        }
        if let Some(rest) = text.strip_prefix("crate::") {
            let crate_name = self.models.models().last().map(|m| m.crate_name.clone()).unwrap_or_default();
            return format!("::{crate_name}::{rest}");
        }
        let first = path_segments.first().cloned().unwrap_or_default();
        match self.imports.get(&first) {
            Some(imported) => {
                let rest: Vec<String> = path_segments.iter().skip(1).cloned().collect();
                let imported = match imported.strip_prefix("crate::") {
                    Some(inner) => {
                        let crate_name = self.models.models().last().map(|m| m.crate_name.clone()).unwrap_or_default();
                        format!("::{crate_name}::{inner}")
                    }
                    None => imported.clone(),
                };
                if rest.is_empty() {
                    imported
                } else {
                    format!("{imported}::{}", rest.join("::"))
                }
            }
            None => format!("::{text}"),
        }
    }

    fn type_model(&self, path: &str) -> R<&'a TypeModel> {
        let canonical = self.models.canonical_path(path);
        self.models
            .find_rust_type(&canonical)
            .or_else(|| self.models.find_rust_type(path))
            .or_else(|| self.models.find_rust_type(&format!("dyn {canonical}")))
            .map(|(_, type_)| type_)
            .ok_or_else(|| format!("the type `{path}` is in no type model"))
    }

    /// The name of a type in the document, with the prefix of its namespace.
    fn xaml_type(&mut self, type_: &TypeModel) -> XamlType {
        let namespace = type_.namespace.clone();
        if self.default_namespaces.contains(&namespace) {
            return XamlType { name: type_.name.clone() };
        }
        let uri = format!("using:{namespace}");
        let prefix = match self.prefixes.iter().find(|(_, existing)| **existing == uri) {
            Some((prefix, _)) => prefix.clone(),
            None => {
                // The prefix a person writes: the last part of the namespace in lower case
                // (`using:FerroUI.Themes.WinUI.Accents` is `accents`), with the part before it
                // when another namespace has that prefix (`winuiconverters`).
                let parts: Vec<String> = namespace.split('.').map(|part| part.to_lowercase()).collect();
                let mut prefix = parts.last().cloned().unwrap_or_default();
                if self.prefixes.contains_key(&prefix) || prefix == "x" {
                    prefix = parts.iter().rev().take(2).rev().cloned().collect::<Vec<_>>().concat();
                }
                while self.prefixes.contains_key(&prefix) {
                    prefix.push('_');
                }
                self.prefixes.insert(prefix.clone(), uri);
                prefix
            }
        };
        XamlType { name: format!("{prefix}:{}", type_.name) }
    }

    fn type_of_path(&mut self, path: &syn::Path) -> R<(XamlType, &'a TypeModel)> {
        let absolute = self.absolute(path);
        let model = self.type_model(&absolute)?;
        Ok((self.xaml_type(model), model))
    }

    /// The type of `T` in `f::<T>(..)` or `T::new()`.
    fn type_of_generic(&mut self, segment: &syn::PathSegment, node: &impl Spanned) -> R<(XamlType, &'a TypeModel)> {
        if let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments {
            return self.type_of_arguments(Some(arguments), node);
        }
        Err(at(node, "a type argument was expected"))
    }

    /// The type of `T` in `::<T>`.
    fn type_of_arguments(&mut self, arguments: Option<&syn::AngleBracketedGenericArguments>, node: &impl Spanned) -> R<(XamlType, &'a TypeModel)> {
        if let Some(arguments) = arguments {
            if let Some(syn::GenericArgument::Type(syn::Type::Path(type_path))) = arguments.args.first() {
                return self.type_of_path(&type_path.path);
            }
            if let Some(syn::GenericArgument::Type(syn::Type::TraitObject(object))) = arguments.args.first() {
                if let Some(syn::TypeParamBound::Trait(bound)) = object.bounds.first() {
                    return self.type_of_path(&bound.path);
                }
            }
        }
        Err(at(node, "a type argument was expected"))
    }

    fn chain_of_types(&self, mut type_: &'a TypeModel) -> Vec<&'a TypeModel> {
        let mut out = vec![type_];
        for _ in 0..64 {
            let Some(base) = &type_.base else { break };
            match self.models.find_rust_type(&base.text) {
                Some((_, found)) if !std::ptr::eq(found, type_) => {
                    out.push(found);
                    type_ = found;
                }
                _ => break,
            }
        }
        out
    }

    /// The registered property of the accessor `X::name_property()` in `expr`: its owner and
    /// its name.
    fn property(&self, expr: &Expr) -> R<(&'a TypeModel, String, RegisteredKind)> {
        let call = match strip_try(expr) {
            Expr::Call(call) => call,
            Expr::MethodCall(method) if method.method == "property" => match &*method.receiver {
                Expr::Call(call) => call,
                other => return Err(at(other, "a property accessor was expected")),
            },
            other => return Err(at(other, "a property accessor was expected")),
        };
        let Expr::Path(ExprPath { path, .. }) = &*call.func else { return Err(at(call, "a property accessor was expected")) };
        let absolute = self.absolute(path);
        let (owner, registered) =
            self.models.find_accessor(&absolute).ok_or_else(|| at(call, format!("`{absolute}` is no accessor of the models")))?;
        let name = self.models.name_of(registered).ok_or_else(|| at(call, format!("the property of `{absolute}` has no name")))?;
        Ok((owner, name.to_string(), registered.kind.clone()))
    }

    /// The attribute name of the property `expr` on an element of `element`: its name when the
    /// element has it, else `Owner.Name`.
    fn attribute_name(&mut self, element: &'a TypeModel, expr: &Expr) -> R<String> {
        let (owner, name, kind) = self.property(expr)?;
        let chain = self.chain_of_types(element);
        let accessor = accessor_name(expr);
        let listed = chain.iter().any(|type_| {
            std::ptr::eq(*type_, owner)
                || type_.full_name() == owner.full_name()
                || type_.registered.iter().any(|registered| Some(registered.accessor.as_str()) == accessor.as_deref())
        });
        // An attached property is named alone only where the class of the element registers it
        // itself (`TextBox` adds `TextWrapping`); elsewhere by its owner (`TemplatedControl.IsTemplateFocusTarget`).
        let own = element.registered.iter().any(|registered| Some(registered.accessor.as_str()) == accessor.as_deref())
            || std::ptr::eq(element, owner);
        if listed && (kind != RegisteredKind::Attached || own) {
            return Ok(name);
        }
        if kind == RegisteredKind::Attached {
            let owner_type = self.xaml_type(owner);
            return Ok(format!("{}.{name}", owner_type.name));
        }
        let has = chain.iter().any(|type_| {
            type_.registered.iter().any(|registered| self.models.name_of(registered) == Some(name.as_str()))
        });
        if (listed || has) && kind != RegisteredKind::Attached && (listed || !owner_has_attached(owner, &name, self.models)) {
            return Ok(name);
        }
        let owner_type = self.xaml_type(owner);
        Ok(format!("{}.{name}", owner_type.name))
    }

    /// The name of the source of a template binding: `Owner.Name` for an attached property.
    fn source_name(&mut self, expr: &Expr) -> R<String> {
        let (owner, name, kind) = self.property(expr)?;
        if kind == RegisteredKind::Attached {
            let owner = self.xaml_type(owner);
            return Ok(format!("{}.{name}", owner.name));
        }
        Ok(name)
    }

    fn property_name_on(&mut self, element: Option<&'a TypeModel>, expr: &Expr) -> R<String> {
        match element {
            Some(element) => self.attribute_name(element, expr),
            None => {
                let (owner, name, kind) = self.property(expr)?;
                if kind == RegisteredKind::Attached {
                    let owner_type = self.xaml_type(owner);
                    return Ok(format!("{}.{name}", owner_type.name));
                }
                Ok(name)
            }
        }
    }

    // --- the document ---

    fn document(&mut self) -> R<String> {
        let (root, class) = if self.functions.contains_key("merge") {
            let mut root = Node::new("ResourceDictionary");
            let merge = self.function("merge")?;
            self.dictionary_statements(&merge.block.stmts, &mut root, "resources")?;
            (root, None)
        } else if self.functions.contains_key("populate") {
            let mut root = Node::new("Styles");
            let populate = self.function("populate")?;
            self.styles_statements(&populate.block.stmts, &mut root)?;
            (root, (self.document == "WinUITheme.xaml").then_some("FerroUI.Themes.WinUI.WinUITheme"))
        } else {
            return Err("the module has neither `merge` nor `populate`".to_string());
        };
        let mut root = root;
        let mut attributes = Vec::new();
        if let Some(class) = class {
            attributes.push(("x:Class".to_string(), class.to_string()));
        }
        attributes.push(("xmlns".to_string(), NS_FERRO.to_string()));
        attributes.push(("xmlns:x".to_string(), NS_X.to_string()));
        for (prefix, uri) in &self.prefixes {
            attributes.push((format!("xmlns:{prefix}"), uri.clone()));
        }
        if class.is_none() {
            attributes.push(("x:ClassModifier".to_string(), "internal".to_string()));
        }
        attributes.append(&mut root.attributes);
        root.attributes = attributes;
        let mut out = String::new();
        root.write(&mut out, 0);
        Ok(out)
    }

    fn styles_statements(&mut self, statements: &[Stmt], root: &mut Node) -> R<()> {
        let mut dictionary: Option<Node> = None;
        for statement in statements {
            let expr: &Expr = match statement {
                Stmt::Expr(expr, _) => expr,
                Stmt::Local(syn::Local { init: Some(syn::LocalInit { expr, .. }), .. }) => expr,
                _ => continue,
            };
            let expr = strip_try(expr);
            match expr {
                // `let resources = Resources::new(&styles.resources(), scope);`
                Expr::Call(call) if path_of_call(call).ends_with("Resources::new") => {
                    dictionary = Some(Node::new("ResourceDictionary"));
                }
                Expr::Call(call) if path_of_call(call).ends_with("with_styles") => {}
                Expr::Reference(_) => {}
                Expr::Call(call) if path_of_call(call) == "Ok" => {}
                Expr::MethodCall(call) if call.method == "with_styles" => {}
                Expr::MethodCall(call) if call.method == "add" && expr_text(&call.receiver) == "styles" => {
                    let argument = strip_try(&call.args[0]);
                    let Expr::Call(inner) = argument else { return Err(at(argument, "a style was expected")) };
                    let function = path_of_call(inner);
                    if function.ends_with("::build") {
                        let module = function.trim_start_matches("crate::rust_theme::").trim_end_matches("::build");
                        let document = self.document_of_module(module);
                        let mut include = Node::new("StyleInclude");
                        include.attribute("Source", format!("/{document}"));
                        root.child(include);
                    } else if function.ends_with("style") {
                        let style = self.style_call(inner, None)?;
                        root.child(style);
                    } else {
                        return Err(at(argument, format!("`{function}` in a styles document")));
                    }
                }
                other => {
                    if let Some(node) = dictionary.as_mut() {
                        self.dictionary_statement(other, node, "resources")?;
                    } else {
                        return Err(at(other, "a statement of a styles document was expected"));
                    }
                }
            }
        }
        if let Some(dictionary) = dictionary {
            let mut resources = Node::new("Styles.Resources");
            resources.child(dictionary);
            root.children.insert(0, Child::Node(resources));
        }
        Ok(())
    }

    // --- resource dictionaries ---

    fn dictionary_statements(&mut self, statements: &[Stmt], node: &mut Node, target: &str) -> R<()> {
        for statement in statements {
            match statement {
                Stmt::Expr(expr, _) => {
                    if matches!(expr, Expr::Call(call) if path_of_call(call) == "Ok") {
                        continue;
                    }
                    self.dictionary_statement(strip_try(expr), node, target)?;
                }
                Stmt::Macro(_) | Stmt::Item(_) => {}
                Stmt::Local(local) => return Err(at(local, "a binding in a dictionary")),
            }
        }
        Ok(())
    }

    fn dictionary_statement(&mut self, expr: &Expr, node: &mut Node, target: &str) -> R<()> {
        if let Expr::Call(call) = expr {
            // `crate::rust_theme::controls::window::merge(&resources)?`: a document of the
            // ambient unit.
            let function = path_of_call(call);
            if let Some(module) = function.strip_prefix("crate::rust_theme::").and_then(|m| m.strip_suffix("::merge")) {
                self.merged(node).child({
                    let mut include = Node::new("MergeResourceInclude");
                    include.attribute("Source", format!("ferres://FerroUI.Themes.WinUI/{}", self.document_of_module(module)));
                    include
                });
                return Ok(());
            }
            return Err(at(call, format!("`{function}` in a dictionary")));
        }
        let Expr::MethodCall(call) = expr else { return Err(at(expr, "a call of `Resources` was expected")) };
        let _ = target;
        let method = call.method.to_string();
        let args: Vec<&Expr> = call.args.iter().collect();
        let key = || -> R<String> { string_literal(args[0]).ok_or_else(|| at(args[0], "a key was expected")) };
        match method.as_str() {
            "add" => {
                let key = key()?;
                let resource = self.value_resource(args[1], &key)?;
                node.child(resource);
            }
            "text" => {
                let mut string = Node::new("x:String");
                string.attribute("x:Key", key()?);
                string.text = Some(string_literal(args[1]).unwrap_or_default());
                node.child(string);
            }
            "alias" => {
                let mut alias = Node::new("StaticResource");
                alias.attribute("x:Key", key()?);
                alias.attribute("ResourceKey", string_literal(args[1]).unwrap_or_default());
                node.child(alias);
            }
            "color_brush" => {
                let mut brush = Node::new("SolidColorBrush");
                brush.attribute("x:Key", key()?);
                let argb = hex_of(args[1]).ok_or_else(|| at(args[1], "a colour was expected"))?;
                brush.attribute("Color", color_text(argb));
                node.child(brush);
            }
            "static_color_brush" | "dynamic_color_brush" => {
                let mut brush = Node::new("SolidColorBrush");
                brush.attribute("x:Key", key()?);
                let extension = if method == "static_color_brush" { "StaticResource" } else { "DynamicResource" };
                brush.attribute("Color", format!("{{{extension} {}}}", string_literal(args[1]).unwrap_or_default()));
                node.child(brush);
            }
            "deferred" => {
                let key = key()?;
                let body = closure_body(args[1]).ok_or_else(|| at(args[1], "a closure was expected"))?;
                let value = unwrap_call(single(body), &["Ok", "value"]);
                let mut resource = self.deferred_value(value, &key)?;
                resource.attributes.insert(0, ("x:Key".to_string(), key));
                node.child(resource);
            }
            "control_theme" => {
                let theme = self.control_theme(args[0], args[1])?;
                node.child(theme);
            }
            "theme" => {
                let variant = self.variant_key(args[0])?;
                let Expr::Path(function) = args[1] else { return Err(at(args[1], "a function was expected")) };
                let function = self.function(&path_text(&function.path))?;
                let mut theme = Node::new("ResourceDictionary");
                theme.attribute("x:Key", variant);
                self.dictionary_statements(&function.block.stmts, &mut theme, "resources")?;
                self.theme_dictionaries(node).child(theme);
            }
            "include" => {
                let index: u32 = number_text(args[0]).and_then(|n| n.parse().ok()).ok_or_else(|| at(args[0], "a unit index"))?;
                let document = self.units.get(&index).ok_or_else(|| at(args[0], format!("no unit {index}")))?;
                let mut include = Node::new("MergeResourceInclude");
                let source = if self.document.starts_with("Controls/") {
                    format!("ferres://FerroUI.Themes.WinUI/{document}")
                } else {
                    format!("/{document}")
                };
                include.attribute("Source", source);
                self.merged(node).child(include);
            }
            "merge" => {
                let Expr::Call(constructor) = strip_try(args[0]) else { return Err(at(args[0], "a constructor was expected")) };
                let Expr::Path(path) = &*constructor.func else { return Err(at(args[0], "a constructor was expected")) };
                let type_path = Owner(&path.path);
                let (xaml, _) = self.type_of_owner(&type_path)?;
                self.merged(node).child(Node::new(xaml.name));
            }
            other => return Err(at(call, format!("`{other}` of a dictionary"))),
        }
        Ok(())
    }

    fn merged<'n>(&self, node: &'n mut Node) -> &'n mut Node {
        self.property_element(node, "ResourceDictionary.MergedDictionaries", true)
    }

    fn theme_dictionaries<'n>(&self, node: &'n mut Node) -> &'n mut Node {
        self.property_element(node, "ResourceDictionary.ThemeDictionaries", true)
    }

    fn property_element<'n>(&self, node: &'n mut Node, tag: &str, first: bool) -> &'n mut Node {
        let position = node.children.iter().position(|child| matches!(child, Child::Node(n) if n.tag == tag));
        let position = match position {
            Some(position) => position,
            None => {
                if first {
                    let at = node.children.iter().take_while(|c| matches!(c, Child::Node(n) if n.tag.contains('.'))).count();
                    node.children.insert(at, Child::Node(Node::new(tag)));
                    at
                } else {
                    node.children.push(Child::Node(Node::new(tag)));
                    node.children.len() - 1
                }
            }
        };
        match &mut node.children[position] {
            Child::Node(found) => found,
            Child::Comment(_) => unreachable!(),
        }
    }

    fn variant_key(&mut self, expr: &Expr) -> R<String> {
        let text = expr_text(expr);
        Ok(match text.as_str() {
            "ThemeVariant::default()" => "Default".to_string(),
            "ThemeVariant::light()" => "Light".to_string(),
            "ThemeVariant::dark()" => "Dark".to_string(),
            _ => {
                // A field of a class (`WinUITheme::high_contrast()`): `{x:Static Class.Field}`.
                let Expr::Call(call) = strip_try(expr) else { return Err(at(expr, format!("the theme variant `{text}`"))) };
                match self.static_member_text(call)? {
                    Some(text) => text,
                    None => return Err(at(expr, format!("the theme variant `{text}`"))),
                }
            }
        })
    }

    /// `resources.add(key, value)`: a value resource.
    fn value_resource(&mut self, value: &Expr, key: &str) -> R<Node> {
        let (tag, text) = match value {
            Expr::Lit(lit) => match &lit.lit {
                Lit::Float(f) => ("x:Double".to_string(), trim_number(f.base10_digits())),
                Lit::Bool(b) => ("x:Boolean".to_string(), if b.value { "True".to_string() } else { "False".to_string() }),
                Lit::Int(i) if i.suffix() == "i32" => ("x:Int32".to_string(), i.base10_digits().to_string()),
                _ => return Err(at(value, "a literal of a resource")),
            },
            Expr::Unary(_) => ("x:Double".to_string(), number_text(value).unwrap_or_default()),
            _ => {
                let (type_, text) = self.typed_value_text(value)?;
                (type_, text)
            }
        };
        let mut node = Node::new(tag);
        node.attribute("x:Key", key);
        node.text = Some(text);
        Ok(node)
    }

    /// A value that names its type (`Thickness::new(..)`, `Color::from_uint32(..)`): the type in
    /// the document and the text of the value.
    fn typed_value_text(&mut self, value: &Expr) -> R<(String, String)> {
        let value = strip_try(value);
        if let Expr::Call(call) = value {
            let function = path_of_call(call);
            let (type_path, _) = function.rsplit_once("::").unwrap_or(("", ""));
            if let Expr::Path(path) = &*call.func {
                let owner = Owner(&path.path);
                if !owner.is_empty() && type_path != "literal" {
                    let (xaml, _) = self.type_of_owner(&owner)?;
                    let text = self.value_text(value, None)?;
                    return Ok((xaml.name, text));
                }
            }
            if function == "literal" {
                if let Expr::Call(parse) = strip_try(&call.args[0]) {
                    if let Expr::Path(path) = &*parse.func {
                        let owner = Owner(&path.path);
                        let (xaml, _) = self.type_of_owner(&owner)?;
                        return Ok((xaml.name, string_literal(&parse.args[0]).unwrap_or_default()));
                    }
                }
            }
        }
        if let Expr::Path(path) = value {
            let owner = Owner(&path.path);
            let (xaml, _) = self.type_of_owner(&owner)?;
            let text = self.value_text(value, None)?;
            // The named colours are members of `Colors`, values of `Color`.
            let tag = if xaml.name == "Colors" { "Color".to_string() } else { xaml.name };
            return Ok((tag, text));
        }
        Err(at(value, "a typed value was expected"))
    }

    /// `resources.deferred(key, |scope| Ok(value(..)))`: an object resource.
    fn deferred_value(&mut self, value: &Expr, key: &str) -> R<Node> {
        let value = strip_try(value);
        // `crate::rust_theme::density_styles::compact::build(scope)?`: a resource include.
        if let Expr::Call(call) = value {
            let function = path_of_call(call);
            if let Some(module) = function.strip_prefix("crate::rust_theme::").and_then(|m| m.strip_suffix("::build")) {
                let mut include = Node::new("ResourceInclude");
                include.attribute("Source", format!("/{}", self.document_of_module(module)));
                return Ok(include);
            }
            if function.ends_with("control_template") {
                return self.template_node(call, "ControlTemplate", None);
            }
            if function == "literal" || function.ends_with("::with_color") {
                let (tag, text) = self.typed_value_text(value)?;
                if function.ends_with("::with_color") {
                    let mut node = Node::new("SolidColorBrush");
                    node.attribute("Color", self.value_text(&call.args[0], None)?);
                    return Ok(node);
                }
                let mut node = Node::new(tag);
                node.text = Some(text);
                return Ok(node);
            }
        }
        let _ = key;
        self.object_node(value, None)
    }

    // --- control themes and styles ---

    fn control_theme(&mut self, key: &Expr, function: &Expr) -> R<Node> {
        let Expr::Path(function) = function else { return Err(at(function, "a function was expected")) };
        let function = self.function(&path_text(&function.path))?;
        let mut node = Node::new("ControlTheme");
        let key_text = match string_literal(key) {
            Some(key) => key,
            None => {
                let Expr::Call(call) = key else { return Err(at(key, "a key was expected")) };
                let Expr::Path(path) = &*call.func else { return Err(at(key, "a key was expected")) };
                let segment = path.path.segments.last().ok_or_else(|| at(key, "a key"))?;
                let (xaml, _) = self.type_of_generic(segment, key)?;
                format!("{{x:Type {}}}", xaml.name)
            }
        };
        node.attribute("x:Key", key_text);
        let mut target: Option<&'a TypeModel> = None;
        for statement in &function.block.stmts {
            let Stmt::Expr(expr, _) = statement else { continue };
            let expr = strip_try(expr);
            if let Expr::MethodCall(call) = expr {
                if call.method == "target" {
                    let (xaml, model) = self.type_of_arguments(call.turbofish.as_ref(), call)?;
                    node.attribute("TargetType", xaml.name);
                    target = Some(model);
                    continue;
                }
                if call.method == "based_on" {
                    let based = match string_literal(&call.args[0]) {
                        Some(key) => key,
                        None => {
                            let Expr::Call(inner) = &call.args[0] else { return Err(at(call, "a key")) };
                            let Expr::Path(path) = &*inner.func else { return Err(at(call, "a key")) };
                            let segment = path.path.segments.last().ok_or_else(|| at(call, "a key"))?;
                            let (xaml, _) = self.type_of_generic(segment, call)?;
                            format!("{{x:Type {}}}", xaml.name)
                        }
                    };
                    node.attribute("BasedOn", format!("{{StaticResource {based}}}"));
                    continue;
                }
            }
        }
        let statements: Vec<&Stmt> = function.block.stmts.iter().collect();
        self.style_statements(&statements, &mut node, target, target)?;
        Ok(node)
    }

    fn style_statements(&mut self, statements: &[&Stmt], node: &mut Node, target: Option<&'a TypeModel>, parent: Option<&'a TypeModel>) -> R<()> {
        for statement in statements.iter().copied() {
            let Stmt::Expr(expr, _) = statement else { continue };
            let expr = strip_try(expr);
            if matches!(expr, Expr::Call(call) if path_of_call(call) == "Ok") {
                continue;
            }
            let Expr::MethodCall(call) = expr else { return Err(at(expr, "a call of a style was expected")) };
            let method = call.method.to_string();
            let args: Vec<&Expr> = call.args.iter().collect();
            match method.as_str() {
                "target" | "based_on" => {}
                "set" | "set_value" | "set_null" | "dynamic" | "static_resource" | "bind" | "template_bind" => {
                    let property = self.property_name_on(target, args[0])?;
                    let mut setter = Node::new("Setter");
                    setter.attribute("Property", property);
                    match method.as_str() {
                        "set" => {
                            let value = args[1];
                            if let Some(child) = self.template_value(value)? {
                                let mut value_element = Node::new("Setter.Value");
                                value_element.child(child);
                                setter.child(value_element);
                            } else {
                                setter.attribute("Value", self.value_text(value, Some(args[0]))?);
                            }
                        }
                        "set_null" => setter.attribute("Value", "{x:Null}"),
                        "dynamic" => setter.attribute("Value", format!("{{DynamicResource {}}}", string_literal(args[1]).unwrap_or_default())),
                        "static_resource" => setter.attribute("Value", format!("{{StaticResource {}}}", string_literal(args[1]).unwrap_or_default())),
                        "template_bind" => {
                            let source = self.source_name(args[1])?;
                            setter.attribute("Value", format!("{{TemplateBinding {source}}}"));
                        }
                        "bind" => {
                            let value = self.binding_value(args[1], target)?;
                            match value {
                                BindingValue::Text(text) => setter.attribute("Value", text),
                                BindingValue::Node(child) => {
                                    let mut value_element = Node::new("Setter.Value");
                                    value_element.child(child);
                                    setter.child(value_element);
                                }
                            }
                        }
                        _ => {
                            let inner = unwrap_call(args[1], &["value"]);
                            let child = self.object_node(inner, target)?;
                            setter.child(child);
                        }
                    }
                    node.child(setter);
                }
                "template" => {
                    let Expr::Path(function) = args[0] else { return Err(at(args[0], "a template function")) };
                    let mut setter = Node::new("Setter");
                    setter.attribute("Property", "Template");
                    setter.child(self.template_function(&path_text(&function.path), "ControlTemplate", target)?);
                    node.child(setter);
                }
                "set_template" => {
                    let property = self.property_name_on(target, args[0])?;
                    let Expr::Path(function) = args[1] else { return Err(at(args[1], "a template function")) };
                    let mut setter = Node::new("Setter");
                    setter.attribute("Property", property);
                    setter.child(self.template_function(&path_text(&function.path), "Template", target)?);
                    node.child(setter);
                }
                "style" => {
                    let style = self.nested_style(args[0], args[1], target.or(parent))?;
                    node.child(style);
                }
                "animation" => {
                    let animation = self.object_node(args[0], target)?;
                    self.property_element(node, &format!("{}.Animations", node.tag.clone()), false).child(animation);
                }
                other => return Err(at(call, format!("`{other}` of a style"))),
            }
        }
        Ok(())
    }

    fn nested_style(&mut self, selector: &Expr, populate: &Expr, parent: Option<&'a TypeModel>) -> R<Node> {
        let (selector_text, selected) = self.selector(selector, parent)?;
        let mut node = Node::new("Style");
        node.attribute("Selector", selector_text);
        let body = closure_body(populate).ok_or_else(|| at(populate, "a closure was expected"))?;
        let statements = block_statements(body);
        self.style_statements(&statements, &mut node, selected.or(parent), parent)?;
        Ok(node)
    }

    fn style_call(&mut self, call: &ExprCall, parent: Option<&'a TypeModel>) -> R<Node> {
        self.nested_style(&call.args[1], &call.args[2], parent)
    }

    // --- selectors ---

    fn selector(&mut self, expr: &Expr, parent: Option<&'a TypeModel>) -> R<(String, Option<&'a TypeModel>)> {
        let expr = strip_try(expr);
        if let Expr::Call(call) = expr {
            let function = path_of_call(call);
            if function.ends_with("Selectors::or") {
                let Expr::Array(array) = &call.args[0] else { return Err(at(call, "an array of selectors")) };
                let mut parts = Vec::new();
                let mut target = None;
                for alternative in &array.elems {
                    let (text, selected) = self.selector(alternative, parent)?;
                    parts.push(text);
                    target = target.or(selected);
                }
                return Ok((parts.join(", "), target));
            }
        }
        let (root, calls) = chain(expr);
        let mut text = String::new();
        let mut target: Option<&'a TypeModel> = None;
        let mut steps: Vec<(String, Vec<&Expr>, Option<&syn::AngleBracketedGenericArguments>)> = Vec::new();
        if let Expr::Call(call) = root {
            let Expr::Path(path) = &*call.func else { return Err(at(root, "a selector")) };
            let segment = path.path.segments.last().ok_or_else(|| at(root, "a selector"))?;
            let arguments = match &segment.arguments {
                syn::PathArguments::AngleBracketed(arguments) => Some(arguments),
                _ => None,
            };
            steps.push((segment.ident.to_string(), call.args.iter().collect(), arguments));
        }
        for call in calls {
            steps.push((call.method.to_string(), call.args.iter().collect(), call.turbofish.as_ref()));
        }
        for (name, args, generics) in steps {
            match name.as_str() {
                "nesting" => {
                    text.push('^');
                    target = parent;
                }
                "of" | "is" => {
                    let (xaml, model) = self.type_of_arguments(generics, expr)?;
                    let written = xaml.name.replace(':', "|");
                    if name == "is" {
                        let _ = write!(text, ":is({written})");
                    } else {
                        text.push_str(&written);
                    }
                    target = Some(model);
                }
                "class" => {
                    let class = args.first().and_then(|a| string_literal(a)).unwrap_or_default();
                    if class.starts_with(':') {
                        text.push_str(&class);
                    } else {
                        text.push('.');
                        text.push_str(&class);
                    }
                }
                "name" | "named" => {
                    text.push('#');
                    text.push_str(&args.first().and_then(|a| string_literal(a)).unwrap_or_default());
                }
                "template" => {
                    text.push_str(" /template/ ");
                    target = None;
                }
                "child" => {
                    text.push_str(" > ");
                    target = None;
                }
                "descendant" => {
                    text.push(' ');
                    target = None;
                }
                "not" => {
                    let (inner, _) = self.selector(args[0], target.or(parent))?;
                    let _ = write!(text, ":not({inner})");
                }
                "nth_child" | "nth_last_child" => {
                    let step = number_text(args[0]).unwrap_or_default();
                    let offset = number_text(args[1]).unwrap_or_default();
                    let function = if name == "nth_child" { "nth-child" } else { "nth-last-child" };
                    let _ = write!(text, ":{function}({})", nth_text(&step, &offset));
                }
                "property_is" => {
                    let property = self.property_name_on(target.or(parent), args[0])?;
                    let value = self.value_text(args[1], Some(args[0]))?;
                    let _ = write!(text, "[{property}={value}]");
                }
                other => return Err(at(expr, format!("the selector step `{other}`"))),
            }
        }
        Ok((text.replace("  ", " "), target))
    }
}

fn nth_text(step: &str, offset: &str) -> String {
    match (step, offset) {
        ("0", offset) => offset.to_string(),
        ("2", "1") => "odd".to_string(),
        ("2", "0") => "even".to_string(),
        (step, "0") => format!("{step}n"),
        (step, offset) if offset.starts_with('-') => format!("{step}n{offset}"),
        (step, offset) => format!("{step}n+{offset}"),
    }
}

fn collect_uses(tree: &syn::UseTree, prefix: String, imports: &mut HashMap<String, String>) {
    match tree {
        syn::UseTree::Path(path) => {
            let prefix = if prefix.is_empty() {
                if path.ident == "crate" { "crate".to_string() } else { format!("::{}", path.ident) }
            } else {
                format!("{prefix}::{}", path.ident)
            };
            collect_uses(&path.tree, prefix, imports);
        }
        syn::UseTree::Name(name) => {
            imports.insert(name.ident.to_string(), format!("{prefix}::{}", name.ident));
        }
        syn::UseTree::Rename(rename) => {
            imports.insert(rename.rename.to_string(), format!("{prefix}::{}", rename.ident));
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                collect_uses(item, prefix.clone(), imports);
            }
        }
        syn::UseTree::Glob(_) => {}
    }
}

fn path_of_call(call: &ExprCall) -> String {
    match &*call.func {
        Expr::Path(path) => path_text(&path.path),
        _ => String::new(),
    }
}

fn expr_text(expr: &Expr) -> String {
    quote::ToTokens::to_token_stream(expr).to_string().replace(' ', "")
}

fn hex_of(expr: &Expr) -> Option<u32> {
    match strip_try(expr) {
        Expr::Lit(lit) => match &lit.lit {
            Lit::Int(i) => {
                let text = i.to_string();
                let digits = text.trim_start_matches("0x").trim_start_matches("0X");
                u32::from_str_radix(&digits.replace('_', ""), 16).ok()
            }
            _ => None,
        },
        _ => None,
    }
}

fn color_text(argb: u32) -> String {
    format!("#{argb:08X}")
}

enum BindingValue {
    Text(String),
    Node(Node),
}

// --- templates, elements and values -------------------------------------------------------------

impl<'a> Reader<'a> {
    /// A template made by the function `name` (`fn ..(t: &Template<T>) -> Result<..>`): the
    /// template element `tag` with the element the function builds.
    fn template_function(&mut self, name: &str, tag: &str, theme_target: Option<&'a TypeModel>) -> R<Node> {
        let function = self.function(name)?;
        let mut node = Node::new(tag);
        if tag == "ControlTemplate" {
            if let Some(target) = self.template_target(function) {
                let same = theme_target.is_some_and(|theme| std::ptr::eq(theme, target));
                if !same && target.name != "TemplatedControl" {
                    let xaml = self.xaml_type(target);
                    node.attribute("TargetType", xaml.name);
                }
            }
        }
        let parent = self.template_target(function);
        for statement in &function.block.stmts {
            let Stmt::Expr(expr, _) = statement else { continue };
            let content = unwrap_call(expr, &["Ok", "value", "Some"]);
            let mut element = self.element(content, parent)?;
            // Compiled bindings to the data context need its type: the class the paths name
            // (`.member::<T>(..)`), else any object.
            let text = quote::ToTokens::to_token_stream(&function.block).to_string();
            let data_paths = text.contains(". member ::") || text.contains(".member::");
            if (tag != "ControlTemplate" && tag != "WindowDrawnDecorationsTemplate") || data_paths {
                if text.contains("path ()") || text.contains("path()") {
                    let data_type = self.data_type_of(&function.block)?;
                    if !element.attributes.iter().any(|(name, _)| name == "x:DataType") {
                        element.attributes.insert(0, ("x:DataType".to_string(), data_type));
                    }
                }
            }
            node.child(element);
        }
        Ok(node)
    }

    /// The type of the data context of the bindings of a template: the type of the first
    /// `.member::<T>(..)` of its paths, else `x:Object`.
    fn data_type_of(&mut self, block: &syn::Block) -> R<String> {
        struct Finder<'ast>(Option<&'ast syn::AngleBracketedGenericArguments>);
        impl<'ast> syn::visit::Visit<'ast> for Finder<'ast> {
            fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
                if self.0.is_none() && call.method == "member" {
                    self.0 = call.turbofish.as_ref();
                }
                syn::visit::visit_expr_method_call(self, call);
            }
        }
        let mut finder = Finder(None);
        syn::visit::Visit::visit_block(&mut finder, block);
        match finder.0 {
            Some(arguments) => Ok(self.type_of_arguments(Some(arguments), block)?.0.name),
            None => Ok("x:Object".to_string()),
        }
    }

    /// The class of the templated parent of a template function: `T` of `Template<T>`.
    fn template_target(&mut self, function: &ItemFn) -> Option<&'a TypeModel> {
        let syn::FnArg::Typed(argument) = function.sig.inputs.first()? else { return None };
        let syn::Type::Reference(reference) = &*argument.ty else { return None };
        let syn::Type::Path(path) = &*reference.elem else { return None };
        let segment = path.path.segments.last()?;
        let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else { return None };
        let Some(syn::GenericArgument::Type(syn::Type::Path(target))) = arguments.args.first() else { return None };
        let absolute = self.absolute(&target.path);
        self.type_model(&absolute).ok()
    }

    /// The template value `expr` (`control_template(scope, f)`, `focus_adorner(..)`, ..), if it
    /// is one.
    fn template_value(&mut self, expr: &Expr) -> R<Option<Node>> {
        let expr = unwrap_call(expr, &["Some"]);
        let Expr::Call(call) = strip_try(expr) else { return Ok(None) };
        let function = path_of_call(call);
        let name = function.rsplit("::").next().unwrap_or_default().to_string();
        let tag = match name.as_str() {
            "control_template" => "ControlTemplate",
            "focus_adorner" => "FocusAdornerTemplate",
            "items_panel" => "ItemsPanelTemplate",
            "template_of" => "Template",
            "data_template" | "data_template_of" | "data_template_for" => "DataTemplate",
            "decorations_template" => "WindowDrawnDecorationsTemplate",
            _ => return Ok(None),
        };
        Ok(Some(self.template_node(call, tag, None)?))
    }

    fn template_node(&mut self, call: &ExprCall, tag: &str, theme_target: Option<&'a TypeModel>) -> R<Node> {
        let Expr::Path(function) = &call.args[1] else { return Err(at(call, "a template function")) };
        let mut node = self.template_function(&path_text(&function.path), tag, theme_target)?;
        // `data_template_of::<T>`: the type of the data.
        if let Expr::Path(path) = &*call.func {
            if let Some(segment) = path.path.segments.last() {
                if segment.ident == "data_template_of" {
                    let (xaml, _) = self.type_of_generic(segment, call)?;
                    node.attributes.insert(0, ("DataType".to_string(), xaml.name));
                } else if segment.ident == "data_template_for" {
                    node.attributes.insert(0, ("DataType".to_string(), "x:String".to_string()));
                }
            }
        }
        if tag == "WindowDrawnDecorationsTemplate" {
            // The content of the template is the decorations content itself.
        }
        Ok(node)
    }

    /// An element: `t.make(Border::new()).set(..)..control()`.
    fn element(&mut self, expr: &Expr, template_parent: Option<&'a TypeModel>) -> R<Node> {
        let (_, calls) = chain(expr);
        let mut calls = calls.into_iter().skip_while(|call| call.method == "scope").peekable();
        let make = calls.next().ok_or_else(|| at(expr, "an element was expected"))?;
        if make.method != "make" {
            return Err(at(make, format!("`make` was expected, not `{}`", make.method)));
        }
        let Expr::Call(constructor) = strip_try(&make.args[0]) else { return Err(at(make, "a constructor was expected")) };
        let Expr::Path(path) = &*constructor.func else { return Err(at(make, "a constructor was expected")) };
        let type_path = Owner(&path.path);
        let (xaml, model) = self.type_of_owner(&type_path)?;
        let mut node = Node::new(xaml.name.clone());
        let content = self.content_property(model);
        let mut classes: Vec<String> = Vec::new();
        for call in calls {
            let method = call.method.to_string();
            let args: Vec<&Expr> = call.args.iter().collect();
            match method.as_str() {
                "end" | "control" | "upcast" | "into" => {}
                "name" => node.attribute("x:Name", string_literal(args[0]).unwrap_or_default()),
                "class" => classes.push(string_literal(args[0]).unwrap_or_default()),
                "set" => {
                    let property = self.attribute_name(model, args[0])?;
                    let value = args[1];
                    if let Some(child) = self.object_value(value, template_parent)? {
                        self.add_property_child(&mut node, &xaml, model, &content, &property, child);
                    } else {
                        node.attribute(property, self.value_text(value, Some(args[0]))?);
                    }
                }
                "set_value" => {
                    let property = self.attribute_name(model, args[0])?;
                    let inner = unwrap_call(args[1], &["value"]);
                    if is_none(args[1]) {
                        node.attribute(property, "{x:Null}");
                    } else if let Some(child) = self.object_value(inner, template_parent)? {
                        self.add_property_child(&mut node, &xaml, model, &content, &property, child);
                    } else {
                        node.attribute(property, self.value_text(inner, Some(args[0]))?);
                    }
                }
                "dynamic" => {
                    let property = self.attribute_name(model, args[0])?;
                    node.attribute(property, format!("{{DynamicResource {}}}", string_literal(args[1]).unwrap_or_default()));
                }
                "static_resource" => {
                    let property = self.attribute_name(model, args[0])?;
                    node.attribute(property, format!("{{StaticResource {}}}", string_literal(args[1]).unwrap_or_default()));
                }
                "template_bind" => {
                    let property = self.attribute_name(model, args[0])?;
                    let source = self.source_name(args[1])?;
                    node.attribute(property, format!("{{TemplateBinding {source}}}"));
                }
                "template_bind_with" => {
                    let property = self.attribute_name(model, args[0])?;
                    let mut text = String::from("{TemplateBinding");
                    let mut parts = Vec::new();
                    if !is_none(args[1]) {
                        let source = unwrap_call(args[1], &["Some"]);
                        let source = self.source_name(source)?;
                        parts.push(source);
                    }
                    if !is_none(args[2]) {
                        parts.push(format!("Converter={}", self.converter_text(args[2])?));
                    }
                    if !is_none(args[3]) {
                        let mode = expr_text(unwrap_call(args[3], &["Some"]));
                        parts.push(format!("Mode={}", mode.rsplit("::").next().unwrap_or_default()));
                    }
                    if !is_none(args[4]) {
                        parts.push(format!("ConverterParameter={}", self.option_text(args[4])?));
                    }
                    if !parts.is_empty() {
                        text.push(' ');
                        text.push_str(&parts.join(", "));
                    }
                    text.push('}');
                    node.attribute(property, text);
                }
                "bind" => {
                    let property = self.attribute_name(model, args[0])?;
                    match self.binding_value(args[1], Some(model))? {
                        BindingValue::Text(text) => node.attribute(property, text),
                        BindingValue::Node(child) => self.add_property_child(&mut node, &xaml, model, &content, &property, child),
                    }
                }
                "child" | "content" => {
                    let property = self.attribute_name(model, args[0])?;
                    let child = self.element(args[1], template_parent)?;
                    self.add_property_child(&mut node, &xaml, model, &content, &property, child);
                }
                "children" => {
                    let Expr::Array(array) = args[0] else { return Err(at(call, "an array of children")) };
                    for item in &array.elems {
                        let child = self.element(item, template_parent)?;
                        node.child(child);
                    }
                }
                "style" => {
                    let style = self.nested_style(args[0], args[1], None)?;
                    let tag = format!("{}.Styles", xaml.name);
                    self.property_element(&mut node, &tag, false).child(style);
                }
                "with" => self.with_closure(args[0], &mut node, &xaml, model, template_parent)?,
                other => return Err(at(call, format!("`{other}` of an element"))),
            }
        }
        if !classes.is_empty() {
            node.attribute("Classes", classes.join(" "));
        }
        Ok(node)
    }

    fn content_property(&self, model: &'a TypeModel) -> Option<String> {
        self.chain_of_types(model).iter().find_map(|type_| type_.content_property.clone())
    }

    fn add_property_child(&mut self, node: &mut Node, xaml: &XamlType, _model: &'a TypeModel, content: &Option<String>, property: &str, child: Node) {
        if content.as_deref() == Some(property) {
            node.child(child);
        } else {
            let tag = if property.contains('.') { property.to_string() } else { format!("{}.{property}", xaml.name) };
            self.property_element(node, &tag, false).child(child);
        }
    }

    /// `.with(|e| { let items = Type::getter(e); items.add(..); .. Ok(()) })?`, or
    /// `.with(|e| { e.set_x(..); Ok(()) })?`: the items or the value of a property that is not
    /// registered.
    fn with_closure(&mut self, closure: &Expr, node: &mut Node, xaml: &XamlType, model: &'a TypeModel, template_parent: Option<&'a TypeModel>) -> R<()> {
        let body = closure_body(closure).ok_or_else(|| at(closure, "a closure"))?;
        let statements = block_statements(body);
        let mut property: Option<String> = None;
        let mut items: Vec<Node> = Vec::new();
        for statement in statements {
            match statement {
                Stmt::Local(local) => {
                    let Some(init) = &local.init else { continue };
                    let Expr::Call(call) = &*init.expr else { continue };
                    let getter = path_of_call(call);
                    let getter = getter.rsplit("::").next().unwrap_or_default().to_string();
                    property = Some(self.plain_property_name(model, &getter, false).unwrap_or_else(|| pascal(&getter)));
                }
                Stmt::Expr(expr, _) => {
                    let expr = strip_try(expr);
                    if let Expr::MethodCall(call) = expr {
                        if call.method == "add" {
                            items.push(self.item(&call.args[0], template_parent)?);
                        } else if call.method.to_string().starts_with("set_") {
                            let setter = call.method.to_string();
                            property = Some(self.plain_property_name(model, &setter, true).unwrap_or_else(|| pascal(&setter["set_".len()..])));
                            if let Some(value) = self.object_value(&call.args[0], template_parent)? {
                                items.push(value);
                            } else {
                                let name = property.clone().unwrap_or_default();
                                node.attribute(name, self.value_text(&call.args[0], None)?);
                                return Ok(());
                            }
                        } else {
                            // `Type::set_x(e, value)` of a plain property.
                        }
                    } else if let Expr::Call(call) = expr {
                        let function = path_of_call(call);
                        if function == "Ok" {
                            continue;
                        }
                        let setter = function.rsplit("::").next().unwrap_or_default().to_string();
                        let name = self.plain_property_name(model, &setter, true).unwrap_or_else(|| pascal(setter.trim_start_matches("set_")));
                        node.attribute(name, self.value_text(&call.args[1], None)?);
                        return Ok(());
                    }
                }
                _ => {}
            }
        }
        let property = property.ok_or_else(|| at(closure, "the property of the items"))?;
        let tag = format!("{}.{property}", xaml.name);
        let element = self.property_element(node, &tag, false);
        for item in items {
            element.child(item);
        }
        Ok(())
    }

    /// The name of the plain property whose getter (or setter) is the method `method`.
    fn plain_property_name(&self, model: &'a TypeModel, method: &str, setter: bool) -> Option<String> {
        for type_ in self.chain_of_types(model) {
            for property in &type_.properties {
                let accessor = if setter { property.setter.as_ref() } else { property.getter.as_ref() };
                let Some(accessor) = accessor else { continue };
                let path = accessor.callable.path.clone().unwrap_or_default();
                if path.rsplit("::").next() == Some(method) {
                    return Some(property.name.clone());
                }
            }
        }
        None
    }

    /// An item of a collection: an element, a template or an object.
    fn item(&mut self, expr: &Expr, template_parent: Option<&'a TypeModel>) -> R<Node> {
        if let Some(node) = self.object_value(expr, template_parent)? {
            return Ok(node);
        }
        // A binding of a multi binding: the element form of its markup extension.
        if let Ok(BindingValue::Text(text)) = self.binding_value(expr, template_parent) {
            return Ok(extension_element(&text));
        }
        Err(at(expr, "an item was expected"))
    }

    /// The value `expr` when it is an object that is written as an element (an element, a
    /// template, a collection, an object of a class outside the object model, a binding),
    /// else `None` (a value written as text).
    fn object_value(&mut self, expr: &Expr, template_parent: Option<&'a TypeModel>) -> R<Option<Node>> {
        let expr = unwrap_call(expr, &["Some", "value"]);
        if let Some(template) = self.template_value(expr)? {
            return Ok(Some(template));
        }
        let stripped = strip_try(expr);
        match stripped {
            Expr::Block(_) => return Ok(Some(self.object_node(stripped, template_parent)?)),
            Expr::MethodCall(_) => {
                let (root, calls) = chain(stripped);
                if calls.iter().find(|call| call.method != "scope").is_some_and(|call| call.method == "make") {
                    let _ = root;
                    return Ok(Some(self.element(stripped, template_parent)?));
                }
                if calls.iter().any(|call| call.method == "binding") || expr_text(stripped).starts_with("TemplateBinding::") {
                    return match self.binding_value(stripped, template_parent)? {
                        BindingValue::Node(node) => Ok(Some(node)),
                        BindingValue::Text(_) => Ok(None),
                    };
                }
                Ok(None)
            }
            Expr::Call(call) => {
                let function = path_of_call(call);
                let last = function.rsplit("::").next().unwrap_or_default();
                if matches!(last, "setter" | "bound_setter" | "dynamic_resource_binding" | "template_binding") {
                    return Ok(Some(self.object_node(stripped, template_parent)?));
                }
                if function.ends_with("Rc::new") {
                    return Ok(Some(self.object_node(stripped, template_parent)?));
                }
                if last == "new" && call.args.is_empty() {
                    if let Expr::Path(path) = &*call.func {
                        let type_path = Owner(&path.path);
                        if let Ok((xaml, model)) = self.type_of_owner(&type_path) {
                            if !model.object_model {
                                return Ok(Some(Node::new(xaml.name)));
                            }
                        }
                    }
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    /// An object: `scope.make(X::new())..end()`, `{ let object = X::new(); object.set_y(..); .. object }`,
    /// `Rc::new(X::new())`, `setter(property, value)`, a multi binding.
    fn object_node(&mut self, expr: &Expr, template_parent: Option<&'a TypeModel>) -> R<Node> {
        let expr = strip_try(expr);
        match expr {
            Expr::MethodCall(_) => self.element(expr, template_parent),
            Expr::Call(call) => {
                let function = path_of_call(call);
                let last = function.rsplit("::").next().unwrap_or_default();
                match last {
                    "setter" | "bound_setter" => {
                        let mut node = Node::new("Setter");
                        node.attribute("Property", self.property_name_on(template_parent, &call.args[0])?);
                        if last == "setter" {
                            node.attribute("Value", self.value_text(&call.args[1], Some(&call.args[0]))?);
                        } else {
                            match self.binding_value(&call.args[1], template_parent)? {
                                BindingValue::Text(text) => node.attribute("Value", text),
                                BindingValue::Node(child) => node.child(child),
                            }
                        }
                        Ok(node)
                    }
                    "dynamic_resource_binding" => {
                        let mut node = Node::new("DynamicResource");
                        node.attribute("ResourceKey", string_literal(&call.args[0]).unwrap_or_default());
                        Ok(node)
                    }
                    "template_binding" => {
                        let mut node = Node::new("TemplateBinding");
                        let source = self.source_name(&call.args[0])?;
                        node.attribute("Property", source);
                        Ok(node)
                    }
                    "new" if function.ends_with("Rc::new") => self.object_node(&call.args[0], template_parent),
                    "new" => {
                        let Expr::Path(path) = &*call.func else { return Err(at(call, "a constructor")) };
                        let type_path = Owner(&path.path);
                        let (xaml, _) = self.type_of_owner(&type_path)?;
                        Ok(Node::new(xaml.name))
                    }
                    "literal" => {
                        let (tag, text) = self.typed_value_text(expr)?;
                        let mut node = Node::new(tag);
                        node.text = Some(text);
                        Ok(node)
                    }
                    other => Err(at(call, format!("the object `{other}`"))),
                }
            }
            Expr::Block(block) => {
                // `{ let object = X::new(); object.set_y(..); object.add(..); object }`.
                let mut node: Option<Node> = None;
                let mut model: Option<&'a TypeModel> = None;
                let mut xaml: Option<XamlType> = None;
                for statement in &block.block.stmts {
                    match statement {
                        Stmt::Local(local) => {
                            let Some(init) = &local.init else { continue };
                            let init = strip_try(&init.expr);
                            let constructor = match init {
                                Expr::Call(call) if path_of_call(call).ends_with("Rc::new") => &call.args[0],
                                Expr::MethodCall(call) if call.method == "bindings" => continue,
                                other => other,
                            };
                            let (root, calls) = chain(constructor);
                            let Expr::Call(call) = root else { return Err(at(constructor, "a constructor")) };
                            let Expr::Path(path) = &*call.func else { return Err(at(constructor, "a constructor")) };
                            let type_path = Owner(&path.path);
                            let (found_xaml, found) = self.type_of_owner(&type_path)?;
                            let mut created = Node::new(found_xaml.name.clone());
                            for option in calls {
                                self.binding_option(&mut created, option, template_parent)?;
                            }
                            node = Some(created);
                            model = Some(found);
                            xaml = Some(found_xaml);
                        }
                        Stmt::Expr(Expr::MethodCall(call), _) => {
                            let Some(node) = node.as_mut() else { continue };
                            let method = call.method.to_string();
                            if method == "add" {
                                let item = self.item(&call.args[0], template_parent)?;
                                node.child(item);
                            } else if let Some(name) = method.strip_prefix("set_") {
                                let model = model.expect("a model");
                                let property = self.plain_property_name(model, &method, true).unwrap_or_else(|| pascal(name));
                                if let Some(child) = self.object_value(&call.args[0], template_parent)? {
                                    let tag = format!("{}.{property}", xaml.as_ref().map(|x| x.name.clone()).unwrap_or_default());
                                    self.property_element(node, &tag, false).child(child);
                                } else {
                                    let value = match strip_try(&call.args[0]) {
                                        Expr::MethodCall(m) if m.method == "resource" => {
                                            format!("{{StaticResource {}}}", string_literal(&m.args[0]).unwrap_or_default())
                                        }
                                        other => self.value_text(other, None)?,
                                    };
                                    node.attribute(property, value);
                                }
                            } else {
                                return Err(at(call, format!("`{method}` of an object")));
                            }
                        }
                        _ => {}
                    }
                }
                node.ok_or_else(|| at(expr, "an object was expected"))
            }
            other => Err(at(other, "an object was expected")),
        }
    }

    /// An option of a multi binding given with its constructor (`.with_converter_value(..)`).
    fn binding_option(&mut self, node: &mut Node, option: &ExprMethodCall, _template_parent: Option<&'a TypeModel>) -> R<()> {
        let argument = &option.args[0];
        match option.method.to_string().as_str() {
            "with_converter_value" => {
                let inner = unwrap_call(argument, &["Some"]);
                if let Expr::Call(call) = strip_try(inner) {
                    if path_of_call(call).ends_with("Rc::new") {
                        let converter = self.object_node(&call.args[0], None)?;
                        let tag = format!("{}.Converter", node.tag);
                        self.property_element(node, &tag, false).child(converter);
                        return Ok(());
                    }
                }
                node.attribute("Converter", self.converter_text(argument)?);
            }
            "with_converter_parameter" => node.attribute("ConverterParameter", self.option_text(argument)?),
            "with_mode" => node.attribute("Mode", expr_text(argument).rsplit("::").next().unwrap_or_default().to_string()),
            other => return Err(at(option, format!("the option `{other}`"))),
        }
        Ok(())
    }

    /// A converter: `Some(ObjectConverters::is_not_null())`, `Some(t.scope().converter("K")?)`.
    fn converter_text(&mut self, expr: &Expr) -> R<String> {
        let inner = strip_try(unwrap_call(expr, &["Some"]));
        if let Expr::MethodCall(call) = inner {
            if call.method == "converter" || call.method == "multi_converter" {
                return Ok(format!("{{StaticResource {}}}", string_literal(&call.args[0]).unwrap_or_default()));
            }
        }
        if let Expr::Call(call) = inner {
            return self.static_member_text(call).map(|text| text.unwrap_or_default());
        }
        Err(at(expr, "a converter was expected"))
    }

    /// `value("text".to_string())`, `value(Type::member())`: the text of an option.
    fn option_text(&mut self, expr: &Expr) -> R<String> {
        let inner = strip_try(unwrap_call(expr, &["value", "Some"]));
        if let Expr::MethodCall(call) = inner {
            if call.method == "to_string" {
                return Ok(string_literal(&call.receiver).unwrap_or_default());
            }
        }
        if let Expr::Call(call) = inner {
            if let Some(text) = self.static_member_text(call)? {
                return Ok(text);
            }
        }
        self.value_text(inner, None)
    }

    /// `Type::member()` as `{x:Static Type.Member}`, when the type declares the member.
    fn static_member_text(&mut self, call: &ExprCall) -> R<Option<String>> {
        let (owner, function) = self.static_owner(call)?;
        let Some((xaml, model)) = owner else { return Ok(None) };
        for member in model.fields.iter() {
            if member.callable.path.as_deref().and_then(|p| p.rsplit("::").next()) == Some(function.as_str()) || snake(&member.name) == function {
                return Ok(Some(format!("{{x:Static {}.{}}}", xaml.name, member.name)));
            }
        }
        for property in model.static_properties.iter() {
            let path = property.getter.as_ref().and_then(|g| g.callable.path.clone()).unwrap_or_default();
            if path.rsplit("::").next() == Some(function.as_str()) || snake(&property.name) == function {
                return Ok(Some(format!("{{x:Static {}.{}}}", xaml.name, property.name)));
            }
        }
        Ok(None)
    }

    fn static_owner(&mut self, call: &ExprCall) -> R<(Option<(XamlType, &'a TypeModel)>, String)> {
        let Expr::Path(path) = &*call.func else { return Ok((None, String::new())) };
        let function = path.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
        if let Some(qself) = &path.qself {
            if let syn::Type::TraitObject(object) = &*qself.ty {
                if let Some(syn::TypeParamBound::Trait(bound)) = object.bounds.first() {
                    return Ok((self.type_of_path(&bound.path).ok(), function));
                }
            }
            if let syn::Type::Path(type_path) = &*qself.ty {
                return Ok((self.type_of_path(&type_path.path).ok(), function));
            }
        }
        let owner = Owner(&path.path);
        if owner.is_empty() {
            return Ok((None, function));
        }
        Ok((self.type_of_owner(&owner).ok(), function))
    }

    /// A binding as the value of a property: the text of a markup extension, or an element
    /// (a multi binding).
    fn binding_value(&mut self, expr: &Expr, _target: Option<&'a TypeModel>) -> R<BindingValue> {
        let expr = strip_try(expr);
        if let Expr::Block(_) = expr {
            return Ok(BindingValue::Node(self.object_node(expr, None)?));
        }
        if let Expr::Call(call) = expr {
            let function = path_of_call(call);
            if function.ends_with("template_binding") {
                let source = self.source_name(&call.args[0])?;
                return Ok(BindingValue::Text(format!("{{TemplateBinding {source}}}")));
            }
            if function.ends_with("dynamic_resource_binding") {
                return Ok(BindingValue::Node(self.object_node(expr, None)?));
            }
        }
        let (root, calls) = chain(expr);
        // `TemplateBinding::new(Source.property()).with_converter_value(..)`.
        if let Expr::Call(call) = root {
            let function = path_of_call(call);
            if function.ends_with("TemplateBinding::new") {
                let source = self.source_name(&call.args[0])?;
                let mut parts = vec![source];
                for option in calls {
                    match option.method.to_string().as_str() {
                        "with_converter_value" => parts.push(format!("Converter={}", self.converter_text(&option.args[0])?)),
                        "with_mode" => parts.push(format!("Mode={}", expr_text(&option.args[0]).rsplit("::").next().unwrap_or_default())),
                        "with_converter_parameter" => parts.push(format!("ConverterParameter={}", self.option_text(&option.args[0])?)),
                        other => return Err(at(option, format!("the option `{other}`"))),
                    }
                }
                return Ok(BindingValue::Text(format!("{{TemplateBinding {}}}", parts.join(", "))));
            }
            if function.ends_with("ReflectionBinding::new") {
                let path = string_literal(&call.args[0]).unwrap_or_default();
                let mut parts = Vec::new();
                for option in calls {
                    if option.method == "with_relative_source" {
                        let mode = expr_text(&option.args[0]);
                        let mode = mode.rsplit("::").next().unwrap_or_default().trim_end_matches(')').to_string();
                        parts.push(format!("RelativeSource={{RelativeSource {mode}}}"));
                    }
                }
                parts.insert(0, path);
                return Ok(BindingValue::Text(format!("{{ReflectionBinding {}}}", parts.join(", "))));
            }
        }
        // `path()..binding()..`.
        let mut text = String::new();
        let mut options = Vec::new();
        let mut relative = None;
        let mut negations = 0;
        let mut after_binding = false;
        for call in calls {
            let method = call.method.to_string();
            let args: Vec<&Expr> = call.args.iter().collect();
            if after_binding {
                match method.as_str() {
                    "with_mode" => options.push(format!("Mode={}", expr_text(args[0]).rsplit("::").next().unwrap_or_default())),
                    "with_converter_value" => options.push(format!("Converter={}", self.converter_text(args[0])?)),
                    "with_converter_parameter" => options.push(format!("ConverterParameter={}", self.option_text(args[0])?)),
                    "with_string_format" => {
                        let format = unwrap_call(args[0], &["Some"]);
                        let format = match format {
                            Expr::Call(call) => string_literal(&call.args[0]).unwrap_or_default(),
                            _ => String::new(),
                        };
                        options.push(format!("StringFormat='{format}'"));
                    }
                    "with_fallback_value" => options.push(format!("FallbackValue={}", self.option_text(args[0])?)),
                    "with_target_null_value" => options.push(format!("TargetNullValue={}", self.option_text(args[0])?)),
                    other => return Err(at(call, format!("the binding option `{other}`"))),
                }
                continue;
            }
            match method.as_str() {
                "binding" => after_binding = true,
                "ancestor_of" => {
                    let (xaml, _) = self.type_of_arguments(call.turbofish.as_ref(), call)?;
                    let level = number_text(args[0]).unwrap_or_default();
                    let name = xaml.name.replace(':', "|");
                    if level == "0" {
                        let _ = write!(text, "$parent[{name}]");
                    } else {
                        let _ = write!(text, "$parent[{name}, {level}]");
                    }
                }
                "templated_parent" => relative = Some("TemplatedParent"),
                "self_" => text.push_str("$self"),
                "named" => {
                    let _ = write!(text, "#{}", string_literal(args[1]).unwrap_or_default());
                }
                "prop" | "prop_or_null" => {
                    let (owner, name, kind) = self.property(args[0])?;
                    let segment = if kind == RegisteredKind::Attached {
                        let owner = self.xaml_type(owner);
                        format!("({}.{name})", owner.name)
                    } else {
                        name
                    };
                    if !text.is_empty() {
                        text.push('.');
                    }
                    text.push_str(&segment);
                    if method == "prop_or_null" {
                        text.push('?');
                    }
                }
                "member" | "class_member" | "method" => {
                    if !text.is_empty() {
                        text.push('.');
                    }
                    text.push_str(&string_literal(args[0]).unwrap_or_default());
                }
                "not" => negations += 1,
                other => return Err(at(call, format!("the path step `{other}`"))),
            }
        }
        let path = format!("{}{text}", "!".repeat(negations));
        let mut parts = Vec::new();
        if !path.is_empty() {
            parts.push(path);
        }
        if let Some(relative) = relative {
            parts.push(format!("RelativeSource={{RelativeSource {relative}}}"));
        }
        parts.extend(options);
        if parts.is_empty() {
            return Ok(BindingValue::Text("{Binding}".to_string()));
        }
        Ok(BindingValue::Text(format!("{{Binding {}}}", parts.join(", "))))
    }

    /// The text of a value (`Thickness::new(1.0, ..)` is `1,..`), as the document writes it.
    fn value_text(&mut self, expr: &Expr, property: Option<&Expr>) -> R<String> {
        let expr = strip_try(expr);
        if let Some(number) = number_text(expr) {
            return Ok(number);
        }
        match expr {
            Expr::Lit(lit) => match &lit.lit {
                Lit::Str(s) => Ok(s.value()),
                Lit::Bool(b) => Ok(if b.value { "True".to_string() } else { "False".to_string() }),
                Lit::Char(c) => Ok(c.value().to_string()),
                _ => Err(at(expr, "a literal")),
            },
            Expr::Binary(binary) if matches!(binary.op, syn::BinOp::BitOr(_)) => {
                Ok(format!("{}, {}", self.value_text(&binary.left, property)?, self.value_text(&binary.right, property)?))
            }
            Expr::Path(path) => {
                let text = path_text(&path.path);
                if let Some(constant) = text.strip_prefix("f64::") {
                    return Ok(match constant {
                        "NAN" => "NaN",
                        "INFINITY" => "Infinity",
                        "NEG_INFINITY" => "-Infinity",
                        other => other,
                    }
                    .to_string());
                }
                let last = path.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
                if last == "TYPE" {
                    let owner = Owner(&path.path);
                    let (xaml, _) = self.type_of_owner(&owner)?;
                    return Ok(format!("{{x:Type {}}}", xaml.name));
                }
                let owner = Owner(&path.path);
                if let Ok((_, model)) = self.type_of_owner(&owner) {
                    for member in &model.enum_members {
                        let variant = member
                            .rust_variant
                            .clone()
                            .or_else(|| member.rust_value.as_ref().and_then(|v| v.rsplit("::").next().map(str::to_string)));
                        if variant.as_deref() == Some(last.as_str()) {
                            return Ok(member.name.clone());
                        }
                    }
                }
                // A constant of a value type: `FontWeight::SemiBold` is `SemiBold`, `Colors::LIGHT_GRAY`
                // is `LightGray`.
                if last.chars().any(|c| c.is_ascii_lowercase()) {
                    return Ok(last);
                }
                Ok(pascal(&last.to_lowercase()).replace('_', ""))
            }
            Expr::Call(call) => {
                let function = path_of_call(call);
                let last = function.rsplit("::").next().unwrap_or_default().to_string();
                let args: Vec<&Expr> = call.args.iter().collect();
                match last.as_str() {
                    "Some" => self.value_text(args[0], property),
                    "uniform" => self.value_text(args[0], property),
                    "symmetric" => Ok(format!("{},{}", self.value_text(args[0], property)?, self.value_text(args[1], property)?)),
                    "new" if function.ends_with("Thickness::new") || function.ends_with("CornerRadius::new") => {
                        let parts: Vec<String> = args.iter().map(|a| number_text(a).unwrap_or_default()).collect();
                        Ok(parts.join(","))
                    }
                    "from_uint32" => Ok(color_text(hex_of(args[0]).unwrap_or_default())),
                    "literal" => {
                        let Expr::Call(parse) = strip_try(args[0]) else { return Err(at(call, "a parse call")) };
                        match strip_try(&parse.args[0]) {
                            Expr::Call(from) => Ok(string_literal(&from.args[0]).unwrap_or_default()),
                            other => Ok(string_literal(other).unwrap_or_default()),
                        }
                    }
                    "from_milliseconds" | "from_millis" | "from_secs_f64" => {
                        let number: f64 = number_text(args[0]).unwrap_or_default().parse().unwrap_or(0.0);
                        let milliseconds = if last == "from_secs_f64" { number * 1000.0 } else { number };
                        Ok(time_span_text(milliseconds))
                    }
                    "from" if function.ends_with("String::from") => Ok(string_literal(args[0]).unwrap_or_default()),
                    "value" => self.value_text(args[0], property),
                    _ => {
                        if let Some(text) = self.static_member_text(call)? {
                            // A static member of the type of the value is its name.
                            if let Some(property) = property {
                                if self.is_literal_member(call, property)? {
                                    let name = text.trim_end_matches('}').rsplit('.').next().unwrap_or_default().to_string();
                                    return Ok(name);
                                }
                            }
                            return Ok(text);
                        }
                        let (_, function) = self.static_owner(call)?;
                        Ok(pascal(&function))
                    }
                }
            }
            Expr::MethodCall(call) if call.method == "element_ref" => Ok(string_literal(&call.args[0]).unwrap_or_default()),
            Expr::MethodCall(call) if call.method == "expect" => self.value_text(&call.receiver, property),
            Expr::MethodCall(call) if call.method == "to_string" => Ok(string_literal(&call.receiver).unwrap_or_default()),
            Expr::MethodCall(call) if call.method == "property" => {
                let _ = call;
                self.source_name(expr)
            }
            other => Err(at(other, format!("the value `{}`", expr_text(other)))),
        }
    }

    /// Whether the static member `call` is a member of the type of the value of `property`
    /// (`TextTrimming="CharacterEllipsis"`), which the document writes by its name.
    fn is_literal_member(&mut self, call: &ExprCall, property: &Expr) -> R<bool> {
        let (owner, _) = self.static_owner(call)?;
        let Some((_, owner)) = owner else { return Ok(false) };
        let Ok((listed, name, _)) = self.property(property) else { return Ok(false) };
        let Some(registered) = listed.registered.iter().find(|r| self.models.name_of(r) == Some(name.as_str())) else { return Ok(false) };
        Ok(registered.value_type.text.contains(&format!("{}>", owner.name)) || registered.value_type.text.ends_with(&owner.name))
    }
}

fn is_none(expr: &Expr) -> bool {
    matches!(strip_try(expr), Expr::Path(path) if path_text(&path.path) == "None")
}

fn pascal(snake_name: &str) -> String {
    snake_name
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

/// A time span in the text form of XAML: `0:0:0.083`.
fn time_span_text(milliseconds: f64) -> String {
    let total = milliseconds / 1000.0;
    let hours = (total / 3600.0).floor();
    let minutes = ((total - hours * 3600.0) / 60.0).floor();
    let seconds = total - hours * 3600.0 - minutes * 60.0;
    let seconds = format!("{seconds:.7}");
    let seconds = seconds.trim_end_matches('0').trim_end_matches('.');
    format!("{}:{}:{}", hours as i64, minutes as i64, if seconds.is_empty() { "0" } else { seconds })
}

/// The element form of a markup extension (`{Binding A, Mode=B}` is `<Binding Path="A" Mode="B" />`).
fn extension_element(text: &str) -> Node {
    let text = text.trim();
    let body = text.strip_prefix('{').and_then(|body| body.strip_suffix('}')).unwrap_or(text);
    let (name, rest) = body.split_once(' ').unwrap_or((body, ""));
    let mut node = Node::new(name);
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut current = String::new();
    for c in rest.chars() {
        match c {
            '{' => depth += 1,
            '}' => depth -= 1,
            _ => {}
        }
        if c == ',' && depth == 0 {
            parts.push(std::mem::take(&mut current));
        } else {
            current.push(c);
        }
    }
    if !current.trim().is_empty() {
        parts.push(current);
    }
    for part in parts {
        let part = part.trim();
        match part.split_once('=') {
            Some((key, value)) if !key.contains('{') => node.attribute(key.trim(), value.trim().trim_matches('\'')),
            _ => node.attribute(if name == "TemplateBinding" { "Property" } else { "Path" }, part),
        }
    }
    node
}

/// Whether the property `name` of `owner` is attached (`ScrollViewer.IsDeferredScrollingEnabled`).
fn owner_has_attached(owner: &TypeModel, name: &str, models: &ModelSet) -> bool {
    owner.registered.iter().any(|registered| registered.kind == RegisteredKind::Attached && models.name_of(registered) == Some(name))
}

/// The name of the accessor function of `X::name_property()` in `expr`.
fn accessor_name(expr: &Expr) -> Option<String> {
    let call = match strip_try(expr) {
        Expr::Call(call) => call,
        Expr::MethodCall(method) => match &*method.receiver {
            Expr::Call(call) => call,
            _ => return None,
        },
        _ => return None,
    };
    match &*call.func {
        Expr::Path(path) => path.path.segments.last().map(|segment| segment.ident.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modules_are_named_as_the_converter_names_them() {
        assert_eq!(module_of("Controls/Button.xaml"), "controls::button");
        assert_eq!(module_of("Resources/TextBlock_themeresources_v2.5.xaml"), "resources::text_block_themeresources_v2_5");
        assert_eq!(module_of("WinUITheme.xaml"), "win_ui_theme");
        assert_eq!(module_of("Controls/WinUIControls.xaml"), "controls::win_ui_controls");
        assert_eq!(module_of("DensityStyles/Compact.xaml"), "density_styles::compact");
    }

    #[test]
    fn values_are_written_as_documents_write_them() {
        assert_eq!(time_span_text(83.0), "0:0:0.083");
        assert_eq!(time_span_text(367.0), "0:0:0.367");
        assert_eq!(nth_text("2", "1"), "odd");
        assert_eq!(nth_text("3", "-1"), "3n-1");
        assert_eq!(color_text(0xFF00_0000), "#FF000000");
    }

    #[test]
    fn a_markup_extension_becomes_an_element() {
        let mut out = String::new();
        extension_element("{Binding Text, RelativeSource={RelativeSource TemplatedParent}, Converter={x:Static StringConverters.IsNullOrEmpty}}").write(&mut out, 0);
        assert_eq!(
            out,
            "<Binding Path=\"Text\" RelativeSource=\"{RelativeSource TemplatedParent}\" Converter=\"{x:Static StringConverters.IsNullOrEmpty}\" />\n"
        );
    }
}
