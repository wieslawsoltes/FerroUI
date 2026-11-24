use crate::{
    SvgDocument, SvgDrawingContext, SvgError, SvgExportOptions, SvgLayer, SvgLayerOptions,
    SvgOutput,
};
use ferroui_base::platform::{
    IDrawingContextImpl, IRenderTarget, PlatformRenderTargetState,
    RenderTargetDrawingContextProperties, RenderTargetError, RenderTargetProperties,
    RenderTargetSceneInfo,
};
use ferroui_base::{Matrix, Rect, RenderTargetCorruptedError, RenderTargetNotReadyError, Vector};
use ferroui_skia::PictureRenderTarget;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
struct TargetState {
    document: Option<SvgDocument>,
    recording: bool,
    disposed: bool,
    frames: u64,
}

struct FinishRecording(Rc<RefCell<TargetState>>);

impl Drop for FinishRecording {
    fn drop(&mut self) {
        self.0.borrow_mut().recording = false;
    }
}

type Completion = Rc<dyn Fn(&SvgDocument)>;

/// A headless SVG render target implementing FerroUI's normal [`IRenderTarget`] contract.
///
/// Hosts can supply this target to their rendering pipeline to capture a complete frame,
/// including compositor calls, instead of walking visuals with the immediate renderer.
/// The target records one isolated frame; named multi-layer documents use [`SvgDocument`].
/// SVG serialization is explicit and runs only when [`export_last`](Self::export_last) is
/// called, so the render callback does not pay XML/compression costs for unused frames.
///
/// Previous frame pixels are never retained. A completed frame replaces the snapshot only
/// after disposal of its drawing context, and overlapping recordings return NotReady.
#[derive(Default)]
pub struct SvgRenderTarget {
    state: Rc<RefCell<TargetState>>,
    completion: Option<Completion>,
}

impl SvgRenderTarget {
    pub fn new() -> Self {
        Self::default()
    }

    /// Surface integration hook. Invoked after recording finalization with no
    /// target-state borrow held, before publishing the retained document.
    pub(crate) fn with_completion(callback: impl Fn(&SvgDocument) + 'static) -> Self {
        Self {
            state: Rc::default(),
            completion: Some(Rc::new(callback)),
        }
    }

    /// Number of completed recordings, including ones with export diagnostics.
    pub fn frame_count(&self) -> u64 {
        self.state.borrow().frames
    }

    /// Export the most recently completed frame. None means no frame is available yet.
    pub fn export_last(&self, options: &SvgExportOptions) -> Result<Option<SvgOutput>, SvgError> {
        self.state
            .borrow()
            .document
            .as_ref()
            .map(|document| document.export(options))
            .transpose()
    }

    /// Transfer ownership of the last recording for repeated or deferred export.
    pub fn take_document(&self) -> Option<SvgDocument> {
        self.state.borrow_mut().document.take()
    }
}

impl IRenderTarget for SvgRenderTarget {
    fn properties(&self) -> RenderTargetProperties {
        RenderTargetProperties {
            retains_previous_frame_contents: false,
            is_suitable_for_direct_rendering: true,
        }
    }

    fn prefers_full_frame_rendering(&self) -> bool {
        true
    }

    fn create_drawing_context(
        &self,
        scene_info: &RenderTargetSceneInfo,
    ) -> (
        Box<dyn IDrawingContextImpl>,
        RenderTargetDrawingContextProperties,
    ) {
        self.try_create_drawing_context(scene_info)
            .expect("SVG render target must be ready before creating a context")
    }

