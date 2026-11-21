//! Locate the public spelling of a registered type and its separate Rust declaration.
use super::{module_items, rust_body, AuthoredDefinition, Declaration, DeclarationKind, Scan};
use crate::{
    authored_symbols::{RustTypeDeclaration, TypeNameEdit},
    scanner::{
        declarations::read_declaration,
        modules::ItemKind,
        tokens::{text_of, tokens_of},
    },
};
use std::{
    collections::{btree_map::Entry, BTreeMap},
    ops::Range,
    path::Path,
};
use syn::{spanned::Spanned, Item};

impl Scan {
    pub(super) fn type_source_modules(&self) -> BTreeMap<&str, Option<&Path>> {
        let mut modules = BTreeMap::new();
        for file in &self.files {
            match modules.entry(file.module.as_str()) {
                Entry::Vacant(entry) => {
                    entry.insert(Some(file.path.as_path()));
                }
                Entry::Occupied(mut entry) => {
                    entry.insert(None);
                }
            }
        }
        modules
    }

    pub(super) fn type_declaration(
        &self,
        package: &Path,
        path: &str,
        files: &BTreeMap<&str, Option<&Path>>,
    ) -> Option<RustTypeDeclaration> {
        let (module, name) = path.strip_prefix("::")?.rsplit_once("::")?;
        let index = self.modules.find_module(module)?;
        let item = self.modules.modules[index].items.get(name)?;
        if item.kind != ItemKind::Type {
            return None;
        }
        let mut containing = module;
        let file = loop {
            if let Some(file) = files.get(containing) {
                break (*file)?;
            }
            containing = containing.rsplit_once("::")?.0;
        };
        Some(RustTypeDeclaration {
            file: file.strip_prefix(package).ok()?.into(),
            modules: module
                .strip_prefix(containing)?
                .split("::")
                .filter(|part| !part.is_empty())
                .map(str::to_owned)
                .collect(),
            name: name.into(),
        })
    }
}

impl RustTypeDeclaration {
    pub fn range_in(&self, source: &str) -> Option<Range<usize>> {
        let body = rust_body(source);
        let offset = source.len() - body.len();
        let parsed = syn::parse_str::<syn::File>(body).ok()?;
        let mut matches = module_items(&parsed, &self.modules)?
            .iter()
            .filter_map(|item| {
                let name = match item {
                    Item::Struct(item) => &item.ident,
                    Item::Enum(item) => &item.ident,
                    Item::Type(item) => &item.ident,
                    Item::Union(item) => &item.ident,
                    Item::Trait(item) => &item.ident,
                    Item::TraitAlias(item) => &item.ident,
                    _ => return None,
                };
                (name == &self.name).then(|| name.span().byte_range())
            });
        let span = matches.next()?;
        matches
            .next()
            .is_none()
            .then_some(offset + span.start..offset + span.end)
    }
}

impl AuthoredDefinition {
    /// Recheck the lexical registration before attempting a source rename.
    pub fn type_name_edit(&self, source: &str, new_name: &str) -> Option<TypeNameEdit> {
        if self.member.is_some() {
            return None;
        }
        let expected = match self.kind {
            DeclarationKind::Class => "ferro_class",
            DeclarationKind::Static => "ferro_static_type",
            DeclarationKind::MarkupType => "ferro_markup_type",
            DeclarationKind::MarkupEnum => "ferro_markup_enum",
            _ => return None,
        };
        let body = rust_body(source);
        let offset = source.len() - body.len();
        let parsed = syn::parse_str::<syn::File>(body).ok()?;
        let mut matches = module_items(&parsed, &self.modules)?
            .iter()
            .filter_map(|item| self.type_macro_name(item, expected));
        let (span, alias) = matches.next()?;
        if matches.next().is_some() {
            return None;
        }
        let range = offset + span.start..offset + span.end;
        if alias {
            let literal = source.get(range.clone())?;
            return Some(TypeNameEdit::Alias(
                range.start + literal.find('"')? + 1..range.start + literal.rfind('"')?,
            ));
        }
        syn::parse_str::<syn::Ident>(new_name).ok()?;
        (source
            .get(range.clone())?
            .strip_prefix("r#")
            .unwrap_or(&source[range.clone()])
            == self.type_name)
            .then_some(TypeNameEdit::Symbol(range))
    }

    fn type_macro_name(&self, item: &Item, expected: &str) -> Option<(Range<usize>, bool)> {
        let Item::Macro(item) = item else { return None };
        if item.mac.path.segments.last()?.ident != expected {
            return None;
        }
        let tokens = tokens_of(item.mac.tokens.clone());
        match read_declaration(expected, &tokens, item.mac.span().start().line).ok()? {
            Declaration::Class { name, .. } | Declaration::StaticType { name }
                if name == self.owner =>
            {
                Some((tokens.first()?.span().byte_range(), false))
            }
            Declaration::MarkupEnum { name, flags, .. } if name == self.owner => {
                Some((tokens.get(usize::from(flags))?.span().byte_range(), false))
            }
            Declaration::MarkupType { type_, name, .. }
                if text_of(&type_) == self.owner
                    && name.as_deref().unwrap_or(&self.owner) == self.type_name =>
            {
                if name.is_some() {
                    Some((tokens.iter().rev().nth(1)?.span().byte_range(), true))
                } else {
                    Some((type_.last()?.span().byte_range(), false))
                }
            }
            _ => None,
        }
    }
}
