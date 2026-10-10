//! Matching a selector with a control: a type selector and a class
//! selector, each with a control it matches and one it does not.
//!
//! The two benchmarks of the selector that ORs selectors
//! (`OrSelector_One_Match`, `OrSelector_Five_Match`) are not here: they OR
//! instances of a selector class of the benchmark that always matches, and a
//! selector class cannot be declared outside the base crate (the trait of
//! the kinds of selectors and the constructor of a selector from one are
//! private to it).

use crate::harness::Registry;
use ferroui_base::styling::{Selector, Selectors};
use ferroui_base::Ref;
use ferroui_controls::{Calendar, Control};

pub struct SelectorBenchmark {
    not_matching_control: Ref<Control>,
    matching_control: Ref<Calendar>,
    is_calendar_selector: Selector,
    class_selector: Selector,
}

impl SelectorBenchmark {
    pub fn new() -> Self {
        let not_matching_control = Control::new();
        let matching_control = Calendar::new();

        const CLASS_NAME: &str = "selector-class";

        matching_control.classes().add(CLASS_NAME);

        let is_calendar_selector = Selectors::is::<Calendar>();
        let class_selector = Selectors::class(None, CLASS_NAME);

        Self { not_matching_control, matching_control, is_calendar_selector, class_selector }
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
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("styling", "SelectorBenchmark");
    class.benchmark("is_selector_no_match", "", SelectorBenchmark::new, |b| b.is_selector_no_match());
    class.benchmark("is_selector_match", "", SelectorBenchmark::new, |b| b.is_selector_match());
    class.benchmark("class_selector_no_match", "", SelectorBenchmark::new, |b| b.class_selector_no_match());
    class.benchmark("class_selector_match", "", SelectorBenchmark::new, |b| b.class_selector_match());
}

#[cfg(test)]
mod tests {
    #[test]
    fn selector_benchmark() {
        crate::harness::smoke_class(super::register, "SelectorBenchmark");
    }
}
