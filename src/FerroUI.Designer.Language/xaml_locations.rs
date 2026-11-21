//! Map native XAML compiler coordinates to authored elements.
use crate::{LineIndex, Position, Range};
use ferroui_designer_core::{Document, Node};

pub struct XamlLocations<'a> {
    lines: LineIndex,
    nodes: Vec<&'a Node>,
}

impl<'a> XamlLocations<'a> {
    pub fn new(document: &'a Document) -> Self {
        Self {
            lines: LineIndex::new(document.valid_source()),
            nodes: document
                .ast()
                .nodes_in_source_order()
                .filter(|node| node.is_element())
                .collect(),
        }
    }

    /// The compiler reports one-based lines and character columns.
    pub fn element(&self, line: i32, column: i32, type_name: &str) -> Option<&'a Node> {
        let line = u32::try_from(line.checked_sub(1)?).ok()?;
        let column = usize::try_from(column.checked_sub(1)?).ok()?;
        let start = self.lines.offset(Position { line, character: 0 }).ok()?;
        let row = self.lines.source()[start..].split(['\r', '\n']).next()?;
        let content = if line == 0 {
            row.trim_start_matches('\u{feff}')
        } else {
            row
        };
        let first = self.nodes.partition_point(|node| node.span.start < start);
        let mut candidates = self.nodes[first..]
            .iter()
            .copied()
            .take_while(|node| node.span.start < start + row.len())
            .filter(|node| node.local_name() == type_name);
        let first = candidates.next()?;
        let Some(second) = candidates.next() else {
            return Some(first);
        };
        let (offset, _) = content.char_indices().nth(column)?;
        let position = start + row.len() - content.len() + offset;
        [first, second]
            .into_iter()
            .chain(candidates)
            .find(|node| node.span.start <= position && position <= node.name_span.end)
    }

    pub fn name_range(&self, node: &Node) -> Option<Range> {
        self.lines.range(node.name_span).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distinguishes_elements_on_one_line_after_non_ascii_text() {
        let text = "\u{feff}<Panel>\r\n  <!-- 🙂 --><Button /><Button />\r\n</Panel>";
        let document = Document::new(text).unwrap();
        let locations = XamlLocations::new(&document);
        assert_eq!(
            locations.element(1, 2, "Panel").unwrap().local_name(),
            "Panel"
        );
        let second = locations.element(2, 24, "Button").unwrap();
        let offset = text.rfind("Button").unwrap();
        assert_eq!(second.name_span.start, offset);
        assert_eq!(locations.name_range(second).unwrap().start.character, 24);
        assert!(locations.element(2, 1, "Button").is_none());
    }
}
