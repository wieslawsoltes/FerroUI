//! Port of `Pages/DrawerPage/DrawerPagePerformancePage.xaml.cs`: the class of the document
//! `Pages/DrawerPage/DrawerPagePerformancePage.xaml`.
//!
//! Deviation (DEVIATIONS.md, ControlCatalog sample): the heap is not measured and nothing is
//! collected (see `navigation_performance_monitor_helper.rs`); the text of a detail page that
//! describes the garbage collector says what happens here instead.

use crate::markup::xaml_class;
use crate::pages::navigation_demo_helper::{value_text, NavigationDemoHelper};
use crate::pages::navigation_performance_monitor_helper::{NavigationPerformanceMonitorHelper, StackRow};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl,
    VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    Button, CheckBox, ContentControlImpl, ContentPage, Control, ControlImpl, ControlImplExt, DrawerPage, Page,
    ScrollViewer, StackPanel, TextBlock, UserControl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[repr(C)]
pub struct DrawerPagePerformancePage {
    base: UserControl,
    perf: NavigationPerformanceMonitorHelper,
    detail_history: RefCell<Vec<String>>,
    swap_counter: Cell<i32>,
    page_counter: Cell<i32>,
    history_row_cache: RefCell<Vec<StackRow>>,
}

ferro_class!(DrawerPagePerformancePage: UserControl);
ferro_impl_classes!(
    DrawerPagePerformancePage: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(DrawerPagePerformancePage {
    new: DrawerPagePerformancePage::new,
    markup: {
        methods: [
            fn OnMenuItemClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPagePerformancePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_menu_item_click(&sender, e.as_routed_event_args())
                },
            fn OnSwapDetail(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPagePerformancePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_swap_detail(&sender, e.as_routed_event_args())
                },
            fn OnSwap5(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPagePerformancePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_swap5(&sender, e.as_routed_event_args())
                },
            fn OnToggleDrawer(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPagePerformancePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_toggle_drawer(&sender, e.as_routed_event_args())
                },
            fn OnForceGC(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPagePerformancePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_force_gc(&sender, e.as_routed_event_args())
                },
            fn OnClearLog(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPagePerformancePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_clear_log(&sender, e.as_routed_event_args())
                },
            fn OnAutoRefreshChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPagePerformancePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_auto_refresh_changed(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(DrawerPagePerformancePage, "/Pages/DrawerPage/DrawerPagePerformancePage.xaml");

impl ControlImpl for DrawerPagePerformancePage {
    fn on_loaded(this: &Self, e: &RoutedEventArgs) {
        Self::parent_on_loaded(this, e);

        this.perf.op_stopwatch.restart();
        this.perf.track_page(&this.get_control::<ContentPage>("DetailPage").upcast::<Page>());
        this.detail_history.borrow_mut().push(String::from("Home"));
        this.perf.op_stopwatch.stop();

        this.log("Init", "Initial detail page: Home");
        this.refresh_all();
    }

    fn on_unloaded(this: &Self, e: &RoutedEventArgs) {
        Self::parent_on_unloaded(this, e);
        this.perf.stop_auto_refresh();
    }
}

/// `sender as Button`.
fn as_button(sender: &Option<BoxedValue>) -> Option<Ref<Button>> {
    sender.as_ref().and_then(|sender| ValueTypes::as_object(&**sender)).and_then(|sender| sender.cast::<Button>())
}

impl DrawerPagePerformancePage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            perf: NavigationPerformanceMonitorHelper::new(),
            detail_history: RefCell::new(Vec::new()),
            swap_counter: Cell::new(0),
            page_counter: Cell::new(0),
            history_row_cache: RefCell::new(Vec::new()),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    fn drawer_page_control(&self) -> Ref<DrawerPage> {
        self.get_control::<DrawerPage>("DrawerPageControl")
    }

    fn log_panel(&self) -> Ref<StackPanel> {
        self.get_control::<StackPanel>("LogPanel")
    }

    fn text(&self, name: &str) -> Ref<TextBlock> {
        self.get_control::<TextBlock>(name)
    }

    fn on_menu_item_click(&self, sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let Some(button) = as_button(sender) else {
            return;
        };
        let item = value_text(&button.tag()).unwrap_or_else(|| String::from("Home"));

        self.perf.op_stopwatch.restart();
        self.swap_detail_to(&item);
        self.drawer_page_control().set_is_open(false);
        self.perf.stop_metrics(&self.text("LastOpTimeText"));
    }

    fn on_swap_detail(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.page_counter.set(self.page_counter.get() + 1);
        self.perf.op_stopwatch.restart();
        self.swap_detail_to(&format!("Page {}", self.page_counter.get()));
        self.perf.stop_metrics(&self.text("LastOpTimeText"));
    }

    fn on_swap5(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.perf.op_stopwatch.restart();
        for _ in 0..5 {
            self.page_counter.set(self.page_counter.get() + 1);
            self.swap_detail_to(&format!("Page {}", self.page_counter.get()));
        }
        self.perf.stop_metrics(&self.text("LastOpTimeText"));
    }

    fn on_toggle_drawer(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.perf.op_stopwatch.restart();
        let drawer_page_control = self.drawer_page_control();
        drawer_page_control.set_is_open(!drawer_page_control.is_open());
        self.perf.stop_metrics(&self.text("LastOpTimeText"));
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

    fn swap_detail_to(&self, title: &str) {
        self.swap_counter.set(self.swap_counter.get() + 1);
        let swap_counter = self.swap_counter.get();

        let new_detail = NavigationDemoHelper::make_page(
            title,
            &format!(
                "Detail swap #{swap_counter}\n\nEach detail page allocates ~50 KB. \
                 Swap pages and watch the live page instances. The old detail page is replaced \
                 and freed as soon as the last reference to it is dropped."
            ),
            swap_counter,
        );
        new_detail.set_tag(Some(Rc::new(vec![0u8; 51200]) as BoxedValue));
        self.perf.track_page(&new_detail.clone().upcast::<Page>());
        self.detail_history.borrow_mut().push(title.to_string());

        self.drawer_page_control().set_content(Some(Control::boxed(new_detail)));

        self.log("Swap", &format!("Detail \u{2192} \"{title}\""));
        self.refresh_all();
    }

    fn refresh_all(&self) {
        let current_detail = self
            .drawer_page_control()
            .content()
            .and_then(|content| Control::from_boxed(&content))
            .and_then(|content| content.cast::<Page>());
        let header = current_detail
            .and_then(|current_detail| current_detail.header())
            .map_or_else(|| String::from("\u{2014}"), |header| value_text(&Some(header)).unwrap_or_default());
        self.text("CurrentDetailText").set_text(Some(&format!("Current Detail: {header}")));
        self.text("SwapCountText").set_text(Some(&format!("Detail Swaps: {}", self.swap_counter.get())));
        self.text("LiveInstancesText")
            .set_text(Some(&format!("Live Page Instances: {}", self.perf.count_live_instances())));
        self.text("TotalCreatedText").set_text(Some(&format!("Total Pages Created: {}", self.perf.total_created())));
        self.perf.update_heap_delta(&self.text("ManagedMemoryText"), &self.text("MemoryDeltaText"));
        self.refresh_history();
    }

    fn refresh_history(&self) {
        let detail_history = self.detail_history.borrow();
        let start = detail_history.len().saturating_sub(10);
        let count = detail_history.len() - start;

        NavigationPerformanceMonitorHelper::sync_rows(
            &self.get_control::<StackPanel>("HistoryPanel"),
            &self.history_row_cache,
            count,
        );

        let history_row_cache = self.history_row_cache.borrow();
        for (display_idx, row) in history_row_cache.iter().enumerate().take(count) {
            let history_idx = detail_history.len() - 1 - display_idx;
            let is_current = history_idx == detail_history.len() - 1;

            NavigationPerformanceMonitorHelper::update_stack_row(
                row,
                history_idx as i32,
                &detail_history[history_idx],
                is_current,
                false,
            );
        }
    }

    fn log(&self, action: &str, detail: &str) {
        self.perf.log_operation(
            action,
            detail,
            &self.log_panel(),
            &self.get_control::<ScrollViewer>("LogScrollViewer"),
            None,
        );
    }
}
