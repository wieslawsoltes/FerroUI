//! String-key resources and the literal values supported by the isolated preview.
use super::{RsxStyle, resource_theme};
use crate::RsxProjection;
use crate::animation::{ValueSource, syntax as rust};
use crate::local_rust::LocalRust;
use ferroui_designer_core::SourceSpan;
use ferroui_designer_tools::component_palette::PaletteLayer;
use std::collections::BTreeSet;
use syn::{Expr, Lit};

#[derive(Debug)]
pub struct RsxStyleResource {
    pub declaration: SourceSpan,
    pub source: SourceSpan,
    pub key: String,
    pub layer: Option<PaletteLayer>,
    pub shared: Option<SharedRustColor>,
    pub(super) color: Option<ValueSource>,
}

#[derive(Debug)]
pub struct SharedRustColor {
    pub name: String,
    pub value: SourceSpan,
}

impl RsxStyleResource {
    pub fn color(&self) -> Option<String> {
        self.color.as_ref().map(|color| color.value.to_xaml())
    }

    pub fn color_source(&self) -> SourceSpan {
        self.shared
            .as_ref()
            .map_or(self.source, |constant| constant.value)
    }

    pub(super) fn read(call: &syn::ExprMethodCall, bindings: &LocalRust) -> Option<Self> {
        let (layer, key, value) = if call.method == "resource" && call.args.len() == 2 {
            (Some(PaletteLayer::Shared), &call.args[0], &call.args[1])
        } else if call.method == "theme_resource" && call.args.len() == 3 {
            (
                resource_theme::read(&call.args[0], bindings),
                &call.args[1],
                &call.args[2],
            )
        } else {
            return None;
        };
        let initializer = bindings.value(value);
        let color = initializer.as_ref().and_then(|value| {
            ValueSource::read_color(value.expression, |path| bindings.path(path))
        });
        let shared = initializer.filter(|_| color.is_some()).and_then(|value| {
            Some(SharedRustColor {
                name: value.constant?.into(),
                value: rust::span(value.expression),
            })
        });
        Some(Self {
            declaration: rust::method_suffix(call),
            source: rust::span(value),
            key: resource_key(key, bindings)?,
            layer,
            shared,
            color,
        })
    }
}

pub(super) fn resource_key(expression: &Expr, bindings: &LocalRust) -> Option<String> {
    let Expr::Lit(lit) = bindings.value(expression)?.expression else {
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
    pub(crate) fn resource_entries(
        &self,
        layer: PaletteLayer,
    ) -> impl Iterator<Item = &RsxStyleResource> {
        let mut seen = BTreeSet::new();
        self.resources
            .iter()
            .rev()
            .filter(move |entry| entry.layer == Some(layer) && seen.insert(entry.key.as_str()))
    }

    fn resource(&self, key: &str, theme: PaletteLayer) -> Option<&RsxStyleResource> {
        let matching = self.resources.iter().rev().filter(|entry| entry.key == key);
        // A computed variant can affect any themed layer in this dictionary.
        // A shared value still wins before the variant lookup runs.
        matching
            .clone()
            .find(|entry| entry.layer == Some(PaletteLayer::Shared))
            .or_else(|| matching.clone().find(|entry| entry.layer.is_none()))
            .or_else(|| {
                resource_theme::fallback_order(theme)
                    .iter()
                    .find_map(|layer| matching.clone().find(|entry| entry.layer == Some(*layer)))
            })
    }
}

/// Resolved colors from the mapped style dictionaries for one key. A control's
/// local resources or application data can still override these authored values.
pub struct RsxStyleResourceFamily<'a> {
    pub key: &'a str,
    pub default: Option<&'a RsxStyleResource>,
    pub light: Option<&'a RsxStyleResource>,
    pub dark: Option<&'a RsxStyleResource>,
    pub high_contrast: Option<&'a RsxStyleResource>,
}

impl<'a> RsxStyleResourceFamily<'a> {
    pub fn common_declaration(&self) -> Option<&'a RsxStyleResource> {
        let shared = self.default?;
        self.appearances()
            .iter()
            .all(|entry| entry.is_some_and(|entry| entry.declaration == shared.declaration))
            .then_some(shared)
    }

    pub fn can_assign_color(&self) -> bool {
        let entries = [self.default, self.light, self.dark, self.high_contrast];
        entries.iter().any(Option::is_some)
            && entries
                .into_iter()
                .flatten()
                .all(|entry| entry.layer.is_some() && entry.color.is_some())
    }

    fn appearances(&self) -> [Option<&'a RsxStyleResource>; 3] {
        [self.light, self.dark, self.high_contrast]
    }
}

impl RsxProjection<'_> {
    /// Mapped declarations visible from the rule's host, ordered by key.
    pub fn style_resource_families(&self, style: usize) -> Vec<RsxStyleResourceFamily<'_>> {
        let Some((owner, _)) = self
            .style_definitions()
            .into_iter()
            .find(|(_, rule)| rule.declaration.start == style)
        else {
            return Vec::new();
        };
        let scope = self.nodes[owner].span;
        let mut rules = Vec::new();
        for (&owner, styles) in self.styles.iter().rev() {
            let host = self.nodes[owner].span;
            if host.start <= scope.start && scope.end <= host.end {
                for style in styles.iter().rev() {
                    style.visit(&mut |rule| rules.push(rule));
                }
            }
        }
        let keys: BTreeSet<_> = rules
            .iter()
            .flat_map(|rule| rule.resources.iter().map(|entry| entry.key.as_str()))
            .collect();
        keys.into_iter()
            .map(|key| {
                let find = |layer| rules.iter().find_map(|rule| rule.resource(key, layer));
                RsxStyleResourceFamily {
                    key,
                    default: find(PaletteLayer::Default),
                    light: find(PaletteLayer::Light),
                    dark: find(PaletteLayer::Dark),
                    high_contrast: find(PaletteLayer::HighContrast),
                }
            })
            .collect()
    }

    pub fn style_resource_choices(&self, style: usize) -> Vec<RsxStyleResourceFamily<'_>> {
        self.style_resource_families(style)
            .into_iter()
            .filter(RsxStyleResourceFamily::can_assign_color)
            .collect()
    }
}
