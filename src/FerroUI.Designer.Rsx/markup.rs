use crate::animation::{AnimationStyle, RsxAnimation};
use crate::tree::{escape_xml, Element};
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
            let _ = write!(self.source,
                "\n{pad}{INDENT}<Style Selector=\"{selector}\">\n{pad}{INDENT}{INDENT}<Style.Animations>");
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
