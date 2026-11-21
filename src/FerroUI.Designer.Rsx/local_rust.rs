//! Resolve explicit names and constant initializers in the authored Rust file.
mod collect;

use crate::animation::syntax as rust;
use ferroui_designer_core::SourceSpan;
use std::collections::BTreeMap;
use syn::{Expr, ext::IdentExt};

const MAX_INDIRECTION: usize = 32;

pub(crate) struct LocalRust {
    scopes: Vec<Scope>,
}

struct Scope {
    span: SourceSpan,
    parent: Option<usize>,
    module: usize,
    super_module: Option<usize>,
    members: BTreeMap<String, Binding>,
}

enum Binding {
    Import { absolute: bool, path: Vec<String> },
    Module(usize),
    Constant(Box<Expr>),
    Opaque,
}

enum Resolved<'a> {
    External(String),
    Constant { name: &'a str, value: &'a Expr },
}

pub(crate) struct Value<'a> {
    pub(crate) expression: &'a Expr,
    pub(crate) constant: Option<&'a str>,
}

impl LocalRust {
    pub(crate) fn parse(source: &str, offset: usize) -> Self {
        collect::parse(source, offset)
    }

    pub(crate) fn path(&self, expression: &Expr) -> Option<String> {
        match self.resolve(expression)? {
            Resolved::External(path) => Some(path),
            Resolved::Constant { .. } => None,
        }
    }

    pub(crate) fn value<'a>(&'a self, mut expression: &'a Expr) -> Option<Value<'a>> {
        let mut constant = None;
        for _ in 0..MAX_INDIRECTION {
            match expression {
                Expr::Paren(group) => expression = &group.expr,
                Expr::Group(group) => expression = &group.expr,
                Expr::Path(_) => {
                    let Resolved::Constant { name, value } = self.resolve(expression)? else {
                        return None;
                    };
                    expression = value;
                    constant = Some(name);
                }
                _ => {
                    return Some(Value {
                        expression,
                        constant,
                    });
                }
            }
        }
        None
    }

    fn resolve(&self, expression: &Expr) -> Option<Resolved<'_>> {
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
        let offset = rust::span(expression).start;
        let scope = self
            .scopes
            .iter()
            .rposition(|scope| scope.span.contains(offset))?;
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
            // The authored file can be a module file inside another crate root.
            "crate" => Some(Resolved::External(parts.join("::"))),
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
            Binding::Constant(value) if tail.is_empty() => Some(Resolved::Constant { name, value }),
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
