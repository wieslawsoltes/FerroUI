use super::Node;
use std::ops::Range;

/// UTF-8 ranges of the authored name and, for an attached property, its owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperandSpan {
    pub name: Range<usize>,
    pub owner: Option<Range<usize>>,
}

/// A binding path node with the original operand ranges where it names a symbol.
#[derive(Clone, Debug, PartialEq)]
pub struct SpannedNode {
    pub node: Node,
    pub operand: Option<OperandSpan>,
}

#[derive(Default)]
pub(super) struct Nodes<'a> {
    pub nodes: Vec<Node>,
    source: Option<&'a str>,
    spans: Vec<Option<OperandSpan>>,
}

impl<'a> Nodes<'a> {
    pub fn spanned(source: &'a str) -> Self {
        Self {
            source: Some(source),
            ..Self::default()
        }
    }

    pub fn push(&mut self, node: Node) {
        self.push_operand(node, None, None);
    }

    pub fn push_named(&mut self, node: Node, name: &str) {
        self.push_operand(node, Some(name), None);
    }

    pub fn push_operand(&mut self, node: Node, name: Option<&str>, owner: Option<&str>) {
        self.nodes.push(node);
        if let Some(source) = self.source {
            self.spans
                .push(name.filter(|name| !name.is_empty()).map(|name| {
                    OperandSpan {
                        name: range(source, name),
                        owner: owner
                            .filter(|owner| !owner.is_empty())
                            .map(|owner| range(source, owner)),
                    }
                }));
        }
    }

    pub fn into_spanned(self) -> Vec<SpannedNode> {
        self.nodes
            .into_iter()
            .zip(self.spans)
            .map(|(node, operand)| SpannedNode { node, operand })
            .collect()
    }
}

// The grammar passes slices taken from the CharacterReader's input.
fn range(source: &str, text: &str) -> Range<usize> {
    let start = text.as_ptr() as usize - source.as_ptr() as usize;
    start..start + text.len()
}
