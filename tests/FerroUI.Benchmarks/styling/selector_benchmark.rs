//! Matching a selector with a control: a type selector and a class
//! selector, each with a control it matches and one it does not, and the
//! selector that ORs two and five selectors of a class of the benchmark that
//! always matches.
//!
//! The class of selectors of the benchmark (`AlwaysMatchSelector`) implements
//! the trait of the kinds of selectors, which the `testing` feature of the
//! base crate exposes (`ferroui_base::styling::testing::SelectorNode`), as
//! upstream's benchmark project derives it from `Selector` through its access
//! to the internals of the framework.

use crate::harness::Registry;
use ferroui_base::styling::testing::{selector, SelectorNode};
use ferroui_base::styling::{Selector, SelectorMatch, Selectors, Style, StyleBase};
use ferroui_base::{Ref, StyledElement, TypeInfo};
use ferroui_controls::{Calendar, Control};
use std::any::Any;

pub struct SelectorBenchmark {
    not_matching_control: Ref<Control>,
    matching_control: Ref<Calendar>,
    is_calendar_selector: Selector,
    class_selector: Selector,
    or_selector_two: Selector,
    or_selector_five: Selector,
}

impl SelectorBenchmark {
    pub fn new() -> Self {
        let not_matching_control = Control::new();
        let matching_control = Calendar::new();

        const CLASS_NAME: &str = "selector-class";

        matching_control.classes().add(CLASS_NAME);

        let is_calendar_selector = Selectors::is::<Calendar>();
        let class_selector = Selectors::class(None, CLASS_NAME);

        let or_selector_two = Selectors::or([selector(AlwaysMatchSelector), selector(AlwaysMatchSelector)]);
        let or_selector_five = Selectors::or([
            selector(AlwaysMatchSelector),
            selector(AlwaysMatchSelector),
            selector(AlwaysMatchSelector),
            selector(AlwaysMatchSelector),
            selector(AlwaysMatchSelector),
        ]);

        Self {
            not_matching_control,
            matching_control,
            is_calendar_selector,
            class_selector,
            or_selector_two,
            or_selector_five,
        }
    }

    pub fn is_selector_no_match(&self) {
        let _ = self.is_calendar_selector.match_(&self.not_matching_control, None, true);
    }

    pub fn is_selector_match(&self) {
        let _ = self.is_calendar_selector.match_(&self.matching_control, None, true);
    }

    pub fn class_selector_no_match(&self) {
        let _ = self.class_selector.match_(&self.not_matching_control, None, true);
    }

    pub fn class_selector_match(&self) {
        let _ = self.class_selector.match_(&self.matching_control, None, true);
    }

    pub fn or_selector_one_match(&self) {
        let _ = self.or_selector_two.match_(&self.matching_control, None, true);
    }

    pub fn or_selector_five_match(&self) {
        let _ = self.or_selector_five.match_(&self.matching_control, None, true);
    }
}

/// A selector that matches every control of every type.
struct AlwaysMatchSelector;

impl SelectorNode for AlwaysMatchSelector {
    fn in_template(&self) -> bool {
        false
    }

    fn is_combinator(&self) -> bool {
        false
    }

    fn target_type(&self) -> Option<&'static TypeInfo> {
        None
    }

    fn to_string(&self, _owner: Option<&Style>) -> String {
        "Always".to_string()
    }

    fn evaluate(&self, _control: &StyledElement, _parent: Option<&StyleBase>, _subscribe: bool) -> SelectorMatch {
        SelectorMatch::ALWAYS_THIS_TYPE
    }

    fn move_previous(&self) -> Option<&Selector> {
        None
    }

    fn move_previous_or_parent(&self) -> Option<&Selector> {
        None
    }

}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("styling", "SelectorBenchmark");
    class.benchmark("is_selector_no_match", "", SelectorBenchmark::new, |b| b.is_selector_no_match());
    class.benchmark("is_selector_match", "", SelectorBenchmark::new, |b| b.is_selector_match());
    class.benchmark("class_selector_no_match", "", SelectorBenchmark::new, |b| b.class_selector_no_match());
    class.benchmark("class_selector_match", "", SelectorBenchmark::new, |b| b.class_selector_match());
    class.benchmark("or_selector_one_match", "", SelectorBenchmark::new, |b| b.or_selector_one_match());
    class.benchmark("or_selector_five_match", "", SelectorBenchmark::new, |b| b.or_selector_five_match());
}

#[cfg(test)]
mod tests {
    #[test]
    fn selector_benchmark() {
        crate::harness::smoke_class(super::register, "SelectorBenchmark");
    }
}
