//! Native errors at FerroUI's string-valued markup boundary.
use ferroui_base::metadata::MarkupInvokeError;

pub(crate) fn failure(error: impl std::fmt::Display) -> MarkupInvokeError {
    MarkupInvokeError::Failed(error.to_string())
}
