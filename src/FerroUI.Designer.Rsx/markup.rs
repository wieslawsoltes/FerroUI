use crate::RsxProjection;
use crate::animation::{AnimationStyle, RsxAnimation, RsxAnimationIssue};
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
}

struct ControlSource {
    offset: usize,
    name: SourceSpan,
    properties: BTreeMap<String, SourceSpan>,
}

impl RsxSourceMap {
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

    pub fn animation(&self, offset: usize) -> Option<&RsxAnimation> {
        let index = self
            .animations
            .binary_search_by_key(&offset, |(start, _)| *start)
            .ok()?;
        Some(&self.animations[index].1)
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
                .map(|(offset, node)| ControlSource {
                    offset,
                    name: node.name,
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
        };
        (markup.source, map)
    }
}

pub(crate) fn serialize<'a>(
    root: &Element,
    styles: &'a BTreeMap<usize, Vec<AnimationStyle>>,
) -> RsxPreviewMarkup<'a> {
    let mut output = RsxPreviewMarkup::default();
    output.control(root, styles, 0);
    output
}

impl<'a> RsxPreviewMarkup<'a> {
    fn control(
        &mut self,
        element: &Element,
        styles: &'a BTreeMap<usize, Vec<AnimationStyle>>,
        depth: usize,
    ) {
        let pad = INDENT.repeat(depth);
        let namespace = element.class.namespace();
        let prefix = if namespace.is_some() { "ui:" } else { "" };
        let tag = format!("{prefix}{}", element.class.name());
        let rules = styles.get(&self.controls.len());
        self.source.push_str(&pad);
        self.controls.push(self.source.len());
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
            let _ = write!(self.source, " {name}=\"{}\"", escape_xml(value, Some('"')));
        }
        if element.children.is_empty() && rules.is_none() {
            self.source.push_str(" />");
            return;
        }
        self.source.push('>');
        if let Some(rules) = rules {
            self.styles(&tag, rules, depth + 1);
        }
        for child in &element.children {
            self.source.push('\n');
            self.control(child, styles, depth + 1);
        }
        let _ = write!(self.source, "\n{pad}</{tag}>");
    }

    fn styles(&mut self, owner: &str, styles: &'a [AnimationStyle], depth: usize) {
        let pad = INDENT.repeat(depth);
        let _ = write!(self.source, "\n{pad}<{owner}.Styles>");
        for style in styles {
            let selector = escape_xml(&style.selector, Some('"'));
            let _ = write!(
                self.source,
                "\n{pad}{INDENT}<Style Selector=\"{selector}\">\n{pad}{INDENT}{INDENT}<Style.Animations>"
            );
            for clip in &style.clips {
                let _ = write!(self.source, "\n{pad}{INDENT}{INDENT}{INDENT}");
                self.animations.push((self.source.len(), clip));
                self.source.push_str(&clip.xaml);
            }
            let _ = write!(
                self.source,
                "\n{pad}{INDENT}{INDENT}</Style.Animations>\n{pad}{INDENT}</Style>"
            );
        }
        let _ = write!(self.source, "\n{pad}</{owner}.Styles>");
    }
}
