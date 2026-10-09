//! What a visit to a page of the catalog retains: the main view of the
//! catalog in a window that renders through the compositor into a raster
//! framebuffer, its pages selected one after the other as the drawer selects
//! them, and what is alive in the process read after each.
//!
//! Not ports: the upstream sample has no tests, and its runtime collects
//! what a page leaves behind. Here a page that left the navigation page is
//! freed when the last reference to it is dropped, and a visit must cost
//! nothing once it is over, beyond caches that are bounded. The two
//! measurements and the probe are ignored by default (they take minutes); run
//! them with the counting allocator, one at a time:
//!
//! ```sh
//! cargo test -p control-catalog --features count-allocations --lib catalog_tour_memory -- --ignored --nocapture --test-threads=1
//! cargo test -p control-catalog --features count-allocations --lib catalog_revisit_memory -- --ignored --nocapture --test-threads=1
//! ```
//!
//! # The tours
//!
//! `catalog_tour_memory` selects every page in the order of the page list
//! (the home page and the pages of the sections), with the
//! layout, the transition of the navigation page and rendered frames after
//! each, and repeats the tour. It prints, per tour, what is alive at its end
//! (on the home page, which every tour ends with): the bytes and the blocks
//! of the process and the objects of the server compositor, with the growth
//! over the tour before. The first tour fills the caches (the themes, the
//! parsed documents, the fonts and glyphs); from the second on a tour should
//! add nothing.
//!
//! # The revisits
//!
//! `catalog_revisit_memory` measures each page alone: from the home page it
//! selects the page and returns, three times, and reads what is alive each
//! time it is back on the home page. The growth of the second and third
//! round is what a visit to that page (and one to the home page) retains;
//! the smallest growth of the report is the most the home page itself can
//! have retained.
//!
//! # The markup probe
//!
//! `catalog_markup_survivors` narrows a page down: it shows the markup of
//! `CATALOG_TOUR_XAML` (the children of a panel) in the window of the tour
//! in place of the main view, takes it out again and prints the elements of
//! its visual tree that are still alive. With `CATALOG_TOUR_TRACE` what the
//! markup left is reported as the revisits report it.
//!
//! The settings page is left out of both: it reads the application as the
//! application of the catalog, and the tests of the compositor run under
//! the unit test application.
//!
//! # Environment
//!
//! - `CATALOG_TOUR_TOURS`: the number of tours (3).
//! - `CATALOG_TOUR_PAGES`: the headers of the pages to visit, separated by
//!   commas (every page).
//! - `CATALOG_TOUR_TRACE`: records where the blocks a visit leaves behind
//!   were allocated (`allocation_trace.rs`) and prints the call stacks: in
//!   the revisits, the headers of the pages to record (`all` for every
//!   page), each during its second round (the visit to the page and the one
//!   to the home page after it; what is alive of both after the third round
//!   is reported); in the tours, any value records the tour before the last.
//!   The report ends with the references into those blocks from the blocks
//!   that existed before (`allocation_trace::holders`). Recording is slow
//!   and takes memory.
//! - `CATALOG_TOUR_TARGET`: with a recording, a part of the name of a
//!   function (the constructor of a class, as `HomePage::new`): the holders
//!   reported are the ones of the blocks that function allocated, from the
//!   blocks of every epoch, after the reference counts of those blocks. A
//!   holder is printed with the number of its pointers and how many of them
//!   are weak references of the object model; the ones with other pointers
//!   come first, and the one that keeps an object alive is among them.
//! - `CATALOG_TOUR_XAML`: the markup of `catalog_markup_survivors`.
//! - `CATALOG_TOUR_SITES`, `CATALOG_TOUR_FRAMES`: how many call stacks are
//!   printed (12) and how many frames of each (22).
//! - `CATALOG_TOUR_CLASSES`: how many classes the table of objects lists
//!   (60); `CATALOG_TOUR_HOLDERS`, `CATALOG_TOUR_HOLDER_FRAMES`: how many
//!   groups of holders are printed (40) and how many frames of the call
//!   stack that allocated each holder (7).