    fn try_create_drawing_context(
        &self,
        scene_info: &RenderTargetSceneInfo,
    ) -> Result<
        (
            Box<dyn IDrawingContextImpl>,
            RenderTargetDrawingContextProperties,
        ),
        RenderTargetError,
    > {
        {
            let state = self.state.borrow();
            if state.disposed {
                return Err(RenderTargetCorruptedError::new_with_message(
                    "SVG render target is disposed",
                )
                .into());
            }
            if state.recording {
                return Err(RenderTargetNotReadyError::new_with_message(
                    "SVG render target already has an active recording",
                )
                .into());
            }
        }
        if scene_info.size.width <= 0
            || scene_info.size.height <= 0
            || !scene_info.scaling.is_finite()
            || scene_info.scaling <= 0.0
            || !scene_info.scaling.recip().is_finite()
        {
            return Err(RenderTargetCorruptedError::new_with_message(
                "SVG target requires positive pixel dimensions and finite positive scaling",
            )
            .into());
        }
        let mut document = SvgDocument::new(scene_info.logical_size)
            .map_err(|error| RenderTargetCorruptedError::new_with_message(error.to_string()))?;
        // The compositor supplies device-space clips, transforms and frame
        // bitmap blits. Record those pixel coordinates unchanged, then convert
        // the retained frame into logical SVG coordinates exactly once.
        // Keep explicit logical_size: pixel dimensions may have been rounded.
        let bounds = Rect::new(
            0.0,
            0.0,
            f64::from(scene_info.size.width),
            f64::from(scene_info.size.height),
        );
        let pixel_to_logical =
            Matrix::create_scale(scene_info.scaling.recip(), scene_info.scaling.recip());
        let picture_target = Rc::new(PictureRenderTarget::new(
            None,
            None,
            Vector::new(96.0, 96.0),
        ));
        let mut context = picture_target.create_export_drawing_context_with_bounds(bounds, false);
        // The recording cull bounds are an optimization hint, not a clip.
        // Keep this permanent target clip outside the client's audited stack;
        // neither its pop operations nor a larger export viewBox can remove it.
        context.push_clip(bounds);
        let state = self.state.clone();
        let completion = self.completion.clone();
        self.state.borrow_mut().recording = true;
        // Own readiness in the completion closure itself: dropping that
        // closure after a failed native finalization must release the target,
        // even when its body never runs and no picture can be retrieved.
        let finish_recording = FinishRecording(state.clone());
        let context = SvgDrawingContext::new(Box::new(context))
            .with_recording_bounds(bounds)
            .on_completed(move |audit, finalized| {
                let _finish_recording = finish_recording;
                if state.borrow().disposed {
                    return;
                }
                let picture = if finalized {
                    picture_target.get_picture()
                } else {
                    // An error-bearing document carries the failed capture
                    // through the same target/surface result APIs. Its audit
                    // rejects export before any placeholder can be replayed.
                    debug_assert!(!audit.errors.is_empty());
                    skia_safe::Picture::new_placeholder(skia_safe::Rect::from_wh(
                        bounds.width as f32,
                        bounds.height as f32,
                    ))
                };
                document.layers.push(SvgLayer {
                    options: SvgLayerOptions::new("frame", "FerroUI frame")
                        .with_bounds(bounds)
                        .with_transform(pixel_to_logical),
                    picture,
                    audit,
                    bounds,
                    vector_cache: RefCell::new(None),
                    raster_cache: RefCell::new(None),
                });
                if let Some(completion) = completion {
                    completion(&document);
                }
                let mut state = state.borrow_mut();
                // A surface callback can dispose its owner. Its finished frame
                // must then be released rather than resurrecting the target.
                if state.disposed {
                    return;
                }
                state.document = Some(document);
                state.frames += 1;
            });
        Ok((
            Box::new(context),
            RenderTargetDrawingContextProperties {
                previous_frame_is_retained: false,
            },
        ))
    }

    fn platform_render_target_state(&self) -> PlatformRenderTargetState {
        let state = self.state.borrow();
        if state.disposed {
            PlatformRenderTargetState::DISPOSED
        } else if state.recording {
            PlatformRenderTargetState::NOT_READY_TRY_LATER
        } else {
            PlatformRenderTargetState::READY
        }
    }

    fn dispose(&self) {
        let mut state = self.state.borrow_mut();
        state.disposed = true;
        state.document.take();
    }
}
