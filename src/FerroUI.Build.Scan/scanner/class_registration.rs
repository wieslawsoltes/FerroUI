//! Editable source locations for a class registration, using the build scanner's grammar.

use proc_macro2::{Delimiter, Group};
use std::ops::Range;
use syn::spanned::Spanned;

use super::declarations::{Declaration, read_declaration};
use super::tokens::{group_of, ident_of, is_punct, tokens_of};

/// Insertion anchors for a declaration list. All offsets are UTF-8 bytes.
#[derive(Clone, Debug)]
pub struct DeclarationList {
    pub close: usize,
    pub first: Option<usize>,
    pub needs_comma_after: Option<usize>,
}

impl DeclarationList {
    fn of(group: &Group) -> Self {
        let tokens = tokens_of(group.stream());
        Self {
            close: group.span_close().byte_range().start,
            first: tokens.first().map(|token| token.span().byte_range().start),
            needs_comma_after: tokens
                .last()
                .filter(|token| !is_punct(token, ','))
                .map(|token| token.span().byte_range().end),
        }
    }
}

#[derive(Clone, Debug)]
pub struct RegisteredMethod {
    pub name: String,
    pub name_range: Range<usize>,
}

#[derive(Clone, Debug)]
pub struct ClassRegistration {
    pub declaration: DeclarationList,
    pub markup: Option<DeclarationList>,
    pub methods: Option<DeclarationList>,
    pub namespace: Option<String>,
    pub names: Vec<RegisteredMethod>,
}

/// Find one directly authored registration within the file's inline module path.
/// Expanded macros and conditional or ambiguous registrations have no edit anchor.
pub fn class_registration(
    source: &str,
    inline_module: &[&str],
    class: &str,
) -> Result<Option<ClassRegistration>, String> {
    let file = syn::parse_file(source)
        .map_err(|error| format!("Repair the Rust source first: {error}"))?;
    let mut items = file.items.as_slice();
    for segment in inline_module {
        let mut found = items.iter().filter_map(|item| match item {
            syn::Item::Mod(module) if module.ident == *segment => Some(module),
            _ => None,
        });
        let Some(module) = found.next() else {
            return Ok(None);
        };
        if found.next().is_some() || conditional(&module.attrs) {
            return Ok(None);
        }
        let Some((_, body)) = &module.content else {
            return Ok(None);
        };
        items = body;
    }
    let mut registrations = items.iter().filter_map(|item| {
        let syn::Item::Macro(item) = item else {
            return None;
        };
        if item.mac.path.segments.last()?.ident != "ferro_class_info" {
            return None;
        }
        let tokens = tokens_of(item.mac.tokens.clone());
        (tokens.first().and_then(ident_of).as_deref() == Some(class)).then_some((item, tokens))
    });
    let Some((item, tokens)) = registrations.next() else {
        return Ok(None);
    };
    if registrations.next().is_some() || conditional(&item.attrs) {
        return Ok(None);
    }
    let Declaration::ClassInfo { markup, .. } =
        read_declaration("ferro_class_info", &tokens, item.span().start().line)
            .map_err(|error| format!("{} at Rust line {}", error.message, error.line))?
    else {
        return Ok(None);
    };
    let Some(body) = tokens
        .get(1)
        .and_then(|item| group_of(item, Delimiter::Brace))
    else {
        return Ok(None);
    };
    let markup_group = named_group(body, "markup", Delimiter::Brace)?;
    let methods = markup_group
        .as_ref()
        .map(|group| named_group(group, "methods", Delimiter::Bracket))
        .transpose()?
        .flatten();
    let mut names = Vec::new();
    if let (Some(markup), Some(methods)) = (&markup, &methods) {
        let tokens = tokens_of(methods.stream());
        let stated = tokens
            .windows(3)
            .filter(|tokens| {
                ident_of(&tokens[0]).as_deref() == Some("fn")
                    && group_of(&tokens[2], Delimiter::Parenthesis).is_some()
            })
            .filter_map(|tokens| {
                Some(RegisteredMethod {
                    name: ident_of(&tokens[1])?,
                    name_range: tokens[1].span().byte_range(),
                })
            })
            .collect::<Vec<_>>();
        if !markup
            .methods
            .iter()
            .map(|member| &member.name)
            .eq(stated.iter().map(|method| &method.name))
        {
            return Err("The method list has no unambiguous source layout".into());
        }
        names = stated;
    }
    Ok(Some(ClassRegistration {
        declaration: DeclarationList::of(body),
        markup: markup_group.as_ref().map(DeclarationList::of),
        methods: methods.as_ref().map(DeclarationList::of),
        namespace: markup.and_then(|body| body.namespace),
        names,
    }))
}

pub fn rust_identifier(name: &str) -> bool {
    syn::parse_str::<syn::Ident>(name).is_ok()
}

fn conditional(attrs: &[syn::Attribute]) -> bool {
    attrs
        .iter()
        .any(|attr| attr.path().is_ident("cfg") || attr.path().is_ident("cfg_attr"))
}

fn named_group(group: &Group, name: &str, delim: Delimiter) -> Result<Option<Group>, String> {
    let tokens = tokens_of(group.stream());
    let mut parts = tokens.windows(3).filter_map(|tokens| {
        (ident_of(&tokens[0]).as_deref() == Some(name) && is_punct(&tokens[1], ':'))
            .then(|| group_of(&tokens[2], delim))
            .flatten()
    });
    let first = parts.next().cloned();
    if parts.next().is_some() {
        return Err(format!(
            "The class registration has more than one {name} block"
        ));
    }
    Ok(first)
}