use super::allocation_trace::{self as trace, Holder, Site};
use super::allocations::{self, LiveCounts};
use super::frame_benchmark::{RasterSurface, HEIGHT, WIDTH};
use super::support::*;
use crate::models::PageItem;
use crate::view_models::MainWindowViewModel;
use crate::MainView;
use ferroui_base::animation::TimeSpan;
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::testing::{CompositorTestServices, MockWindowImpl, MockWindowingPlatform};
use ferroui_controls::{Control, NavigationPage, Window};
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

/// The frames after a page is selected: the transition of the navigation
/// page takes the first ones, the release of what the page before it held
/// (the server objects of its visuals, released with a later batch) the
/// ones after.
const SETTLE_FRAMES: usize = 8;

/// The time of the global clock between two frames: longer than a frame of
/// a display, so that the transitions end within the frames of a visit.
const FRAME_TIME_MS: f64 = 100.0;

/// The main view of the catalog in a shown window that renders through the
/// compositor of the test into a raster surface.
pub(super) struct Tour {
    // Declared before the services: closed and dropped before the
    // application scope ends.
    window: Ref<Window>,
    main_view: Ref<MainView>,
    view_model: Rc<MainWindowViewModel>,
    clock: Rc<TestGlobalClock>,
    time: Cell<f64>,
    window_impl: Rc<MockWindowImpl>,
    _surface: std::sync::Arc<RasterSurface>,
    services: CompositorTestServices,
}

/// What is alive at a moment of a tour.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Alive {
    pub live: LiveCounts,
    /// The objects of the server compositor.
    pub server_objects: usize,
}

impl Tour {
    pub(super) fn start() -> Tour {
        let clock = Rc::new(TestGlobalClock::default());
        let services = start_catalog_compositor_application_with_clock(clock.clone());
        let window_impl = MockWindowingPlatform::create_window_mock_with_size(WIDTH, HEIGHT);
        services.setup(&window_impl);
        let surface = RasterSurface::new();
        window_impl.setup_surfaces(vec![surface.clone() as std::sync::Arc<dyn IPlatformRenderSurface>]);
        let window = Window::with_impl(window_impl.clone());
        window.set_width(WIDTH);
        window.set_height(HEIGHT);
        let view_model = MainWindowViewModel::new();
        let main_view = MainView::new();
        main_view.set_data_context(Some(view_model.clone() as BoxedValue));
        window.set_content(Some(Control::boxed(&main_view)));
        window.show();
        let tour = Tour {
            window,
            main_view,
            view_model,
            clock,
            time: Cell::new(0.0),
            window_impl,
            _surface: surface,
            services,
        };
        tour.settle();
        tour
    }

    pub(super) fn view_model(&self) -> &Rc<MainWindowViewModel> {
        &self.view_model
    }

    /// The window of the catalog.
    pub(super) fn window(&self) -> &Ref<Window> {
        &self.window
    }

    /// The platform implementation of the window: raw input is given to its
    /// input callback.
    pub(super) fn window_impl(&self) -> &Rc<MockWindowImpl> {
        &self.window_impl
    }

    /// The compositor the window renders through.
    pub(super) fn compositor(&self) -> &Rc<ferroui_base::rendering::composition::Compositor> {
        self.services.compositor()
    }

    /// Whether `item` is shown: it is the current page of the catalog, alone
    /// on the stack of the navigation page, and the navigation is over.
    pub(super) fn is_shown(&self, item: &Rc<PageItem>) -> bool {
        let navigation_page = self.navigation_page();
        let is_current = self.view_model.current_page_item().is_some_and(|current| Rc::ptr_eq(&current, item));
        is_current && !navigation_page.is_navigating() && navigation_page.stack_depth() == 1
    }

