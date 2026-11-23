//! The frames of a GStreamer session as DMA-BUFs for a renderer on its own Vulkan device (#575).
//!
//! The GL route ([`NativeGlFrameProducer`]) stays the default: [`LinuxFrameSource::from_interop`]
//! takes it wherever the renderer has a context of OpenGL. This route is tried only where it does
//! not, and only with `FERROUI_MEDIA_DMA_BUF=1`, until a run in the Linux virtual machine has
//! proved it. It takes single-plane XRGB and XBGR buffers with an explicit modifier. A frame that
//! cannot be imported is refused; nothing is read on the CPU.

use crate::{native::NativeFrame, unsupported};
use ferroui_base::platform::{
    KnownPlatformGraphicsExternalImageHandleTypes as HandleTypes, PlatformGraphicsDrmFormat,
};
use ferroui_base::rendering::composition::ICompositionGpuInterop;
use ferroui_media::{frames::*, *};
use ferroui_media_opengl::NativeGlFrameProducer;

/// `DRM_FORMAT_XRGB8888`.
pub(crate) const DRM_XR24: u32 = 0x3432_5258;
/// `DRM_FORMAT_XBGR8888`.
pub(crate) const DRM_XB24: u32 = 0x3432_4258;
/// `DRM_FORMAT_MOD_INVALID`: the implicit layout, which this route does not take.
const MOD_INVALID: u64 = 0x00ff_ffff_ffff_ffff;

/// What a frame of the route leases: the buffer of the native sample.
pub(crate) struct DmaBufFrame(pub(crate) NativeFrame);

/// The caps names of the pairs this route takes.
fn caps_formats(pairs: &[PlatformGraphicsDrmFormat]) -> Vec<String> {
    let mut formats = Vec::new();
    for pair in pairs {
        if pair.modifier == MOD_INVALID {
            continue;
        }
        let name = match pair.format {
            DRM_XR24 => "XR24",
            DRM_XB24 => "XB24",
            _ => continue,
        };
        let value = if pair.modifier == 0 { name.to_owned() } else { format!("{name}:0x{:016x}", pair.modifier) };
        if !formats.contains(&value) {
            formats.push(value);
        }
    }
    formats
}

/// A producer of DMA-BUF frames for one Vulkan device of the renderer.
#[derive(Clone)]
pub struct DmaBufFrameProducer {
    pub(crate) budget: FrameBudget,
    pub(crate) device: DeviceIdentity,
    pub(crate) generation: u64,
    formats: Vec<String>,
}

impl DmaBufFrameProducer {
    /// The producer of `interop`, when the route is turned on and the renderer imports DMA-BUFs.
    pub fn from_interop(interop: &dyn ICompositionGpuInterop, budget: FrameBudget) -> Result<Self, MediaError> {
        if !std::env::var("FERROUI_MEDIA_DMA_BUF").is_ok_and(|value| value == "1") {
            return Err(unsupported("The DMA-BUF route is off"));
        }
        if !interop.supported_image_handle_types().iter().any(|v| v == HandleTypes::DMA_BUF_FILE_DESCRIPTOR) {
            return Err(unsupported("The renderer imports no DMA-BUF"));
        }
        let uuid = interop.device_uuid().ok_or_else(|| unsupported("The renderer names no device"))?;
        let formats = caps_formats(&interop.supported_dma_buf_formats().unwrap_or_default());
        if formats.is_empty() {
            return Err(unsupported("No explicitly negotiated native DMA-BUF formats"));
        }
        Ok(Self {
            budget,
            device: DeviceIdentity::Uuid(uuid),
            generation: interop.shared_context_generation(),
            formats,
        })
    }

    pub(crate) fn formats(&self) -> &[String] {
        &self.formats
    }
}

/// Where the frames of a session go: the GL route, the default, or DMA-BUFs for a renderer with
/// no context of OpenGL.
#[derive(Clone)]
pub enum LinuxFrameSource {
    /// Textures of the renderer's context of OpenGL.
    Gl(NativeGlFrameProducer),
    /// DMA-BUFs imported by the renderer's Vulkan device.
    DmaBuf(DmaBufFrameProducer),
}

