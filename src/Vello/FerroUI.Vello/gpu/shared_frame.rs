//! A frame of a producer (a video or a camera) shared with the GPU modes: a bitmap whose pixels
//! are the producer's own storage, wrapped as a texture of the device, never copied to memory.
//!
//! The first time the bitmap is drawn on its device, one render pass converts the frame into a
//! texture of the pool in the canvas's premultiplied RGBA (`effect_passes::frame_conversion_texture`:
//! BGRA storage, a bottom-up first row, full-range Rec. 709 video). The frame's lease is held until
//! the queue has done that pass and is then given back, so the producer can reuse its storage at
//! once; the bitmap draws the converted texture from then on. Drawn in the CPU mode or on another
//! device, the bitmap draws nothing: there is no path through memory.

use crate::gpu::{VelloDeviceTexture, VelloWgpuDevice};
use ferroui_base::media::imaging::BitmapEncoderOptions;
use ferroui_base::platform::IBitmapImpl;
use ferroui_base::rendering::composition::frames::{FrameColor, FrameFormat, FrameLease, GpuCompletion, GpuTransferReason};
use ferroui_base::{PixelSize, Vector};
use std::any::Any;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

/// A frame of a producer as a bitmap of the GPU modes (see the module).
pub struct VelloSharedFrameBitmap {
    device: Arc<VelloWgpuDevice>,
    pixel_size: PixelSize,
    rec709: bool,
    bottom_up: bool,
    opaque: bool,
    conversion: GpuTransferReason,
    state: Mutex<State>,
}

struct State {
    /// The producer's storage as a texture of the device, until it is converted.
    source: Option<Arc<VelloDeviceTexture>>,
    /// The lease of the frame, until the conversion was read by the device.
    frame: Option<FrameLease>,
    /// The frame in premultiplied RGBA, once it was converted.
    converted: Option<Arc<VelloDeviceTexture>>,
}

impl VelloSharedFrameBitmap {
    /// The bitmap of `frame`, whose storage `source` wraps on `source`'s device.
    pub fn new(source: Arc<VelloDeviceTexture>, frame: FrameLease) -> Self {
        let metadata = frame.metadata();
        let rec709 = metadata.color == FrameColor::Rec709Full;
        let conversion = if rec709 {
            GpuTransferReason::ColorSpaceConversion
        } else if metadata.format != FrameFormat::Rgba8 || !metadata.top_left_origin {
            GpuTransferReason::PixelFormatConversion
        } else {
            GpuTransferReason::DeviceCompatibility
        };
        Self {
            device: source.device().clone(),
            pixel_size: PixelSize::new(metadata.width as i32, metadata.height as i32),
            rec709,
            bottom_up: !metadata.top_left_origin,
            opaque: metadata.alpha == ferroui_base::rendering::composition::frames::FrameAlpha::Opaque,
            conversion,
            state: Mutex::new(State { source: Some(source), frame: Some(frame), converted: None }),
        }
    }

    /// The device the frame is a texture of.
    pub fn device(&self) -> &Arc<VelloWgpuDevice> {
        &self.device
    }

    /// The frame as a texture of premultiplied RGBA of its device, converted when it is first
    /// asked for. `None` once the bitmap is disposed, when the frame was replaced before it was
    /// converted, or when the device could not run the conversion.
    pub fn texture(&self) -> Option<Arc<VelloDeviceTexture>> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(converted) = &state.converted {
            return Some(converted.clone());
        }
        let frame = state.frame.clone()?;
        if !frame.is_current() || self.device.is_lost() {
            return None;
        }
        let source = state.source.clone()?;
        let converted =
            crate::gpu::effect_passes::frame_conversion_texture(&self.device, &source, self.rec709, self.bottom_up, self.opaque)?;
        frame.counters().gpu_conversion(self.conversion);
        // The pass was submitted: the lease goes with it until the queue has done it.
        let completion = GpuCompletion::new();
        if completion.retain(frame).is_err() {
            completion.mark_indeterminate();
        }
        self.device.release_frames_after_submitted_work(completion);
        state.frame = None;
        state.source = None;
        state.converted = Some(converted.clone());
        Some(converted)
    }
}

impl IBitmapImpl for VelloSharedFrameBitmap {
    fn dpi(&self) -> Vector {
        Vector::new(96.0, 96.0)
    }

    fn pixel_size(&self) -> PixelSize {
        self.pixel_size
    }

    fn version(&self) -> i32 {
        1
    }

    fn save(&self, _: &mut dyn Write, _: &BitmapEncoderOptions) -> io::Result<()> {
        Err(io::Error::new(io::ErrorKind::Unsupported, "GPU frame export requires an explicit counted capture"))
    }

    fn dispose(&self) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.source = None;
        state.frame = None;
        state.converted = None;
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl crate::i_drawable_bitmap_impl::IDrawableBitmapImpl for VelloSharedFrameBitmap {
    /// None: a frame of a producer is drawn on its device only.
    fn image(&self) -> Option<peniko::ImageData> {
        None
    }
}
