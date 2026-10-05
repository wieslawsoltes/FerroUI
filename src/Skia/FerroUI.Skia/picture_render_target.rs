use crate::drawing_context_impl::{CanvasSource, CreateInfo, DrawingContextImpl};
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
    pub fn new(gpu: Option<Rc<dyn ISkiaGpu>>, gr_context: Option<Rc<dyn ISkiaGrContext>>, dpi: Vector) -> Self {
        Self { gpu, gr_context, dpi, picture: Rc::new(RefCell::new(None)) }
    }

    /// Takes the picture recorded by the last drawing context.
    ///
    /// # Panics
    /// Panics when no picture has been recorded since the last call.
    pub fn get_picture(&self) -> Picture {
        self.picture.borrow_mut().take().unwrap_or_else(|| panic!("No picture has been recorded"))
    }

    /// Creates a drawing context that records a picture of the given size.
    /// The picture becomes available when the context is disposed.
    pub fn create_drawing_context(&self, size: Size, scale_to_dpi: bool) -> DrawingContextImpl {
        let mut size = size;
        if scale_to_dpi {
            size = Size::new(size.width * (self.dpi.x / 96.0), size.height * (self.dpi.y / 96.0));
        }

        let mut recorder = PictureRecorder::new();
        {
            let canvas =
                recorder.begin_recording(Rect::new(0.0, 0.0, size.width as f32, size.height as f32), false);
            canvas.restore_to_count(1);
            canvas.reset_matrix();
        }

        let picture = self.picture.clone();

        let create_info = CreateInfo {
            canvas: Some(CanvasSource::Recorder {
                recorder,
                on_finished: Box::new(move |recorded| *picture.borrow_mut() = recorded),
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