impl From<NativeGlFrameProducer> for LinuxFrameSource {
    fn from(producer: NativeGlFrameProducer) -> Self {
        Self::Gl(producer)
    }
}

impl LinuxFrameSource {
    /// The GL route where the renderer has it, otherwise the DMA-BUF route when it is turned on.
    /// The error is the GL route's, as it is without this route.
    pub fn from_interop(interop: &dyn ICompositionGpuInterop, budget: FrameBudget) -> Result<Self, MediaError> {
        match NativeGlFrameProducer::from_interop(interop, budget.clone()) {
            Ok(producer) => Ok(Self::Gl(producer)),
            Err(error) => DmaBufFrameProducer::from_interop(interop, budget).map(Self::DmaBuf).map_err(|_| error),
        }
    }

    /// The budget of the frames.
    pub fn budget(&self) -> FrameBudget {
        match self {
            Self::Gl(producer) => producer.budget(),
            Self::DmaBuf(producer) => producer.budget.clone(),
        }
    }

    /// The same source with another budget.
    pub fn with_budget(&self, budget: FrameBudget) -> Self {
        match self {
            Self::Gl(producer) => Self::Gl(producer.with_budget(budget)),
            Self::DmaBuf(producer) => Self::DmaBuf(DmaBufFrameProducer { budget, ..producer.clone() }),
        }
    }
}

/// The importer of the frames of a [`LinuxFrameSource`]: a DMA-BUF frame by its buffer, any other
/// by the importer of the GL route.
#[cfg(feature = "controls")]
pub struct NativeGlTextureImporter;

#[cfg(feature = "controls")]
impl ferroui_media_controls::TextureFrameImporter for NativeGlTextureImporter {
    fn import(
        &self,
        interop: &dyn ICompositionGpuInterop,
        frame: &FrameLease,
    ) -> Result<ferroui_media_controls::ImportedTextureFrame, MediaError> {
        use ferroui_base::platform::{
            LeasedPlatformHandle, PlatformGraphicsExternalImageDmaBufProperties, PlatformGraphicsExternalImageFormat,
            PlatformGraphicsExternalImageProperties,
        };
        use ferroui_media_controls::TextureFrameImporter as _;
        let Some(DmaBufFrame(native)) = frame.resource::<DmaBufFrame>() else {
            return ferroui_media_opengl::NativeGlTextureImporter.import(interop, frame);
        };
        let m = frame.metadata();
        if m.device_generation != interop.shared_context_generation()
            || m.device != interop.device_uuid().map(DeviceIdentity::Uuid)
        {
            return Err(unsupported("Frame and renderer devices differ"));
        }
        let info = native.info();
        let (fd, stride, offset) = info.plane(0);
        let format = match info.format {
            DRM_XR24 => PlatformGraphicsExternalImageFormat::B8G8R8A8UNorm,
            _ => PlatformGraphicsExternalImageFormat::R8G8B8A8UNorm,
        };
        let handle = LeasedPlatformHandle::new(fd as isize, Some(HandleTypes::DMA_BUF_FILE_DESCRIPTOR), frame.clone());
        let image = interop.import_image(
            std::rc::Rc::new(handle),
            PlatformGraphicsExternalImageProperties {
                width: m.width as i32,
                height: m.height as i32,
                format,
                top_left_origin: m.top_left_origin,
                memory_size: info.bytes,
                memory_offset: u64::from(offset),
                vulkan_properties: None,
                dma_buf_properties: Some(PlatformGraphicsExternalImageDmaBufProperties {
                    drm_format: info.format,
                    drm_modifier: info.modifier,
                    plane_count: 1,
                    plane_fds: None,
                    plane_strides: Some(vec![stride]),
                    plane_offsets: Some(vec![offset]),
                }),
            },
        );
        // The stream hands a frame over only once the fences of its buffer have signalled.
        let producer_ready = GpuCompletion::new();
        producer_ready.complete();
        Ok(ferroui_media_controls::ImportedTextureFrame { image, producer_ready })
    }
}
