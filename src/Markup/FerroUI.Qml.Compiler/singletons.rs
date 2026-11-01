//! Resolve type constants and singleton names in each document. References
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
            let mut resolve = |parts: &[&str]| -> Result<Option<Expr>, LinkError> {
                let qualified = context
                    .imports
                    .iter()
                    .any(|i| i.alias.as_deref() == Some(parts[0]));
                let (name, tail) = match (qualified, parts) {
                    (true, [ns, ty, tail @ ..]) => (format!("{ns}.{ty}"), tail),
                    (false, [ty, tail @ ..]) => (ty.to_string(), tail),
                    _ => return Ok(None),
                };
                let singleton = self
                    .resolver
                    .singleton(&context.origin, &context.imports, &name)
                    .map_err(|m| context.error(span, m))?;
                if !tail.is_empty() {
                    let source = if let Some(template) = context.inline.get(&name) {
                        return enum_value(template, tail).map_err(|m| context.error(span, m));
                    } else if singleton.is_some() {
                        singleton
                    } else {
                        self.resolver
                            .resolve(&context.origin, &context.imports, &name)
                            .map_err(|m| context.error(span, m))?
                    };
                    return match source {
                        Some(source) => enum_value(&source.document.root, tail)
                            .map_err(|m| context.error(span, m)),
                        None => Ok(None),
                    };
                }
                let Some(source) = singleton else {
                    return Ok(None);
                };
                if let Some(id) = self.singleton_ids.get(&source.key) {
                    return Ok(Some(Expr::Name(id.clone())));
                }
                if !source
                    .document
                    .pragmas
                    .iter()
                    .any(|p| p.name == "Singleton" && p.values.is_empty())
                    || source.document.unsupported_pragma(true).is_some()
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
                let scope = self.scope_id(&source.key);
                crate::scope::isolate(&mut root, self.serial, &id, scope);
                self.collect_scripts(&source)?;
                let defining = Context::new(source.key.clone(), &source.document, self.resolver)?;
                self.stack.push(source.key);
                self.object(&defining, &mut root)?;
                // Expanding a composite base keeps its root ID member. The
                // singleton identity belongs to the exported document.
                if let Some(member) = root
                    .members
                    .iter_mut()
                    .find(|m| matches!(m.kind, MemberKind::Id(_)))
                {
                    member.scope = Some(scope);
                }
                self.stack.pop();
                self.singletons.push(root);
                Ok(Some(Expr::Name(id)))
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

fn enum_value(template: &Object, path: &[&str]) -> Result<Option<Expr>, String> {
    let (group, key) = match path {
        [key] => (None, *key),
        [group, key] => (Some(*group), *key),
        _ => return Ok(None),
    };
    let mut result = None;
    for member in &template.members {
        if let MemberKind::Enum { name, values } = &member.kind {
            if group.is_some_and(|g| g != name) {
                continue;
            }
            if let Some((_, n)) = values.iter().find(|(k, _)| k == key) {
                if result.is_some() {
                    return Err(format!("ambiguous enum key '{key}'; qualify the enum name"));
                }
                result = Some(Expr::Number(f64::from(*n)));
            }
        }
    }
    Ok(result)
}

type Lookup<'a> = dyn FnMut(&[&str]) -> Result<Option<Expr>, LinkError> + 'a;

/// A type export, enum scope and key, optionally preceded by a module qualifier.
fn type_path<'a>(expr: &'a Expr, parts: &mut Vec<&'a str>) -> bool {
    let name = match expr {
        Expr::Name(name) | Expr::ScopedName { name, .. } => name,
        Expr::Member(base, name) if type_path(base, parts) => name,
        _ => return false,
    };
    if parts.len() == 4 || !name.chars().next().is_some_and(char::is_uppercase) {
        return false;
    }
    parts.push(name);
    true
}

fn javascript_names(
    js: &mut JavaScript,
    locals: &HashSet<String>,
    resolve: &mut Lookup<'_>,
) -> Result<(), LinkError> {
    for name in std::mem::take(&mut js.references) {
        let parts: Vec<_> = name.split('.').collect();
        if locals.contains(parts[0]) || js.imports.contains_key(parts[0]) {
            continue;
        }
        if let Some(Expr::Name(id)) = resolve(&parts)? {
            js.ids.insert(name, id);
        }
    }
    Ok(())
}
fn expression(
    expr: &mut Expr,
    locals: &HashSet<String>,
    resolve: &mut Lookup<'_>,
) -> Result<(), LinkError> {
    if let Expr::JavaScript(js) = expr {
        return javascript_names(js, locals, resolve);
    }
    let mut parts = Vec::new();
    if type_path(expr, &mut parts) && !locals.contains(parts[0]) {
        if let Some(value) = resolve(&parts)? {
            *expr = value;
            return Ok(());
        }
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
        Expr::Record(fields) => {
            for (_, value) in fields {
                expression(value, locals, resolve)?;
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
            Stmt::JavaScript(js) => javascript_names(js, &locals, resolve)?,
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
            Stmt::While { test, body, .. } => {
                expression(test, &locals, resolve)?;
                statements(body, locals.clone(), resolve)?;
            }
            Stmt::For {
                init,
                test,
                update,
                body,
            } => {
                let mut scope = locals.clone();
                if let Some(init) = init {
                    if let Stmt::Let { name, .. } = init.as_ref() {
                        scope.insert(name.clone());
                    }
                    statements(std::slice::from_mut(init), scope.clone(), resolve)?;
                }
                if let Some(test) = test {
                    expression(test, &scope, resolve)?;
                }
                if let Some(update) = update {
                    statements(std::slice::from_mut(update), scope.clone(), resolve)?;
                }
                statements(body, scope, resolve)?;
            }
            Stmt::Block(stmts) => statements(stmts, locals.clone(), resolve)?,
            _ => (),
        }
    }
    Ok(())
}
