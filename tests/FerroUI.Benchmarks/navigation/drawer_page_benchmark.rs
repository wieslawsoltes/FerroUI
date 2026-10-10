//! Creating a drawer page, and opening and closing one.
//!
//! The upstream constructors first register the parsers of the resource URI
//! scheme with the URI type of their platform; the URI type of the framework
//! reads that scheme itself, so there is nothing to register here.

use crate::harness::Registry;
use crate::test_root::TestRoot;
use ferroui_base::animation::{ClockBase, IClock, IGlobalClock, PlayState, TimeSpan};
use ferroui_base::reactive::{IDisposable, IObservable, IObserver};
use ferroui_base::styling::{IStyle, Styles};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::Ref;
use ferroui_controls::testing::{NullRenderer, TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Control, DrawerPage, TextBlock};
use ferroui_themes_fluent::FluentTheme;
use std::rc::Rc;

/// The global clock of the upstream unit test library: a clock that ticks
/// when it is told to, which the benchmarks never do.
struct MockGlobalClock {
    base: ClockBase,
}

impl MockGlobalClock {
    fn new() -> Self {
        Self { base: ClockBase::new() }
    }

    #[allow(dead_code)]
    fn pulse(&self, system_time: TimeSpan) {
        self.base.pulse(system_time, || {});
    }
}

impl IObservable<TimeSpan> for MockGlobalClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
        self.base.subscribe(observer)
    }
}

impl IClock for MockGlobalClock {
    fn play_state(&self) -> PlayState {
        self.base.play_state()
    }

    fn set_play_state(&self, value: PlayState) {
        self.base.set_play_state(value)
    }
}

impl IGlobalClock for MockGlobalClock {}

/// The theme of the application: a styles collection that holds the Fluent
/// theme.
fn theme() -> Rc<dyn IStyle> {
    let styles = Styles::new();
    styles.add(FluentTheme::new().as_style());
    styles.into()
}

/// A text block with the text `text`.
fn text_block(text: &str) -> Ref<TextBlock> {
    let text_block = TextBlock::new();
    text_block.set_text(Some(text));
    text_block
}

/// Measures the cost of creating a DrawerPage and applying its template.
pub struct DrawerPageCreationBenchmark {
    root: Ref<TestRoot>,
    /// Disposed when the benchmark is dropped, after the tree (the fields
    /// drop in the order of their declaration).
    _app: UnitTestApplicationScope,
}

impl DrawerPageCreationBenchmark {
    pub fn new() -> Self {
        let app = UnitTestApplication::start(
            TestServices::styled_window().with_theme(theme).with_global_clock(Rc::new(MockGlobalClock::new())),
        );
        let root = TestRoot::with_global_styles(true, None);
        root.set_renderer(NullRenderer::new());
        root.layout_manager().execute_initial_layout_pass();

        Self { root, _app: app }
    }

    pub fn create(&self) {
        let drawer = DrawerPage::new();
        drawer.set_drawer(Some(Control::boxed(text_block("Drawer"))));
        drawer.set_content(Some(Control::boxed(text_block("Content"))));
        self.root.set_child(&drawer);
        self.root.layout_manager().execute_layout_pass();
        Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::LOADED));
    }
}

/// Measures the cost of opening and then closing a DrawerPage in a single
/// iteration. The DrawerPage starts closed each iteration; the benchmark
/// performs one open and one close so both layout passes are captured
/// symmetrically.
pub struct DrawerPageToggleBenchmark {
    root: Ref<TestRoot>,
    /// Set by the iteration setup.
    drawer: Option<Ref<DrawerPage>>,
    /// Disposed when the benchmark is dropped, after the tree (the fields
    /// drop in the order of their declaration).
    _app: UnitTestApplicationScope,
}

impl DrawerPageToggleBenchmark {
    pub fn new() -> Self {
        let app = UnitTestApplication::start(
            TestServices::styled_window().with_theme(theme).with_global_clock(Rc::new(MockGlobalClock::new())),
        );
        let root = TestRoot::with_global_styles(true, None);
        root.set_renderer(NullRenderer::new());
        root.layout_manager().execute_initial_layout_pass();

        Self { root, drawer: None, _app: app }
    }

    pub fn iteration_setup(&mut self) {
        let drawer = DrawerPage::new();
        drawer.set_drawer(Some(Control::boxed(text_block("Drawer"))));
        drawer.set_content(Some(Control::boxed(text_block("Content"))));
        drawer.set_is_open(false);
        self.root.set_child(&drawer);
        self.root.layout_manager().execute_layout_pass();
        Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::LOADED));
        self.drawer = Some(drawer);
    }

    /// Open the drawer then close it; captures the full toggle cycle cost.
    pub fn open_and_close(&self) {
        let drawer = self.drawer.as_ref().expect("the iteration setup ran");

        drawer.set_is_open(true);
        self.root.layout_manager().execute_layout_pass();

        drawer.set_is_open(false);
        self.root.layout_manager().execute_layout_pass();
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("navigation", "DrawerPageCreationBenchmark");
    class.benchmark("create", "", DrawerPageCreationBenchmark::new, |b| b.create());

    let mut class = registry.class("navigation", "DrawerPageToggleBenchmark");
    class.benchmark_with_iteration_setup(
        "open_and_close",
        "",
        DrawerPageToggleBenchmark::new,
        |b| b.iteration_setup(),
        |b| b.open_and_close(),
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn drawer_page_creation_benchmark() {
        crate::harness::smoke_class(super::register, "DrawerPageCreationBenchmark");
    }

    #[test]
    fn drawer_page_toggle_benchmark() {
        crate::harness::smoke_class(super::register, "DrawerPageToggleBenchmark");
    }
}
