//! Creating a button under the Fluent theme, making it the child of a root
//! and laying it out.

use crate::harness::Registry;
use crate::test_root::TestRoot;
use ferroui_base::styling::{IStyle, Styles};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::Ref;
use ferroui_controls::testing::{NullRenderer, TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::Button;
use ferroui_themes_fluent::FluentTheme;
use std::rc::Rc;

pub struct FluentBenchmark {
    root: Ref<TestRoot>,
    /// Disposed when the benchmark is dropped, after the tree (the fields
    /// drop in the order of their declaration).
    _app: UnitTestApplicationScope,
}

impl FluentBenchmark {
    pub fn new() -> Self {
        let app = Self::create_app();
        let root = TestRoot::with_global_styles(true, None);
        root.set_renderer(NullRenderer::new());

        root.layout_manager().execute_initial_layout_pass();

        Self { root, _app: app }
    }

    pub fn create_button(&self) {
        let button = Button::new();
        self.root.set_child(&button);
        self.root.layout_manager().execute_layout_pass();
        Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::LOADED));
    }

    fn create_app() -> UnitTestApplicationScope {
        let services = TestServices { theme: Some(Rc::new(Self::load_fluent_theme)), ..TestServices::default() };

        UnitTestApplication::start(services)
    }

    fn load_fluent_theme() -> Rc<dyn IStyle> {
        let styles = Styles::new();
        styles.add(FluentTheme::new().as_style());
        styles.into()
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("themes", "FluentBenchmark");
    class.benchmark("create_button", "", FluentBenchmark::new, |b| b.create_button());
}

#[cfg(test)]
mod tests {
    #[test]
    fn fluent_benchmark() {
        crate::harness::smoke_class(super::register, "FluentBenchmark");
    }
}
