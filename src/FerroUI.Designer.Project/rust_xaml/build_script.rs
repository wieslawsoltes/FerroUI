//! Identify the authored FerroUI builders without inspecting generated Rust.
use super::{string_literal, RustPathAnchor};
use std::collections::BTreeMap;
use syn::{Expr, ExprMethodCall, Item, LitStr, Local, Macro, Pat, UseTree};

pub(super) enum Site<'source> {
    Path(&'source LitStr, RustPathAnchor),
    Key(&'source LitStr, &'source Macro),
    Root(&'source LitStr),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Builder {
    Build,
    Group,
}

#[derive(Clone, Copy)]
struct Binding<'source> {
    value: Option<&'source Expr>,
    builder: Option<Builder>,
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

    pub(super) fn bind(&mut self, local: &'source Local) {
        let mut pattern = &local.pat;
        if let Pat::Type(typed) = pattern {
            pattern = &typed.pat;
        }
        let Pat::Ident(name) = pattern else { return };
        let initial = local.init.as_ref().map(|init| transparent(&init.expr));
        let binding = Binding {
            value: if name.mutability.is_none() {
                initial.map(|expr| self.value(expr))
            } else {
                None
            },
            builder: initial.and_then(|expr| self.builder(expr)),
        };
        self.scopes
            .last_mut()
            .unwrap()
            .insert(name.ident.to_string(), binding);
    }

    pub(super) fn paths(&self, call: &'source ExprMethodCall) -> Vec<Site<'source>> {
        let Some(argument) = call.args.first().filter(|_| call.args.len() == 1) else {
            return Vec::new();
        };
        let value = self.value(argument);
        match (
            self.builder(&call.receiver),
            call.method.to_string().as_str(),
        ) {
            (Some(Builder::Build), "input") => string_literal(value)
                .map(|literal| Site::Path(literal, RustPathAnchor::Package))
                .into_iter()
                .collect(),
            (Some(Builder::Build), "xaml_root") => {
                string_literal(value).map(Site::Root).into_iter().collect()
            }
            (Some(Builder::Group), "documents") => self
                .keys(value)
                .into_iter()
                .map(|(key, input)| Site::Key(key, input))
                .collect(),
            (Some(Builder::Group), "class_document") => {
                let Some(literal) = string_literal(value) else {
                    return Vec::new();
                };
                if let Some(entries) = self.documents(&call.receiver) {
                    return self
                        .keys(entries)
                        .into_iter()
                        .filter(|(key, _)| key.value() == literal.value())
                        .map(|(_, input)| Site::Key(literal, input))
                        .collect();
                }
                vec![Site::Path(literal, RustPathAnchor::XamlRoot)]
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

    fn documents(&self, expression: &'source Expr) -> Option<&'source Expr> {
        let mut expression = self.value(expression);
        while let Expr::MethodCall(call) = expression {
            if call.method == "documents" {
                return call.args.first();
            }
            expression = self.value(&call.receiver);
        }
        None
    }

    fn keys(&self, value: &'source Expr) -> Vec<(&'source LitStr, &'source Macro)> {
        let Expr::Array(array) = self.value(value) else {
            return Vec::new();
        };
        array
            .elems
            .iter()
            .filter_map(|entry| {
                let Expr::Tuple(pair) = self.value(entry) else {
                    return None;
                };
                if pair.elems.len() != 2 {
                    return None;
                }
                let key = string_literal(self.value(pair.elems.first()?))?;
                let Expr::Macro(input) = self.value(pair.elems.last()?) else {
                    return None;
                };
                Some((key, &input.mac))
            })
            .collect()
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

    fn binding(&self, path: &syn::ExprPath) -> Option<Binding<'source>> {
        let name = path.path.get_ident()?.to_string();
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(&name).copied())
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
