//! Resolve build-identified declarations against a current Rust file.
use super::{
    Builder, Scan, Site,
    declarations::{
        Accessor, Declaration, MarkupBody, read_declaration, read_property, read_registration,
    },
    tokens::{Cursor, text_of, tokens_of},
};
use crate::authored_symbols::{AuthoredDefinition, AuthoredSymbols, DeclarationKind};
use std::{
    ops::Range,
    path::{Component, Path},
};
use syn::spanned::Spanned;
use syn::{ImplItem, Item, Macro, Type};

impl Builder {
    pub(super) fn record_definition(
        &mut self,
        index: usize,
        site: &Site,
        kind: DeclarationKind,
        owner: &str,
        member: Option<&str>,
        registered_name: Option<&str>,
    ) {
        if !self.capture_definitions || site.expanded {
            return;
        }
        let file = &self.source.files[site.file];
        let module = self.source.modules.module_path(site.module);
        let Some(inline) = module.strip_prefix(&file.module) else {
            return;
        };
        let model = &self.model.types[index];
        self.authored_definitions.push(AuthoredDefinition {
            rust_type: model.rust_path.text.clone(),
            type_name: model.name.clone(),
            file: file.path.clone(),
            modules: inline
                .split("::")
                .filter(|part| !part.is_empty())
                .map(str::to_owned)
                .collect(),
            kind,
            owner: owner.to_owned(),
            member: member.map(str::to_owned),
            registered_name: registered_name.map(str::to_owned),
        });
    }
}

impl Scan {
    pub fn authored_symbols(&self, manifest: &Path) -> AuthoredSymbols {
        let package = manifest.parent();
        let mut definitions = self
            .authored_definitions
            .iter()
            .filter_map(|definition| {
                let file = definition.file.strip_prefix(package?).ok()?;
                if !file
                    .components()
                    .all(|part| matches!(part, Component::Normal(_)))
                {
                    return None;
                }
                Some(AuthoredDefinition {
                    file: file.to_owned(),
                    ..definition.clone()
                })
            })
            .collect::<Vec<_>>();
        definitions.sort();
        definitions.dedup();
        AuthoredSymbols {
            manifest: manifest.to_owned(),
            assembly: self.model.name.clone(),
            crate_name: self.model.crate_name.clone(),
            definitions,
        }
    }
}

impl AuthoredDefinition {
    /// Re-locate the same symbol after unrelated edits within its Rust file.
    pub fn range_in(&self, source: &str) -> Option<Range<usize>> {
        self.locate(source, false)
    }

    /// Spell the new public name without disturbing the Rust accessor or string delimiters.
    pub fn property_name_range(&self, source: &str, new_name: &str) -> Option<Range<usize>> {
        self.member.as_ref()?;
        if self.kind != DeclarationKind::Registered {
            syn::parse_str::<syn::Ident>(new_name).ok()?;
        }
        let range = self.locate(source, true)?;
        if self.kind != DeclarationKind::Registered {
            return Some(range);
        }
        let literal = source.get(range.clone())?;
        Some(range.start + literal.find('"')? + 1..range.start + literal.rfind('"')?)
    }

    fn locate(&self, source: &str, public_name: bool) -> Option<Range<usize>> {
        let body = rust_body(source);
        let offset = source.len() - body.len();
        let parsed = syn::parse_str::<syn::File>(body).ok()?;
        let mut items = parsed.items.as_slice();
        for expected in &self.modules {
            let mut modules = items.iter().filter_map(|item| match item {
                Item::Mod(module) if module.ident == expected => module.content.as_ref(),
                _ => None,
            });
            let (_, body) = modules.next()?;
            if modules.next().is_some() {
                return None;
            }
            items = body;
        }
        let mut matches = Vec::new();
        for item in items {
            match item {
                Item::Macro(item) => {
                    matches.extend(self.macro_ranges(&item.mac, None, public_name))
                }
                Item::Impl(item)
                    if self.kind == DeclarationKind::Registered && item.trait_.is_none() =>
                {
                    let Type::Path(ty) = &*item.self_ty else {
                        continue;
                    };
                    let Some(owner) = ty.path.segments.last() else {
                        continue;
                    };
                    let owner = owner.ident.to_string();
                    for member in &item.items {
                        if let ImplItem::Macro(member) = member {
                            matches.extend(self.macro_ranges(
                                &member.mac,
                                Some(&owner),
                                public_name,
                            ));
                        }
                    }
                }
                _ => {}
            }
        }
        let [found] = matches.as_slice() else {
            return None;
        };
        (found.start < found.end).then_some(offset + found.start..offset + found.end)
    }

