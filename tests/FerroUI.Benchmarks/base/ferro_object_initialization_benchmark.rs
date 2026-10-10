//! Creating a button.

use crate::harness::Registry;
use ferroui_base::Ref;
use ferroui_controls::Button;

pub struct FerroObjectInitializationBenchmark;

impl FerroObjectInitializationBenchmark {
    pub fn new() -> Self {
        Self
    }

    pub fn initialize_button(&self) -> Ref<Button> {
        Button::new()
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("base", "FerroObjectInitializationBenchmark");
    class.benchmark("initialize_button", "", FerroObjectInitializationBenchmark::new, |b| b.initialize_button());
}

#[cfg(test)]
mod tests {
    #[test]
    fn ferro_object_initialization_benchmark() {
        crate::harness::smoke_class(super::register, "FerroObjectInitializationBenchmark");
    }
}
