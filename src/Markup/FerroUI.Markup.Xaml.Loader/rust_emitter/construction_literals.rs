//! Scalar operands of a resolved Rust value constructor. Preserve the compiler's concrete
//! functions, casts and enum variants. Instances of the same call share an adapter while
//! each use supplies its own literal bytes.
use super::CallInput;
use quote::ToTokens;
use syn::{Expr, Lit};

pub(in super::super) fn closed_value(source: &str, inputs: &mut Vec<CallInput>) -> Option<String> {
    let expr = syn::parse_str::<Expr>(source).ok()?;
    let before = inputs.len();
    let result = lower(&expr, inputs);
    if result.is_none() {
        inputs.truncate(before);
    }
    result
}

fn lower(expr: &Expr, inputs: &mut Vec<CallInput>) -> Option<String> {
    match expr {
        Expr::MethodCall(call)
            if call.method == "base_uri"
                && call.args.is_empty()
                && call.turbofish.is_none()
                && matches!(&*call.receiver, Expr::Path(p) if p.qself.is_none() && p.path.is_ident("context")) =>
        {
            inputs.push(CallInput::DocumentBaseUri);
            Some("args.context()?.base_uri()".into())
        }
        Expr::Path(path) if path.qself.is_some() => {
            let receiver = path.qself.as_ref()?;
            let segments = &path.path.segments;
            if receiver.as_token.is_none() || receiver.position + 1 != segments.len()
                || path.path.leading_colon.is_none() || segments.iter().any(|s| !s.arguments.is_none()) {
                return None;
            }
            let names = segments.iter().map(|s| s.ident.to_string()).collect::<Vec<_>>().join("::");
            let (trait_path, field) = match names.as_str() {
                "ferroui_base::metadata::MarkupTyped::MARKUP" => ("::ferroui_base::metadata::MarkupTyped", "MARKUP"),
                "ferroui_base::StaticType::TYPE" => ("::ferroui_base::StaticType", "TYPE"),
                _ => return None,
            };
            Some(format!("<{} as {trait_path}>::{field}", cast_type(&receiver.ty)?))
        }
        Expr::Path(path)
            if path.qself.is_none()
                && path
                    .path
                    .segments
                    .iter()
                    .all(|segment| segment.arguments.is_none())
                && (path.path.leading_colon.is_some()
                    || path.path.segments.first().is_some_and(|segment| {
                        segment.ident == "crate" || segment.ident == "rt"
                    })) =>
        {
            // Cargo's host changes its own absolute crate prefix to `crate::` in
            // generated code. Keep paths in the emitter's canonical spelling.
            let prefix = if path.path.leading_colon.is_some() {
                "::"
            } else {
                ""
            };
            let names = path
                .path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect::<Vec<_>>();
            Some(format!("{prefix}{}", names.join("::")))
        }
        Expr::Call(call) => {
            let function = lower(&call.func, inputs)?;
            let arguments = call
                .args
                .iter()
                .map(|e| lower(e, inputs))
                .collect::<Option<Vec<_>>>()?;
            Some(format!("{function}({})", arguments.join(", ")))
        }
        Expr::Reference(reference) if reference.mutability.is_none() => {
            Some(format!("&({})", lower(&reference.expr, inputs)?))
        }
        Expr::Cast(cast) => {
            let ty = cast_type(&cast.ty)?;
            Some(format!("{} as {ty}", lower(&cast.expr, inputs)?))
        }
        Expr::Paren(paren) => Some(format!("({})", lower(&paren.expr, inputs)?)),
        Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Neg(_)) => {
            // Preserve the signed Rust expression: the largest negative integer can have
            // an unsigned magnitude which is not representable as its positive counterpart.
            match unary.expr.as_ref() {
                Expr::Lit(lit) if matches!(lit.lit, Lit::Float(_)) => {
                    Some(format!("-({})", lower(&unary.expr, inputs)?))
                }
                Expr::Lit(lit) if matches!(lit.lit, Lit::Int(_)) => {
                    Some(expr.to_token_stream().to_string())
                }
                _ => None,
            }
        }
        Expr::Lit(literal) => {
            let (input, read) = match &literal.lit {
                Lit::Str(s) => (
                    CallInput::Text(super::super::rust_string_literal(&s.value())),
                    "text",
                ),
                Lit::Bool(b) => (CallInput::Bool(b.value), "boolean"),
                Lit::Float(n) if n.suffix() == "f64" => (
                    CallInput::Double(n.base10_parse::<f64>().ok()?.to_bits()),
                    "double",
                ),
                Lit::Int(n) if n.suffix() == "i32" => {
                    (CallInput::Int(n.base10_parse().ok()?), "int")
                }
                Lit::Int(n) if n.suffix() == "u32" => {
                    (CallInput::Uint(n.base10_parse().ok()?), "uint")
                }
                Lit::Int(_) | Lit::Float(_) | Lit::Char(_) => {
                    return Some(literal.to_token_stream().to_string())
                }
                _ => return None,
            };
            inputs.push(input);
            Some(format!("args.{read}()?"))
        }
        _ => None,
    }
}

