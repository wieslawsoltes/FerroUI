//! String-key resources and the literal values supported by the isolated preview.
use super::RsxStyle;
use crate::RsxProjection;
use crate::animation::{ValueShape, ValueSource, syntax as rust};
use ferroui_designer_core::SourceSpan;
use std::collections::{BTreeMap, BTreeSet};
use syn::{Expr, Lit};

#[derive(Debug)]
pub struct RsxStyleResource {
    pub declaration: SourceSpan,
    pub source: SourceSpan,
    pub key: String,
    pub(super) color: Option<ValueSource>,
}

impl RsxStyleResource {
    pub fn color(&self) -> Option<String> {
        self.color.as_ref().map(|color| color.value.to_xaml())
    }

    pub(super) fn read(call: &syn::ExprMethodCall) -> Option<Self> {
        if call.args.len() != 2 {
            return None;
        }
        let key = &call.args[0];
        let value = &call.args[1];
        Some(Self {
            declaration: rust::method_suffix(call),
            source: rust::span(value),
            key: literal_key(key)?,
            color: ValueShape::COLOR.read(value),
        })
    }
}

pub(super) fn literal_key(expression: &Expr) -> Option<String> {
    let Expr::Lit(lit) = expression else {
        return None;
    };
    let Lit::Str(lit) = &lit.lit else { return None };
    let key = lit.value();
    valid_key(&key).then_some(key)
}

pub(super) fn valid_key(key: &str) -> bool {
    !key.is_empty()
        && key.trim() == key
        && key.chars().count() <= 256
        && key
            .chars()
            .all(|ch| !ch.is_control() && !matches!(ch, '\u{fffe}' | '\u{ffff}'))
}

impl RsxStyle {
    /// Repeated registrations in a rule follow ResourceDictionary::set.
    pub(crate) fn resource_entries(&self) -> impl Iterator<Item = &RsxStyleResource> {
        let mut seen = BTreeSet::new();
        self.resources
            .iter()
            .rev()
            .filter(move |entry| seen.insert(entry.key.as_str()))
    }
}

impl RsxProjection<'_> {
    /// Mapped color keys visible from the rule's host. The applied binding still
    /// resolves from the matched element, including any closer runtime override.
    pub fn style_resource_choices(&self, style: usize) -> Vec<&RsxStyleResource> {
        let Some((owner, _)) = self
            .style_definitions()
            .into_iter()
            .find(|(_, rule)| rule.declaration.start == style)
        else {
            return Vec::new();
        };
        let scope = self.nodes[owner].span;
        let mut resources = BTreeMap::new();
        for (&owner, styles) in self.styles.iter().rev() {
            let host = self.nodes[owner].span;
            if host.start <= scope.start && scope.end <= host.end {
                for style in styles.iter().rev() {
                    style.visit(&mut |rule| {
                        for entry in rule.resource_entries() {
                            resources.entry(entry.key.as_str()).or_insert(entry);
                        }
                    });
                }
            }
        }
        resources
            .into_values()
            .filter(|entry| entry.color.is_some())
            .collect()
    }
}
