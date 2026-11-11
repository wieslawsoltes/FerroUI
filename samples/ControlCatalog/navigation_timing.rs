//! Opt-in navigation diagnostics (`FERROUI_CATALOG_NAV_TIMING=1`).
//! Times are cumulative wall time on the UI thread, not GPU presentation time.

use crate::models::PageItem;
use ferroui_base::Ref;
use ferroui_controls::Page;
use std::cell::Cell;
use std::time::Instant;

fn enabled() -> bool {
    ferroui_base::ferro_thread_local! {
        static ENABLED: bool = std::env::var("FERROUI_CATALOG_NAV_TIMING").as_deref() == Ok("1");
    }
    ENABLED.with(|enabled| *enabled)
}

#[derive(Clone)]
pub(crate) struct NavigationTiming {
    header: String,
    start: Instant,
}

impl NavigationTiming {
    pub(crate) fn start(item: &PageItem) -> Option<Self> {
        enabled().then(|| Self {
            header: item.header(),
            start: Instant::now(),
        })
    }

    pub(crate) fn mark(&self, phase: &str) {
        println!(
            "Navigation timing: {} | {phase} | {:.3} ms",
            self.header,
            self.start.elapsed().as_secs_f64() * 1000.0
        );
    }

    pub(crate) fn observe_loaded(&self, page: &Ref<Page>) {
        let timing = self.clone();
        let reported = Cell::new(false);
        page.loaded(move |_, _| {
            if !reported.replace(true) {
                timing.mark("loaded");
            }
        });
    }
}

pub(crate) struct MarkupTiming {
    path: String,
    start: Instant,
}

impl MarkupTiming {
    pub(crate) fn start(path: &str) -> Option<Self> {
        enabled().then(|| Self {
            path: path.to_string(),
            start: Instant::now(),
        })
    }
}

impl Drop for MarkupTiming {
    fn drop(&mut self) {
        println!(
            "Markup timing: {} | {:.3} ms",
            self.path,
            self.start.elapsed().as_secs_f64() * 1000.0
        );
    }
}