/// Spell a resolved cast target as a Rust type. In particular, a brush literal can be
/// cast to an Rc of an exported trait. Preserve that coercion and its crate-qualified paths.
fn cast_type(ty: &syn::Type) -> Option<String> {
    fn path(p: &syn::Path) -> Option<String> {
        let mut text = if p.leading_colon.is_some() {
            "::".to_string()
        } else {
            String::new()
        };
        for (i, segment) in p.segments.iter().enumerate() {
            if i > 0 {
                text.push_str("::");
            }
            text.push_str(&segment.ident.to_string());
            match &segment.arguments {
                syn::PathArguments::None => {}
                syn::PathArguments::AngleBracketed(generic) => {
                    let args = generic
                        .args
                        .iter()
                        .map(|arg| match arg {
                            syn::GenericArgument::Type(ty) => cast_type(ty),
                            _ => None,
                        })
                        .collect::<Option<Vec<_>>>()?;
                    text.push('<');
                    text.push_str(&args.join(", "));
                    text.push('>');
                }
                _ => return None,
            }
        }
        Some(text)
    }
    match ty {
        syn::Type::Path(ty) if ty.qself.is_none() => path(&ty.path),
        syn::Type::TraitObject(ty) if ty.dyn_token.is_some() && ty.bounds.len() == 1 => {
            let syn::TypeParamBound::Trait(bound) = ty.bounds.first()? else {
                return None;
            };
            if bound.lifetimes.is_some() || !matches!(bound.modifier, syn::TraitBoundModifier::None)
            {
                return None;
            }
            Some(format!("dyn {}", path(&bound.path)?))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_base_uri_is_read_at_execution() {
        let mut inputs = Vec::new();
        let expr = r#"::theme::Font::new(context.base_uri(), "::font")"#;
        assert_eq!(closed_value(expr, &mut inputs).as_deref(),
            Some("::theme::Font::new(args.context()?.base_uri(), args.text()?)"));
        assert!(matches!(inputs.as_slice(), [CallInput::DocumentBaseUri, CallInput::Text(_)]));
        inputs.clear();
        assert!(closed_value("another_context.base_uri()", &mut inputs).is_none());
        assert!(closed_value("context.base_uri(4)", &mut inputs).is_none());
        assert_eq!(closed_value(r#""context.base_uri()""#, &mut inputs).as_deref(), Some("args.text()?"));
        assert!(matches!(inputs.as_slice(), [CallInput::Text(_)]));
    }

    #[test]
    fn resolved_metadata_operands_retain_the_type_and_trait() {
        let mut inputs = Vec::new();
        let image = "::core::option::Option::Some(rt::markup_handle(<dyn ::ferroui_base::media::IImage as ::ferroui_base::metadata::MarkupTyped>::MARKUP))";
        assert_eq!(closed_value(image, &mut inputs).as_deref(), Some(image));
        let class = "<crate::views::View as ::ferroui_base::StaticType>::TYPE";
        assert_eq!(closed_value(class, &mut inputs).as_deref(), Some(class));
        assert!(closed_value("<crate::views::View as ::other::StaticType>::TYPE", &mut inputs).is_none());
        assert!(inputs.is_empty());
    }

    #[test]
    fn retains_exact_trait_object_casts_of_brush_literals() {
        let mut inputs = Vec::new();
        let text = closed_value(
            "::theme::Brush::new(0xff00ff00_u32) as ::std::rc::Rc<dyn ::theme::IBrush>",
            &mut inputs,
        );
        assert_eq!(
            text.as_deref(),
            Some("::theme::Brush::new(args.uint()?) as ::std::rc::Rc<dyn ::theme::IBrush>")
        );
        assert!(matches!(inputs.as_slice(), [CallInput::Uint(0xff00ff00)]));
    }

    #[test]
    fn inputs_follow_nested_constructor_order_and_paths_keep_the_crate_prefix() {
        let mut operands = Vec::new();
        let text = closed_value(
            "::theme::Converter::new(::theme::Side::Left, -0.5f64, 0xffu32, \"left\")",
            &mut operands,
        );
        assert_eq!(text.as_deref(), Some("::theme::Converter::new(::theme::Side::Left, -(args.double()?), args.uint()?, args.text()?)"));
        assert!(
            matches!(operands.as_slice(), [CallInput::Double(n), CallInput::Uint(255), CallInput::Text(s)] if *n == 0.5_f64.to_bits() && s == "\"left\"")
        );
    }

    #[test]
    fn partial_expression_referring_to_a_local_leaves_no_operands() {
        let mut operands = vec![CallInput::Bool(true)];
        assert!(closed_value(
            "::theme::Converter::new(2i32, local.clone())",
            &mut operands
        )
        .is_none());
        assert!(matches!(operands.as_slice(), [CallInput::Bool(true)]));
    }
}
