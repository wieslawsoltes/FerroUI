use crate::{SvgDocument, SvgError, SvgExportOptions, SvgOutput};
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use std::any::Any;
use std::sync::{Arc, Mutex};

/// The most recent frame of an SVG surface. A failure replaces the previous result instead
/// of leaving stale artwork that could be mistaken for a successful export.
pub struct SvgSurfaceFrame {
    pub sequence: u64,
    /// The options revision used for this export, useful when options change during rendering.
    pub options_revision: u64,
    pub result: Result<SvgOutput, SvgError>,
}

struct SurfaceState {
    options: SvgExportOptions,
    revision: u64,
    frames: u64,
    latest: Option<SvgSurfaceFrame>,
}

/// A thread-safe headless surface recognized by [`crate::SvgPlatformRenderInterface`].
/// Supply it in a top-level/compositor target's surface list. Completed frames are exported
/// on the render thread, and the most recent result can be taken on the application thread.
/// It never writes files or invokes application callbacks implicitly.
///
/// Clones share state. Only the latest result is retained, preventing an unbounded queue when
/// a producer renders faster than the caller consumes frames. Use [`crate::SvgRenderTarget`]
/// directly when serialization should instead be explicitly deferred.
#[derive(Clone)]
pub struct SvgSurface {
    state: Arc<Mutex<SurfaceState>>,
}

impl SvgSurface {
    pub fn new(options: SvgExportOptions) -> Result<Self, SvgError> {
        options.validate()?;
        Ok(Self {
            state: Arc::new(Mutex::new(SurfaceState {
                options,
                revision: 0,
                frames: 0,
                latest: None,
            })),
        })
    }

    pub fn options(&self) -> SvgExportOptions {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .options
            .clone()
    }

    /// Validate and replace policy for subsequent exports. The current retained result is not
    /// rewritten; its revision states which options produced it.
    pub fn set_options(&self, options: SvgExportOptions) -> Result<u64, SvgError> {
        options.validate()?;
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.options = options;
        state.revision = state.revision.wrapping_add(1);
        Ok(state.revision)
    }

    pub fn frame_count(&self) -> u64 {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).frames
    }

    /// Move out the newest result without cloning SVG/image data. None means there is no new
    /// result since the last call. Readiness of the render surface does not depend on this call.
    pub fn take_frame(&self) -> Option<SvgSurfaceFrame> {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .latest
            .take()
    }

    pub(crate) fn publish(&self, document: &SvgDocument) {
        let (options, revision) = {
            let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            (state.options.clone(), state.revision)
        };
        // No shared lock during Skia replay, XML processing or compression.
        let result = document.export(&options);
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.frames = state.frames.wrapping_add(1);
        state.latest = Some(SvgSurfaceFrame {
            sequence: state.frames,
            options_revision: revision,
            result,
        });
    }
}

impl Default for SvgSurface {
    fn default() -> Self {
        Self::new(SvgExportOptions::default()).expect("default SVG policy is valid")
    }
}

impl IPlatformRenderSurface for SvgSurface {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