    /// The pages the drawer offers, in the order of the page list, without
    /// the settings page.
    pub(super) fn pages(&self) -> Vec<Rc<PageItem>> {
        let settings = self.view_model.settings_item();
        crate::smoke::pages(&self.view_model).into_iter().filter(|page| !Rc::ptr_eq(page, &settings)).collect()
    }

    /// One frame: the jobs of the dispatcher (the layout among them), a
    /// tick of the global clock, a rendered frame and the jobs it posted.
    pub(super) fn frame(&self) {
        Dispatcher::ui_thread().run_jobs(None);
        let time = self.time.get() + FRAME_TIME_MS;
        self.time.set(time);
        self.clock.pulse(TimeSpan::from_milliseconds(time));
        Dispatcher::ui_thread().run_jobs(None);
        self.services.render_loop().tick();
        Dispatcher::ui_thread().run_jobs(None);
    }

    pub(super) fn settle(&self) {
        for _ in 0..SETTLE_FRAMES {
            self.frame();
        }
    }

    fn navigation_page(&self) -> Ref<NavigationPage> {
        self.main_view.get_control::<NavigationPage>("NavPage")
    }

    /// Selects the page as the drawer does and runs the frames of the
    /// visit. Returns whether the page is shown: it is the current page of
    /// the catalog, alone on the stack of the navigation page, and the
    /// navigation is over.
    pub(super) fn show(&self, item: &Rc<PageItem>) -> bool {
        self.view_model.navigate_to_item(item);
        self.settle();
        self.is_shown(item)
    }

    pub(super) fn alive(&self) -> Alive {
        Alive { live: allocations::live(), server_objects: self.services.compositor().server().object_count() }
    }
}

impl Drop for Tour {
    fn drop(&mut self) {
        self.window.close();
    }
}

