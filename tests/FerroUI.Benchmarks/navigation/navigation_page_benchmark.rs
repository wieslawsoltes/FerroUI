//! Creating a navigation page, and pushing and popping pages on one.
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
use ferroui_controls::{ContentPage, NavigationPage};
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
fn content_page(header: &str) -> Ref<ContentPage> {
    let page = ContentPage::new();
    let header: BoxedValue = Rc::new(header.to_string());
    page.set_header(Some(header));
    page
}

/// Measures the cost of creating a NavigationPage and applying its template.
pub struct NavigationPageCreationBenchmark {
    root: Ref<TestRoot>,
    /// Disposed when the benchmark is dropped, after the tree (the fields
    /// drop in the order of their declaration).
    _app: UnitTestApplicationScope,
}

impl NavigationPageCreationBenchmark {
    pub fn new() -> Self {
        let app = UnitTestApplication::start(TestServices::styled_window().with_theme(theme));
        let root = TestRoot::with_global_styles(true, None);
        root.set_renderer(NullRenderer::new());
        root.layout_manager().execute_initial_layout_pass();

        Self { root, _app: app }
    }

    pub fn create(&self) {
        let nav = NavigationPage::new();
        nav.set_page_transition(None);
        self.root.set_child(&nav);
        self.root.layout_manager().execute_layout_pass();
        Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::LOADED));
    }
}

/// Measures push and pop on a NavigationPage that already has two pages on
/// its stack. Transitions are disabled so only the stack-management and
/// layout cost is captured.
pub struct NavigationPageStackBenchmark {
    root: Ref<TestRoot>,
    /// Set by the iteration setup.
    nav: Option<Ref<NavigationPage>>,
    /// Disposed when the benchmark is dropped, after the tree (the fields
    /// drop in the order of their declaration).
    _app: UnitTestApplicationScope,
}

impl NavigationPageStackBenchmark {
    pub fn new() -> Self {
        let app = UnitTestApplication::start(TestServices::styled_window().with_theme(theme));
        let root = TestRoot::with_global_styles(true, None);
        root.set_renderer(NullRenderer::new());
        root.layout_manager().execute_initial_layout_pass();

        Self { root, nav: None, _app: app }
    }

    pub fn iteration_setup(&mut self) {
        let nav = NavigationPage::new();
        nav.set_page_transition(None);
        self.root.set_child(&nav);
        self.root.layout_manager().execute_layout_pass();
        // Upstream blocks on the tasks: without a transition they have
        // completed when the calls return.
        nav.push_async(content_page("Root")).result().expect("the navigation was not canceled");
        nav.push_async(content_page("Page 2")).result().expect("the navigation was not canceled");
        self.root.layout_manager().execute_layout_pass();
        Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::LOADED));
        self.nav = Some(nav);
    }

    fn nav(&self) -> &Ref<NavigationPage> {
        self.nav.as_ref().expect("the iteration setup ran")
    }

    /// Push a page onto a stack that already has two entries (depth 2 -> 3).
    pub fn push(&self) {
        self.nav().push_async(content_page("Page 3")).result().expect("the navigation was not canceled");
        self.root.layout_manager().execute_layout_pass();
    }

    /// Pop the top page from a stack with two entries (depth 2 -> 1).
    pub fn pop(&self) {
        self.nav().pop_async().result().expect("the navigation was not canceled");
        self.root.layout_manager().execute_layout_pass();
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("navigation", "NavigationPageCreationBenchmark");
    class.benchmark("create", "", NavigationPageCreationBenchmark::new, |b| b.create());

    let mut class = registry.class("navigation", "NavigationPageStackBenchmark");
    class.benchmark_with_iteration_setup(
        "push",
        "",
        NavigationPageStackBenchmark::new,
        |b| b.iteration_setup(),
        |b| b.push(),
    );
    class.benchmark_with_iteration_setup(
        "pop",
        "",
        NavigationPageStackBenchmark::new,
        |b| b.iteration_setup(),
        |b| b.pop(),
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn navigation_page_creation_benchmark() {
        crate::harness::smoke_class(super::register, "NavigationPageCreationBenchmark");
    }

    #[test]
    fn navigation_page_stack_benchmark() {
        crate::harness::smoke_class(super::register, "NavigationPageStackBenchmark");
    }
}
