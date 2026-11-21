use crate::{RsxNode, RsxProjection};
use crate::animation::{RsxAnimation, RsxAnimationIssue};
use crate::states::{ProjectedStates, RsxStateGroup};
use crate::styles::RsxStyle;
use crate::tree::{Element, escape_xml};
use ferroui_designer_core::SourceSpan;
use ferroui_designer_language::FRAMEWORK_NAMESPACE;
use std::collections::BTreeMap;
use std::fmt::Write;

const INDENT: &str = "    ";

/// Projected markup and the byte offsets that carry authored Rust identities.
/// Generated style, property and keyframe elements do not index into the control list.
#[derive(Default)]
pub struct RsxPreviewMarkup<'a> {
    pub source: String,
    pub controls: Vec<usize>,
    pub animations: Vec<(usize, &'a RsxAnimation)>,
    pub state_groups: Vec<(usize, &'a RsxStateGroup)>,
    style_setters: Vec<(usize, SourceSpan)>,
}

/// Source locations for an accepted projection. Keys are XML byte offsets
/// so each native host can use its own reconciled document node IDs.
pub struct RsxSourceMap {
    pub invocation: SourceSpan,
    pub blocks: Vec<SourceSpan>,
    pub issues: Vec<RsxAnimationIssue>,
    pub omitted_count: usize,
    controls: Vec<ControlSource>,
    animations: Vec<(usize, RsxAnimation)>,
    state_groups: Vec<(usize, RsxStateGroup)>,
    style_setters: Vec<(usize, SourceSpan)>,
}

struct ControlSource {
    offset: usize,
    name: SourceSpan,
    properties: BTreeMap<String, SourceSpan>,
    editable_children: bool,
    can_create_animation: bool,
    can_create_state_group: bool,
}

impl RsxSourceMap {
    pub fn editable_children(&self, offset: usize) -> bool {
        self.control(offset)
            .is_some_and(|(index, _)| self.controls[index].editable_children)
    }

    pub fn control(&self, offset: usize) -> Option<(usize, SourceSpan)> {
        let index = self
            .controls
            .binary_search_by_key(&offset, |control| control.offset)
            .ok()?;
        Some((index, self.controls[index].name))
    }

    pub fn property(&self, offset: usize, name: &str) -> Option<SourceSpan> {
        let (index, _) = self.control(offset)?;
        self.controls[index].properties.get(name).copied()
    }

    pub fn style_setter(&self, offset: usize) -> Option<SourceSpan> {
        let index = self.style_setters.binary_search_by_key(&offset, |(start, _)| *start).ok()?;
        Some(self.style_setters[index].1)
    }

    pub fn can_create_animation(&self, offset: usize) -> bool {
        self.control(offset).is_some_and(|(index, _)| self.controls[index].can_create_animation)
    }

    pub fn can_create_state_group(&self, offset: usize) -> bool {
        self.control(offset).is_some_and(|(index, _)| self.controls[index].can_create_state_group)
    }

    pub fn animation(&self, offset: usize) -> Option<&RsxAnimation> {
        let index = self
            .animations
            .binary_search_by_key(&offset, |(start, _)| *start)
            .ok()?;
        Some(&self.animations[index].1)
    }

    pub fn state_group(&self, owner_offset: usize, name: &str) -> Option<&RsxStateGroup> {
        self.state_groups.iter().find_map(|(offset, source)| {
            (*offset == owner_offset && source.name() == name).then_some(source)
        })
    }
}

impl RsxProjection<'_> {
    pub fn mapped_markup(&self) -> (String, RsxSourceMap) {
        let markup = self.preview_markup();
        let map = RsxSourceMap {
            invocation: self.invocation,
            blocks: self.blocks.clone(),
            issues: self.animation_issues.clone(),
            omitted_count: self.omitted_count,
            controls: markup
                .controls
                .into_iter()
                .zip(&self.nodes)
                .enumerate()
                .map(|(index, (offset, node))| ControlSource {
                    offset,
                    name: node.name,
                    editable_children: node.editable_children(),
                    can_create_animation: self.can_create_animation(index),
                    can_create_state_group: self.can_create_state_group(index),
                    properties: node
                        .properties
                        .iter()
                        .chain(&node.expressions)
                        .map(|(name, span)| (name.clone(), *span))
                        .collect(),
                })
                .collect(),
            animations: markup
                .animations
                .into_iter()
                .map(|(offset, clip)| (offset, clip.clone()))
                .collect(),
            state_groups: markup.state_groups.into_iter().map(|(offset, group)| (offset, group.clone())).collect(),
            style_setters: markup.style_setters,
        };
        (markup.source, map)
    }
}

pub(crate) fn serialize<'a>(
    root: &Element,
    nodes: &[RsxNode],
    styles: &'a BTreeMap<usize, Vec<RsxStyle>>,
    states: &'a BTreeMap<usize, ProjectedStates>,
) -> RsxPreviewMarkup<'a> {
    let mut output = RsxPreviewMarkup::default();
    output.control(root, nodes, styles, states, 0);
    output
}

