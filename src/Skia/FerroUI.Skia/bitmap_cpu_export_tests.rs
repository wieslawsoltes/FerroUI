use super::*;
use crate::leased_gpu_bitmap::LeasedGpuBitmap;
use crate::SurfaceRenderTargetCreateInfo;
use ferroui_base::media::imaging::BitmapEncoderOptions;
use ferroui_base::platform::{AlphaFormat, PixelFormat};
use ferroui_base::rendering::composition::frames::{
    DeviceIdentity, FrameAlpha, FrameBudget, FrameColor, FrameFormat, FrameIdentity, FrameMetadata,
    FramePlane, FrameRegion, FrameRotation,
};
use ferroui_base::{PixelSize, Vector};
use std::any::Any;
use std::io::{self, Write};

fn raster_image() -> skia_safe::Image {
    let mut surface = skia_safe::surfaces::raster_n32_premul((2, 2)).unwrap();
    surface.canvas().clear(skia_safe::Color::RED);
    surface.image_snapshot()
}

#[test]
fn every_cpu_bitmap_kind_remains_exportable() {
    let immutable = ImmutableBitmap::from_image(raster_image(), None);
    assert_eq!(is_bitmap_cpu_readable(&immutable), Some(true));
    let writeable = WriteableBitmapImpl::new(
        PixelSize::new(2, 2),
        Vector::new(96.0, 96.0),
        PixelFormat::RGBA8888,
        AlphaFormat::Premul,
    );
    assert_eq!(is_bitmap_cpu_readable(&*writeable), Some(true));
    let render_target = RenderTargetBitmapImpl::new(PixelSize::new(2, 2), Vector::new(96.0, 96.0));
    assert_eq!(is_bitmap_cpu_readable(&render_target), Some(true));
    let layer = SurfaceRenderTarget::new(SurfaceRenderTargetCreateInfo {
        width: 2,
        height: 2,
        dpi: Vector::new(96.0, 96.0),
        ..Default::default()
    });
    assert_eq!(is_bitmap_cpu_readable(&layer), Some(true));
}

#[test]
fn leased_frames_require_their_gpu_context_even_with_a_raster_test_image() {
    // No hardware fixture is needed to exercise the leased-frame contract:
    // LeasedGpuBitmap::draw returns without painting when its GPU context is
    // missing, regardless of how this test image's pixels happen to be stored.
    let frame = FrameBudget::new(Default::default())
        .lease(
            FrameMetadata {
                identity: FrameIdentity {
                    source: 1,
                    generation: 1,
                    sequence: 1,
                },
                width: 2,
                height: 2,
                visible: FrameRegion {
                    x: 0,
                    y: 0,
                    width: 2,
                    height: 2,
                },
                format: FrameFormat::Bgra8,
                color: FrameColor::Srgb,
                alpha: FrameAlpha::Opaque,
                rotation: FrameRotation::None,
                mirrored: false,
                top_left_origin: true,
                timestamp: std::time::Duration::ZERO,
                device: Some(DeviceIdentity::Context(1)),
                device_generation: 1,
                planes: vec![FramePlane {
                    offset: 0,
                    stride: 8,
                    length: 16,
                }],
                retained_bytes: 16,
            },
            (),
        )
        .unwrap();
    let bitmap = LeasedGpuBitmap::new(raster_image(), frame, 1);
    assert!(try_get_drawable_bitmap(&bitmap).is_some());
    assert_eq!(is_bitmap_cpu_readable(&bitmap), Some(false));
}

struct ForeignBitmap;

impl IBitmapImpl for ForeignBitmap {
    fn dpi(&self) -> Vector {
        Vector::new(96.0, 96.0)
    }
    fn pixel_size(&self) -> PixelSize {
        PixelSize::new(2, 2)
    }
    fn version(&self) -> i32 {
        0
    }
    fn save(&self, _: &mut dyn Write, _: &BitmapEncoderOptions) -> io::Result<()> {
        Err(io::Error::new(io::ErrorKind::Unsupported, "test bitmap"))
    }
    fn dispose(&self) {}
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[test]
fn foreign_bitmaps_remain_distinct_from_unreadable_skia_images() {
    assert_eq!(is_bitmap_cpu_readable(&ForeignBitmap), None);
}
