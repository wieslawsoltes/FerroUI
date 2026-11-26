use crate::{
    gpu::ISkiaGrContext, i_drawable_bitmap_impl::IDrawableBitmapImpl,
    immutable_bitmap::ImmutableBitmap,
};
use ferroui_base::rendering::composition::frames::{FrameColor, FrameError, FrameLease};
use ferroui_base::{
    media::imaging::BitmapEncoderOptions, platform::IBitmapImpl, PixelSize, Vector,
};
use skia_safe::{Canvas, Image, Paint, Rect, SamplingOptions};
use std::{
    any::Any,
    io::{self, Write},
    sync::Mutex,
};

/// A GPU-only immutable image. The ordinary composition bitmap path draws it;
/// CPU export/locking is unavailable. Each draw retains its native lease until
/// completion on the actual rendering context's queue.
pub(crate) struct LeasedGpuBitmap {
    image: ImmutableBitmap,
    frame: Mutex<Option<FrameLease>>,
    context_id: u64,
    color: FrameColor,
    /// The `id<MTLTexture>` the image wraps on Metal, as an address: valid while the image lives
    /// (the wrapper holds a reference). 0 on the other routes.
    #[cfg_attr(not(all(feature = "media3d", target_vendor = "apple", not(ferro_skia_apple_ganesh))), allow(dead_code))]
    metal_texture: usize,
    /// The name of the `GL_TEXTURE_2D` the image borrows on GL, valid while the frame is held. 0
    /// on the other routes and for a texture of another target.
    #[cfg_attr(not(all(feature = "media3d", ferro_skia_ganesh_gl)), allow(dead_code))]
    gl_texture: u32,
}
impl LeasedGpuBitmap {
    pub(crate) fn new(image: Image, frame: FrameLease, context_id: u64) -> Self {
        Self::with_metal_texture(image, frame, context_id, std::ptr::null_mut())
    }

    /// A frame whose image wraps the texture `metal_texture` (null on the other routes).
    pub(crate) fn with_metal_texture(
        image: Image,
        frame: FrameLease,
        context_id: u64,
        metal_texture: *mut std::ffi::c_void,
    ) -> Self {
        let color = frame.metadata().color;
        Self {
            image: ImmutableBitmap::from_image(image, None),
            frame: Mutex::new(Some(frame)),
            context_id,
            color,
            metal_texture: metal_texture as usize,
            gl_texture: 0,
        }
    }

    /// A frame whose image borrows the `GL_TEXTURE_2D` named `gl_texture`.
    #[cfg_attr(not(ferro_skia_ganesh_gl), allow(dead_code))]
    pub(crate) fn with_gl_texture(image: Image, frame: FrameLease, context_id: u64, gl_texture: u32) -> Self {
        Self { gl_texture, ..Self::new(image, frame, context_id) }
    }

    /// The name and size of the `GL_TEXTURE_2D` of the frame, while the frame is held and current
    /// on `context_id`.
    #[cfg(all(feature = "media3d", ferro_skia_ganesh_gl))]
    pub(crate) fn gl_texture(&self, context_id: u64) -> Option<(i32, (u32, u32))> {
        let held = self.frame.lock().unwrap();
        let frame = held.as_ref().filter(|frame| frame.is_current())?;
        if self.context_id != context_id || self.gl_texture == 0 {
            return None;
        }
        let metadata = frame.metadata();
        Some((self.gl_texture as i32, (metadata.width, metadata.height)))
    }

    /// The `id<MTLTexture>` of the frame, while the frame is held and current on `context_id`.
    #[cfg(all(feature = "media3d", target_vendor = "apple", not(ferro_skia_apple_ganesh)))]
    pub(crate) fn metal_texture(&self, context_id: u64) -> Option<*mut std::ffi::c_void> {
        let held = self.frame.lock().unwrap();
        let usable = self.context_id == context_id
            && self.metal_texture != 0
            && held.as_ref().is_some_and(FrameLease::is_current);
        usable.then_some(self.metal_texture as *mut std::ffi::c_void)
    }

