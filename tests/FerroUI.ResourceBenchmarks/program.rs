//! Fresh-process resource registration, first-use and theme startup probes.
use ferroui_base::controls::{ResourceDictionary, ResourceKey};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::media::{Color, SolidColorBrush};
use ferroui_base::styling::{ControlTheme, ThemeVariant};
use ferroui_controls::Button;
use ferroui_markup_xaml::xaml_il::runtime::compiled as rt;
use ferroui_themes_winui::WinUITheme;
use serde_json::{json, Value};
use std::hint::black_box;
use std::io::Write;
use std::time::Instant;

static DOCUMENT: rt::DocumentInfo = rt::DocumentInfo { base_uri: Some("ferres://ResourceBenchmarks/Rows.xaml"), namespaces: &[] };

fn ready(process_start: Instant) -> f64 {
    let elapsed = process_start.elapsed().as_secs_f64() * 1e6;
    // The parent timestamps this marker before materialization/teardown begins.
    println!("RESOURCE_BENCH_READY");
    std::io::stdout().flush().unwrap();
    elapsed
}

fn measure<T>(name: &str, output: &mut Vec<Value>, work: impl FnOnce() -> T) -> T {
    let before = allocations::snapshot();
    let start = Instant::now();
    let value = black_box(work());
    let elapsed = start.elapsed().as_secs_f64() * 1e6;
    let after = allocations::snapshot();
    output.push(json!({"phase":name, "elapsed_us":elapsed,
        "allocations": if cfg!(feature="count-allocations") { Some(after.0-before.0) } else { None },
        "allocated_bytes": if cfg!(feature="count-allocations") { Some(after.1-before.1) } else { None }}));
    value
}

fn rows(count: usize, alias: bool, process_start: Instant) -> (Vec<Value>, f64) {
    let keys: Vec<&'static str> = (0..count).map(|i| Box::leak(format!("Resource{i}").into_boxed_str()) as &'static str).collect();
    let lookups: Vec<ResourceKey> = keys.iter().map(|key| ResourceKey::from(*key)).collect();
    let literals: &'static [rt::LiteralBrushRow] = Box::leak(keys.iter().map(|key| rt::LiteralBrushRow {
        key, color: 0xFF33_6699, namespaces: None, line: 1, position: 1,
    }).collect::<Vec<_>>().into_boxed_slice());
    let aliases: &'static [rt::ResourceAliasRow] = Box::leak(keys.iter().map(|key| rt::ResourceAliasRow {
        key, target: "Accent", namespaces: None, line: 1, position: 1, value_line: 1, value_position: 10,
    }).collect::<Vec<_>>().into_boxed_slice());
    let dictionary = ResourceDictionary::new();
    dictionary.ensure_capacity(count + 1);
    let accent = SolidColorBrush::new();
    accent.set_color(Color::from_uint32(0xFF33_6699));
    dictionary.add("Accent", rt::to_value(accent.clone()));
    let context = rt::populate_context(None, &DOCUMENT, rt::to_value(dictionary.clone()));
    context.push_parent(rt::to_value(dictionary.clone()));
    let mut output = Vec::with_capacity(4);
    measure("register", &mut output, || {
        if alias { rt::add_resource_aliases(&dictionary, &context, &DOCUMENT, aliases) }
        else { rt::add_literal_brushes(&dictionary, &context, &DOCUMENT, literals) }.unwrap();
    });
    assert_eq!(dictionary.count(), count+1);
    assert!(dictionary.contains_deferred_key(&lookups[0]));
    let first = measure("first_lookup", &mut output, || dictionary.try_get_value(&lookups[0]).flatten().unwrap());
    let object = ValueTypes::as_object(&*first).unwrap().cast::<SolidColorBrush>().unwrap();
    assert_eq!(object.color(), Color::from_uint32(0xFF33_6699));
    if alias { assert!(object.ptr_eq(&accent)); }
    assert!(std::rc::Rc::ptr_eq(&first, &dictionary.try_get_value(&lookups[0]).flatten().unwrap()));
    let ready_us = ready(process_start);
    measure("materialize_all", &mut output, || {
        for key in &lookups { black_box(dictionary.try_get_value(key).flatten().unwrap()); }
    });
    measure("steady_10000", &mut output, || {
        for i in 0..10_000 { black_box(dictionary.try_get_value(&lookups[i % count]).flatten().unwrap()); }
    });
    (output, ready_us)
}

