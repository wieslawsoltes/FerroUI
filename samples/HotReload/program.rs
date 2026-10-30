#[cfg(all(debug_assertions, feature = "desktop"))]
fn main() {
    use ferroui_desktop::AppBuilderDesktopExtensions;
    hot_reload_demo::set_action(std::rc::Rc::new(|| {
        ferroui_hot_patch::hot_reloadable(counter_click)
    }));
    hot_reload_demo::set_view_builder(std::rc::Rc::new(|source| {
        ferroui_hot_patch::hot_reloadable_with(build_view, source)
    }));
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    ferroui_controls::AppBuilder::configure::<hot_reload_demo::App>()
        .use_platform_detect()
        .start_with_classic_desktop_lifetime(&args);
}
// This is the explicit patch boundary. Its state is in the stable library.
#[cfg(debug_assertions)]
fn counter_click() -> i32 {
    hot_reload_demo::increment(1)
}
// Pure candidate builder. The stable-library adapter owns commit and rollback.
#[cfg(debug_assertions)]
fn build_view(source: String) -> String {
    source
}
#[cfg(not(all(debug_assertions, feature = "desktop")))]
fn main() {
    eprintln!("Run this development sample in a debug build with the desktop feature.");
}
