//! Port of `Pages/NavigationPerformanceMonitorHelper.cs`.
//!
//! Deviation (DEVIATIONS.md, ControlCatalog sample): the original measures
//! the managed heap (`GC.GetTotalMemory`) and forces collections
//! (`GC.Collect`). There is no garbage-collected heap here, and the sample
//! has no statistics of the allocator: a page is freed when the last
//! reference to it is dropped. The helper therefore reports what can be
//! measured (the pages that are alive, through weak references, and the
//! time of an operation) and says that the heap figure is not available;
//! forcing a collection only refreshes. Not ported: `InitHeap`, the previous
//! heap size and the three brushes of the heap delta, which have no subject.

use crate::pages::navigation_demo_helper::{parse_color, value_text, NavigationDemoHelper};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{FontFamily, FontWeight, IBrush, SolidColorBrush, TextTrimming};
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::DispatcherTimer;
use ferroui_base::utilities::DateTime;
use ferroui_base::{BoxedValue, CornerRadius, Ref, Thickness, WeakRef};
use ferroui_controls::{Border, CheckBox, ContentPage, DockPanel, Page, ScrollViewer, StackPanel, TextBlock};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

/// What the heap texts of the pages say in place of a size.
pub(crate) const NOT_AVAILABLE: &str = "not available";

/// `System.Diagnostics.Stopwatch`, as far as the pages use it.
#[derive(Default)]
pub(crate) struct Stopwatch {
    /// When the running measurement started.
    started: Cell<Option<Instant>>,
    /// The time of the measurements that were stopped.
    elapsed: Cell<Duration>,
}

impl Stopwatch {
    /// Stops the measurement, resets the elapsed time to zero and starts measuring.
    pub(crate) fn restart(&self) {
        self.elapsed.set(Duration::ZERO);
        self.started.set(Some(Instant::now()));
    }

    pub(crate) fn stop(&self) {
        if let Some(started) = self.started.take() {
            self.elapsed.set(self.elapsed.get() + started.elapsed());
        }
    }

    pub(crate) fn is_running(&self) -> bool {
        self.started.get().is_some()
    }

    pub(crate) fn elapsed_milliseconds(&self) -> u128 {
        let running = self.started.get().map_or(Duration::ZERO, |started| started.elapsed());
        (self.elapsed.get() + running).as_millis()
    }
}

/// A reusable stack/history row: its container, its badge and the texts of
/// the index, of the title and of the label.
#[derive(Clone)]
pub(crate) struct StackRow {
    pub(crate) container: Ref<Border>,
    pub(crate) badge: Ref<Border>,
    pub(crate) index_text: Ref<TextBlock>,
    pub(crate) title_text: Ref<TextBlock>,
    pub(crate) badge_text: Ref<TextBlock>,
}

/// Shared helpers for the performance-monitor demo pages
/// (NavigationPage, TabbedPage, DrawerPage, ContentPage).
#[derive(Default)]
pub(crate) struct NavigationPerformanceMonitorHelper {
    tracked_pages: RefCell<Vec<WeakRef<Page>>>,
    /// The timer and the subscription of its tick.
    auto_refresh_timer: RefCell<Option<(Rc<DispatcherTimer>, Rc<dyn IDisposable>)>>,
    pub(crate) op_stopwatch: Stopwatch,
    total_created: Cell<i32>,
}

impl NavigationPerformanceMonitorHelper {
    thread_local! {
        static CURRENT_BORDER_BRUSH: Rc<dyn IBrush> = SolidColorBrush::with_color(parse_color("#0078D4")).into();
        static DEFAULT_BORDER_BRUSH: Rc<dyn IBrush> = SolidColorBrush::with_color(parse_color("#CCCCCC")).into();
    }

    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn total_created(&self) -> i32 {
        self.total_created.get()
    }

    /// Track a newly-created page via a weak reference and increment the
    /// number of the pages created.
    pub(crate) fn track_page(&self, page: &Ref<Page>) {
        self.total_created.set(self.total_created.get() + 1);
        self.tracked_pages.borrow_mut().push(page.downgrade());
    }

    /// Count live (not yet freed) tracked page instances.
    pub(crate) fn count_live_instances(&self) -> i32 {
        let mut tracked_pages = self.tracked_pages.borrow_mut();
        tracked_pages.retain(|page| page.upgrade().is_some());
        tracked_pages.len() as i32
    }

