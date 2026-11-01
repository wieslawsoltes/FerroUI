//! Lower each executable expression into Rust. Static alias paths and
//! assignment destinations remain structural metadata for object linking.
use ferroui_qml_compiler::{Binding, Document, Expr, Member, MemberKind, Stmt};
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashMap;

type FirstRead = Option<(String, Option<String>)>;

#[derive(Default)]
pub(crate) struct Functions {
    // Native closures take the receiver and lexical environment from the call
    // frame. Share identical emitted bodies, retaining the scheduling hint as
    // part of an evaluator's identity. Source locations stay on the IR member.
    evaluators: HashMap<(String, FirstRead), usize>,
    procedures: HashMap<String, usize>,
    pub expressions: Vec<TokenStream>,
    pub first_reads: Vec<TokenStream>,
    pub scripts: Vec<TokenStream>,
}
pub(crate) fn lower(document: &mut Document) -> Result<Functions, String> {
    let mut functions = Functions::default();
    for root in std::iter::once(&mut document.root)
        .chain(&mut document.singletons)
        .chain(document.components.values_mut())
    {
        members(&mut root.members, &mut functions)?;
    }
    Ok(functions)
}

fn members(group: &mut [Member], out: &mut Functions) -> Result<(), String> {
    for member in group {
        match &mut member.kind {
            MemberKind::Child(object)
            | MemberKind::OnProperty { object, .. }
            | MemberKind::Component { object, .. } => members(&mut object.members, out)?,
            MemberKind::Group { members: group, .. } => members(group, out)?,
            MemberKind::Property { ty, .. } if ty == "alias" => (),
            MemberKind::Property {
                value: Some(binding),
                ..
            }
            | MemberKind::Binding { value: binding, .. } => match binding {
                Binding::Object(object) => members(&mut object.members, out)?,
                Binding::Objects(objects) => {
                    for object in objects {
                        members(&mut object.members, out)?;
                    }
                }
                Binding::Expr(Expr::JavaScript(_)) => (),
                Binding::Expr(expr) if expr.as_literal().is_some() => (),
                Binding::Expr(expr) => replace(expr, out)?,
                Binding::Script(body) => statements(body, out)?,
            },
            MemberKind::Function { body, .. } | MemberKind::Handler { body, .. } => {
                statements(body, out)?
            }
            _ => (),
        }
    }
    Ok(())
}

