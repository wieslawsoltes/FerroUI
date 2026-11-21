//! Resolve explicit names and constant initializers in authored Rust sources.
mod collect;
mod returned_builder;

use crate::animation::syntax as rust;
use ferroui_designer_core::SourceSpan;
use ferroui_designer_project::RustModuleSources;
use std::collections::BTreeMap;
use syn::{Expr, ext::IdentExt};

const MAX_INDIRECTION: usize = 32;

pub(crate) struct LocalRust {
    scopes: Vec<Scope>,
    files: Vec<String>,
    active: usize,
    target_known: bool,
}

struct Scope {
    span: SourceSpan,
    parent: Option<usize>,
    module: usize,
    super_module: Option<usize>,
    file: usize,
    indexed: bool,
    members: BTreeMap<String, Binding>,
}

enum Binding {
    Import { absolute: bool, path: Vec<String> },
    Module(usize),
    Constant(Box<syn::ItemConst>),
    Builder(Box<Expr>),
    Opaque,
}

enum Resolved<'a> {
    External(String),
    Constant {
        name: &'a str,
        item: &'a syn::ItemConst,
        scope: usize,
    },
    Builder {
        name: &'a str,
        expression: &'a Expr,
        scope: usize,
    },
}

pub(crate) use returned_builder::ReturnedBuilder;

pub(crate) struct Value<'a> {
    pub(crate) expression: &'a Expr,
    pub(crate) constant: Option<&'a str>,
    ty: Option<&'a syn::Type>,
    scope: usize,
}

impl LocalRust {
    pub(crate) fn parse(source: &str, offset: usize, modules: Option<&RustModuleSources>) -> Self {
        collect::parse(source, offset, modules)
    }

    pub(crate) fn path(&self, expression: &Expr) -> Option<String> {
        self.path_in(self.active_scope(expression)?, expression)
    }

    pub(crate) fn value_path(&self, value: &Value<'_>, expression: &Expr) -> Option<String> {
        self.path_in(value.scope, expression)
    }

    pub(crate) fn value_uri(&self, value: &Value<'_>) -> Option<&str> {
        let file = self.scopes[value.scope].file;
        (file != self.active).then(|| self.files[file].as_str())
    }

    pub(crate) fn value_type_path(&self, value: &Value<'_>) -> Option<String> {
        let syn::Type::Path(ty) = value.ty? else {
            return None;
        };
        self.path_in(
            value.scope,
            &Expr::Path(syn::ExprPath {
                attrs: Vec::new(),
                qself: ty.qself.clone(),
                path: ty.path.clone(),
            }),
        )
    }

    fn path_in(&self, scope: usize, expression: &Expr) -> Option<String> {
        match self.resolve(scope, expression)? {
            Resolved::External(path) => Some(path),
            Resolved::Constant { .. } | Resolved::Builder { .. } => None,
        }
    }

    pub(crate) fn value<'a>(&'a self, expression: &'a Expr) -> Option<Value<'a>> {
        self.value_in(self.active_scope(expression)?, expression)
    }

    pub(crate) fn operand<'a>(
        &'a self,
        owner: &Value<'a>,
        expression: &'a Expr,
    ) -> Option<Value<'a>> {
        self.value_in(owner.scope, expression)
    }

    fn value_in<'a>(&'a self, scope: usize, expression: &'a Expr) -> Option<Value<'a>> {
        let mut value = Value {
            expression,
            constant: None,
            ty: None,
            scope,
        };
        for _ in 0..MAX_INDIRECTION {
            match value.expression {
                Expr::Paren(group) if group.attrs.is_empty() => value.expression = &group.expr,
                Expr::Group(group) if group.attrs.is_empty() => value.expression = &group.expr,
                Expr::Path(path) if path.attrs.is_empty() => {
                    let Resolved::Constant { name, item, scope } =
                        self.resolve(value.scope, value.expression)?
                    else {
                        return Some(value);
                    };
                    value = Value {
                        expression: &item.expr,
                        constant: Some(name),
                        ty: Some(item.ty.as_ref()),
                        scope,
                    };
                }
                _ => return Some(value),
            }
        }
        None
    }

    fn active_scope(&self, expression: &Expr) -> Option<usize> {
        let offset = rust::span(expression).start;
        self.scopes
            .iter()
            .rposition(|scope| scope.file == self.active && scope.span.contains(offset))
    }

    fn resolve(&self, scope: usize, expression: &Expr) -> Option<Resolved<'_>> {
        let Expr::Path(site) = expression else {
            return None;
        };
        if site.qself.is_some()
            || site
                .path
                .segments
                .iter()
                .any(|part| !part.arguments.is_empty())
        {
            return None;
        }
        let names: Vec<_> = site
            .path
            .segments
            .iter()
            .map(|part| part.ident.unraw().to_string())
            .collect();
        let parts: Vec<_> = names.iter().map(String::as_str).collect();
        self.follow(scope, site.path.leading_colon.is_some(), &parts, 0)
    }

    fn follow(
        &self,
        scope: usize,
        absolute: bool,
        parts: &[&str],
        depth: usize,
    ) -> Option<Resolved<'_>> {
        if depth >= MAX_INDIRECTION {
            return None;
        }
        if absolute {
            return Some(Resolved::External(parts.join("::")));
        }
        let (&first, rest) = parts.split_first()?;
        let module = self.scopes[scope].module;
        match first {
            "self" => self.member(module, rest, depth + 1),
            "super" => {
                let mut module = self.scopes[module].super_module?;
                let mut rest = rest;
                while rest.first() == Some(&"super") {
                    module = self.scopes[module].super_module?;
                    rest = &rest[1..];
                }
                self.member(module, rest, depth + 1)
            }
            "crate" => {
                // Registered getters may be qualified by `crate` even when the
                // syntax index does not recognize their registration macro.
                self.target_known
                    .then(|| self.member(0, rest, depth + 1))
                    .flatten()
                    .or_else(|| Some(Resolved::External(parts.join("::"))))
            }
            _ => {
                let mut current = Some(scope);
                while let Some(index) = current {
                    let scope = &self.scopes[index];
                    if let Some((name, binding)) = scope.members.get_key_value(first) {
                        return self.binding(index, name, binding, rest, depth + 1);
                    }
                    current = scope.parent;
                }
                Some(Resolved::External(parts.join("::")))
            }
        }
    }

    fn member(&self, scope: usize, parts: &[&str], depth: usize) -> Option<Resolved<'_>> {
        if depth >= MAX_INDIRECTION {
            return None;
        }
        let (first, rest) = parts.split_first()?;
        let (name, binding) = self.scopes[scope].members.get_key_value(*first)?;
        self.binding(scope, name, binding, rest, depth + 1)
    }

    fn binding<'a>(
        &'a self,
        scope: usize,
        name: &'a str,
        binding: &'a Binding,
        tail: &[&str],
        depth: usize,
    ) -> Option<Resolved<'a>> {
        if depth >= MAX_INDIRECTION {
            return None;
        }
        match binding {
            Binding::Constant(item) if tail.is_empty() => {
                Some(Resolved::Constant { name, item, scope })
            }
            Binding::Builder(expression) if tail.is_empty() => Some(Resolved::Builder {
                name,
                expression,
                scope,
            }),
            Binding::Module(module) => self.member(*module, tail, depth + 1),
            Binding::Import { absolute, path } => {
                let parts: Vec<_> = path
                    .iter()
                    .map(String::as_str)
                    .chain(tail.iter().copied())
                    .collect();
                self.follow(scope, *absolute, &parts, depth + 1)
            }
            _ => None,
        }
    }
}