fn theme(full: bool, process_start: Instant) -> (Vec<Value>, f64) {
    let mut output = Vec::with_capacity(2);
    // An explicitly selected Button/shared-resource subset, in canonical order.
    // It is a fixture, not a claim about the compiler's minimum root closure.
    let subset: rt::ThemeParts = Box::leak(ferroui_themes_winui::compiled_xaml::THEME_UNITS.iter().copied().filter(|unit| {
        unit.index < 5 || unit.name == "Resources/Button_themeresources.xaml"
            || unit.name == "Controls/Button.xaml" || unit.name == "Controls/PathIcon.xaml"
    }).collect::<Vec<_>>().into_boxed_slice());
    let theme = measure("theme_register", &mut output, || {
        if full { WinUITheme::new() } else { WinUITheme::with_parts(None, subset) }
    });
    let key = ResourceKey::Type(Button::TYPE);
    let value = measure("button_theme_first_lookup", &mut output, || {
        theme.try_get_resource(&key, Some(&ThemeVariant::light())).flatten().unwrap()
    });
    assert!(ValueTypes::as_object(&*value).unwrap().is::<ControlTheme>());
    let ready_us = ready(process_start);
    black_box(&theme);
    (output, ready_us)
}

fn main() {
    let process_start = Instant::now();
    let args: Vec<_> = std::env::args().collect();
    let scenario = args.get(1).map(String::as_str).unwrap_or("literal");
    let count: usize = args.get(2).map(|value| value.parse().unwrap()).unwrap_or(1000);
    assert!(count > 0);
    // Initialize the allocator's TLS before any measured region.
    black_box(allocations::snapshot());
    let (samples, ready_us) = match scenario {
        "literal" => rows(count, false, process_start), "alias" => rows(count, true, process_start),
        "theme_full" => theme(true, process_start), "theme_button" => theme(false, process_start),
        _ => panic!("unknown scenario {scenario}"),
    };
    println!("{}", json!({"scenario":scenario,"rows":count,"counted":cfg!(feature="count-allocations"),
        "process_main_to_verified_us":ready_us,"samples":samples}));
}

mod allocations {
    #[cfg(not(feature="count-allocations"))]
    pub fn snapshot() -> (u64,u64) { (0,0) }
    #[cfg(feature="count-allocations")]
    mod counted {
        use std::alloc::{GlobalAlloc, Layout, System};
        use std::cell::Cell;
        thread_local! { static COUNTS: Cell<(u64,u64)> = const { Cell::new((0,0)) }; }
        pub fn snapshot() -> (u64,u64) { COUNTS.try_with(Cell::get).unwrap_or((0,0)) }
        fn add(bytes: usize) { let _ = COUNTS.try_with(|counts| { let (n,b)=counts.get(); counts.set((n+1,b+bytes as u64)); }); }
        struct Allocator;
        // SAFETY: arguments are forwarded unchanged to System. TLS counters do
        // not allocate and tolerate thread teardown.
        unsafe impl GlobalAlloc for Allocator {
            unsafe fn alloc(&self, layout: Layout) -> *mut u8 { add(layout.size()); unsafe { System.alloc(layout) } }
            unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 { add(layout.size()); unsafe { System.alloc_zeroed(layout) } }
            unsafe fn dealloc(&self, p: *mut u8, layout: Layout) { unsafe { System.dealloc(p,layout) } }
            unsafe fn realloc(&self, p: *mut u8, layout: Layout, n: usize) -> *mut u8 {
                add(n.saturating_sub(layout.size())); unsafe { System.realloc(p,layout,n) }
            }
        }
        #[global_allocator] static ALLOCATOR: Allocator = Allocator;
    }
    #[cfg(feature="count-allocations")]
    pub use counted::snapshot;
}
