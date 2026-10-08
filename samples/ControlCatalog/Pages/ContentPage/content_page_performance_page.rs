//! Port of `Pages/ContentPage/ContentPagePerformancePage.xaml.cs`: the class of the document
//! `Pages/ContentPage/ContentPagePerformancePage.xaml`.
//!
//! Deviation (DEVIATIONS.md, ControlCatalog sample): the heap is not measured and nothing is
//! collected (see `navigation_performance_monitor_helper.rs`); the texts of the page that
//! describe the garbage collector say what happens here instead.

use crate::markup::xaml_class;
use crate::pages::navigation_demo_helper::{value_text, NavigationDemoHelper};
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
    CheckBox, ComboBox, ContentControlImpl, ContentPage, ControlImpl, ControlImplExt, NavigationPage, Page,
    ScrollViewer, StackPanel, TextBlock, UserControl,
};
use mini_mvvm::start_async;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const ALLOCATION_SIZES: [usize; 3] = [51_200, 512_000, 2_097_152];

#[repr(C)]
pub struct ContentPagePerformancePage {
    base: UserControl,
    perf: NavigationPerformanceMonitorHelper,
    page_counter: Cell<i32>,
    stack_row_cache: RefCell<Vec<StackRow>>,
}

ferro_class!(ContentPagePerformancePage: UserControl);
ferro_impl_classes!(
    ContentPagePerformancePage: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(ContentPagePerformancePage {
    new: ContentPagePerformancePage::new,
    markup: {
        methods: [
            fn OnPush(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ContentPagePerformancePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_push(&sender, e.as_routed_event_args())
                },
            fn OnPush5(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ContentPagePerformancePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_push5(&sender, e.as_routed_event_args())
                },
            fn OnPop(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ContentPagePerformancePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_pop(&sender, e.as_routed_event_args())
                },
            fn OnPopToRoot(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ContentPagePerformancePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_pop_to_root(&sender, e.as_routed_event_args())
                },
            fn OnForceGC(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ContentPagePerformancePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_force_gc(&sender, e.as_routed_event_args())
                },
            fn OnClearLog(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ContentPagePerformancePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_clear_log(&sender, e.as_routed_event_args())
                },
            fn OnAutoRefreshChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ContentPagePerformancePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_auto_refresh_changed(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(ContentPagePerformancePage, "/Pages/ContentPage/ContentPagePerformancePage.xaml");

impl ControlImpl for ContentPagePerformancePage {
    /// `async void`.
    fn on_loaded(this: &Self, e: &RoutedEventArgs) {
        Self::parent_on_loaded(this, e);

        // The handlers belong to a child of the page: they hold the page weakly.
        let nav_page = this.nav_page();
        let on_stack_changed = || {
            let weak = this.to_ref().downgrade();
            move || {
                if let Some(this) = weak.upgrade() {
                    this.refresh_all();
                }
            }
        };
        let pushed = on_stack_changed();
        nav_page.pushed(move |_e| pushed());
        let popped = on_stack_changed();
        nav_page.popped(move |_e| popped());
        let popped_to_root = on_stack_changed();
        nav_page.popped_to_root(move |_e| popped_to_root());

        this.page_counter.set(this.page_counter.get() + 1);
        let this = this.to_ref();
        drop(start_async(async move {
            let page = this.build_page("Home", this.page_counter.get());
            if this.nav_page().push_async(page).await.is_err() {
                return;
            }
            this.log("Init", "Pushed root page");
        }));
    }

    fn on_unloaded(this: &Self, e: &RoutedEventArgs) {
        Self::parent_on_unloaded(this, e);
        this.perf.stop_auto_refresh();
    }
}

impl ContentPagePerformancePage {
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
        this
    }

    fn nav_page(&self) -> Ref<NavigationPage> {
        self.get_control::<NavigationPage>("NavPage")
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
            let page = this.build_page(&format!("Page {page_counter}"), page_counter);
            let header = value_text(&page.header()).unwrap_or_default();
            if this.nav_page().push_async(page).await.is_err() {
                return;
            }
            this.log("Push", &format!("Pushed \"{header}\""));
        }));
    }

    /// `async void`.
    fn on_push5(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let this = self.to_ref();
        drop(start_async(async move {
            let first = this.page_counter.get() + 1;
            for _ in 0..5 {
                this.page_counter.set(this.page_counter.get() + 1);
                let page_counter = this.page_counter.get();
                let page = this.build_page(&format!("Page {page_counter}"), page_counter);
                if this.nav_page().push_async(page).await.is_err() {
                    return;
                }
            }
            this.log("Push \u{00D7}5", &format!("Pushed pages {first}\u{2013}{}", this.page_counter.get()));
        }));
    }

    /// `async void`.
    fn on_pop(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let this = self.to_ref();
        drop(start_async(async move {
            let nav_page = this.nav_page();
            if nav_page.stack_depth() > 1 {
                let popped = nav_page.current_page();
                if nav_page.pop_async().await.is_err() {
                    return;
                }
                let header = popped.and_then(|popped| value_text(&popped.header())).unwrap_or_default();
                this.log("Pop", &format!("Popped \"{header}\""));
            }
        }));
    }

    /// `async void`.
    fn on_pop_to_root(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let this = self.to_ref();
        drop(start_async(async move {
            let nav_page = this.nav_page();
            if nav_page.stack_depth() > 1 {
                let removed = nav_page.stack_depth() - 1;
                if nav_page.pop_to_root_async().await.is_err() {
                    return;
                }
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
        let nav_page = self.nav_page();
        self.text("StackDepthText").set_text(Some(&format!("Stack Depth: {}", nav_page.stack_depth())));
        self.text("LiveInstancesText")
            .set_text(Some(&format!("Live Page Instances: {}", self.perf.count_live_instances())));
        self.text("TotalCreatedText").set_text(Some(&format!("Total Pages Created: {}", self.perf.total_created())));
        self.perf.update_heap_delta(&self.text("ManagedMemoryText"), &self.text("MemoryDeltaText"));

        NavigationPerformanceMonitorHelper::refresh_stack_panel(
            &self.get_control::<StackPanel>("StackVisPanel"),
            &self.stack_row_cache,
            &nav_page.navigation_stack(),
            nav_page.current_page().as_ref(),
        );
    }

    fn log(&self, action: &str, detail: &str) {
        self.perf.log_operation(
            action,
            detail,
            &self.log_panel(),
            &self.get_control::<ScrollViewer>("LogScrollViewer"),
            Some(&format!("depth {}", self.nav_page().stack_depth())),
        );
    }

    fn build_page(&self, title: &str, index: i32) -> Ref<ContentPage> {
        let selected_index = self.get_control::<ComboBox>("WeightCombo").selected_index();
        let weight_index = if selected_index >= 0 { selected_index } else { 0 };
        let alloc_bytes = ALLOCATION_SIZES[weight_index.clamp(0, ALLOCATION_SIZES.len() as i32 - 1) as usize];
        let weight_label = match weight_index {
            1 => "~500 KB",
            2 => "~2 MB",
            _ => "~50 KB",
        };

        let page = NavigationDemoHelper::make_page(
            title,
            &format!(
                "Stack position #{index}  \u{00B7}  Weight: {weight_label}\n\n\
                 Pop this page to release the weight shown above. \
                 Live Instances decreases as soon as the last reference to the page is dropped."
            ),
            index,
        );
        page.set_tag(Some(Rc::new(vec![0u8; alloc_bytes]) as BoxedValue));
        self.perf.track_page(&page.clone().upcast::<Page>());
        page
    }
}