fn statements(body: &mut Vec<Stmt>, out: &mut Functions) -> Result<(), String> {
    if body.is_empty() || matches!(body.as_slice(), [Stmt::JavaScript(_)]) {
        return Ok(());
    }
    let code = super::scripts::body(body)?;
    let index = *out.procedures.entry(code.to_string()).or_insert_with(|| {
        let index = out.scripts.len();
        out.scripts.push(quote!(|frame| { let _ = &frame; #code }));
        index
    });
    *body = vec![Stmt::Native(index)];
    Ok(())
}

fn replace(expr: &mut Expr, out: &mut Functions) -> Result<(), String> {
    let code = emit(expr)?;
    let read = expr.first_read();
    let key = (
        code.to_string(),
        read.map(|(name, root)| (name.to_owned(), root.map(str::to_owned))),
    );
    let index = *out.evaluators.entry(key).or_insert_with(|| {
        let index = out.expressions.len();
        out.first_reads.push(match read {
            Some((name, Some(root))) => quote!(Some((#name,Some(#root)))),
            Some((name, None)) => quote!(Some((#name,None))),
            None => quote!(None),
        });
        out.expressions
            .push(quote!(|frame| { let _ = &frame; Ok(#code) }));
        index
    });
    *expr = Expr::Native(index);
    Ok(())
}

pub(super) fn name(expr: &Expr) -> Option<(&str, TokenStream)> {
    match expr {
        Expr::Name(ident) => Some((ident, quote!(None))),
        Expr::ScopedName { name, root } => Some((name, quote!(Some(#root)))),
        _ => None,
    }
}
pub(super) fn emit(expr: &Expr) -> Result<TokenStream, String> {
    Ok(match expr {
        Expr::JavaScript(_) => {
            return Err("an external JavaScript expression must own its execution boundary".into())
        }
        Expr::Native(_) => return Err("cannot compile a previously lowered expression".into()),
        Expr::Null => quote!(V::Null),
        Expr::Undefined => quote!(V::Undefined),
        Expr::Bool(value) => quote!(V::Bool(#value)),
        Expr::Number(n) => {
            let bits = n.to_bits();
            quote!(V::Number(f64::from_bits(#bits)))
        }
        Expr::String(value) => quote!(V::String(String::from(#value))),
        Expr::Name(ident) => quote!(frame.name(#ident,None)?),
        Expr::ResolvedId { name, target } => quote!(frame.resolved_name(#name,#target)?),
        Expr::ScopedName { name, root } => quote!(frame.name(#name,Some(#root))?),
        Expr::Record(fields) => {
            let pairs = fields
                .iter()
                .map(|(name, value)| {
                    let value = emit(value)?;
                    Ok(quote!((String::from(#name),#value)))
                })
                .collect::<Result<Vec<_>, String>>()?;
            quote!({let fields=std::collections::BTreeMap::from([#(#pairs),*]);frame.record(fields)})
        }
        Expr::Array(items) => {
            let items = items.iter().map(emit).collect::<Result<Vec<_>, _>>()?;
            quote!({
                let items = vec![#(#items),*];
                frame.array(items)
            })
        }
        Expr::Conditional(test, yes, no) => {
            let (test, yes, no) = (emit(test)?, emit(yes)?, emit(no)?);
            quote!(if c::truthy(&#test) { #yes } else { #no })
        }
        Expr::Member(base, field) => {
            if let Some((ident, root)) = name(base) {
                quote!(frame.named_member(#ident,#root,#field)?)
            } else {
                let base = emit(base)?;
                quote!({ let value=#base; frame.member(value,#field)? })
            }
        }
        Expr::Index(base, key) => {
            let (base, key) = (emit(base)?, emit(key)?);
            quote!({ let value=#base; let index=#key; frame.index(value,index)? })
        }
        Expr::Unary(op, value) if op == "typeof" && name(value).is_some() => {
            let (ident, root) = name(value).unwrap();
            quote!(frame.typeof_name(#ident,#root)?)
        }
        Expr::Unary(op, value) => {
            let value = emit(value)?;
            quote!({let v=#value;frame.unary(#op,v)?})
        }
        Expr::Binary(op, left, right) => {
            let (left, right) = (emit(left)?, emit(right)?);
            match op.as_str() {
                "&&" => quote!({ let a=#left; if c::truthy(&a) { #right } else { a } }),
                "||" => quote!({ let a=#left; if c::truthy(&a) { a } else { #right } }),
                "??" => {
                    quote!({ let a=#left; if matches!(a,V::Null|V::Undefined) { #right } else { a } })
                }
                _ => quote!({let a=#left; let b=#right; frame.binary(#op,a,b)?}),
            }
        }
        Expr::Call(callee, args) => {
            let (receiver, method) = if let Some((ident, root)) = name(callee) {
                (quote!(frame.local_receiver(#ident,#root)?), ident)
            } else if let Expr::Member(base, method) = callee.as_ref() {
                let receiver = if let Some((ident, root)) = name(base) {
                    quote!(frame.named_receiver(#ident,#root)?)
                } else {
                    let base = emit(base)?;
                    quote!(c::receiver(#base)?)
                };
                (receiver, method.as_str())
            } else {
                return Err("unsupported call target; expected a named method or signal".into());
            };
            let args = args.iter().map(emit).collect::<Result<Vec<_>, _>>()?;
            quote!({ let receiver=#receiver; let args=vec![#(#args),*]; frame.invoke(receiver,#method,args)? })
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowering_reuses_equivalent_evaluators_and_script_bodies() {
        let mut document = ferroui_qml_compiler::parse(
            r#"import QtQml
            QtObject {
                property int a: input + 1
                property int b: input + 1
                property int c: input - 1
                function f() { return input }
                function g() { return input }
            }"#,
        )
        .unwrap();
        let functions = lower(&mut document).unwrap();
        assert_eq!(functions.expressions.len(), 2);
        assert_eq!(functions.first_reads.len(), 2);
        assert_eq!(functions.scripts.len(), 1);
        let indices: Vec<_> = document
            .root
            .members
            .iter()
            .filter_map(|member| match &member.kind {
                MemberKind::Property {
                    value: Some(Binding::Expr(Expr::Native(index))),
                    ..
                } => Some(*index),
                _ => None,
            })
            .collect();
        assert_eq!(indices, [0, 0, 1]);
    }
}