    /// Update heap and delta text blocks. Call from the refresh of a page.
    ///
    /// The heap is not measured (see the module): the text says so and there
    /// is no delta.
    pub(crate) fn update_heap_delta(&self, heap_text: &TextBlock, delta_text: &TextBlock) {
        heap_text.set_text(Some(&format!("Managed Heap: {NOT_AVAILABLE}")));
        delta_text.set_text(Some(""));
    }

    /// Stop the stopwatch and write elapsed ms to the given text block.
    pub(crate) fn stop_metrics(&self, last_op_text: &TextBlock) {
        if !self.op_stopwatch.is_running() {
            return;
        }
        self.op_stopwatch.stop();
        last_op_text.set_text(Some(&format!("Last Op: {} ms", self.op_stopwatch.elapsed_milliseconds())));
    }

    /// Invoke the refresh callback. The original forces a full collection
    /// first; here a page that is no longer referenced is freed already.
    pub(crate) fn force_gc(&self, refresh: impl FnOnce()) {
        refresh();
    }

    /// Start a 2-second auto-refresh timer. The timer is held by the helper:
    /// `refresh` must not hold the owner of the helper strongly.
    pub(crate) fn start_auto_refresh(&self, refresh: impl Fn() + 'static) {
        if self.auto_refresh_timer.borrow().is_some() {
            return;
        }
        let timer = DispatcherTimer::new();
        timer.set_interval(Duration::from_secs(2));
        let tick = timer.tick(move |_| refresh());
        timer.start();
        *self.auto_refresh_timer.borrow_mut() = Some((timer, tick));
    }

    /// Stop the auto-refresh timer.
    pub(crate) fn stop_auto_refresh(&self) {
        let timer = self.auto_refresh_timer.borrow_mut().take();
        if let Some((timer, tick)) = timer {
            timer.stop();
            tick.dispose();
        }
    }

    /// Toggle auto-refresh based on a check box.
    pub(crate) fn on_auto_refresh_changed(&self, check: &CheckBox, refresh: impl Fn() + 'static) {
        if check.is_checked() == Some(true) {
            self.start_auto_refresh(refresh);
        } else {
            self.stop_auto_refresh();
        }
    }

    /// Append a timestamped log entry to a stack panel inside a scroll
    /// viewer. The entry of the original also states the size of the heap.
    pub(crate) fn log_operation(
        &self,
        action: &str,
        detail: &str,
        log_panel: &StackPanel,
        log_scroll: &ScrollViewer,
        extra_info: Option<&str>,
    ) {
        let timing = self.op_stopwatch.elapsed_milliseconds();
        let extra = extra_info.map(|extra_info| format!("  {extra_info},")).unwrap_or_default();

        let entry = TextBlock::new();
        entry.set_text(Some(&format!(
            "{}  [{action}]  {detail}  \u{2014}{extra} {timing} ms",
            DateTime::now().to_string_with("HH:mm:ss")
        )));
        entry.set_font_size(10.0);
        entry.set_font_family(FontFamily::new("Cascadia Mono,Consolas,Menlo,monospace"));
        entry.set_padding(Thickness::symmetric(6.0, 2.0));
        entry.set_text_trimming(<dyn TextTrimming>::character_ellipsis());
        log_panel.children().add(entry);
        log_scroll.scroll_to_end();
    }

    /// Build a tracked content page with a dummy allocation of
    /// `alloc_bytes` bytes (50 KB in the original where it gives none).
    pub(crate) fn build_tracked_page(&self, title: &str, index: i32, alloc_bytes: usize) -> Ref<ContentPage> {
        let page =
            NavigationDemoHelper::make_page(title, &format!("Stack position #{index}\nPush more pages ..."), index);
        page.set_tag(Some(Rc::new(vec![0u8; alloc_bytes]) as BoxedValue));
        self.track_page(&page.clone().upcast::<Page>());
        page
    }

