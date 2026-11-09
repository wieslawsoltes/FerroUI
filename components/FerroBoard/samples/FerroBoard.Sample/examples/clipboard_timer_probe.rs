//! Real native dispatcher/timer probe using local text; never reads or writes the OS clipboard.
use ferroboard_controls::services::ClipboardMonitor;
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
use ferroui_controls::{AppBuilder, Application};
use ferroui_desktop::AppBuilderDesktopExtensions;
use ferroui_system::threading::CancellationToken;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    AppBuilder::configure::<Application>()
        .use_platform_detect()
        .setup_without_starting();
    let text = Rc::new(RefCell::new("baseline".to_owned()));
    let input = text.clone();
    let changes = Rc::new(Cell::new(0));
    let seen = changes.clone();
    let monitor = ClipboardMonitor::new(
        move || Box::pin(std::future::ready(Ok(Some(input.borrow().clone())))),
        move |value| {
            assert_eq!(value, "changed");
            seen.set(seen.get() + 1);
            Dispatcher::ui_thread().begin_invoke_shutdown(DispatcherPriority::NORMAL);
        },
    );
    monitor.set_enabled(true);
    Dispatcher::ui_thread().run_jobs(None);
    *text.borrow_mut() = "changed".into();
    let watchdog = DispatcherTimer::with_callback(
        Duration::from_secs(5),
        DispatcherPriority::NORMAL,
        |timer| {
            timer.stop();
            Dispatcher::ui_thread().begin_invoke_shutdown(DispatcherPriority::NORMAL);
        },
    );
    Dispatcher::ui_thread().main_loop(&CancellationToken::none());
    watchdog.stop();
    monitor.set_enabled(false);
    if changes.get() != 1 {
        return Err("native clipboard sampling timer did not emit exactly once".into());
    }
    println!(
        "native dispatcher clipboard sampling timer verified with local fixture; OS clipboard untouched"
    );
    Ok(())
}
