//! Creating a carousel page, and navigating between its pages.
//!
//! The upstream constructors first register the parsers of the resource URI
//! scheme with the URI type of their platform; the URI type of the framework
//! reads that scheme itself, so there is nothing to register here.

use crate::harness::Registry;
use crate::test_root::TestRoot;
use ferroui_base::styling::{IStyle, Styles};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::testing::{NullRenderer, TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{CarouselPage, ContentPage, Page};
use ferroui_themes_fluent::FluentTheme;
use std::rc::Rc;

/// The theme of the application: a styles collection that holds the Fluent
/// theme.
fn theme() -> Rc<dyn IStyle> {
    let styles = Styles::new();
    styles.add(FluentTheme::new().as_style());
    styles.into()
}

/// A content page with the header `header`.
fn content_page(header: String) -> Ref<Page> {
    let page = ContentPage::new();
    let header: BoxedValue = Rc::new(header);
    page.set_header(Some(header));
    page.upcast()
}

/// Measures the cost of creating a CarouselPage and applying its template
/// as the number of pages grows.
pub struct CarouselPageCreationBenchmark {
    root: Ref<TestRoot>,
    page_count: i32,
    /// Disposed when the benchmark is dropped, after the tree (the fields
    /// drop in the order of their declaration).
    _app: UnitTestApplicationScope,
}

impl CarouselPageCreationBenchmark {
    pub fn new(page_count: i32) -> Self {
        let app = UnitTestApplication::start(TestServices::styled_window().with_theme(theme));
        let root = TestRoot::with_global_styles(true, None);
        root.set_renderer(NullRenderer::new());
        root.layout_manager().execute_initial_layout_pass();

        Self { root, page_count, _app: app }
    }

    pub fn create(&self) {
        let cp = CarouselPage::new();
        cp.set_page_transition(None);
        for i in 0..self.page_count {
            cp.pages().expect("the carousel page has pages").add(content_page(format!("Page {i}")));
        }
        self.root.set_child(&cp);
        self.root.layout_manager().execute_layout_pass();
        Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::LOADED));
    }
}

/// Measures the cost of navigating between pages on a CarouselPage with
/// five pages (no transition).
pub struct CarouselPageNavigationBenchmark {
    root: Ref<TestRoot>,
    /// Set by the iteration setup.
    carousel_page: Option<Ref<CarouselPage>>,
    /// Disposed when the benchmark is dropped, after the tree (the fields
    /// drop in the order of their declaration).
    _app: UnitTestApplicationScope,
}

impl CarouselPageNavigationBenchmark {
    pub fn new() -> Self {
        let app = UnitTestApplication::start(TestServices::styled_window().with_theme(theme));
        let root = TestRoot::with_global_styles(true, None);
        root.set_renderer(NullRenderer::new());
        root.layout_manager().execute_initial_layout_pass();

        Self { root, carousel_page: None, _app: app }
    }

    pub fn iteration_setup(&mut self) {
        let carousel_page = CarouselPage::new();
        carousel_page.set_page_transition(None);
        for i in 0..5 {
            carousel_page.pages().expect("the carousel page has pages").add(content_page(format!("Page {i}")));
        }
        self.root.set_child(&carousel_page);
        self.root.layout_manager().execute_layout_pass();
        carousel_page.set_selected_index(0);
        self.root.layout_manager().execute_layout_pass();
        Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::LOADED));
        self.carousel_page = Some(carousel_page);
    }

    /// Navigate forward from page 0 to page 1 (no transition).
    pub fn navigate_next(&self) {
        self.carousel_page.as_ref().expect("the iteration setup ran").set_selected_index(1);
        self.root.layout_manager().execute_layout_pass();
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("navigation", "CarouselPageCreationBenchmark");
    for page_count in [2, 5, 10] {
        class.benchmark(
            "create",
            format!("PageCount={page_count}"),
            move || CarouselPageCreationBenchmark::new(page_count),
            |b| b.create(),
        );
    }

    let mut class = registry.class("navigation", "CarouselPageNavigationBenchmark");
    class.benchmark_with_iteration_setup(
        "navigate_next",
        "",
        CarouselPageNavigationBenchmark::new,
        |b| b.iteration_setup(),
        |b| b.navigate_next(),
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn carousel_page_creation_benchmark() {
        crate::harness::smoke_class(super::register, "CarouselPageCreationBenchmark");
    }

    #[test]
    fn carousel_page_navigation_benchmark() {
        crate::harness::smoke_class(super::register, "CarouselPageNavigationBenchmark");
    }
}