    /// Create a reusable stack/history row (badge + title + label).
    pub(crate) fn create_stack_row() -> StackRow {
        let index_text = TextBlock::new();
        index_text.set_font_size(10.0);
        index_text.set_font_weight(FontWeight::SemiBold);
        index_text.set_horizontal_alignment(HorizontalAlignment::Center);
        index_text.set_vertical_alignment(VerticalAlignment::Center);

        let badge = Border::new();
        badge.set_width(22.0);
        badge.set_height(22.0);
        badge.set_corner_radius(CornerRadius::uniform(11.0));
        badge.set_vertical_alignment(VerticalAlignment::Center);
        badge.set_child(index_text.clone());

        let title_text = TextBlock::new();
        title_text.set_vertical_alignment(VerticalAlignment::Center);
        title_text.set_text_trimming(<dyn TextTrimming>::character_ellipsis());
        title_text.set_margin(Thickness::new(6.0, 0.0, 0.0, 0.0));

        let badge_text = TextBlock::new();
        badge_text.set_font_size(10.0);
        badge_text.set_opacity(0.5);
        badge_text.set_vertical_alignment(VerticalAlignment::Center);
        badge_text.set_margin(Thickness::new(4.0, 0.0, 0.0, 0.0));
        badge_text.set_is_visible(false);

        let row = DockPanel::new();
        row.children().add(badge.clone());
        row.children().add(title_text.clone());
        row.children().add(badge_text.clone());

        let container = Border::new();
        container.set_corner_radius(CornerRadius::uniform(6.0));
        container.set_padding(Thickness::symmetric(8.0, 6.0));
        container.set_child(row);

        StackRow { container, badge, index_text, title_text, badge_text }
    }

    /// Update a stack row with page data.
    pub(crate) fn update_stack_row(row: &StackRow, stack_index: i32, title: &str, is_current: bool, is_root: bool) {
        row.badge.set_background(Some(NavigationDemoHelper::get_page_brush(stack_index)));
        row.index_text.set_text(Some(&(stack_index + 1).to_string()));
        row.title_text.set_text(Some(title));
        row.title_text.set_font_weight(if is_current { FontWeight::SemiBold } else { FontWeight::Normal });

        let label = if is_current {
            Some("current")
        } else if is_root {
            Some("root")
        } else {
            None
        };
        row.badge_text.set_text(Some(label.unwrap_or("")));
        row.badge_text.set_is_visible(label.is_some());

        row.container.set_border_brush(Some(if is_current {
            Self::CURRENT_BORDER_BRUSH.with(Clone::clone)
        } else {
            Self::DEFAULT_BORDER_BRUSH.with(Clone::clone)
        }));
        row.container.set_border_thickness(Thickness::uniform(if is_current { 2.0 } else { 1.0 }));
    }

    /// Makes `panel` show the first `count` rows of `row_cache`, growing the
    /// cache as needed: the part of the original's refreshes that is the
    /// same for a stack and for a history.
    pub(crate) fn sync_rows(panel: &StackPanel, row_cache: &RefCell<Vec<StackRow>>, count: usize) {
        let mut row_cache = row_cache.borrow_mut();
        while row_cache.len() < count {
            row_cache.push(Self::create_stack_row());
        }

        let children = panel.children();
        while children.count() > count {
            children.remove_at(children.count() - 1);
        }
        while children.count() < count {
            children.add(row_cache[children.count()].container.clone());
        }

        for (display_idx, row) in row_cache.iter().enumerate().take(count) {
            if !children.get(display_idx).ptr_eq(&row.container) {
                children.set(display_idx, row.container.clone());
            }
        }
    }

    /// Sync a stack panel of stack rows with data, growing/shrinking the row cache as needed.
    pub(crate) fn refresh_stack_panel(
        panel: &StackPanel,
        row_cache: &RefCell<Vec<StackRow>>,
        stack: &[Ref<Page>],
        current_page: Option<&Ref<Page>>,
    ) {
        let count = stack.len();
        Self::sync_rows(panel, row_cache, count);

        let row_cache = row_cache.borrow();
        for (display_idx, row) in row_cache.iter().enumerate().take(count) {
            let stack_idx = count - 1 - display_idx;
            let page = &stack[stack_idx];
            let is_current = current_page.is_some_and(|current_page| current_page.ptr_eq(page));
            let is_root = stack_idx == 0;

            let title = value_text(&page.header()).unwrap_or_else(|| String::from("(untitled)"));
            Self::update_stack_row(row, stack_idx as i32, &title, is_current, is_root);
        }
    }
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn a_stopwatch_measures_while_it_runs() {
        let stopwatch = Stopwatch::default();
        assert!(!stopwatch.is_running());
        assert_eq!(0, stopwatch.elapsed_milliseconds());

        stopwatch.restart();
        assert!(stopwatch.is_running());
        std::thread::sleep(Duration::from_millis(5));
        stopwatch.stop();
        assert!(!stopwatch.is_running());
        let elapsed = stopwatch.elapsed_milliseconds();
        assert!(elapsed >= 5);
        // A stopped stopwatch keeps its time.
        std::thread::sleep(Duration::from_millis(5));
        assert_eq!(elapsed, stopwatch.elapsed_milliseconds());
    }
}
