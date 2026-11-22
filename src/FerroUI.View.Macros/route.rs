//! `#[derive(Route)]`: a typed route, whose pattern and fields are checked against each other
//! when the crate is compiled.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Field, Fields, Ident, LitStr};

/// A segment of a pattern, as `router.rs` of `ferroui-view` reads it.
#[derive(Debug, PartialEq)]
enum Segment {
    Literal(String),
    Param(String),
    Rest,
}

fn parse_pattern(pattern: &str) -> Result<Vec<Segment>, String> {
    let mut segments = Vec::new();
    for part in pattern.split('/').filter(|part| !part.is_empty()) {
        if matches!(segments.last(), Some(Segment::Rest)) {
            return Err(format!("the pattern {pattern:?} has segments after `*`, which matches the rest of a path"));
        }
        segments.push(match part {
            "*" => Segment::Rest,
            _ => match part.strip_prefix(':') {
                Some("") => return Err(format!("the pattern {pattern:?} has a `:` with no parameter name")),
                Some(name) => {
                    if segments.contains(&Segment::Param(name.to_string())) {
                        return Err(format!("the pattern {pattern:?} names the parameter `{name}` twice"));
                    }
                    Segment::Param(name.to_string())
                }
                None => Segment::Literal(part.to_string()),
            },
        });
    }
    Ok(segments)
}

fn is_rest(field: &Field) -> syn::Result<bool> {
    let mut rest = false;
    for attribute in field.attrs.iter().filter(|attribute| attribute.path().is_ident("route")) {
        let word: Ident = attribute.parse_args().map_err(|_| {
            syn::Error::new_spanned(attribute, "a field of a route takes `#[route(rest)]`, for the `*` of its pattern")
        })?;
        if word != "rest" {
            return Err(syn::Error::new_spanned(word, "a field of a route takes `#[route(rest)]`"));
        }
        rest = true;
    }
    Ok(rest)
}

/// The expansion of `#[derive(Route)]`.
pub(crate) fn expand(input: &DeriveInput) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new_spanned(name, "`Route` is derived for a struct with named fields, or a unit struct"));
    };
    let fields: Vec<&Field> = match &data.fields {
        Fields::Named(fields) => fields.named.iter().collect(),
        Fields::Unit => Vec::new(),
        Fields::Unnamed(_) => {
            return Err(syn::Error::new_spanned(name, "a route names its parameters: use named fields"));
        }
    };
    let mut patterns = input.attrs.iter().filter(|attribute| attribute.path().is_ident("route"));
    let Some(attribute) = patterns.next() else {
        return Err(syn::Error::new_spanned(name, "a route needs its pattern: `#[route(\"/people/:id\")]`"));
    };
    if let Some(second) = patterns.next() {
        return Err(syn::Error::new_spanned(second, "a route has one pattern"));
    }
    let literal: LitStr = attribute
        .parse_args()
        .map_err(|_| syn::Error::new_spanned(attribute, "the pattern is a string: `#[route(\"/people/:id\")]`"))?;
    let pattern = literal.value();
    let segments = parse_pattern(&pattern).map_err(|message| syn::Error::new(literal.span(), message))?;

    // Every parameter of the pattern has its field, and every field its parameter.
    let mut rest_field = None;
    for field in &fields {
        if is_rest(field)? {
            if rest_field.replace(*field).is_some() {
                return Err(syn::Error::new_spanned(field, "a route has one `#[route(rest)]` field"));
            }
        }
    }
    let field_named = |wanted: &str| {
        fields.iter().copied().find(|field| {
            field.ident.as_ref().is_some_and(|ident| syn::ext::IdentExt::unraw(ident) == wanted)
        })
    };
    let mut to_path = Vec::new();
    for segment in &segments {
        to_path.push(match segment {
            Segment::Literal(text) => {
                let text = format!("/{text}");
                quote! { path.push_str(#text); }
            }
            Segment::Param(parameter) => {
                let Some(field) = field_named(parameter) else {
                    let message = format!(
                        "the pattern {pattern:?} has the parameter `{parameter}`, and `{name}` has no field of that \
                         name; add a field `{parameter}`, or take `:{parameter}` out of the pattern"
                    );
                    return Err(syn::Error::new(literal.span(), message));
                };
                let ident = &field.ident;
                quote! { ::ferroui_view::router::__push_param(&mut path, &self.#ident); }
            }
            Segment::Rest => {
                let Some(field) = rest_field else {
                    let message = format!(
                        "the pattern {pattern:?} ends in `*`; mark the field that holds the rest of the path with \
                         `#[route(rest)]`"
                    );
                    return Err(syn::Error::new(literal.span(), message));
                };
                let ident = &field.ident;
                quote! { ::ferroui_view::router::__push_rest(&mut path, &self.#ident); }
            }
        });
    }
    if let Some(field) = rest_field {
        if !segments.contains(&Segment::Rest) {
            let message = format!("`#[route(rest)]` needs a `*` at the end of the pattern {pattern:?}");
            return Err(syn::Error::new_spanned(field, message));
        }
    }
    let mut inits = Vec::new();
    for field in &fields {
        let ident = &field.ident;
        if rest_field.is_some_and(|rest| std::ptr::eq(rest, *field)) {
            inits.push(quote! { #ident: ::ferroui_view::router::__rest(matched)? });
            continue;
        }
        let parameter = field.ident.as_ref().map(|ident| syn::ext::IdentExt::unraw(ident).to_string());
        let parameter = parameter.unwrap_or_default();
        if !segments.contains(&Segment::Param(parameter.clone())) {
            let message = format!(
                "the field `{parameter}` is not a parameter of the pattern {pattern:?}; add `:{parameter}` to the \
                 pattern, or take the field out"
            );
            return Err(syn::Error::new_spanned(field, message));
        }
        inits.push(quote! { #ident: ::ferroui_view::router::__param(matched, #parameter)? });
    }
    let build = match &data.fields {
        Fields::Unit => quote! { Self },
        _ => quote! { Self { #(#inits),* } },
    };
    let matched = if inits.is_empty() { format_ident!("_matched") } else { format_ident!("matched") };
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();
    Ok(quote! {
        impl #impl_generics ::ferroui_view::router::TypedRoute for #name #type_generics #where_clause {
            const PATTERN: &'static str = #literal;

            fn to_path(&self) -> ::std::string::String {
                let mut path = ::std::string::String::new();
                #(#to_path)*
                if path.is_empty() {
                    path.push('/');
                }
                path
            }

            fn from_match(
                #matched: &::ferroui_view::router::RouteMatch,
            ) -> ::std::result::Result<Self, ::ferroui_view::router::RouteParamError> {
                ::std::result::Result::Ok(#build)
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patterns_are_read_like_the_router_reads_them() {
        assert_eq!(parse_pattern("/").unwrap(), []);
        assert_eq!(
            parse_pattern("/people/:id/*").unwrap(),
            [Segment::Literal("people".to_string()), Segment::Param("id".to_string()), Segment::Rest]
        );
        assert!(parse_pattern("/*/a").is_err());
        assert!(parse_pattern("/:id/:id").is_err());
        assert!(parse_pattern("/:").is_err());
    }
}
