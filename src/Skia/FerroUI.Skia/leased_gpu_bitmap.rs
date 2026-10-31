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
}
impl LeasedGpuBitmap {
    pub(crate) fn new(image: Image, frame: FrameLease, context_id: u64) -> Self {
        let color = frame.metadata().color;
        Self {
            image: ImmutableBitmap::from_image(image, None),
            frame: Mutex::new(Some(frame)),
            context_id,
            color,
        }
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
            counters.gpu_conversion();
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
