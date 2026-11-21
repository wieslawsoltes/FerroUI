//! Identify the authored FerroUI builders without inspecting generated Rust.
mod groups;
mod reads;

use self::groups::{GroupInputs, GroupSpec};
use self::reads::ReadPath;
use super::{macros::MacroSyntax, string_literal, RustPathAnchor};
use std::collections::BTreeMap;
use syn::{Expr, ExprMethodCall, Item, LitStr, Local, Macro, Pat, UseTree};

pub(super) enum Site<'source> {
    Path(&'source LitStr, RustPathAnchor),
    Key(DocumentKey<'source>),
    Registration(&'source LitStr, DocumentKey<'source>, bool),
    FileDocuments,
    Root(&'source LitStr),
    Discovery,
}

#[derive(Clone, Copy)]
pub(super) enum DocumentInput<'source> {
    Include(&'source Macro),
    Read(&'source LitStr),
}

#[derive(Clone, Copy)]
pub(super) struct DocumentKey<'source> {
    pub literal: &'source LitStr,
    pub expression: &'source Expr,
    pub input: DocumentInput<'source>,
    pub shares_path: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Builder {
    Build,
    Group,
}

struct Binding<'source> {
    value: Option<&'source Expr>,
    builder: Option<Builder>,
    path: Option<ReadPath<'source>>,
    read: Option<&'source LitStr>,
    document: Option<DocumentKey<'source>>,
    documents: Option<Vec<DocumentKey<'source>>>,
    group: Option<GroupSpec<'source>>,
}

pub(super) struct BuildScript<'source> {
    imports: BTreeMap<String, Builder>,
    scopes: Vec<BTreeMap<String, Binding<'source>>>,
}

impl<'source> BuildScript<'source> {
    pub(super) fn new(file: &'source syn::File) -> Self {
        let mut script = Self {
            imports: BTreeMap::new(),
            scopes: vec![BTreeMap::new()],
        };
        for item in &file.items {
            match item {
                Item::Use(item) => {
                    if let UseTree::Path(path) = &item.tree {
                        if path.ident == "ferroui_build" {
                            imports(&path.tree, &mut script.imports);
                        }
                    }
                }
                Item::Const(item) => {
                    script.scopes[0].insert(
                        item.ident.to_string(),
                        Binding {
                            value: Some(&item.expr),
                            builder: None,
                            path: string_literal(&item.expr).map(ReadPath::Literal),
                            read: None,
                            document: None,
                            documents: None,
                            group: None,
                        },
                    );
                }
                _ => {}
            }
        }
        script
    }

    pub(super) fn enter(&mut self) {
        self.scopes.push(BTreeMap::new());
    }
    pub(super) fn leave(&mut self) {
        self.scopes.pop();
    }

    pub(super) fn bind(&mut self, local: &'source Local, macros: &MacroSyntax) {
        let mut pattern = &local.pat;
        if let Pat::Type(typed) = pattern {
            pattern = &typed.pat;
        }
        let Pat::Ident(name) = pattern else { return };
        let initial = local.init.as_ref().map(|init| transparent(&init.expr));
        let fixed = initial.filter(|_| name.mutability.is_none());
        let binding = Binding {
            value: fixed.map(|expr| self.value(expr)),
            builder: initial.and_then(|expr| self.builder(expr)),
            path: fixed.and_then(|expr| self.read_path(expr, macros)),
            read: fixed.and_then(|expr| self.read_source(expr, macros)),
            document: fixed.and_then(|expr| self.document_key(expr, macros)),
            documents: fixed.and_then(|expr| self.keys(expr, macros)),
            group: fixed.and_then(|expr| self.group(expr, macros)),
        };
        self.scopes
            .last_mut()
            .unwrap()
            .insert(name.ident.to_string(), binding);
    }

    pub(super) fn paths(
        &self,
        call: &'source ExprMethodCall,
        macros: &MacroSyntax,
    ) -> Vec<Site<'source>> {
        let builder = self.builder(&call.receiver);
        if builder == Some(Builder::Build) && call.method == "compile_xaml" && call.args.is_empty()
        {
            return vec![Site::Discovery];
        }
        let Some(argument) = call.args.first().filter(|_| call.args.len() == 1) else {
            return Vec::new();
        };
        let value = self.value(argument);
        match (builder, call.method.to_string().as_str()) {
            (Some(Builder::Build), "compile_group")
                if self.builder(argument) == Some(Builder::Group) =>
            {
                self.compile_group(argument, macros)
            }
            (Some(Builder::Build), "input") => string_literal(value)
                .map(|literal| Site::Path(literal, RustPathAnchor::Package))
                .into_iter()
                .collect(),
            (Some(Builder::Build), "xaml_root") => {
                string_literal(value).map(Site::Root).into_iter().collect()
            }
            (Some(Builder::Group), "documents") => self
                .keys(argument, macros)
                .into_iter()
                .flatten()
                .map(Site::Key)
                .collect(),
            (Some(Builder::Group), "document_files") => vec![Site::FileDocuments],
            (Some(Builder::Group), "class_document") => {
                let Some(literal) = string_literal(value) else {
                    return Vec::new();
                };
                match self.group(&call.receiver, macros).map(|group| group.inputs) {
                    Some(GroupInputs::Inline(documents)) => documents
                        .into_iter()
                        .filter(|key| key.literal.value() == literal.value())
                        .map(|key| {
                            Site::Key(DocumentKey {
                                literal,
                                expression: transparent(argument),
                                ..key
                            })
                        })
                        .collect(),
                    Some(GroupInputs::Discovery) => {
                        vec![Site::Path(literal, RustPathAnchor::XamlRoot)]
                    }
                    _ => Vec::new(),
                }
            }
            _ => Vec::new(),
        }
    }

    fn builder(&self, expression: &Expr) -> Option<Builder> {
        match transparent(expression) {
            Expr::Path(path) => self.binding(path)?.builder,
            Expr::MethodCall(call)
                if !matches!(call.method.to_string().as_str(), "run" | "execute") =>
            {
                self.builder(&call.receiver)
            }
            Expr::Call(call) => {
                let Expr::Path(path) = &*call.func else {
                    return None;
                };
                let mut parts = path.path.segments.iter();
                let first = &parts.next()?.ident;
                let (builder, function) = if first == "ferroui_build" {
                    (
                        builder_name(&parts.next()?.ident.to_string())?,
                        &parts.next()?.ident,
                    )
                } else {
                    (*self.imports.get(&first.to_string())?, &parts.next()?.ident)
                };
                if parts.next().is_some() {
                    return None;
                }
                match builder {
                    Builder::Build if function == "from_env" => Some(builder),
                    Builder::Group if function == "new" => Some(builder),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn keys(
        &self,
        value: &'source Expr,
        macros: &MacroSyntax,
    ) -> Option<Vec<DocumentKey<'source>>> {
        if let Expr::Path(path) = transparent(value) {
            if let Some(documents) = self.binding(path)?.documents.as_ref() {
                return Some(documents.clone());
            }
        }
        let Expr::Array(array) = self.value(value) else {
            return None;
        };
        Some(
            array
                .elems
                .iter()
                .filter_map(|entry| self.document_key(entry, macros))
                .collect(),
        )
    }

    fn document_key(
        &self,
        value: &'source Expr,
        macros: &MacroSyntax,
    ) -> Option<DocumentKey<'source>> {
        if let Expr::Path(path) = transparent(value) {
            if let Some(document) = self.binding(path)?.document {
                return Some(document);
            }
        }
        let Expr::Tuple(pair) = self.value(value) else {
            return None;
        };
        if pair.elems.len() != 2 {
            return None;
        }
        let expression = transparent(pair.elems.first()?);
        let literal = string_literal(self.value(expression))?;
        let contents = pair.elems.last()?;
        let input = match self.value(contents) {
            Expr::Macro(input) => DocumentInput::Include(&input.mac),
            _ => DocumentInput::Read(self.read_source(contents, macros)?),
        };
        let shares_path = matches!(input, DocumentInput::Read(path) if literal.span().byte_range() == path.span().byte_range());
        Some(DocumentKey {
            literal,
            expression,
            input,
            shares_path,
        })
    }

    fn value(&self, expression: &'source Expr) -> &'source Expr {
        let mut expression = transparent(expression);
        for _ in 0..16 {
            let Expr::Path(path) = expression else { break };
            let Some(value) = self.binding(path).and_then(|binding| binding.value) else {
                break;
            };
            expression = transparent(value);
        }
        expression
    }

    fn binding(&self, path: &syn::ExprPath) -> Option<&Binding<'source>> {
        let name = path.path.get_ident()?.to_string();
        self.scopes.iter().rev().find_map(|scope| scope.get(&name))
    }
}

fn transparent(mut expression: &Expr) -> &Expr {
    loop {
        expression = match expression {
            Expr::Reference(value) => &value.expr,
            Expr::Paren(value) => &value.expr,
            Expr::Group(value) => &value.expr,
            _ => return expression,
        };
    }
}

fn imports(tree: &UseTree, bindings: &mut BTreeMap<String, Builder>) {
    match tree {
        UseTree::Group(group) => group.items.iter().for_each(|item| imports(item, bindings)),
        UseTree::Rename(alias) => {
            if let Some(builder) = builder_name(&alias.ident.to_string()) {
                bindings.insert(alias.rename.to_string(), builder);
            }
        }
        UseTree::Name(name) => {
            let name = name.ident.to_string();
            if let Some(builder) = builder_name(&name) {
                bindings.insert(name, builder);
            }
        }
        _ => {}
    }
}

fn builder_name(name: &str) -> Option<Builder> {
    match name {
        "Build" => Some(Builder::Build),
        "XamlGroup" => Some(Builder::Group),
        _ => None,
    }
}