    fn macro_ranges(
        &self,
        item: &Macro,
        impl_owner: Option<&str>,
        public_name: bool,
    ) -> Vec<Range<usize>> {
        let Some(macro_name) = item.path.segments.last().map(|part| part.ident.to_string()) else {
            return Vec::new();
        };
        let expected = match self.kind {
            DeclarationKind::Class => "ferro_class",
            DeclarationKind::Static => "ferro_static_type",
            DeclarationKind::MarkupType => "ferro_markup_type",
            DeclarationKind::MarkupEnum => "ferro_markup_enum",
            DeclarationKind::ClassInfo => "ferro_class_info",
            DeclarationKind::Registered if macro_name == "ferro_property" => {
                let tokens = tokens_of(item.tokens.clone());
                let mut cursor = Cursor::new(&tokens, item.span().start().line);
                return read_property(&mut cursor)
                    .ok()
                    .filter(|(owner, _)| owner.as_deref().or(impl_owner) == Some(&self.owner))
                    .and_then(|(_, property)| self.accessor_range(property, public_name))
                    .into_iter()
                    .collect();
            }
            DeclarationKind::Registered => "ferro_properties",
        };
        if macro_name != expected {
            return Vec::new();
        }
        let tokens = tokens_of(item.tokens.clone());
        let Ok(declaration) = read_declaration(expected, &tokens, item.span().start().line) else {
            return Vec::new();
        };
        match declaration {
            Declaration::Class { name, .. } | Declaration::StaticType { name }
                if name == self.owner && self.member.is_none() =>
            {
                tokens
                    .first()
                    .map(|token| token.span().byte_range())
                    .into_iter()
                    .collect()
            }
            Declaration::MarkupType {
                type_, name, body, ..
            } if text_of(&type_) == self.owner => {
                if self.member.is_some() {
                    self.property_ranges(body)
                } else if name.as_deref().unwrap_or(&self.owner) == self.type_name {
                    type_
                        .first()
                        .zip(type_.last())
                        .map(|(first, last)| {
                            first.span().byte_range().start..last.span().byte_range().end
                        })
                        .into_iter()
                        .collect()
                } else {
                    Vec::new()
                }
            }
            Declaration::MarkupEnum {
                name, flags, body, ..
            } if name == self.owner => {
                if self.member.is_some() {
                    self.property_ranges(body)
                } else {
                    tokens
                        .get(usize::from(flags))
                        .map(|token| token.span().byte_range())
                        .into_iter()
                        .collect()
                }
            }
            Declaration::ClassInfo {
                name,
                markup: Some(body),
                ..
            } if name == self.owner => self.property_ranges(body),
            Declaration::Properties {
                owner, accessors, ..
            } if owner == self.owner => accessors
                .into_iter()
                .filter_map(|accessor| self.accessor_range(accessor, public_name))
                .collect(),
            _ => Vec::new(),
        }
    }

    fn accessor_range(&self, property: Accessor, public_name: bool) -> Option<Range<usize>> {
        if self.member.as_deref() != Some(&property.name) {
            return None;
        }
        if self.registered_name.is_none() {
            return (!public_name).then_some(property.name_range);
        }
        let registration = read_registration(&property.body);
        if registration.name != self.registered_name {
            return None;
        }
        if public_name {
            registration.name_range
        } else {
            Some(property.name_range)
        }
    }

    fn property_ranges(&self, body: MarkupBody) -> Vec<Range<usize>> {
        body.properties
            .into_iter()
            .filter(|property| Some(property.name.as_str()) == self.member.as_deref())
            .filter_map(|property| property.name_range)
            .collect()
    }
}

fn rust_body(source: &str) -> &str {
    let body = source.strip_prefix('\u{feff}').unwrap_or(source);
    match body.strip_prefix("#!") {
        Some(after) if !after.trim_start().starts_with('[') => {
            body.find('\n').map_or("", |end| &body[end..])
        }
        _ => body,
    }
}
