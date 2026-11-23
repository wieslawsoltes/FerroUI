use crate::{native, NativeDevice, NativeFrame};
use ferroui_media::{frames::*, *};
use ferroui_media_opengl::{NativeGlFrameProducer, ReadyGlFrame};

/// The same Windows media sessions can publish through ANGLE or native D3D11.
#[derive(Clone)]
pub enum WindowsFrameProducer {
    Gl(NativeGlFrameProducer),
    #[cfg(feature = "direct2d")]
    Direct3D11 {
        budget: FrameBudget,
        device: DeviceIdentity,
        generation: u64,
    },
    /// A renderer on another API of the same adapter: a GPU copy shared by an NT handle (#575).
    /// Tried only where the GL route is not there, and only with `FERROUI_MEDIA_NT_HANDLE=1`.
    SharedHandle {
        budget: FrameBudget,
        device: DeviceIdentity,
        luid: Vec<u8>,
        generation: u64,
    },
}
impl From<NativeGlFrameProducer> for WindowsFrameProducer {
    fn from(value: NativeGlFrameProducer) -> Self {
        Self::Gl(value)
    }
}
impl WindowsFrameProducer {
    /// The GL route where the renderer has it, otherwise the NT handle route when it is turned on.
    /// The error is the GL route's, as it is without that route.
    pub fn from_interop(
        interop: &dyn ferroui_base::rendering::composition::ICompositionGpuInterop,
        budget: FrameBudget,
    ) -> Result<Self, MediaError> {
        match NativeGlFrameProducer::from_interop(interop, budget.clone()) {
            Ok(producer) => Ok(Self::Gl(producer)),
            Err(error) => {
                use ferroui_base::platform::KnownPlatformGraphicsExternalImageHandleTypes as Handles;
                let handle = Handles::D3D11_TEXTURE_NT_HANDLE;
                let switched = std::env::var("FERROUI_MEDIA_NT_HANDLE").is_ok_and(|value| value == "1");
                match interop.device_luid() {
                    Some(luid)
                        if switched
                            && interop.shared_context_id().is_none()
                            && interop.supported_image_handle_types().iter().any(|v| v == handle) =>
                    {
                        Ok(Self::SharedHandle {
                            budget,
                            device: DeviceIdentity::Luid(luid.clone()),
                            luid,
                            generation: interop.shared_context_generation(),
                        })
                    }
                    _ => Err(error),
                }
            }
        }
    }
    #[cfg(feature = "direct2d")]
    pub fn from_direct2d(
        interop: &dyn ferroui_base::rendering::composition::ICompositionGpuInterop,
        budget: FrameBudget,
    ) -> Result<Self, MediaError> {
        let luid = ferroui_direct2d::device_luid();
        if interop.device_luid() != Some(luid.clone())
            || interop.shared_context_id().is_some()
            || !interop
                .supported_image_handle_types()
                .iter()
                .any(|v| v == "SharedFrameD3D11Texture2D")
        {
            return Err(MediaError::new(
                MediaErrorKind::Unsupported,
                "No admitted native D3D11 media importer",
            ));
        }
        Ok(Self::Direct3D11 {
            budget,
            device: DeviceIdentity::Luid(luid),
            generation: interop.shared_context_generation(),
        })
    }
    pub fn budget(&self) -> FrameBudget {
        match self {
            Self::Gl(p) => p.budget(),
            #[cfg(feature = "direct2d")]
            Self::Direct3D11 { budget, .. } => budget.clone(),
            Self::SharedHandle { budget, .. } => budget.clone(),
        }
    }
    pub fn device(&self) -> DeviceIdentity {
        match self {
            Self::Gl(p) => p.device(),
            #[cfg(feature = "direct2d")]
            Self::Direct3D11 { device, .. } => device.clone(),
            Self::SharedHandle { device, .. } => device.clone(),
        }
    }
    pub fn device_generation(&self) -> u64 {
        match self {
            Self::Gl(p) => p.device_generation(),
            #[cfg(feature = "direct2d")]
            Self::Direct3D11 { generation, .. } => *generation,
            Self::SharedHandle { generation, .. } => *generation,
        }
    }
    pub fn with_budget(&self, budget: FrameBudget) -> Self {
        match self {
            Self::Gl(p) => Self::Gl(p.with_budget(budget)),
            #[cfg(feature = "direct2d")]
            Self::Direct3D11 {
                device, generation, ..
            } => Self::Direct3D11 {
                budget,
                device: device.clone(),
                generation: *generation,
            },
            Self::SharedHandle { device, luid, generation, .. } => Self::SharedHandle {
                budget,
                device: device.clone(),
                luid: luid.clone(),
                generation: *generation,
            },
        }
    }
    pub fn format(&self) -> FrameFormat {
        match self {
            Self::Gl(_) => FrameFormat::Rgba8,
            #[cfg(feature = "direct2d")]
            Self::Direct3D11 { .. } => FrameFormat::Bgra8,
            Self::SharedHandle { .. } => FrameFormat::Bgra8,
        }
    }
    pub fn native_device(&self) -> MediaFuture<NativeDevice> {
        match self {
            Self::Gl(p) => NativeDevice::from_renderer(p),
            #[cfg(feature = "direct2d")]
            Self::Direct3D11 { generation, .. } => {
                let generation = *generation;
                Box::pin(async move {
                    if generation != ferroui_direct2d::device_generation() {
                        return Err(MediaError::new(
                            MediaErrorKind::DeviceLost,
                            "The native D3D11 device was replaced",
                        ));
                    }
                    NativeDevice::from_direct2d()
                })
            }
            Self::SharedHandle { luid, .. } => {
                let device = NativeDevice::for_luid(luid);
                Box::pin(std::future::ready(device))
            }
        }
    }
    pub(crate) fn import(
        &self,
        metadata: FrameMetadata,
        frame: NativeFrame,
    ) -> MediaFuture<Option<ReadyWindowsFrame>> {
        match self {
            Self::Gl(p) => {
                let future = native::import(p, metadata, frame);
                Box::pin(async move { Ok(future.await?.map(ReadyWindowsFrame::Gl)) })
            }
            #[cfg(feature = "direct2d")]
            Self::Direct3D11 {
                budget,
                device,
                generation,
            } => {
                let result = (|| {
                    if metadata.device.as_ref() != Some(device)
                        || metadata.device_generation != *generation
                        || *generation != ferroui_direct2d::device_generation()
                    {
                        return Err(MediaError::new(
                            MediaErrorKind::DeviceLost,
                            "Stale native D3D11 frame",
                        ));
                    }
                    let reservation = match budget.reserve(metadata) {
                        Ok(value) => value,
                        Err(FrameError::BudgetExceeded) => return Ok(None),
                        Err(error) => {
                            return Err(MediaError::new(
                                MediaErrorKind::Unsupported,
                                error.to_string(),
                            ))
                        }
                    };
                    let texture = frame.direct2d_texture()?;
                    Ok(Some(ReadyWindowsFrame::Direct3D11(
                        reservation.lease(texture),
                    )))
                })();
                Box::pin(async move { result })
            }
            Self::SharedHandle { budget, device, generation, .. } => {
                let result = (|| {
                    if metadata.device.as_ref() != Some(device) || metadata.device_generation != *generation {
                        return Err(MediaError::new(MediaErrorKind::DeviceLost, "Stale shared frame"));
                    }
                    let reservation = match budget.reserve(metadata) {
                        Ok(value) => value,
                        Err(FrameError::BudgetExceeded) => return Ok(None),
                        Err(error) => {
                            return Err(MediaError::new(MediaErrorKind::Unsupported, error.to_string()))
                        }
                    };
                    let copy = std::sync::Arc::new(frame.share()?);
                    let lease = reservation.lease(SharedHandleFrame(copy.clone()));
                    Ok(Some(ReadyWindowsFrame::SharedHandle(lease, copy)))
                })();
                Box::pin(async move { result })
            }
        }
    }
}
/// What a frame of the NT handle route leases: its shared copy.
pub(crate) struct SharedHandleFrame(pub(crate) std::sync::Arc<crate::native::SharedFrameCopy>);
pub(crate) enum ReadyWindowsFrame {
    Gl(ReadyGlFrame),
    #[cfg(feature = "direct2d")]
    Direct3D11(FrameLease),
    /// Handed over once its GPU copy has completed.
    SharedHandle(FrameLease, std::sync::Arc<crate::native::SharedFrameCopy>),
}
impl ReadyWindowsFrame {
    pub fn take_if_ready(&self) -> Result<Option<FrameLease>, MediaError> {
        match self {
            Self::Gl(f) => f.take_if_ready(),
            #[cfg(feature = "direct2d")]
            Self::Direct3D11(f) => Ok(Some(f.clone())),
            Self::SharedHandle(f, copy) => Ok(copy.is_ready()?.then(|| f.clone())),
        }
    }
}
