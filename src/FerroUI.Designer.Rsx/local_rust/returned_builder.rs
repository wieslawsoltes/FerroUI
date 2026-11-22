//! Locate a directly returned builder expression without executing Rust.
use super::{LocalRust, MAX_INDIRECTION, Resolved};
use syn::{Expr, ItemFn, Pat, Stmt};

pub(crate) struct ReturnedBuilder<'a> {
    pub(crate) scope: RustSourceScope<'a>,
    pub(crate) name: &'a str,
    pub(crate) expression: &'a Expr,
}

#[derive(Clone, Copy)]
pub(crate) struct RustSourceScope<'a> {
    resolver: &'a LocalRust,
    scope: usize,
}

impl LocalRust {
    pub(crate) fn source_scope(&self, expression: &Expr) -> Option<RustSourceScope<'_>> {
        Some(RustSourceScope {
            resolver: self,
            scope: self.active_scope(expression)?,
        })
    }
}

impl<'a> RustSourceScope<'a> {
    pub(crate) fn uri(self) -> Option<&'a str> {
        let file = self.resolver.scopes[self.scope].file;
        (file != self.resolver.active).then(|| self.resolver.files[file].as_str())
    }

    pub(crate) fn path(self, expression: &Expr) -> Option<String> {
        self.resolver.path_in(self.scope, expression)
    }

    pub(crate) fn returned_builder(mut self, assignment: &Expr) -> Option<ReturnedBuilder<'a>> {
        let mut expression = ungroup(assignment);
        let mut resolved = None;
        for _ in 0..MAX_INDIRECTION {
            let Expr::Call(call) = expression else {
                return resolved;
            };
            if !call.attrs.is_empty() || !call.args.is_empty() {
                return resolved;
            }
            let Some(Resolved::Builder {
                name,
                expression: body,
                scope: owner,
            }) = self.resolver.resolve(self.scope, &call.func)
            else {
                return resolved;
            };
            self.scope = owner;
            expression = body;
            resolved = Some(ReturnedBuilder {
                scope: self,
                name,
                expression: body,
            });
        }
        None
    }
}

pub(super) fn definition(function: &ItemFn) -> Option<Expr> {
    let sig = &function.sig;
    if !sig.inputs.is_empty()
        || !sig.generics.params.is_empty()
        || sig.generics.where_clause.is_some()
        || sig.asyncness.is_some()
        || sig.unsafety.is_some()
        || sig.abi.is_some()
        || sig.variadic.is_some()
        || function.attrs.iter().any(|attribute| {
            !["doc", "inline", "must_use"]
                .iter()
                .any(|name| attribute.path().is_ident(*name))
        })
    {
        return None;
    }
    if let [Stmt::Macro(mac)] = function.block.stmts.as_slice() {
        return (mac.attrs.is_empty() && mac.semi_token.is_none()).then(|| {
            Expr::Macro(syn::ExprMacro { attrs: Vec::new(), mac: mac.mac.clone() })
        });
    }
    let value = match function.block.stmts.as_slice() {
        [statement] => tail(statement)?,
        [Stmt::Local(local), statement] if local.attrs.is_empty() => {
            let pattern = match &local.pat {
                Pat::Type(typed) if typed.attrs.is_empty() => typed.pat.as_ref(),
                pattern => pattern,
            };
            let Pat::Ident(binding) = pattern else {
                return None;
            };
            let initializer = local.init.as_ref()?;
            let Expr::Path(returned) = ungroup(tail(statement)?) else {
                return None;
            };
            if binding.by_ref.is_some()
                || binding.mutability.is_some()
                || binding.subpat.is_some()
                || !binding.attrs.is_empty()
                || initializer.diverge.is_some()
                || !returned.attrs.is_empty()
                || returned.qself.is_some()
                || !returned.path.is_ident(&binding.ident)
            {
                return None;
            }
            initializer.expr.as_ref()
        }
        _ => return None,
    };
    let expression = ungroup(value);
    match expression {
        Expr::Call(call) if call.attrs.is_empty() => Some(expression.clone()),
        Expr::Macro(mac) if mac.attrs.is_empty() => Some(expression.clone()),
        _ => None,
    }
}

fn tail(statement: &Stmt) -> Option<&Expr> {
    match statement {
        Stmt::Expr(Expr::Return(value), _) if value.attrs.is_empty() => value.expr.as_deref(),
        Stmt::Expr(expression, None) => Some(expression),
        _ => None,
    }
}

fn ungroup(mut expression: &Expr) -> &Expr {
    loop {
        expression = match expression {
            Expr::Paren(group) if group.attrs.is_empty() => &group.expr,
            Expr::Group(group) if group.attrs.is_empty() => &group.expr,
            _ => return expression,
        };
    }
}
