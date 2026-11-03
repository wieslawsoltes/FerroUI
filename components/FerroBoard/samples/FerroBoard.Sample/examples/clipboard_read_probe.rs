//! Opt-in native OS read capability probe. Never writes or prints clipboard contents.
use ferroboard_controls::services::SystemClipboardService;
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
use ferroui_controls::{AppBuilder, Application};
use ferroui_desktop::AppBuilderDesktopExtensions;
use ferroui_system::threading::CancellationToken;
use std::{cell::RefCell, rc::Rc, time::Duration};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    AppBuilder::configure::<Application>()
        .use_platform_detect()
        .setup_without_starting();
    let outcome = Rc::new(RefCell::new(None));
    let output = outcome.clone();
    let service = SystemClipboardService::new();
    drop(
        Dispatcher::ui_thread().invoke_async_task_local(move || async move {
            // Discard the text immediately. Neither contents, length nor hashes are retained.
            let status = service
                .read_text()
                .await
                .map(|_| ())
                .map_err(|error| error.kind());
            *output.borrow_mut() = Some(status);
            Dispatcher::ui_thread().begin_invoke_shutdown(DispatcherPriority::NORMAL);
        }),
    );
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
    match outcome.take() {
        Some(Ok(())) => println!(
            "Native clipboard read completed. Clipboard contents were neither modified nor recorded."
        ),
        Some(Err(kind)) => return Err(format!("Native clipboard read failed: {kind:?}").into()),
        None => return Err("Native clipboard read timed out".into()),
    }
    Ok(())
}