pub(super) fn environment(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

pub(super) fn environment_number(name: &str, default: usize) -> usize {
    environment(name).and_then(|value| value.parse().ok()).unwrap_or(default)
}

/// The pages of the tour: the ones `CATALOG_TOUR_PAGES` names, or all.
pub(super) fn selected_pages(tour: &Tour) -> Vec<Rc<PageItem>> {
    let pages = tour.pages();
    match environment("CATALOG_TOUR_PAGES") {
        Some(names) => {
            let names: Vec<&str> = names.split(',').map(str::trim).collect();
            pages.into_iter().filter(|page| names.contains(&page.header().as_str())).collect()
        }
        None => pages,
    }
}

pub(super) fn kilobytes(bytes: i64) -> String {
    format!("{:.1}", bytes as f64 / 1024.0)
}

/// A growth in kilobytes, with its sign.
pub(super) fn growth_kilobytes(bytes: i64) -> String {
    format!("{:+.1}", bytes as f64 / 1024.0)
}

/// The frames of a call stack that say where the allocation was made: the
/// frames of the allocator and of the collections of the standard library
/// between the allocation and the code that asked for it are left out.
fn named_frames(frames: &[usize], symbols: &mut HashMap<usize, String>) -> Vec<String> {
    const SKIPPED: &[&str] = &[
        "<control_catalog::tests::alloc",
        "control_catalog::tests::alloc",
        "__rust",
        "_RNv",
        "alloc::",
        "<alloc::",
        "hashbrown::",
        "<hashbrown::",
        "core::",
        "<core::",
        "std::collections",
        "<std::collections",
        "<T as core::",
        "<T as alloc::",
        "<I as core::",
        "<I as alloc::",
        "<&T as core::",
        "<&mut T as core::",
        "<F as core::",
    ];
    let names: Vec<String> = frames
        .iter()
        .map(|address| symbols.entry(*address).or_insert_with(|| trace::symbol(*address)).clone())
        .collect();
    let first = names.iter().position(|name| !SKIPPED.iter().any(|skipped| name.starts_with(skipped))).unwrap_or(0);
    names[first..].to_vec()
}

/// Prints what a recording found alive: the totals, the functions that
/// allocated the most (the first frame outside the allocator), and the
/// largest call stacks.
fn print_sites(title: &str, sites: &[Site]) {
    let blocks: u64 = sites.iter().map(|site| site.allocations).sum();
    let bytes: u64 = sites.iter().map(|site| site.bytes).sum();
    println!("  {title}: {blocks} blocks, {} KB, still alive, from {} call stacks", kilobytes(bytes as i64), sites.len());
    if sites.is_empty() {
        return;
    }
    let mut symbols = HashMap::new();
    let named: Vec<(&Site, Vec<String>)> = sites.iter().map(|site| (site, named_frames(&site.frames, &mut symbols))).collect();

    let mut by_function: HashMap<&str, (u64, u64)> = HashMap::new();
    let mut by_class: HashMap<&str, (u64, u64)> = HashMap::new();
    for (site, frames) in &named {
        let function = frames.first().map(String::as_str).unwrap_or("?");
        let entry = by_function.entry(function).or_default();
        entry.0 += site.allocations;
        entry.1 += site.bytes;
        // The object of a class is allocated by `instantiate`, called by the
        // constructor of the class.
        if let Some(index) = frames.iter().position(|name| name.ends_with("type_system::instantiate")) {
            if index <= 1 {
                let class = frames.get(index + 1).map(String::as_str).unwrap_or("?");
                let entry = by_class.entry(class).or_default();
                entry.0 += site.allocations;
                entry.1 += site.bytes;
            }
        }
    }
    let count = environment_number("CATALOG_TOUR_SITES", 12);
    let print_table = |heading: &str, table: HashMap<&str, (u64, u64)>, count: usize| {
        let mut rows: Vec<(&str, (u64, u64))> = table.into_iter().collect();
        rows.sort_by(|a, b| b.1 .1.cmp(&a.1 .1).then_with(|| a.0.cmp(b.0)));
        println!("    {heading}:");
        for (name, (blocks, bytes)) in rows.iter().take(count) {
            println!("      {:>9} KB {:>7} blocks  {name}", kilobytes(*bytes as i64), blocks);
        }
    };
    print_table("objects of classes, by constructor", by_class, count * 3);
    print_table("by allocating function", by_function, count * 3);

    let frames_shown = environment_number("CATALOG_TOUR_FRAMES", 22);
    println!("    the largest call stacks:");
    for (site, frames) in named.iter().take(count) {
        println!("      {} KB in {} blocks:", kilobytes(site.bytes as i64), site.allocations);
        for frame in frames.iter().take(frames_shown) {
            println!("        {frame}");
        }
    }
}

/// What allocated a block, in a line: the constructor of the class when the
/// block is an object, with the function that asked for the block.
fn allocation_in_a_line(frames: &[String]) -> String {
    let function = frames.first().map(String::as_str).unwrap_or("?");
    match frames.iter().position(|name| name.ends_with("type_system::instantiate")) {
        Some(index) if index <= 1 => frames.get(index + 1).cloned().unwrap_or_else(|| function.to_string()),
        _ => function.to_string(),
    }
}

/// Prints the objects of classes among the blocks of `epoch`, by the
/// constructor of the class: how many are alive, with the strong references
/// to each, and how many were dropped while weak references keep their
/// memory.
fn print_objects(epoch: u32) {
    let symbols = std::cell::RefCell::new(HashMap::new());
    let constructor = |frames: &[usize]| -> Option<String> {
        let names = named_frames(frames, &mut symbols.borrow_mut());
        let index = names.iter().position(|name| name.ends_with("type_system::instantiate")).filter(|index| *index <= 1)?;
        names.get(index + 1).cloned()
    };
    let mut classes: HashMap<String, (Vec<usize>, usize)> = HashMap::new();
    for (frames, strong, _) in trace::counts(epoch, &|frames| constructor(frames).is_some()) {
        let entry = classes.entry(constructor(&frames).unwrap_or_default()).or_default();
        if strong == 0 {
            entry.1 += 1;
        } else {
            entry.0.push(strong);
        }
    }
    let mut rows: Vec<(String, (Vec<usize>, usize))> = classes.into_iter().collect();
    rows.sort_by(|a, b| (b.1 .0.len() + b.1 .1).cmp(&(a.1 .0.len() + a.1 .1)).then_with(|| a.0.cmp(&b.0)));
    println!("    objects of classes: alive (the strong references to each), dropped with their memory held weakly:");
    for (class, (mut alive, dropped)) in rows.into_iter().take(environment_number("CATALOG_TOUR_CLASSES", 60)) {
        alive.sort_unstable();
        let mut counts: Vec<String> = Vec::new();
        for strong in alive.iter() {
            let text = strong.to_string();
            if counts.last() != Some(&text) {
                counts.push(text);
            }
        }
        println!("      {:>5} alive ({}) {:>5} dropped  {class}", alive.len(), counts.join(","), dropped);
    }
}

/// The holders of the blocks of `epoch`: of the ones `CATALOG_TOUR_TARGET`
/// names, from every block; without it of all, from the blocks allocated
/// before.
fn holders_of(epoch: u32) -> Vec<Holder> {
    match environment("CATALOG_TOUR_TARGET") {
        Some(target) => {
            let symbols = std::cell::RefCell::new(HashMap::new());
            let is_target = |frames: &[usize]| {
                let names = named_frames(frames, &mut symbols.borrow_mut());
                names.iter().take(3).any(|name| name.contains(&target))
            };
            trace::holders(epoch, true, &is_target)
        }
        None => trace::holders(epoch, false, &|_| true),
    }
}

/// Prints the references into the blocks of a recording.
fn print_holders(epoch: u32, holders: &[Holder]) {
    let references: u64 = holders.iter().map(|holder| holder.references).sum();
    println!("    {references} references, by what allocated the holder and what it points into:");
    let mut symbols = HashMap::new();
    let count = environment_number("CATALOG_TOUR_HOLDERS", 40);
    let frames_shown = environment_number("CATALOG_TOUR_HOLDER_FRAMES", 7);
    for holder in holders.iter().take(count) {
        let holder_frames = named_frames(&holder.holder, &mut symbols);
        let target_frames = named_frames(&holder.target, &mut symbols);
        let age = match holder.epoch.cmp(&epoch) {
            std::cmp::Ordering::Less => "before",
            std::cmp::Ordering::Equal => "in the same round",
            std::cmp::Ordering::Greater => "after",
        };
        println!(
            "      {} ({} weak) into {}, from blocks of {} bytes allocated {age}",
            holder.references,
            holder.weak,
            allocation_in_a_line(&target_frames),
            holder.bytes
        );
        println!("        (target allocated by: {})", target_frames.iter().take(4).cloned().collect::<Vec<_>>().join(" < "));
        for frame in holder_frames.iter().take(frames_shown) {
            println!("        held by a block of {frame}");
        }
    }
}

#[test]
#[ignore = "measurement: run with --features count-allocations --ignored --nocapture --test-threads=1"]
fn catalog_tour_memory() {
    measure_tours("catalog tour", |_| (), |_, _, _| ());
}

/// The measurement of the tours: `after_shown` runs on every page a tour
/// shows, after the frames of the visit, with what `setup` made of the tour.
pub(super) fn measure_tours<D>(title: &str, setup: impl Fn(&Tour) -> D, after_shown: impl Fn(&Tour, &D, &Rc<PageItem>)) {
    let tours = environment_number("CATALOG_TOUR_TOURS", 3).max(1);
    let traced = environment("CATALOG_TOUR_TRACE").is_some();
    if traced {
        trace::start();
    }
    let tour = Tour::start();
    let driver = setup(&tour);
    let pages = selected_pages(&tour);
    let home = tour.view_model().home_item();
    println!("{title}: {} pages, {tours} tours", pages.len());
    if !allocations::ENABLED {
        println!("  without the feature count-allocations: the bytes and blocks read zero");
    }

    let mut not_shown: Vec<String> = Vec::new();
    let mut ends: Vec<Alive> = Vec::new();
    for number in 1..=tours {
        // The tour before the last is recorded, the home page it ends with
        // included; what the last tour leaves of it is reported.
        let recorded = traced && tours > 1 && number == tours - 1;
        let epoch = if recorded || (traced && number == tours) { trace::next_epoch() } else { 0 };
        for page in &pages {
            if !tour.show(page) && number == 1 {
                not_shown.push(page.header());
            }
            after_shown(&tour, &driver, page);
        }
        // Every tour ends on the home page: the ends compare like with like.
        tour.show(&home);
        tour.settle();
        let sites = if traced && tours > 1 && number == tours {
            print_objects(epoch - 1);
            let report = (epoch - 1, trace::alive(epoch - 1), holders_of(epoch - 1));
            trace::clear();
            Some(report)
        } else {
            None
        };
        let end = tour.alive();
        match ends.last() {
            Some(before) => println!(
                "  tour {number}: {} KB alive ({} KB), {} blocks ({:+}), {} server objects ({:+}); per page {} KB",
                kilobytes(end.live.bytes),
                growth_kilobytes(end.live.bytes - before.live.bytes),
                end.live.allocations,
                end.live.allocations - before.live.allocations,
                end.server_objects,
                end.server_objects as i64 - before.server_objects as i64,
                growth_kilobytes((end.live.bytes - before.live.bytes) / pages.len().max(1) as i64),
            ),
            None => println!(
                "  tour {number}: {} KB alive, {} blocks, {} server objects",
                kilobytes(end.live.bytes),
                end.live.allocations,
                end.server_objects
            ),
        }
        if let Some((epoch, sites, holders)) = sites {
            print_sites(&format!("tour {}", number - 1), &sites);
            print_holders(epoch, &holders);
        }
        ends.push(end);
    }
    if !not_shown.is_empty() {
        println!("  not shown: {}", not_shown.join(", "));
    }
}

/// The page the navigation page of the catalog shows.
pub(super) fn shown_page(tour: &Tour) -> ferroui_base::WeakRef<ferroui_controls::Page> {
    let stack = tour.view_model().navigator().expect("the navigator of the catalog").navigation_stack();
    stack[0].downgrade()
}

/// The first thing a visit must not retain is the page itself. The home page
/// declares a data template (gap C317); Border, Calendar and TextBlock stand
/// for the pages without one, and the others are the ones that stayed alive
/// for gaps C322 to C326: the page that is its own data context, and pages
/// with validated controls, sliders and styles under an element.
#[test]
fn the_catalog_frees_the_page_it_navigated_away_from() {
    let tour = Tour::start();
    let pages = tour.pages();
    let home = tour.view_model().home_item();
    for name in ["Border", "Calendar", "TextBlock", "Buttons", "NumericUpDown", "Slider", "CalendarDatePicker", "Flex Panel"] {
        let page = pages.iter().find(|page| page.header() == name).expect("a page of the list");
        assert!(tour.show(&home));
        let home_page = shown_page(&tour);
        assert!(tour.show(page), "{name}");
        let page = shown_page(&tour);
        assert!(home_page.upgrade().is_none(), "the home page is alive after {name} replaced it");
        assert!(tour.show(&home));
        assert!(page.upgrade().is_none(), "{name} is alive after the home page replaced it");
    }
}

/// Visits `page` from `home` and returns; what is alive back on `home`.
fn round_trip(tour: &Tour, page: &Rc<PageItem>, home: &Rc<PageItem>, after_shown: &dyn Fn(&Rc<PageItem>)) -> (bool, Alive) {
    let shown = tour.show(page);
    after_shown(page);
    tour.show(home);
    (shown, tour.alive())
}

#[test]
#[ignore = "measurement: run with --features count-allocations --ignored --nocapture --test-threads=1"]
fn catalog_revisit_memory() {
    measure_revisits("catalog revisits", |_| (), |_, _, _| ());
}

/// The measurement of the revisits: `after_shown` runs on the page of a
/// round, after the frames of the visit and before the return to the home
/// page, with what `setup` made of the tour.
pub(super) fn measure_revisits<D>(title: &str, setup: impl Fn(&Tour) -> D, after_shown: impl Fn(&Tour, &D, &Rc<PageItem>)) {
    let traced: Vec<String> =
        environment("CATALOG_TOUR_TRACE").map(|names| names.split(',').map(|name| name.trim().to_string()).collect()).unwrap_or_default();
    if !traced.is_empty() {
        trace::start();
    }
    let tour = Tour::start();
    let driver = setup(&tour);
    let after_shown = |page: &Rc<PageItem>| after_shown(&tour, &driver, page);
    let home = tour.view_model().home_item();
    let pages: Vec<Rc<PageItem>> = selected_pages(&tour).into_iter().filter(|page| !Rc::ptr_eq(page, &home)).collect();
    println!("{title}: {} pages; growth of the second and of the third visit over the one before", pages.len());
    println!("  {:<28} {:>12} {:>9} {:>8} | {:>12} {:>9} {:>8}", "page", "KB", "blocks", "server", "KB", "blocks", "server");

    // The home page and a page fill what the pages share before the first
    // page is measured.
    tour.show(&home);
    if let Some(page) = pages.last() {
        round_trip(&tour, page, &home, &after_shown);
    }

    let mut total = (0i64, 0i64);
    let mut reports = Vec::new();
    for page in &pages {
        let header = page.header();
        let record = traced.iter().any(|name| name == "all" || *name == header);
        let (shown, first) = round_trip(&tour, page, &home, &after_shown);
        let epoch = if record { trace::next_epoch() } else { 0 };
        let (_, second) = round_trip(&tour, page, &home, &after_shown);
        if record {
            trace::next_epoch();
        }
        let (_, third) = round_trip(&tour, page, &home, &after_shown);
        if record {
            print_objects(epoch);
        }
        let sites = if record { Some((epoch, trace::alive(epoch), holders_of(epoch))) } else { None };
        let growth = |after: &Alive, before: &Alive| {
            (
                after.live.bytes - before.live.bytes,
                after.live.allocations - before.live.allocations,
                after.server_objects as i64 - before.server_objects as i64,
            )
        };
        let (second, third) = (growth(&second, &first), growth(&third, &second));
        println!(
            "  {:<28} {:>12} {:>9} {:>8} | {:>12} {:>9} {:>8}{}",
            header,
            if record { String::from("(recording)") } else { growth_kilobytes(second.0) },
            second.1,
            second.2,
            growth_kilobytes(third.0),
            third.1,
            third.2,
            if shown { "" } else { "  (not shown)" },
        );
        total.0 += third.0;
        total.1 += third.1;
        if let Some(sites) = sites {
            reports.push((header, sites));
        }
    }
    println!("  {:<28} {:>12} {:>9}", "all (third visits)", growth_kilobytes(total.0), total.1);
    trace::clear();
    for (header, (epoch, sites, holders)) in &reports {
        print_sites(header, sites);
        print_holders(*epoch, holders);
    }
}

/// Shows `root` in the window of the tour in place of the main view, with
/// rendered frames, and takes it out again (`before_it_is_taken_out` runs
/// between the two). Returns the number of elements of the visual tree it
/// had while shown and the ones that are still alive, by class, indented by
/// their depth.
fn shown_and_taken_out(tour: &Tour, root: Ref<Control>, before_it_is_taken_out: impl FnOnce()) -> (usize, Vec<String>) {
    tour.window.set_content(Some(Control::boxed(&root)));
    tour.settle();
    let mut elements: Vec<(String, ferroui_base::WeakRef<ferroui_base::Visual>)> = Vec::new();
    let mut pending: Vec<(usize, Ref<ferroui_base::Visual>)> = vec![(0, root.clone().upcast())];
    while let Some((depth, visual)) = pending.pop() {
        elements.push((format!("{}{}", "  ".repeat(depth), visual.get_type().name()), visual.downgrade()));
        for child in visual.get_visual_children().iter().rev() {
            pending.push((depth + 1, child.clone()));
        }
    }
    before_it_is_taken_out();
    tour.window.set_content(Some(Control::boxed(&tour.main_view)));
    drop(root);
    tour.settle();
    let alive = elements.iter().filter(|(_, element)| element.upgrade().is_some()).map(|(name, _)| name.clone()).collect();
    (elements.len(), alive)
}

/// The controls of the compiled Fluent theme that stayed alive after they
/// left the tree, each for a cycle of its template: the text box (the owner
/// of its validation errors, gap C322, and the reflection bindings of a
/// multi binding of its template, whose type resolver held the context of
/// the build with the root it built, gap C323: the context of compiled markup
/// is the one held this way, a template the run-time loader builds is freed
/// without the fix), the slider (the disposable of its own handler, C324)
/// and the date picker (the style with a dynamic resource under its text
/// box, C325).
#[test]
fn gap_c323_controls_of_the_compiled_theme_are_freed() {
    use ferroui_controls::{CalendarDatePicker, Slider, TextBox};
    let tour = Tour::start();
    let controls: [(&str, Ref<Control>); 3] = [
        ("TextBox", TextBox::new().upcast()),
        ("Slider", Slider::new().upcast()),
        ("CalendarDatePicker", CalendarDatePicker::new().upcast()),
    ];
    for (name, control) in controls {
        let (elements, alive) = shown_and_taken_out(&tour, control, || {});
        assert!(elements > 1, "{name} has its template");
        assert!(alive.is_empty(), "{name}: alive after it left the tree: {alive:?}");
    }
}

/// What a piece of markup leaves behind: the elements of `CATALOG_TOUR_XAML`
/// (the children of a panel, in the namespace of the framework) are shown in
/// the window of the catalog, with the Fluent theme and rendered frames, and
/// taken out again; the elements that are still alive, of the visual tree
/// they had while shown, are printed by class. With `CATALOG_TOUR_TRACE`
/// the blocks the markup left are reported as the revisits report theirs.
#[test]
#[ignore = "measurement: set CATALOG_TOUR_XAML; run with --ignored --nocapture --test-threads=1"]
fn catalog_markup_survivors() {
    use ferroui_base::metadata::from_markup_value;
    let Some(xaml) = environment("CATALOG_TOUR_XAML") else {
        println!("CATALOG_TOUR_XAML is not set");
        return;
    };
    let traced = environment("CATALOG_TOUR_TRACE").is_some();
    if traced {
        trace::start();
    }
    let tour = Tour::start();
    let epoch = if traced { trace::next_epoch() } else { 0 };
    let document = format!(
        "<Panel xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>{xaml}</Panel>"
    );
    let root = from_markup_value::<Ref<Control>>(&Some(load_text(&document))).expect("a control");
    let (elements, alive) = shown_and_taken_out(&tour, root, || {
        if traced {
            // What the main view allocates when it is shown again is not the markup's.
            trace::next_epoch();
        }
    });
    println!("markup survivors: {} of {elements} elements are alive", alive.len());
    for name in alive {
        println!("  {name}");
    }
    if traced {
        print_objects(epoch);
        let report = (trace::alive(epoch), holders_of(epoch));
        trace::clear();
        print_sites("the markup", &report.0);
        print_holders(epoch, &report.1);
    }
}

/// The harness itself: a page of the list is shown by a visit, and the
/// visit is over when the frames of the visit have run.
#[test]
fn a_tour_shows_the_page_it_selects() {
    let tour = Tour::start();
    let pages = tour.pages();
    let buttons = pages.iter().find(|page| page.header() == "Buttons").expect("the buttons page");
    assert!(tour.show(buttons));
    assert!(tour.show(&tour.view_model().home_item()));
    assert!(tour.alive().server_objects > 0);
}
