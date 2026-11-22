//! Resolve predicates that the selected Cargo artifact can prove.
use syn::{punctuated::Punctuated, Attribute, Expr, Lit, Meta, Token};

pub(super) fn enabled(attributes: &[Attribute], features: Option<&[String]>) -> Option<bool> {
    all(attributes.iter().filter_map(|attribute| {
        if attribute.path().is_ident("cfg") {
            Some(
                attribute
                    .parse_args::<Meta>()
                    .ok()
                    .and_then(|predicate| evaluate(&predicate, features, 0)),
            )
        } else if attribute.path().is_ident("cfg_attr") {
            // An active cfg_attr can change the module path or add other
            // attributes. Only ignore it when the artifact rules it out.
            let inactive = attribute
                .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
                .ok()
                .and_then(|items| evaluate(items.first()?, features, 0))
                == Some(false);
            Some(inactive.then_some(true))
        } else {
            None
        }
    }))
}

fn evaluate(predicate: &Meta, features: Option<&[String]>, depth: usize) -> Option<bool> {
    if depth >= 32 {
        return None;
    }
    match predicate {
        Meta::NameValue(value) if value.path.is_ident("feature") => {
            let Expr::Lit(value) = &value.value else {
                return None;
            };
            let Lit::Str(feature) = &value.lit else {
                return None;
            };
            let feature = feature.value();
            Some(features?.contains(&feature))
        }
        Meta::List(values) => {
            let arguments = values
                .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
                .ok()?;
            let predicates = || {
                arguments
                    .iter()
                    .map(|arg| evaluate(arg, features, depth + 1))
            };
            if values.path.is_ident("all") {
                all(predicates())
            } else if values.path.is_ident("any") {
                all(predicates().map(|value| value.map(|value| !value))).map(|value| !value)
            } else if values.path.is_ident("not") && arguments.len() == 1 {
                predicates().next().flatten().map(|value| !value)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn all(predicates: impl Iterator<Item = Option<bool>>) -> Option<bool> {
    let mut result = Some(true);
    for predicate in predicates {
        match predicate {
            Some(false) => return Some(false),
            None => result = None,
            Some(true) => {}
        }
    }
    result
}
