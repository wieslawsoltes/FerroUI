//! Creating a control, making it the child of a styled root and laying it
//! out.

use crate::harness::Registry;
use crate::test_root::TestRoot;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::Ref;
use ferroui_controls::testing::{NullRenderer, TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Button, Calendar, Control, Decorator, ScrollViewer, TextBox};

pub struct ControlsBenchmark {
    root: Ref<TestRoot>,
    /// Disposed when the benchmark is dropped, after the tree (the fields
    /// drop in the order of their declaration).
    _app: UnitTestApplicationScope,
}

impl ControlsBenchmark {
    pub fn new() -> Self {
        let app = UnitTestApplication::start(TestServices::styled_window());

        let root = TestRoot::with_global_styles(true, None);
        root.set_renderer(NullRenderer::new());

        root.layout_manager().execute_initial_layout_pass();

        Self { root, _app: app }
    }

    pub fn create_calendar(&self) {
        let calendar = Calendar::new();

        self.root.set_child(&calendar);

        self.root.layout_manager().execute_layout_pass();
        Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::LOADED));
    }

    pub fn create_calendar_with_loaded(&self) {
        let subscription = Control::loaded_event().add_class_handler::<Control>(|_c, _s| {});

        let calendar = Calendar::new();

        self.root.set_child(&calendar);

        self.root.layout_manager().execute_layout_pass();
        Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::LOADED));

        subscription.dispose();
    }

    pub fn create_control(&self) {
        let control = Control::new();

        self.root.set_child(&control);

        self.root.layout_manager().execute_layout_pass();
        Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::LOADED));
    }

    pub fn create_decorator(&self) {
        let control = Decorator::new();

        self.root.set_child(&control);

        self.root.layout_manager().execute_layout_pass();
        Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::LOADED));
    }

    pub fn create_scroll_viewer(&self) {
        let control = ScrollViewer::new();

        self.root.set_child(&control);

        self.root.layout_manager().execute_layout_pass();
        Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::LOADED));
    }

    pub fn create_button(&self) {
        let button = Button::new();

        self.root.set_child(&button);

        self.root.layout_manager().execute_layout_pass();
        Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::LOADED));
    }

    pub fn create_text_box(&self) {
        let text_box = TextBox::new();

        self.root.set_child(&text_box);

        self.root.layout_manager().execute_layout_pass();
        Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::LOADED));
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("layout", "ControlsBenchmark");
    class.benchmark("create_calendar", "", ControlsBenchmark::new, |b| b.create_calendar());
    class.benchmark("create_calendar_with_loaded", "", ControlsBenchmark::new, |b| b.create_calendar_with_loaded());
    class.benchmark("create_control", "", ControlsBenchmark::new, |b| b.create_control());
    class.benchmark("create_decorator", "", ControlsBenchmark::new, |b| b.create_decorator());
    class.benchmark("create_scroll_viewer", "", ControlsBenchmark::new, |b| b.create_scroll_viewer());
    class.benchmark("create_button", "", ControlsBenchmark::new, |b| b.create_button());
    class.benchmark("create_text_box", "", ControlsBenchmark::new, |b| b.create_text_box());
}

#[cfg(test)]
mod tests {
    #[test]
    fn controls_benchmark() {
        crate::harness::smoke_class(super::register, "ControlsBenchmark");
    }
}
