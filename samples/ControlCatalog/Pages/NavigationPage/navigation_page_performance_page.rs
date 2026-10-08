//! Port of `Pages/NavigationPage/NavigationPagePerformancePage.xaml.cs`: the class of the document
//! `Pages/NavigationPage/NavigationPagePerformancePage.xaml`.
//!
//! Deviation (DEVIATIONS.md, ControlCatalog sample): the heap is not measured and nothing is
//! collected (see `navigation_performance_monitor_helper.rs`).

use crate::markup::xaml_class;
use crate::pages::navigation_demo_helper::value_text;
use crate::pages::navigation_performance_monitor_helper::{NavigationPerformanceMonitorHelper, StackRow};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl,
    VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    CheckBox, ContentControlImpl, ControlImpl, ControlImplExt, NavigationPage, ScrollViewer, StackPanel, TextBlock,
    UserControl,
};
use mini_mvvm::start_async;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The size of the dummy allocation of a page (the default of `BuildTrackedPage`).
const ALLOC_BYTES: usize = 51200;

#[repr(C)]
pub struct NavigationPagePerformancePage {
    base: UserControl,
    perf: NavigationPerformanceMonitorHelper,
    page_counter: Cell<i32>,
    stack_row_cache: RefCell<Vec<StackRow>>,
}

