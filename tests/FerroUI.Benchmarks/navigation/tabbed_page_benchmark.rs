//! Creating a tabbed page, and switching its selected tab.
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
use ferroui_controls::{ContentPage, Page, TabbedPage};
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

/// Measures the cost of creating a TabbedPage and applying its template as
/// the number of tabs grows.
pub struct TabbedPageCreationBenchmark {
    root: Ref<TestRoot>,
    tab_count: i32,
    /// Disposed when the benchmark is dropped, after the tree (the fields
    /// drop in the order of their declaration).
    _app: UnitTestApplicationScope,
}

impl TabbedPageCreationBenchmark {
    pub fn new(tab_count: i32) -> Self {
        let app = UnitTestApplication::start(TestServices::styled_window().with_theme(theme));
        let root = TestRoot::with_global_styles(true, None);
        root.set_renderer(NullRenderer::new());
        root.layout_manager().execute_initial_layout_pass();

        Self { root, tab_count, _app: app }
    }

    pub fn create(&self) {
        let tp = TabbedPage::new();
        tp.set_page_transition(None);
        for i in 0..self.tab_count {
            tp.pages().expect("the tabbed page has pages").add(content_page(format!("Tab {i}")));
        }
        self.root.set_child(&tp);
        self.root.layout_manager().execute_layout_pass();
        Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::LOADED));
    }
}

/// Measures the cost of switching the selected tab on a TabbedPage with
/// five tabs (no transition).
pub struct TabbedPageSwitchBenchmark {
    root: Ref<TestRoot>,
    /// Set by the iteration setup.
    tabbed_page: Option<Ref<TabbedPage>>,
    /// Disposed when the benchmark is dropped, after the tree (the fields
    /// drop in the order of their declaration).
    _app: UnitTestApplicationScope,
}

impl TabbedPageSwitchBenchmark {
    pub fn new() -> Self {
        let app = UnitTestApplication::start(TestServices::styled_window().with_theme(theme));
        let root = TestRoot::with_global_styles(true, None);
        root.set_renderer(NullRenderer::new());
        root.layout_manager().execute_initial_layout_pass();

        Self { root, tabbed_page: None, _app: app }
    }

    pub fn iteration_setup(&mut self) {
        let tabbed_page = TabbedPage::new();
        tabbed_page.set_page_transition(None);
        for i in 0..5 {
            tabbed_page.pages().expect("the tabbed page has pages").add(content_page(format!("Tab {i}")));
        }
        self.root.set_child(&tabbed_page);
        self.root.layout_manager().execute_layout_pass();
        tabbed_page.set_selected_index(0);
        self.root.layout_manager().execute_layout_pass();
        Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::LOADED));
        self.tabbed_page = Some(tabbed_page);
    }

    /// Switch from tab 0 to tab 1 (no transition).
    pub fn switch_tab(&self) {
        self.tabbed_page.as_ref().expect("the iteration setup ran").set_selected_index(1);
        self.root.layout_manager().execute_layout_pass();
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("navigation", "TabbedPageCreationBenchmark");
    for tab_count in [2, 5, 10] {
        class.benchmark(
            "create",
            format!("TabCount={tab_count}"),
            move || TabbedPageCreationBenchmark::new(tab_count),
            |b| b.create(),
        );
    }

    let mut class = registry.class("navigation", "TabbedPageSwitchBenchmark");
    class.benchmark_with_iteration_setup(
        "switch_tab",
        "",
        TabbedPageSwitchBenchmark::new,
        |b| b.iteration_setup(),
        |b| b.switch_tab(),
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn tabbed_page_creation_benchmark() {
        crate::harness::smoke_class(super::register, "TabbedPageCreationBenchmark");
    }

    #[test]
    fn tabbed_page_switch_benchmark() {
        crate::harness::smoke_class(super::register, "TabbedPageSwitchBenchmark");
    }
}