    /// Keeps the storage of the frame until the GPU has done what `gr` submits next, for a 3D
    /// render that samples it. `false` when the frame cannot be read any more.
    #[cfg(all(
        feature = "media3d",
        any(ferro_skia_ganesh_gl, all(target_vendor = "apple", not(ferro_skia_apple_ganesh)))
    ))]
    pub(crate) fn retain_for_render(&self, gr: &dyn ISkiaGrContext) -> bool {
        if gr.frame_context_id() != Some(self.context_id) {
            return false;
        }
        let Some(frame) = self.frame.lock().unwrap().clone() else {
            return false;
        };
        if !frame.is_current() {
            return false;
        }
        let counters = frame.counters();
        if gr.retain_frame(frame).is_err() {
            return false;
        }
        if self.color == FrameColor::Rec709Full {
            counters.gpu_conversion(
                ferroui_base::rendering::composition::frames::GpuTransferReason::ColorSpaceConversion,
            );
        }
        true
    }
}
impl IBitmapImpl for LeasedGpuBitmap {
    fn dpi(&self) -> Vector {
        self.image.dpi()
    }
    fn pixel_size(&self) -> PixelSize {
        self.image.pixel_size()
    }
    fn version(&self) -> i32 {
        1
    }
    fn save(&self, _: &mut dyn Write, _: &BitmapEncoderOptions) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "GPU frame export requires an explicit counted capture",
        ))
    }
    fn dispose(&self) {
        self.image.dispose();
        let frame = self.frame.lock().unwrap().take();
        drop(frame);
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
impl IDrawableBitmapImpl for LeasedGpuBitmap {
    fn draw(
        &self,
        gr: Option<&dyn ISkiaGrContext>,
        canvas: &Canvas,
        source: &Rect,
        destination: &Rect,
        sampling: SamplingOptions,
        paint: &Paint,
    ) {
        let Some(gr) = gr.filter(|gr| gr.frame_context_id() == Some(self.context_id)) else {
            return;
        };
        let Some(frame) = self.frame.lock().unwrap().clone() else {
            return;
        };
        if !frame.is_current() {
            return;
        }
        let counters = frame.counters();
        if gr.retain_frame(frame).is_err() {
            return;
        }
        let conversion = match frame_color_filter(self.color) {
            Ok(value) => value,
            Err(_) => return,
        };
        let mut final_paint = paint.clone();
        if let Some(conversion) = conversion {
            final_paint.set_color_filter(match paint.color_filter() {
                Some(outer) => outer.composed(conversion),
                None => Some(conversion),
            });
            counters.gpu_conversion(ferroui_base::rendering::composition::frames::GpuTransferReason::ColorSpaceConversion);
        }
        self.image.draw(
            Some(gr),
            canvas,
            source,
            destination,
            sampling,
            &final_paint,
        );
    }
}

/// FerroUI's ordinary canvas uses untagged sRGB values. Normalize known SDR
/// video in the sampling shader, rather than relying on destination tagging or
/// allocating an intermediate image. Rec.709 uses the BT.1886 reference EOTF.
pub(crate) fn frame_color_filter(
    color: FrameColor,
) -> Result<Option<skia_safe::ColorFilter>, FrameError> {
    match color {
        FrameColor::Srgb => Ok(None),
        FrameColor::Rec709Full => {
            ferroui_base::ferro_thread_local! {
                static VIDEO_COLOR: Option<skia_safe::ColorFilter> = skia_safe::RuntimeEffect::make_for_color_filter(
                    "half4 main(half4 c) { float3 v = pow(max(float3(c.rgb), float3(0)), float3(2.4)); float3 low = 12.92*v; float3 high = 1.055*pow(v, float3(1.0/2.4))-0.055; return half4(mix(low, high, step(float3(0.0031308), v)), c.a); }", None)
                    .ok().and_then(|effect| effect.make_color_filter(skia_safe::Data::new_empty(), None));
            }
            VIDEO_COLOR.with(|filter| filter.clone().map(Some).ok_or(FrameError::UnsupportedColor))
        }
        _ => Err(FrameError::UnsupportedColor),
    }
}