ferro_class!(NavigationPagePerformancePage: UserControl);
ferro_impl_classes!(
    NavigationPagePerformancePage: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(NavigationPagePerformancePage {
    new: NavigationPagePerformancePage::new,
    markup: {
        methods: [
            fn OnPush(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPagePerformancePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_push(&sender, e.as_routed_event_args())
                },
            fn OnPush5(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPagePerformancePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_push5(&sender, e.as_routed_event_args())
                },
            fn OnPop(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPagePerformancePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_pop(&sender, e.as_routed_event_args())
                },
            fn OnPopToRoot(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPagePerformancePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_pop_to_root(&sender, e.as_routed_event_args())
                },
            fn OnForceGC(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPagePerformancePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_force_gc(&sender, e.as_routed_event_args())
                },
            fn OnClearLog(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPagePerformancePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_clear_log(&sender, e.as_routed_event_args())
                },
            fn OnAutoRefreshChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPagePerformancePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_auto_refresh_changed(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(NavigationPagePerformancePage, "/Pages/NavigationPage/NavigationPagePerformancePage.xaml");

impl ControlImpl for NavigationPagePerformancePage {
    fn on_unloaded(this: &Self, e: &RoutedEventArgs) {
        Self::parent_on_unloaded(this, e);
        this.perf.stop_auto_refresh();
    }
}

impl NavigationPagePerformancePage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            perf: NavigationPerformanceMonitorHelper::new(),
            page_counter: Cell::new(0),
            stack_row_cache: RefCell::new(Vec::new()),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        // The handlers belong to a child of the page: they hold the page weakly.
        let demo_nav = this.demo_nav();
        let on_stack_changed = || {
            let weak = this.downgrade();
            move || {
                if let Some(this) = weak.upgrade() {
                    this.refresh_all();
                }
            }
        };
        let pushed = on_stack_changed();
        demo_nav.pushed(move |_e| pushed());
        let popped = on_stack_changed();
        demo_nav.popped(move |_e| popped());
        let popped_to_root = on_stack_changed();
        demo_nav.popped_to_root(move |_e| popped_to_root());

        drop(start_async(Self::initialize_async(this.clone())));
        this
    }

    async fn initialize_async(this: Ref<Self>) {
        this.perf.op_stopwatch.restart();
        this.page_counter.set(this.page_counter.get() + 1);
        let page = this.perf.build_tracked_page("Home", this.page_counter.get(), ALLOC_BYTES);
        if this.demo_nav().push_async_with_transition(page, None).await.is_err() {
            return;
        }
        this.perf.op_stopwatch.stop();
        this.log("Init", "Pushed root page");
    }

    fn demo_nav(&self) -> Ref<NavigationPage> {
        self.get_control::<NavigationPage>("DemoNav")
    }

    fn log_panel(&self) -> Ref<StackPanel> {
        self.get_control::<StackPanel>("LogPanel")
    }

    fn text(&self, name: &str) -> Ref<TextBlock> {
        self.get_control::<TextBlock>(name)
    }

    /// `async void`.
    fn on_push(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let this = self.to_ref();
        drop(start_async(async move {
            this.page_counter.set(this.page_counter.get() + 1);
            let page_counter = this.page_counter.get();
            let page = this.perf.build_tracked_page(&format!("Page {page_counter}"), page_counter, ALLOC_BYTES);
            let header = value_text(&page.header()).unwrap_or_default();
            this.perf.op_stopwatch.restart();
            if this.demo_nav().push_async(page).await.is_err() {
                return;
            }
            this.perf.stop_metrics(&this.text("LastOpTimeText"));
            this.log("Push", &format!("Pushed \"{header}\""));
        }));
    }

    /// `async void`.
    fn on_push5(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let this = self.to_ref();
        drop(start_async(async move {
            let first = this.page_counter.get() + 1;
            this.perf.op_stopwatch.restart();
            for _ in 0..5 {
                this.page_counter.set(this.page_counter.get() + 1);
                let page_counter = this.page_counter.get();
                let page = this.perf.build_tracked_page(&format!("Page {page_counter}"), page_counter, ALLOC_BYTES);
                if this.demo_nav().push_async(page).await.is_err() {
                    return;
                }
            }
            this.perf.stop_metrics(&this.text("LastOpTimeText"));
            this.log("Push \u{00D7}5", &format!("Pushed pages {first}\u{2013}{}", this.page_counter.get()));
        }));
    }

    /// `async void`.
    fn on_pop(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let this = self.to_ref();
        drop(start_async(async move {
            let demo_nav = this.demo_nav();
            if demo_nav.stack_depth() > 1 {
                let header = demo_nav.current_page().and_then(|page| value_text(&page.header())).unwrap_or_default();
                this.perf.op_stopwatch.restart();
                if demo_nav.pop_async().await.is_err() {
                    return;
                }
                this.perf.stop_metrics(&this.text("LastOpTimeText"));
                this.log("Pop", &format!("Popped \"{header}\""));
            }
        }));
    }

    /// `async void`.
    fn on_pop_to_root(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let this = self.to_ref();
        drop(start_async(async move {
            let demo_nav = this.demo_nav();
            if demo_nav.stack_depth() > 1 {
                let removed = demo_nav.stack_depth() - 1;
                this.perf.op_stopwatch.restart();
                if demo_nav.pop_to_root_async().await.is_err() {
                    return;
                }
                this.perf.stop_metrics(&this.text("LastOpTimeText"));
                this.log("PopToRoot", &format!("Removed {removed} page(s)"));
            }
        }));
    }

    fn on_force_gc(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.perf.force_gc(|| self.refresh_all());
        self.log("GC", "Refreshed: nothing to collect, a released page is freed at once");
    }

    fn on_clear_log(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.log_panel().children().clear();
    }

    fn on_auto_refresh_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        // The timer is held by the helper of the page: its handler holds the page weakly.
        let weak = self.to_ref().downgrade();
        self.perf.on_auto_refresh_changed(&self.get_control::<CheckBox>("AutoRefreshCheck"), move || {
            if let Some(this) = weak.upgrade() {
                this.refresh_all();
            }
        });
    }

    fn refresh_all(&self) {
        let demo_nav = self.demo_nav();
        self.text("StackDepthText").set_text(Some(&format!("Stack Depth: {}", demo_nav.stack_depth())));
        self.text("LiveInstancesText")
            .set_text(Some(&format!("Live Page Instances: {}", self.perf.count_live_instances())));
        self.text("TotalCreatedText").set_text(Some(&format!("Total Pages Created: {}", self.perf.total_created())));
        self.perf.update_heap_delta(&self.text("ManagedMemoryText"), &self.text("MemoryDeltaText"));

        NavigationPerformanceMonitorHelper::refresh_stack_panel(
            &self.get_control::<StackPanel>("StackItemsPanel"),
            &self.stack_row_cache,
            &demo_nav.navigation_stack(),
            demo_nav.current_page().as_ref(),
        );
    }

    fn log(&self, action: &str, detail: &str) {
        self.perf.log_operation(
            action,
            detail,
            &self.log_panel(),
            &self.get_control::<ScrollViewer>("LogScrollViewer"),
            Some(&format!("depth {}", self.demo_nav().stack_depth())),
        );
    }
}