impl<'a> RsxPreviewMarkup<'a> {
    fn control(
        &mut self,
        element: &Element,
        nodes: &[RsxNode],
        styles: &'a BTreeMap<usize, Vec<RsxStyle>>,
        states: &'a BTreeMap<usize, ProjectedStates>,
        depth: usize,
    ) {
        let pad = INDENT.repeat(depth);
        let namespace = element.class.namespace();
        let prefix = if namespace.is_some() { "ui:" } else { "" };
        let tag = format!("{prefix}{}", element.class.name());
        let index = self.controls.len();
        let rules = styles.get(&index);
        let groups = states.get(&index);
        let columns = nodes.get(index).and_then(|node| node.columns.as_ref()).filter(|columns| columns.projected());
        self.source.push_str(&pad);
        let offset = self.source.len();
        self.controls.push(offset);
        let _ = write!(self.source, "<{tag}");
        if depth == 0 {
            let _ = write!(self.source, " xmlns=\"{FRAMEWORK_NAMESPACE}\"");
        }
        if let Some(namespace) = namespace {
            let _ = write!(
                self.source,
                " xmlns:ui=\"{}\"",
                escape_xml(namespace, Some('"'))
            );
        }
        for (name, value) in &element.properties {
            if name == "Classes" && groups.is_some() { continue; }
            let _ = write!(self.source, " {name}=\"{}\"", escape_xml(value, Some('"')));
        }
        if let Some(groups) = groups {
            let classes = element.properties.get("Classes").map_or("", String::as_str);
            let classes = format!("{} {}", classes, groups.anchor);
            let _ = write!(self.source, " Classes=\"{}\"", escape_xml(classes.trim(), Some('"')));
            self.state_groups.extend(groups.groups.iter().map(|group| (offset, group)));
        }
        if element.children.is_empty() && rules.is_none() && groups.is_none() && columns.is_none() {
            self.source.push_str(" />");
            return;
        }
        self.source.push('>');
        if rules.is_some() || groups.is_some() {
            let rules = rules.map_or(&[][..], Vec::as_slice);
            let markup = groups.map_or(&[][..], |groups| groups.markup.as_slice());
            self.styles(&tag, rules, markup, depth + 1);
        }
        if let Some(columns) = columns {
            columns.render(&mut self.source, &pad);
        }
        for child in &element.children {
            self.source.push('\n');
            self.control(child, nodes, styles, states, depth + 1);
        }
        let _ = write!(self.source, "\n{pad}</{tag}>");
    }

    fn styles(&mut self, owner: &str, styles: &'a [RsxStyle], states: &[String], depth: usize) {
        let pad = INDENT.repeat(depth);
        let _ = write!(self.source, "\n{pad}<{owner}.Styles>");
        for style in styles {
            self.style(style, depth + 1);
        }
        for state in states {
            let _ = write!(self.source, "\n{pad}{INDENT}{state}");
        }
        let _ = write!(self.source, "\n{pad}</{owner}.Styles>");
    }

    fn style(&mut self, style: &'a RsxStyle, depth: usize) {
        let Some(selector) = &style.preview_selector else { return };
        let pad = INDENT.repeat(depth);
        let escaped = escape_xml(&selector.value, Some('"'));
        let _ = write!(self.source, "\n{pad}<Style Selector=\"{escaped}\"");
        for (alias, namespace) in &selector.namespaces {
            let namespace = escape_xml(namespace, Some('"'));
            let _ = write!(self.source, " xmlns:{alias}=\"{namespace}\"");
        }
        self.source.push('>');
        style_resources::write(&mut self.source, style, &pad);
        for setter in &style.setters {
            let (Some(field), Some(value)) = (&setter.field, setter.value()) else { continue };
            let value = escape_xml(&value, Some('"'));
            let name = &field.name;
            let owner = field.owner_name();
            let _ = write!(self.source, "\n{pad}{INDENT}");
            self.style_setters.push((self.source.len(), setter.source));
            self.source.push_str("<Setter xmlns:x=\"http://schemas.microsoft.com/winfx/2006/xaml\"");
            let prefix = if let Some(namespace) = &field.owner.namespace {
                let namespace = escape_xml(namespace, Some('"'));
                let _ = write!(self.source, " xmlns:rsxOwner=\"{namespace}\"");
                "rsxOwner:"
            } else { "" };
            let owner = format!("{prefix}{owner}");
            let _ = write!(self.source, " x:SetterTargetType=\"{owner}\" Property=\"{owner}.{name}\"");
            if let Some(key) = setter.resource_key() {
                let key = resource_key_attribute(key);
                let _ = write!(self.source, "><Setter.Value><DynamicResource ResourceKey=\"{key}\"/></Setter.Value></Setter>");
            } else {
                let _ = write!(self.source, " Value=\"{value}\"/>");
            }
        }
        if !style.clips.is_empty() {
            let _ = write!(self.source, "\n{pad}{INDENT}<Style.Animations>");
            for clip in &style.clips {
                let _ = write!(self.source, "\n{pad}{INDENT}{INDENT}");
                self.animations.push((self.source.len(), clip));
                self.source.push_str(&clip.xaml);
            }
            let _ = write!(self.source, "\n{pad}{INDENT}</Style.Animations>");
        }
        for child in &style.children {
            self.style(child, depth + 1);
        }
        let _ = write!(self.source, "\n{pad}</Style>");
    }

}

#[path = "markup_style_resources.rs"]
mod style_resources;

fn resource_key_attribute(key: &str) -> String {
    let escaped = escape_xml(key, Some('"'));
    if key.starts_with('{') { format!("{{}}{escaped}") } else { escaped }
}
