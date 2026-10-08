//! Port of `Pages/TabbedPage/TabbedPagePerformancePage.xaml.cs`: the class of the document
//! `Pages/TabbedPage/TabbedPagePerformancePage.xaml`.
//!
//! Deviation (DEVIATIONS.md, ControlCatalog sample): the heap and the total of the allocations
//! are not measured and nothing is collected (see `navigation_performance_monitor_helper.rs`).

use crate::markup::{user_control_class, xaml_class};
use crate::pages::navigation_demo_helper::boxed_text;
use crate::pages::navigation_performance_monitor_helper::{NavigationPerformanceMonitorHelper, NOT_AVAILABLE};
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{ContentPage, Control, Page, PageList, TabbedPage, TextBlock, UserControl};
use std::cell::Cell;
use std::rc::Rc;

#[repr(C)]
pub struct TabbedPagePerformancePage {
    base: UserControl,
    perf: NavigationPerformanceMonitorHelper,
    counter: Cell<i32>,
}

user_control_class!(TabbedPagePerformancePage);
ferro_class_info!(TabbedPagePerformancePage {
    new: TabbedPagePerformancePage::new,
    markup: {
        methods: [
            fn OnAdd5(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPagePerformancePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_add5(&sender, e.as_routed_event_args())
                },
            fn OnAdd20(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPagePerformancePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_add20(&sender, e.as_routed_event_args())
                },
            fn OnRemove5(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPagePerformancePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_remove5(&sender, e.as_routed_event_args())
                },
            fn OnClearAll(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPagePerformancePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_clear_all(&sender, e.as_routed_event_args())
                },
            fn OnForceGC(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPagePerformancePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_force_gc(&sender, e.as_routed_event_args())
                },
            fn OnRefresh(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPagePerformancePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_refresh(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(TabbedPagePerformancePage, "/Pages/TabbedPage/TabbedPagePerformancePage.xaml");

impl TabbedPagePerformancePage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), perf: NavigationPerformanceMonitorHelper::new(), counter: Cell::new(0) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        // The handler of an event of the control itself holds it weakly.
        let weak = this.downgrade();
        this.loaded(move |_, _| {
            let Some(this) = weak.upgrade() else { return };
            this.add_tabs(5);
            // The handler belongs to a child of the control: it holds the control weakly.
            let weak = this.downgrade();
            this.demo_tabs().selection_changed(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.refresh_stats();
                }
            });
        });
        this
    }

    fn demo_tabs(&self) -> Ref<TabbedPage> {
        self.get_control::<TabbedPage>("DemoTabs")
    }

    fn text(&self, name: &str) -> Ref<TextBlock> {
        self.get_control::<TextBlock>(name)
    }

    /// `(IList)DemoTabs.Pages!`.
    ///
    /// # Panics
    /// Panics if the tabbed page has no list of pages (a null reference in the managed
    /// original).
    fn pages(&self) -> PageList {
        self.demo_tabs().pages().expect("the pages of the tabbed page")
    }

    fn add_tabs(&self, count: i32) {
        let pages = self.pages();
        self.perf.op_stopwatch.restart();
        for _ in 0..count {
            self.counter.set(self.counter.get() + 1);
            let idx = self.counter.get();

            let text = TextBlock::new();
            text.set_text(Some(&format!("Tab {idx}")));
            text.set_horizontal_alignment(HorizontalAlignment::Center);
            text.set_vertical_alignment(VerticalAlignment::Center);
            text.set_font_size(18.0);
            text.set_opacity(0.7);

            let page = ContentPage::new();
            page.set_header(Some(boxed_text(&format!("T{idx}"))));
            page.set_content(Some(Control::boxed(text)));
            page.set_tag(Some(Rc::new(vec![0u8; 51200]) as BoxedValue));
            let page = page.upcast::<Page>();
            self.perf.track_page(&page);
            pages.add(page);
        }

        self.perf.stop_metrics(&self.text("LastOpTimeText"));
        self.refresh_stats();
    }

    fn remove_tabs(&self, count: i32) {
        let pages = self.pages();
        self.perf.op_stopwatch.restart();
        let mut i = 0;
        while i < count && pages.count() > 0 {
            pages.remove_at(pages.count() - 1);
            i += 1;
        }

        self.perf.stop_metrics(&self.text("LastOpTimeText"));
        self.refresh_stats();
    }

    fn on_add5(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.add_tabs(5);
    }

    fn on_add20(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.add_tabs(20);
    }

    fn on_remove5(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.remove_tabs(5);
    }

    fn on_clear_all(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let pages = self.pages();
        self.perf.op_stopwatch.restart();
        while pages.count() > 0 {
            pages.remove_at(pages.count() - 1);
        }
        self.perf.stop_metrics(&self.text("LastOpTimeText"));
        self.refresh_stats();
    }

    fn on_force_gc(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.perf.force_gc(|| self.refresh_stats());
    }

    fn on_refresh(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.refresh_stats();
    }

    fn refresh_stats(&self) {
        let pages = self.pages();
        self.text("TabCountText").set_text(Some(&format!("Tab count: {}", pages.count())));
        self.text("LiveCountText").set_text(Some(&format!(
            "Live instances: {} / {} tracked",
            self.perf.count_live_instances(),
            self.perf.total_created()
        )));
        self.text("HeapText").set_text(Some(&format!("Heap: {NOT_AVAILABLE}")));
        self.text("AllocText").set_text(Some(&format!("Total allocated: {NOT_AVAILABLE}")));
    }
}
