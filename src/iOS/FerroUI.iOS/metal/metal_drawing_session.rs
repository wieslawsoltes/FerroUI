//! One frame drawn to a drawable of the Metal layer of a view.

use super::{MetalDevice, SurfaceShared};
use ferroui_base::PixelSize;
use ferroui_metal::{IMetalDevice, IMetalPlatformSurfaceRenderingSession};
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::{
    MTLBlitCommandEncoder, MTLCommandBuffer, MTLCommandEncoder, MTLCommandQueue, MTLDevice, MTLDrawable, MTLOrigin,
    MTLRegion, MTLSize, MTLStorageMode, MTLTexture, MTLTextureDescriptor, MTLTextureUsage,
};
use objc2_quartz_core::CAMetalDrawable;
use std::cell::RefCell;
use std::ffi::c_void;
use std::ptr::NonNull;
use std::rc::Rc;
use std::sync::Arc;

/// The pixels of a frame, read back from the texture it was drawn to.
/// An addition of the port, for the checks of an application that tests
/// itself (`FerroView::capture_next_frame`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameCapture {
    /// The width of the frame in pixels.
    pub width: usize,
    /// The height of the frame in pixels.
    pub height: usize,
    /// The number of bytes of a row of `pixels`.
    pub bytes_per_row: usize,
    /// The pixel format of the texture (`MTLPixelFormat`; 80 is
    /// `BGRA8Unorm`).
    pub pixel_format: usize,
    /// The pixels, row by row from the top, four bytes each.
    pub pixels: Vec<u8>,
}

impl FrameCapture {
    /// The four bytes of the pixel at `x`, `y`, in the order of the pixel
    /// format; none outside the frame.
    pub fn pixel(&self, x: usize, y: usize) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let offset = y * self.bytes_per_row + x * 4;
        self.pixels.get(offset..offset + 4)?.try_into().ok()
    }
}

/// One frame drawn to the Metal layer of a view; disposing the session
/// presents the frame.
pub struct MetalDrawingSession {
    device: Rc<dyn IMetalDevice>,
    drawable: RefCell<Option<Retained<ProtocolObject<dyn CAMetalDrawable>>>>,
    texture: Retained<ProtocolObject<dyn MTLTexture>>,
    size: PixelSize,
    scaling: f64,
    shared: Arc<SurfaceShared>,
    capturable: bool,
}

impl MetalDrawingSession {
    pub(crate) fn new(
        device: Rc<dyn IMetalDevice>,
        drawable: Retained<ProtocolObject<dyn CAMetalDrawable>>,
        size: PixelSize,
        scaling: f64,
        shared: Arc<SurfaceShared>,
        capturable: bool,
    ) -> Self {
        let texture = drawable.texture();
        Self { device, drawable: RefCell::new(Some(drawable)), texture, size, scaling, shared, capturable }
    }

    fn queue(&self) -> Retained<ProtocolObject<dyn MTLCommandQueue>> {
        match self.device.as_any().downcast_ref::<MetalDevice>() {
            Some(device) => device.queue(),
            None => panic!("The Metal device belongs to a different platform backend."),
        }
    }

    /// Reads the pixels of the frame back: the texture is copied to a
    /// texture the processor can read, by a command that runs after the
    /// commands that drew the frame (they were committed to the same
    /// queue).
    fn capture(&self, queue: &ProtocolObject<dyn MTLCommandQueue>) -> Option<FrameCapture> {
        let device = self.device.as_any().downcast_ref::<MetalDevice>()?;
        let (width, height) = (self.texture.width(), self.texture.height());
        let pixel_format = self.texture.pixelFormat();

        // SAFETY: a descriptor of a two-dimensional texture of the size
        // and the format of the texture of the drawable, without mipmaps.
        let descriptor = unsafe {
            MTLTextureDescriptor::texture2DDescriptorWithPixelFormat_width_height_mipmapped(
                pixel_format,
                width,
                height,
                false,
            )
        };
        descriptor.setStorageMode(MTLStorageMode::Shared);
        descriptor.setUsage(MTLTextureUsage::ShaderRead);
        let copy = device.mtl_device().newTextureWithDescriptor(&descriptor)?;

        let command_buffer = queue.commandBuffer()?;
        let blit = command_buffer.blitCommandEncoder()?;
        let origin = MTLOrigin { x: 0, y: 0, z: 0 };
        let size = MTLSize { width, height, depth: 1 };
        // SAFETY: both textures are two-dimensional, of one size and one
        // format, with one slice and one level, and the region copied is
        // the whole of each. The source was obtained from a layer that is
        // not "framebuffer only" (`capturable`), so it may be read.
        unsafe {
            blit.copyFromTexture_sourceSlice_sourceLevel_sourceOrigin_sourceSize_toTexture_destinationSlice_destinationLevel_destinationOrigin(
                &self.texture,
                0,
                0,
                origin,
                size,
                &copy,
                0,
                0,
                origin,
            );
        }
        blit.endEncoding();
        command_buffer.commit();
        command_buffer.waitUntilCompleted();

        let bytes_per_row = width * 4;
        let mut pixels = vec![0u8; bytes_per_row * height];
        let Some(bytes) = NonNull::new(pixels.as_mut_ptr().cast::<c_void>()) else {
            return None;
        };
        // SAFETY: the buffer has `bytes_per_row * height` bytes, which is
        // what a region of the whole texture of a format of four bytes a
        // pixel needs (the layer has such a format: the capture is refused
        // below for another), and the texture is in shared storage, which
        // the processor may read once the copy has completed.
        unsafe {
            copy.getBytes_bytesPerRow_fromRegion_mipmapLevel(bytes, bytes_per_row, MTLRegion { origin, size }, 0);
        }

        Some(FrameCapture { width, height, bytes_per_row, pixel_format: pixel_format.0, pixels })
    }
}

/// Whether a pixel format has four bytes a pixel, of those a Metal layer
/// can have: `BGRA8Unorm` (80), `BGRA8Unorm_sRGB` (81).
fn has_four_bytes_a_pixel(pixel_format: usize) -> bool {
    pixel_format == 80 || pixel_format == 81
}

impl IMetalPlatformSurfaceRenderingSession for MetalDrawingSession {
    fn texture(&self) -> *mut c_void {
        Retained::as_ptr(&self.texture) as *mut c_void
    }

    fn size(&self) -> PixelSize {
        self.size
    }

    fn scaling(&self) -> f64 {
        self.scaling
    }

    fn is_y_flipped(&self) -> bool {
        false
    }

    fn dispose(&self) {
        let Some(drawable) = self.drawable.borrow_mut().take() else {
            return;
        };
        let queue = self.queue();

        if self.capturable && has_four_bytes_a_pixel(self.texture.pixelFormat().0) {
            if let Some(callback) = self.shared.take_capture() {
                match self.capture(&queue) {
                    Some(capture) => callback(capture),
                    // The request stays for a later frame.
                    None => self.shared.request_capture(callback),
                }
            }
        }

        let Some(buffer) = queue.commandBuffer() else {
            panic!("The command queue of the Metal device gave no command buffer.");
        };
        let drawable: &ProtocolObject<dyn MTLDrawable> = ProtocolObject::from_ref(&*drawable);
        buffer.presentDrawable(drawable);
        buffer.commit();
        self.shared.frame_presented();
    }
}
