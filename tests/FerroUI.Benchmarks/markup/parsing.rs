//! Parsing a selector from its text.

use crate::harness::Registry;
use ferroui_controls::{ListBox, TextBlock, TextBox};
use ferroui_markup::markup::parsers::SelectorParser;

pub struct Parsing;

impl Parsing {
    pub fn new() -> Self {
        Self
    }

    pub fn parse_complex_selector(&self) {
        let selector_string = "ListBox > TextBox /template/ TextBlock[IsFocused=True]";
        let parser = SelectorParser::new(|_ns, s| match s {
            "ListBox" => Some(ListBox::TYPE),
            "TextBox" => Some(TextBox::TYPE),
            "TextBlock" => Some(TextBlock::TYPE),
            _ => None,
        });
        let _selector = parser.parse(selector_string).expect("the selector is valid");
    }
}

impl Default for Parsing {
    fn default() -> Self {
        Self::new()
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("markup", "Parsing");
    class.benchmark("parse_complex_selector", "", Parsing::new, |b| b.parse_complex_selector());
}

#[cfg(test)]
mod tests {
    #[test]
    fn parsing() {
        crate::harness::smoke_class(super::register, "Parsing");
    }
}
