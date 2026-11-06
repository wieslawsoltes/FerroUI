//! Optional QR recognition; callers borrow native analysis planes and retain
//! the source frame while decoding. Preview never depends on this package.
#![forbid(unsafe_code)]
use ferroui_media::frames::{
    FrameIdentity, FrameLease, FrameMetadata, FrameRegion, FrameRotation, FrameTransferCounters,
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
#[cfg(feature = "rust-decoder")]
mod decoder;
#[cfg(feature = "rust-decoder")]
pub use decoder::decode;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QrError {
    InvalidPlane,
    InputTooLarge,
    Busy,
    Throttled,
    Cancelled,
    DecodeFailure,
    StaleFrame,
}
impl std::fmt::Display for QrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidPlane => "the frame has no luminance plane the decoder can read",
            Self::InputTooLarge => "the frame is larger than the decoder accepts",
            Self::Busy => "a decode is already running",
            Self::Throttled => "decodes are throttled; try again later",
            Self::Cancelled => "the decode was cancelled",
            Self::DecodeFailure => "no QR code was decoded",
            Self::StaleFrame => "the frame was replaced before it was decoded",
        })
    }
}
impl std::error::Error for QrError {}
#[derive(Clone, Copy, Debug)]
pub struct QrOptions {
    pub max_width: u32,
    pub max_height: u32,
    pub interval: Duration,
    pub max_age: Duration,
}
impl Default for QrOptions {
    fn default() -> Self {
        Self {
            max_width: 1280,
            max_height: 720,
            interval: Duration::from_millis(100),
            max_age: Duration::from_millis(250),
        }
    }
}
/// Full unrotated camera-image luma, possibly uniformly downscaled. ROI/crop
/// adaptation must be performed explicitly by the native analysis provider.
/// Row/pixel stride are in bytes. No allocation/repacking at this boundary.
pub struct AnalysisPlane<'a> {
    pub data: &'a [u8],
    pub width: u32,
    pub height: u32,
    pub row_stride: usize,
    pub pixel_stride: usize,
}
impl AnalysisPlane<'_> {
    pub fn validate(&self, options: QrOptions) -> Result<(), QrError> {
        if self.width == 0 || self.height == 0 || self.pixel_stride == 0 {
            return Err(QrError::InvalidPlane);
        }
        if self.width > options.max_width || self.height > options.max_height {
            return Err(QrError::InputTooLarge);
        }
        let row = (self.width as usize - 1)
            .checked_mul(self.pixel_stride)
            .and_then(|n| n.checked_add(1))
            .ok_or(QrError::InvalidPlane)?;
        let required = (self.height as usize - 1)
            .checked_mul(self.row_stride)
            .and_then(|n| n.checked_add(row))
            .ok_or(QrError::InvalidPlane)?;
        if row > self.row_stride || required > self.data.len() {
            return Err(QrError::InvalidPlane);
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GeometryQuality {
    ProjectedGrid,
    EstimatedFromFinderPatterns,
    NativeCorners,
}
/// Geometry captured with the analyzed image, in the same logical top-left
/// texture plane as result corners. It owns no native storage or frame lease.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QrFrameGeometry {
    pub width: u32,
    pub height: u32,
    pub visible: FrameRegion,
    pub rotation: FrameRotation,
    pub mirrored: bool,
    pub top_left_origin: bool,
}
impl From<&FrameMetadata> for QrFrameGeometry {
    fn from(frame: &FrameMetadata) -> Self {
        Self {
            width: frame.width,
            height: frame.height,
            visible: frame.visible,
            rotation: frame.rotation,
            mirrored: frame.mirrored,
            top_left_origin: frame.top_left_origin,
        }
    }
}
impl QrFrameGeometry {
    pub fn validate(&self) -> Result<(), QrError> {
        let r = self.visible;
        if self.width == 0
            || self.height == 0
            || r.width == 0
            || r.height == 0
            || r.x.checked_add(r.width).is_none_or(|x| x > self.width)
            || r.y.checked_add(r.height).is_none_or(|y| y > self.height)
        {
            Err(QrError::InvalidPlane)
        } else {
            Ok(())
        }
    }
}
#[derive(Clone, Debug)]
pub struct QrCandidate {
    pub frame: FrameIdentity,
    pub timestamp: Duration,
    pub frame_geometry: QrFrameGeometry,
    pub text: Option<String>,
    /// Decoder's corrected QR codewords, not mislabeled payload bytes.
    pub codewords: Option<Vec<u8>>,
    /// Byte-mode segments as exposed by the decoder. Mixed modes may also
    /// contain numeric/alphanumeric content; these are not the full payload.
    pub byte_segments: Vec<Vec<u8>>,
    /// Full payload bytes only when an engine actually exposes them.
    pub payload_bytes: Option<Vec<u8>>,
    /// Normalized corners in the published frame's unrotated texture plane;
    /// presentation rotation/mirror is applied by the common geometry mapping.
    pub corners: [(f64, f64); 4],
    pub geometry: GeometryQuality,
}
struct GateState {
    epoch: u64,
    source: u64,
    generation: u64,
    busy: bool,
    last: Option<Duration>,
    sequence: Option<u64>,
    enabled: bool,
}
#[derive(Clone)]
pub struct QrAnalysisGate {
    state: Arc<Mutex<GateState>>,
    options: QrOptions,
}
pub struct QrAnalysis {
    state: Arc<Mutex<GateState>>,
    epoch: u64,
    pub frame: FrameLease,
    options: QrOptions,
    begun_at: Duration,
    initial_age: Duration,
}
impl Drop for QrAnalysis {
    fn drop(&mut self) {
        let mut s = self.state.lock().unwrap();
        s.busy = false;
    }
}
impl QrAnalysisGate {
    pub fn new(options: QrOptions) -> Self {
        Self {
            state: Arc::new(Mutex::new(GateState {
                epoch: 0,
                source: 0,
                generation: 0,
                busy: false,
                last: None,
                sequence: None,
                enabled: false,
            })),
            options,
        }
    }
    pub fn activate(&self, source: u64, generation: u64) {
        let mut s = self.state.lock().unwrap();
        s.epoch = s.epoch.checked_add(1).expect("analysis epoch exhausted");
        s.source = source;
        s.generation = generation;
        s.last = None;
        s.sequence = None;
        s.enabled = true;
        // Outstanding old work still owns the single analysis slot. Its Drop
        // releases that slot even when publication has moved to another epoch.
    }
    pub fn stop(&self) {
        let mut s = self.state.lock().unwrap();
        s.enabled = false;
    }
    pub fn begin(&self, frame: FrameLease, now: Duration) -> Result<QrAnalysis, QrError> {
        let mut s = self.state.lock().unwrap();
        let id = frame.metadata().identity;
        if !frame.is_current()
            || !s.enabled
            || id.source != s.source
            || id.generation != s.generation
        {
            return Err(QrError::Cancelled);
        }
        if s.busy {
            return Err(QrError::Busy);
        }
        let initial_age = frame.acquisition_age();
        if initial_age > self.options.max_age || s.sequence.is_some_and(|last| id.sequence <= last)
        {
            return Err(QrError::StaleFrame);
        }
        if s.last.is_some_and(|last| {
            now.checked_sub(last)
                .is_none_or(|d| d < self.options.interval)
        }) {
            return Err(QrError::Throttled);
        }
        s.busy = true;
        s.last = Some(now);
        s.sequence = Some(id.sequence);
        Ok(QrAnalysis {
            state: self.state.clone(),
            epoch: s.epoch,
            frame,
            options: self.options,
            begun_at: now,
            initial_age,
        })
    }
}
impl QrAnalysis {
    pub fn options(&self) -> QrOptions {
        self.options
    }
    pub fn is_valid(&self) -> bool {
        let s = self.state.lock().unwrap();
        self.frame.is_current() && s.enabled && s.epoch == self.epoch
    }
    pub fn is_current(&self, now: Duration) -> bool {
        self.is_valid()
            && self.frame.acquisition_age() <= self.options.max_age
            && now
                .checked_sub(self.begun_at)
                .and_then(|elapsed| elapsed.checked_add(self.initial_age))
                .is_some_and(|age| age <= self.options.max_age)
    }
    pub fn counters(&self) -> FrameTransferCounters {
        self.frame.counters()
    }
}
/// Optional publication policy; the default continuous stream preserves all
/// symbols. Accept/feedback do not authenticate or interpret application data.
#[derive(Default)]
pub struct QrPublicationPolicy {
    pub deduplicate: bool,
    pub stop_after_acceptance: bool,
    last: Vec<String>,
    suppressed: Option<String>,
    stopped: bool,
}
impl QrPublicationPolicy {
    pub fn publish(&mut self, candidates: Vec<QrCandidate>) -> Vec<QrCandidate> {
        if self.stopped {
            return Vec::new();
        }
        let visible: Vec<_> = candidates.iter().filter_map(|c| c.text.clone()).collect();
        let output = candidates
            .into_iter()
            .filter(|c| {
                c.text.as_ref().is_none_or(|t| {
                    self.suppressed.as_ref() != Some(t)
                        && (!self.deduplicate || !self.last.contains(t))
                })
            })
            .collect();
        if self
            .suppressed
            .as_ref()
            .is_some_and(|t| !visible.contains(t))
        {
            self.suppressed = None;
        }
        self.last = visible;
        output
    }
    pub fn suppress(&mut self, text: String) {
        self.suppressed = Some(text);
    }
    pub fn clear_feedback(&mut self) {
        self.suppressed = None;
    }
    pub fn accept(&mut self) {
        if self.stop_after_acceptance {
            self.stopped = true;
        }
    }
    pub fn reset(&mut self) {
        self.last.clear();
        self.suppressed = None;
        self.stopped = false;
    }
}
#[cfg(test)]
mod tests;
