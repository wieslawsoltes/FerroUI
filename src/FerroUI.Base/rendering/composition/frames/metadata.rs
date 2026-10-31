use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FrameIdentity {
    pub source: u64,
    pub generation: u64,
    pub sequence: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum DeviceIdentity {
    Luid(Vec<u8>),
    Uuid(Vec<u8>),
    /// Identity scoped to a renderer instance, never a portable native handle.
    Context(u64),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameFormat {
    Rgba8,
    Bgra8,
    Nv12,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameAlpha {
    Opaque,
    Premultiplied,
    Straight,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameColor {
    Srgb,
    Rec709Limited,
    Rec709Full,
    Unsupported,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameRotation {
    None,
    Clockwise90,
    Clockwise180,
    Clockwise270,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FramePlane {
    pub offset: u64,
    pub stride: u64,
    pub length: u64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameMetadata {
    pub identity: FrameIdentity,
    pub width: u32,
    pub height: u32,
    pub visible: FrameRegion,
    pub format: FrameFormat,
    pub color: FrameColor,
    pub alpha: FrameAlpha,
    pub rotation: FrameRotation,
    pub mirrored: bool,
    pub top_left_origin: bool,
    /// Source presentation timestamp. Its origin belongs to the source and may
    /// change after seeking; use FrameLease::acquisition_age for host expiry.
    pub timestamp: Duration,
    pub device: Option<DeviceIdentity>,
    /// Graphics-resource generation, independent of source replacement.
    pub device_generation: u64,
    pub planes: Vec<FramePlane>,
    /// Actual retained native allocation, not just visible pixel bytes.
    pub retained_bytes: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameError {
    InvalidDimensions,
    InvalidRegion,
    InvalidPlanes,
    UnsupportedColor,
    BudgetExceeded,
    StaleGeneration,
    ProducerNotReady,
    DeviceMismatch,
    UnsupportedFormat,
    MissingSynchronization,
    MissingImporter,
    SoftwareGraphics,
    CompletionAlreadySignaled,
    IndeterminateCompletion,
    CpuTransferForbidden,
}
impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for FrameError {}
impl FrameMetadata {
    pub fn validate(&self, max_dimension: u32) -> Result<(), FrameError> {
        if self.width == 0
            || self.height == 0
            || self.width > max_dimension
            || self.height > max_dimension
        {
            return Err(FrameError::InvalidDimensions);
        }
        let r = self.visible;
        if r.width == 0
            || r.height == 0
            || r.x.checked_add(r.width).is_none_or(|x| x > self.width)
            || r.y.checked_add(r.height).is_none_or(|y| y > self.height)
        {
            return Err(FrameError::InvalidRegion);
        }
        if self.retained_bytes == 0 {
            return Err(FrameError::InvalidPlanes);
        }
        if self.color == FrameColor::Unsupported {
            return Err(FrameError::UnsupportedColor);
        }
        let heights = match self.format {
            FrameFormat::Rgba8 | FrameFormat::Bgra8 => {
                vec![(self.height as u64, self.width as u64 * 4)]
            }
            FrameFormat::Nv12 => vec![
                (self.height as u64, self.width as u64),
                (
                    (self.height as u64).div_ceil(2),
                    (self.width as u64).div_ceil(2) * 2,
                ),
            ],
        };
        if self.planes.len() != heights.len() {
            return Err(FrameError::InvalidPlanes);
        }
        for (plane, (rows, minimum_stride)) in self.planes.iter().zip(heights) {
            let required = plane
                .stride
                .checked_mul(rows - 1)
                .and_then(|v| v.checked_add(minimum_stride));
            if plane.stride < minimum_stride
                || required.is_none_or(|v| v > plane.length)
                || plane
                    .offset
                    .checked_add(plane.length)
                    .is_none_or(|v| v > self.retained_bytes)
            {
                return Err(FrameError::InvalidPlanes);
            }
        }
        Ok(())
    }
    /// Maps normalized visible-image coordinates through rotation and mirror.
    /// The caller maps the result into the actual drawn content rectangle.
    pub fn orient(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        let (x, y) = match self.rotation {
            FrameRotation::None => (x, y),
            FrameRotation::Clockwise90 => (1.0 - y, x),
            FrameRotation::Clockwise180 => (1.0 - x, 1.0 - y),
            FrameRotation::Clockwise270 => (y, 1.0 - x),
        };
        Some((if self.mirrored { 1.0 - x } else { x }, y))
    }
}
