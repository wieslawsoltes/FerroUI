use super::{DeviceIdentity, FrameError, FrameFormat, FrameMetadata};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PresentationPolicy {
    #[default]
    GpuPreferred,
    GpuOnly,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Acceleration {
    Hardware,
    Software,
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationRoute {
    SharedTexture,
    GpuConversion,
    CpuFallback,
}
#[derive(Clone, Debug)]
pub struct FrameImportCapabilities {
    pub device: DeviceIdentity,
    pub generation: u64,
    pub graphics: Acceleration,
    pub formats: Vec<FrameFormat>,
    pub direct_sampling: bool,
    pub gpu_conversion: bool,
    pub synchronized: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PresentationSelection {
    pub route: PresentationRoute,
    /// Unknown graphics identity is allowed but never hardware verification.
    pub graphics: Acceleration,
    pub fallback_reason: Option<FrameError>,
}
impl FrameImportCapabilities {
    pub fn select(
        &self,
        frame: &FrameMetadata,
        policy: PresentationPolicy,
        cpu_fallback_available: bool,
    ) -> Result<PresentationSelection, FrameError> {
        let rejection = if self.graphics == Acceleration::Software {
            Some(FrameError::SoftwareGraphics)
        } else if frame.device_generation != self.generation {
            Some(FrameError::StaleGeneration)
        } else if frame.device.as_ref() != Some(&self.device) {
            Some(FrameError::DeviceMismatch)
        } else if !self.formats.contains(&frame.format) {
            Some(FrameError::UnsupportedFormat)
        } else if !self.synchronized {
            Some(FrameError::MissingSynchronization)
        } else if !self.direct_sampling && !self.gpu_conversion {
            Some(FrameError::MissingImporter)
        } else {
            None
        };
        if let Some(reason) = rejection {
            if policy == PresentationPolicy::GpuOnly || !cpu_fallback_available {
                return Err(reason);
            }
            return Ok(PresentationSelection {
                route: PresentationRoute::CpuFallback,
                graphics: self.graphics,
                fallback_reason: Some(reason),
            });
        }
        Ok(PresentationSelection {
            route: if self.direct_sampling {
                PresentationRoute::SharedTexture
            } else {
                PresentationRoute::GpuConversion
            },
            graphics: self.graphics,
            fallback_reason: None,
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PixelTransferPurpose {
    Preview,
    Analysis,
    Photo,
    Diagnostic,
    Probe,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PixelTransferCount {
    pub attempts: u64,
    pub bytes: u64,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FrameTransferSnapshot {
    pub preview: PixelTransferCount,
    pub analysis: PixelTransferCount,
    pub photo: PixelTransferCount,
    pub diagnostic: PixelTransferCount,
    pub probe: PixelTransferCount,
    pub gpu_imports: u64,
    pub gpu_copies: u64,
    pub gpu_conversions: u64,
    pub gpu_operations: Vec<GpuTransferCount>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuTransferReason {
    /// A browser-owned video/image is transferred directly to the active GL
    /// texture. Browser/driver internals remain opaque to framework counters.
    NativeImageTransfer,
    ColorSpaceConversion,
    PixelFormatConversion,
    DeviceCompatibility,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GpuTransferCount {
    pub reason: GpuTransferReason,
    pub copies: u64,
    pub conversions: u64,
}
/// Counters are always present: a disabled diagnostic feature cannot turn a
/// prohibited transfer into a convincing zero. Call before touching pixels.
#[derive(Clone, Default)]
pub struct FrameTransferCounters(Arc<Mutex<FrameTransferSnapshot>>);
impl FrameTransferCounters {
    pub fn snapshot(&self) -> FrameTransferSnapshot {
        self.0.lock().unwrap().clone()
    }
    pub fn cpu_transfer(
        &self,
        purpose: PixelTransferPurpose,
        bytes: u64,
        route: PresentationRoute,
    ) -> Result<(), FrameError> {
        let mut counts = self.0.lock().unwrap();
        let count = match purpose {
            PixelTransferPurpose::Preview => &mut counts.preview,
            PixelTransferPurpose::Analysis => &mut counts.analysis,
            PixelTransferPurpose::Photo => &mut counts.photo,
            PixelTransferPurpose::Diagnostic => &mut counts.diagnostic,
            PixelTransferPurpose::Probe => &mut counts.probe,
        };
        count.attempts = count.attempts.saturating_add(1);
        if purpose == PixelTransferPurpose::Preview && route != PresentationRoute::CpuFallback {
            return Err(FrameError::CpuTransferForbidden);
        }
        count.bytes = count.bytes.saturating_add(bytes);
        Ok(())
    }
    pub fn gpu_import(&self) {
        let mut c = self.0.lock().unwrap();
        c.gpu_imports = c.gpu_imports.saturating_add(1);
    }
    pub fn gpu_conversion(&self, reason: GpuTransferReason) {
        let mut c = self.0.lock().unwrap();
        c.gpu_conversions = c.gpu_conversions.saturating_add(1);
        Self::operation(&mut c, reason, false);
    }
    pub fn gpu_copy(&self, reason: GpuTransferReason) {
        let mut c = self.0.lock().unwrap();
        c.gpu_copies = c.gpu_copies.saturating_add(1);
        Self::operation(&mut c, reason, true);
    }
    fn operation(c: &mut FrameTransferSnapshot, reason: GpuTransferReason, copy: bool) {
        if !c.gpu_operations.iter().any(|v| v.reason == reason) {
            c.gpu_operations.push(GpuTransferCount {
                reason,
                copies: 0,
                conversions: 0,
            });
        }
        let operation = c
            .gpu_operations
            .iter_mut()
            .find(|v| v.reason == reason)
            .unwrap();
        if copy {
            operation.copies = operation.copies.saturating_add(1);
        } else {
            operation.conversions = operation.conversions.saturating_add(1);
        }
    }
}
