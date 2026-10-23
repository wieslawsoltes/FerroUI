//! Port of `Diagnostics/Screenshots/BaseRenderToStreamHandler.cs`.

use crate::diagnostics::{IScreenshotHandler, VisualExtensions};
use ferroui_base::input::LocalBoxFuture;
use ferroui_base::Ref;
use ferroui_controls::Control;
use std::io::Write;

/// Base class for render Screenshto to stream
///
/// An abstract class upstream: an implementation supplies the stream
/// ([`get_stream`](Self::get_stream)) and is an [`IScreenshotHandler`] whose `take` renders the
/// control to that stream, as the non-virtual `Take` of the original does.
pub trait BaseRenderToStreamHandler {
    /// Get stream to write a screenshot to.
    ///
    /// Resolves to the stream to render the control to, or to `None` when there is none (the
    /// screenshot is then not taken).
    fn get_stream(&self, control: &Ref<Control>) -> LocalBoxFuture<std::io::Result<Option<Box<dyn Write>>>>;
}

impl<T: BaseRenderToStreamHandler> IScreenshotHandler for T {
    fn take(&self, control: &Ref<Control>) -> LocalBoxFuture<std::io::Result<()>> {
        let stream = self.get_stream(control);
        let control = control.clone();
        Box::pin(async move {
            // `await using var output = await GetStream(control)`: the stream is released when
            // the screenshot is written.
            let output = stream.await?;
            if let Some(mut output) = output {
                control.render_to(&mut *output)?;
                output.flush()?;
            }
            Ok::<(), std::io::Error>(())
        })
    }
}
