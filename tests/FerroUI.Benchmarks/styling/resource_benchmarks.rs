//! Finding a resource from a control ten decorators below a root whose
//! global styles hold a few hundred styles with resources: a resource of the
//! first style, one of the last style and one that does not exist.

use crate::harness::Registry;
use crate::test_root::TestRoot;
use crate::test_styles::TestStyles;
use ferroui_base::animation::{IClock, IGlobalClock, PlayState, TimeSpan};
use ferroui_base::controls::ResourceKey;
use ferroui_base::platform::{StandardAssetLoader, StandardRuntimePlatform};
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::styling::{IStyle, Style, Styles};
use ferroui_base::Ref;
use ferroui_controls::testing::{
    HeadlessCursorFactoryStub, MockWindowingPlatform, NullRenderer, TestServices, UnitTestApplication,
    UnitTestApplicationScope,
};
use ferroui_controls::{Button, Control, Decorator};
use std::cell::Cell;
use std::rc::Rc;

const LOOKUP_COUNT: i32 = 100;

pub struct ResourceBenchmarks {
    search_start: Ref<Control>,
    /// The tree the control is in: a control does not keep its parents.
    _root: Ref<TestRoot>,
    /// The keys of the lookups (constant strings upstream).
    pre_theme: ResourceKey,
    post_theme: ResourceKey,
    not_present: ResourceKey,
    /// Disposed when the benchmark is dropped, after the tree (the fields
    /// drop in the order of their declaration).
    _app: UnitTestApplicationScope,
}

impl ResourceBenchmarks {
    fn create_app() -> UnitTestApplicationScope {
        let services = TestServices {
            asset_loader: Some(Rc::new(StandardAssetLoader::new(None))),
            global_clock: Some(Rc::new(MockGlobalClock::new())),
            platform: Some(Rc::new(StandardRuntimePlatform::new())),
            standard_cursor_factory: Some(Rc::new(HeadlessCursorFactoryStub)),
            theme: Some(Rc::new(Self::create_theme)),
            windowing_platform: Some(MockWindowingPlatform::new()),
            ..TestServices::default()
        };

        UnitTestApplication::start(services)
    }

    fn create_theme() -> Rc<dyn IStyle> {
        let pre_host = Style::new();
        pre_host.resources().add("preTheme", None);

        let post_host = Style::new();
        post_host.resources().add("postTheme", None);

        let styles = Styles::new();
        styles.add(&pre_host);
        styles.add(&TestStyles::new(50, 3, 5, 0));
        styles.add(&post_host);
        styles.into()
    }

    pub fn new() -> Self {
        let search_start: Ref<Control> = Button::new().upcast();

        let app = Self::create_app();

        let root = TestRoot::with_global_styles(true, None);
        root.set_renderer(NullRenderer::new());

        let mut current: Ref<Decorator> = root.clone().upcast();

        for _ in 0..10 {
            let child = Decorator::new();

            current.set_child(&child);

            current = child;
        }

        current.set_child(&search_start);

        Self {
            search_start,
            _root: root,
            pre_theme: ResourceKey::from("preTheme"),
            post_theme: ResourceKey::from("postTheme"),
            not_present: ResourceKey::from("notPresent"),
            _app: app,
        }
    }

    pub fn find_pre_resource(&self) {
        for _ in 0..LOOKUP_COUNT {
            let _ = self.search_start.find_resource(&self.pre_theme);
        }
    }

    pub fn find_post_resource(&self) {
        for _ in 0..LOOKUP_COUNT {
            let _ = self.search_start.find_resource(&self.post_theme);
        }
    }

    pub fn find_not_existing_resource(&self) {
        for _ in 0..LOOKUP_COUNT {
            let _ = self.search_start.find_resource(&self.not_present);
        }
    }
}

/// The global clock of the application of the benchmark (the mock clock of
/// the upstream test library): it ticks when it is told to, which the
/// benchmark never does.
struct MockGlobalClock {
    subject: LightweightSubject<TimeSpan>,
    play_state: Cell<PlayState>,
}

impl MockGlobalClock {
    fn new() -> Self {
        Self { subject: LightweightSubject::new(), play_state: Cell::new(PlayState::Run) }
    }
}

impl IObservable<TimeSpan> for MockGlobalClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
        self.subject.subscribe(observer)
    }
}

impl IClock for MockGlobalClock {
    fn play_state(&self) -> PlayState {
        self.play_state.get()
    }

    fn set_play_state(&self, value: PlayState) {
        self.play_state.set(value)
    }
}

impl IGlobalClock for MockGlobalClock {}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("styling", "ResourceBenchmarks");
    class.benchmark("find_pre_resource", "", ResourceBenchmarks::new, |b| b.find_pre_resource());
    class.benchmark("find_post_resource", "", ResourceBenchmarks::new, |b| b.find_post_resource());
    class.benchmark("find_not_existing_resource", "", ResourceBenchmarks::new, |b| b.find_not_existing_resource());
}

#[cfg(test)]
mod tests {
    #[test]
    fn resource_benchmarks() {
        crate::harness::smoke_class(super::register, "ResourceBenchmarks");
    }
}
