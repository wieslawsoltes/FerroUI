use super::clipboard_helper::{CatchUnwind, ClipboardHelper};
use crate::{Control, TopLevel};
use ferroui_base::input::platform::{ClipboardExtensions, ClipboardType};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::threading::{Dispatcher, DispatcherTask};
use std::any::Any;

pub(crate) struct PrimarySelectionHelper;

impl PrimarySelectionHelper {
    /// Publishes text to the primary selection clipboard, if available.
    /// Failures are logged. The text is only realized on platforms
    /// supporting the primary selection.
    ///
    /// Like the asynchronous method of the reference, everything up to the
    /// first pending await runs before this returns. Every failure is
    /// logged, whatever its kind, and so is a panic of the clipboard (the
    /// reference catches every exception here).
    pub(crate) fn publish_text_async(
        source: &Control,
        text_factory: impl FnOnce() -> Option<String> + 'static,
    ) -> DispatcherTask<()> {
        let source = source.to_ref();

        Dispatcher::ui_thread().to_task_scheduler().start_local(async move {
            let Some(primary_selection) = TopLevel::get_top_level(Some(&source))
                .and_then(|top_level| top_level.try_get_clipboard(ClipboardType::PrimarySelection))
            else {
                return;
            };

            let Some(text) = text_factory().filter(|text| !text.is_empty()) else {
                return;
            };

            let result = CatchUnwind::new(async move { primary_selection.set_text_async(Some(&text)).await }).await;

            let error = match result {
                Ok(Ok(())) => return,
                Ok(Err(error)) => error.to_string(),
                Err(payload) => ClipboardHelper::describe_panic(&*payload),
            };

            if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::CONTROL) {
                let source: &dyn Any = &source;
                logger.log_with_values(Some(source), "Failed to write text to primary selection: {Error}", &[&error]);
            }
        })
    }
}
