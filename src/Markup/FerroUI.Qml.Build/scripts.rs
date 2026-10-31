//! Emit Rust control flow for QML methods, handlers and script bindings.
//! Block completion and nonlocal return are explicit across lexical scopes.
use super::native::{emit, name};
use ferroui_qml_compiler::{Expr, Stmt};
use proc_macro2::TokenStream;
use quote::quote;

pub(super) fn body(statements: &[Stmt]) -> Result<TokenStream, String> {
    let mut code = TokenStream::new();
    for statement in statements {
        code.extend(step(statement)?);
        if matches!(statement, Stmt::Return(_) | Stmt::Break | Stmt::Continue) {
            return Ok(
                quote!({ #[allow(unused_assignments,unused_variables,unused_mut)] let mut last=V::Undefined; #code }),
            );
        }
    }
    Ok(
        quote!({ #[allow(unused_assignments,unused_mut)] let mut last=V::Undefined; #code Ok(c::Flow::Next(last)) }),
    )
}

fn scoped(statements: &[Stmt]) -> Result<TokenStream, String> {
    let names = statements.iter().filter_map(|s| match s {
        Stmt::Let { name, .. } => Some(name),
        _ => None,
    });
    let body = body(statements)?;
    Ok(quote!(frame.block(&[#(#names),*],|frame| {let _=&frame; #body})?))
}

fn completion(flow: TokenStream) -> TokenStream {
    quote!(match #flow { c::Flow::Next(value)=>last=value, other=>return Ok(other) })
}

fn target(expr: &Expr) -> Result<TokenStream, String> {
    Ok(match expr {
        Expr::Name(ident) => quote!(frame.target_name(#ident,None)?),
        Expr::ScopedName { name, root } => quote!(frame.target_name(#name,Some(#root))?),
        Expr::Member(base, field) => {
            if let Some((base, root)) = name(base) {
                quote!(frame.target_named_member(#base,#root,#field)?)
            } else {
                let base = emit(base)?;
                quote!({let object=#base;frame.target_member(object,#field)?})
            }
        }
        Expr::Index(base, key) => {
            let (base, key) = (emit(base)?, emit(key)?);
            quote!({let object=#base;let index=#key;frame.target_index(object,index)?})
        }
        _ => return Err("invalid assignment target".into()),
    })
}

fn value(expr: Option<&Expr>) -> Result<TokenStream, String> {
    expr.map_or_else(|| Ok(quote!(V::Undefined)), emit)
}

fn step(stmt: &Stmt) -> Result<TokenStream, String> {
    Ok(match stmt {
        Stmt::JavaScript(_) => {
            return Err("an external JavaScript script must own its execution boundary".into())
        }
        Stmt::Native(_) => return Err("cannot lower an already compiled script".into()),
        Stmt::Expr(expr) => {
            let expr = emit(expr)?;
            quote!(last=#expr;)
        }
        Stmt::Return(expr) => {
            let expr = value(expr.as_ref())?;
            quote!(return Ok(c::Flow::Return(#expr));)
        }
        Stmt::Let { name, value: expr } => {
            let expr = value(expr.as_ref())?;
            quote!({let value=#expr;frame.declare(#name,value);})
        }
        Stmt::Assign {
            target: expr,
            op,
            value,
        } => {
            let (place, rhs) = (target(expr)?, emit(value)?);
            quote!({let place=#place;last=frame.assign(place,#op,|frame|{let _=&frame;Ok(#rhs)})?;})
        }
        Stmt::Block(stmts) => completion(scoped(stmts)?),
        Stmt::If {
            test,
            then_body,
            else_body,
        } => {
            let (test, yes, no) = (emit(test)?, scoped(then_body)?, scoped(else_body)?);
            completion(quote!(if c::truthy(&#test) {#yes} else {#no}))
        }
        Stmt::Break => quote!(return Ok(c::Flow::Break);),
        Stmt::Continue => quote!(return Ok(c::Flow::Continue);),
        Stmt::While {
            test,
            body,
            post_test,
        } => {
            let (test, body) = (emit(test)?, scoped(body)?);
            let condition = quote!(if !c::truthy(&#test) {break;});
            let (before, after) = if *post_test {
                (TokenStream::new(), condition)
            } else {
                (condition, TokenStream::new())
            };
            quote!({loop {#before frame.iteration()?;
                match #body {c::Flow::Next(v)=>last=v, c::Flow::Break=>break, c::Flow::Continue=>(), other=>return Ok(other)}
                #after
            }})
        }
        Stmt::For {
            init,
            test,
            update,
            body,
        } => {
            let names = init.as_deref().into_iter().filter_map(|s| {
                if let Stmt::Let { name, .. } = s {
                    Some(name)
                } else {
                    None
                }
            });
            let init = init.as_deref().map(step).transpose()?;
            let update = update.as_deref().map(step).transpose()?;
            let stop = test
                .as_ref()
                .map(emit)
                .transpose()?
                .map(|t| quote!(if !c::truthy(&#t) {break;}));
            let body = scoped(body)?;
            completion(quote!(frame.block(&[#(#names),*],|frame| {
                #[allow(unused_assignments,unused_mut)] let mut last=V::Undefined;
                #init
                let mut loop_value=V::Undefined;
                loop { #stop frame.iteration()?;
                   match #body {c::Flow::Next(v)=>loop_value=v, c::Flow::Break=>break, c::Flow::Continue=>(), other=>return Ok(other)}
                   #update
                }
                let _=last;
                Ok(c::Flow::Next(loop_value))
            })?))
        }
    })
}
