//! Attribute layout changes only separators between original XML tokens.
use super::{AttributeLayout, SelfClosingSpace, Writer, xml_whitespace};
use ferroui_designer_core::{Node, SourceSpan};

impl Writer<'_> {
    pub(super) fn opening_tag(&mut self, node: &Node, depth: usize) {
        let bracket = node.opening_span.end - if node.self_closing { 2 } else { 1 };
        let mut previous = node.name_span.end;
        let layout = self.options.attribute_layout;
        let multiline = !node.attributes.is_empty()
            && match layout {
                AttributeLayout::OnePerLine => true,
                AttributeLayout::Wrap => self.exceeds_column(node, depth),
                AttributeLayout::Preserve | AttributeLayout::Inline => false,
            };

        if layout == AttributeLayout::Preserve {
            previous = node
                .attributes
                .last()
                .map_or(previous, |attr| attr.span.end);
        } else {
            let separator = if multiline {
                format!("{}{}", self.newline, self.indent.repeat(depth + 1))
            } else {
                " ".into()
            };
            for attr in &node.attributes {
                self.replace(
                    SourceSpan::new(previous, attr.span.start),
                    separator.clone(),
                );
                previous = attr.span.end;
            }
        }

        let span = SourceSpan::new(previous, bracket);
        let gap = &self.source[span.range()];
        if !xml_whitespace(gap)
            || (layout == AttributeLayout::Preserve && gap.contains(['\n', '\r']))
        {
            return;
        }
        let formatted = if multiline && self.options.closing_bracket_on_new_line {
            format!("{}{}", self.newline, self.indent.repeat(depth))
        } else if node.self_closing {
            match self.options.self_closing_space {
                SelfClosingSpace::Space => " ".into(),
                SelfClosingSpace::None => String::new(),
                SelfClosingSpace::Preserve
                    if layout != AttributeLayout::Preserve && gap.contains('\n') =>
                {
                    " ".into()
                }
                SelfClosingSpace::Preserve => return,
            }
        } else if layout != AttributeLayout::Preserve && !node.attributes.is_empty() {
            String::new()
        } else {
            return;
        };
        self.replace(span, formatted);
    }

    fn exceeds_column(&self, node: &Node, depth: usize) -> bool {
        // An existing multiline value stays a multiline token under any layout.
        let mut width = 2 + depth * self.options.tab_size as usize + node.name.chars().count();
        if node.self_closing {
            let gap =
                &self.source[node.attributes.last().unwrap().span.end..node.opening_span.end - 2];
            width += 1 + match self.options.self_closing_space {
                SelfClosingSpace::None => 0,
                SelfClosingSpace::Space => 1,
                SelfClosingSpace::Preserve if gap.contains('\n') => 1,
                SelfClosingSpace::Preserve => gap.chars().count(),
            };
        }
        for attr in &node.attributes {
            let token = &self.source[attr.span.range()];
            if token.contains(['\n', '\r']) {
                return true;
            }
            width += 1 + token.chars().count();
        }
        width > self.options.wrap_column as usize
    }
}
