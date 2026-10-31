//! Rewrite singleton names in the importing document's lexical scope. References
//! from other files converge on the same private object identity in the linked IR.
use super::*;
use std::collections::HashSet;

impl<R: ComponentResolver> Linker<'_, R> {
    pub(super) fn singleton_names(
        &mut self,
        context: &Context,
        members: &mut [Member],
    ) -> Result<(), LinkError> {
        for member in members {
            let span = member.span;
            let mut resolve = |name: &str| -> Result<Option<String>, LinkError> {
                let Some(source) = self
                    .resolver
                    .singleton(&context.origin, &context.imports, name)
                    .map_err(|m| context.error(span, m))?
                else {
                    return Ok(None);
                };
                if let Some(id) = self.singleton_ids.get(&source.key) {
                    return Ok(Some(id.clone()));
                }
                if !matches!(&source.document.pragmas[..], [pragma] if pragma.name=="Singleton" && pragma.values.is_empty())
                {
                    return Err(context.error(
                        span,
                        format!("singleton '{}' requires pragma Singleton", source.key),
                    ));
                }
                // Register the identity before walking its references. Two
                // singletons may refer to each other without a binding cycle.
                self.serial += 1;
                let id = format!("$qml:singleton:{}", self.serial);
                self.singleton_ids.insert(source.key.clone(), id.clone());
                let mut root = source.document.root.clone();
                crate::scope::isolate(&mut root, self.serial, &id);
                let defining = Context::new(source.key.clone(), &source.document, self.resolver)?;
                self.stack.push(source.key);
                self.object(&defining, &mut root)?;
                self.stack.pop();
                self.singletons.push(root);
                Ok(Some(id))
            };
            match &mut member.kind {
                MemberKind::Binding { value, .. }
                | MemberKind::Property {
                    value: Some(value), ..
                } => match value {
                    Binding::Expr(expr) => expression(expr, &HashSet::new(), &mut resolve)?,
                    Binding::Script(body) => statements(body, HashSet::new(), &mut resolve)?,
                    _ => (),
                },
                MemberKind::Function {
                    parameters, body, ..
                } => statements(
                    body,
                    parameters.iter().map(|p| p.name.clone()).collect(),
                    &mut resolve,
                )?,
                MemberKind::Handler {
                    parameters, body, ..
                } => statements(body, parameters.iter().cloned().collect(), &mut resolve)?,
                MemberKind::Group { members, .. } => self.singleton_names(context, members)?,
                _ => (),
            }
        }
        Ok(())
    }
}

type Lookup<'a> = dyn FnMut(&str) -> Result<Option<String>, LinkError> + 'a;

fn name(expr: &Expr) -> Option<&str> {
    match expr {
        Expr::Name(name) | Expr::ScopedName { name, .. }
            if name.chars().next().is_some_and(char::is_uppercase) =>
        {
            Some(name)
        }
        _ => None,
    }
}

fn expression(
    expr: &mut Expr,
    locals: &HashSet<String>,
    resolve: &mut Lookup<'_>,
) -> Result<(), LinkError> {
    if let Expr::Member(qualifier, ty) = expr {
        if let Some(ns) = name(qualifier).filter(|n| !locals.contains(*n)) {
            if ty.chars().next().is_some_and(char::is_uppercase) {
                if let Some(id) = resolve(&format!("{ns}.{ty}"))? {
                    *expr = Expr::Name(id);
                    return Ok(());
                }
            }
        }
    }
    if let Some(ty) = name(expr).filter(|n| !locals.contains(*n)) {
        if let Some(id) = resolve(ty)? {
            *expr = Expr::Name(id);
        }
        return Ok(());
    }
    match expr {
        Expr::Unary(_, arg) | Expr::Member(arg, _) => expression(arg, locals, resolve)?,
        Expr::Index(a, b) | Expr::Binary(_, a, b) => {
            expression(a, locals, resolve)?;
            expression(b, locals, resolve)?;
        }
        Expr::Conditional(c, a, b) => {
            for arg in [c, a, b] {
                expression(arg, locals, resolve)?;
            }
        }
        Expr::Array(args) => {
            for arg in args {
                expression(arg, locals, resolve)?;
            }
        }
        Expr::Call(callee, args) => {
            expression(callee, locals, resolve)?;
            for arg in args {
                expression(arg, locals, resolve)?;
            }
        }
        _ => (),
    }
    Ok(())
}

fn statements(
    body: &mut [Stmt],
    mut locals: HashSet<String>,
    resolve: &mut Lookup<'_>,
) -> Result<(), LinkError> {
    for stmt in body.iter() {
        if let Stmt::Let { name, .. } = stmt {
            locals.insert(name.clone());
        }
    }
    for stmt in body {
        match stmt {
            Stmt::Expr(expr)
            | Stmt::Return(Some(expr))
            | Stmt::Let {
                value: Some(expr), ..
            } => expression(expr, &locals, resolve)?,
            Stmt::Assign { target, value, .. } => {
                expression(target, &locals, resolve)?;
                expression(value, &locals, resolve)?;
            }
            Stmt::If {
                test,
                then_body,
                else_body,
            } => {
                expression(test, &locals, resolve)?;
                statements(then_body, locals.clone(), resolve)?;
                statements(else_body, locals.clone(), resolve)?;
            }
            Stmt::Block(stmts) => statements(stmts, locals.clone(), resolve)?,
            _ => (),
        }
    }
    Ok(())
}
