use crate::drawing_context_impl::{CanvasSource, CreateInfo, DrawingContextImpl, RecordedLeases};
use crate::gpu::{ISkiaGpu, ISkiaGrContext};
use ferroui_base::{Size, Vector};
use skia_safe::{Picture, PictureRecorder, Rect};
use std::cell::RefCell;
use std::rc::Rc;

/// A render target that records the drawing into a Skia picture.
pub struct PictureRenderTarget {
    gpu: Option<Rc<dyn ISkiaGpu>>,
    gr_context: Option<Rc<dyn ISkiaGrContext>>,
    dpi: Vector,
    picture: Rc<RefCell<Option<Picture>>>,
}

impl PictureRenderTarget {
    /// Creates a picture render target for intermediate surfaces of the
    /// given GPU and DPI.
    pub fn new(
        gpu: Option<Rc<dyn ISkiaGpu>>,
        gr_context: Option<Rc<dyn ISkiaGrContext>>,
        dpi: Vector,
    ) -> Self {
        Self {
            gpu,
            gr_context,
            dpi,
            picture: Rc::new(RefCell::new(None)),
        }
    }

    /// Takes the picture recorded by the last drawing context.
    ///
    /// # Panics
    /// Panics when no picture has been recorded since the last call.
    pub fn get_picture(&self) -> Picture {
        self.picture
            .borrow_mut()
            .take()
            .unwrap_or_else(|| panic!("No picture has been recorded"))
    }

    /// Creates a drawing context that records a picture of the given size.
    /// The picture becomes available when the context is disposed.
    pub fn create_drawing_context(&self, size: Size, scale_to_dpi: bool) -> DrawingContextImpl {
        self.create_recording_context(ferroui_base::Rect::from_size(size), scale_to_dpi, false)
    }

    /// Creates an export recording that also captures custom drawing through
    /// [`crate::ISkiaApiLeaseFeature`]. Unlike a surface lease, the recorded lease
    /// has no surface or GPU target; its canvas commands are inserted in order
    /// before the next drawing operation, or when the context is disposed.
    ///
    /// The lease canvas does not expose the surrounding clip and changes to its
    /// state do not persist between calls. Its drawing is still subject to the
    /// parent recording's clip, layers and transforms. Exporters must treat
    /// native drawing as unclassified when choosing a vector fidelity policy.
    pub fn create_export_drawing_context(
        &self,
        size: Size,
        scale_to_dpi: bool,
    ) -> DrawingContextImpl {
        self.create_export_drawing_context_with_bounds(
            ferroui_base::Rect::from_size(size),
            scale_to_dpi,
        )
    }

    /// Creates an export recording with explicit bounds in drawing coordinates.
    /// Bounds may have a nonzero or negative origin. No translation is applied
    /// to the context, so setting its transform retains the usual semantics.
    /// When scaling to DPI, both the origin and size of the recording bounds
    /// are scaled by the corresponding DPI components.
    pub fn create_export_drawing_context_with_bounds(
        &self,
        bounds: ferroui_base::Rect,
        scale_to_dpi: bool,
    ) -> DrawingContextImpl {
        self.create_recording_context(bounds, scale_to_dpi, true)
    }

    fn create_recording_context(
        &self,
        mut bounds: ferroui_base::Rect,
        scale_to_dpi: bool,
        capture_leases: bool,
    ) -> DrawingContextImpl {
        if scale_to_dpi {
            let (x_scale, y_scale) = (self.dpi.x / 96.0, self.dpi.y / 96.0);
            bounds = ferroui_base::Rect::new(
                bounds.x * x_scale,
                bounds.y * y_scale,
                bounds.width * x_scale,
                bounds.height * y_scale,
            );
        }
        let bounds = Rect::new(
            bounds.x as f32,
            bounds.y as f32,
            (bounds.x + bounds.width) as f32,
            (bounds.y + bounds.height) as f32,
        );

        let mut recorder = PictureRecorder::new();
        {
            let canvas = recorder.begin_recording(bounds, false);
            canvas.restore_to_count(1);
            canvas.reset_matrix();
        }

        let picture = self.picture.clone();

        let create_info = CreateInfo {
            canvas: Some(CanvasSource::Recorder {
                recorder,
                on_finished: Box::new(move |recorded| *picture.borrow_mut() = recorded),
                leases: capture_leases.then(|| Rc::new(RecordedLeases::new(bounds))),
            }),
            scale_drawing_to_dpi: scale_to_dpi,
            dpi: self.dpi,
            disable_subpixel_text_rendering: true,
            gr_context: self.gr_context.clone(),
            gpu: self.gpu.clone(),
            ..CreateInfo::default()
        };

        DrawingContextImpl::new(create_info, Vec::new())
    }

    /// Releases the recorded picture.
    pub fn dispose(&self) {
        self.picture.borrow_mut().take();
    }
}
