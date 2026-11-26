use super::SelectorName;
use std::ops::Range;

/// Byte ranges of an authored selector operand. Prefixes and comparison values
/// lie outside these ranges.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectorOperand {
    Type(Range<usize>),
    Property(Range<usize>),
    AttachedProperty {
        owner: Range<usize>,
        property: Range<usize>,
    },
}

pub(super) struct SourceRanges<'a> {
    input: &'a str,
    pub names: Option<Vec<SelectorName>>,
    pub operands: Option<Vec<SelectorOperand>>,
}

impl<'a> SourceRanges<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            names: None,
            operands: None,
        }
    }

    pub fn record_name(&mut self, text: &str, in_template: bool) {
        if let Some(names) = &mut self.names {
            names.push(SelectorName {
                range: range(self.input, text),
                in_template,
                ancestor: false,
            });
        }
    }

    pub fn record_type(&mut self, text: &str) {
        if let Some(operands) = &mut self.operands {
            operands.push(SelectorOperand::Type(range(self.input, text)));
        }
    }

    pub fn record_property(&mut self, text: &str) {
        if let Some(operands) = &mut self.operands {
            operands.push(SelectorOperand::Property(range(self.input, text)));
        }
    }

    pub fn record_attached(&mut self, owner: &str, property: &str) {
        if let Some(operands) = &mut self.operands {
            operands.push(SelectorOperand::AttachedProperty {
                owner: range(self.input, owner),
                property: range(self.input, property),
            });
        }
    }
}

// CharacterReader returns slices of this same input, including zero-length operands.
fn range(input: &str, text: &str) -> Range<usize> {
    let start = text.as_ptr() as usize - input.as_ptr() as usize;
    start..start + text.len()
}
