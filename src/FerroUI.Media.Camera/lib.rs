//! Camera discovery, explicit acquisition, sessions and still capture.
//! Depends on the shared media base, independently of video playback and QR.
#![forbid(unsafe_code)]
pub use ferroui_media as media;
use ferroui_media::frames::{FrameBudget, FrameLease, FrameMailbox, PresentationPolicy};
use std::{
    cell::{Cell, RefCell},
    future::Future,
    pin::Pin,
    rc::Rc,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

pub type CameraFuture<T> = Pin<Box<dyn Future<Output = Result<T, CameraError>>>>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraErrorKind {
    InvalidRequest,
    NoDevice,
    NotDetermined,
    Denied,
    Restricted,
    Busy,
    Disconnected,
    Unsupported,
    Cancelled,
    NativeFailure,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CameraError {
    pub kind: CameraErrorKind,
    pub message: String,
}
impl CameraError {
    pub fn new(kind: CameraErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}
impl std::fmt::Display for CameraError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for CameraError {}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraFacing {
    Front,
    Rear,
    External,
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraPermission {
    NotDetermined,
    Granted,
    Denied,
    Restricted,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraFormat {
    pub width: u32,
    pub height: u32,
    /// Preserves negotiated fractional rates rather than rounding to integers.
    pub frames_per_second: f64,
}
#[derive(Clone, Debug)]
pub struct CameraDevice {
    pub id: String,
    pub label: Option<String>,
    pub facing: CameraFacing,
    pub formats: Vec<CameraFormat>,
}
#[derive(Clone, Debug, Default)]
pub struct CameraSelection {
    pub device_id: Option<String>,
    pub preferred_facing: Option<CameraFacing>,
    pub format: Option<CameraFormat>,
    pub exact: bool,
}
#[derive(Clone, Debug)]
pub struct CameraStartOptions {
    pub selection: CameraSelection,
    pub presentation: PresentationPolicy,
}
impl Default for CameraStartOptions {
    fn default() -> Self {
        Self {
            selection: CameraSelection::default(),
            presentation: PresentationPolicy::GpuPreferred,
        }
    }
}
#[derive(Clone, Debug)]
pub struct CameraSettings {
    pub device: CameraDevice,
    pub format: CameraFormat,
    pub native_still: bool,
    pub torch: bool,
    pub zoom: Option<(f64, f64)>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhotoOrigin {
    NativeStill,
    PreviewSnapshot,
}
#[derive(Clone, Debug)]
pub struct CameraPhoto {
    pub encoded: Arc<[u8]>,
    pub content_type: String,
    pub width: u32,
    pub height: u32,
    pub timestamp: Duration,
    pub rotation_degrees: u16,
    pub mirrored: bool,
    pub origin: PhotoOrigin,
}
#[derive(Clone, Copy, Debug)]
pub enum CameraControl {
    Torch(bool),
    Zoom(f64),
    Focus(f64, f64),
    Exposure(f64),
}
pub trait NativeCameraSession {
    fn settings(&self) -> CameraSettings;
    fn take_frame(&self) -> Result<Option<FrameLease>, CameraError>;
    fn photo(&self) -> CameraFuture<CameraPhoto>;
    fn set_control(&self, control: CameraControl) -> Result<(), CameraError>;
    fn stop(&self);
}
pub trait CameraProvider {
    /// No acquisition or consent prompt. Labels/formats may remain unknown.
    fn devices(&self) -> CameraFuture<Vec<CameraDevice>>;
    fn permission(&self) -> CameraPermission;
    /// Owns provisional resources until committed. Dropping the returned future
    /// must retire those resources, including late permission results. Never
    /// requests microphone access.
    fn start(
        &self,
        options: CameraStartOptions,
        source: u64,
        generation: u64,
        budget: FrameBudget,
    ) -> CameraFuture<Rc<dyn NativeCameraSession>>;
}
struct PendingStart {
    pending: RefCell<Option<CameraFuture<Rc<dyn NativeCameraSession>>>>,
    cancelled: Cell<bool>,
    waker: RefCell<Option<std::task::Waker>>,
}
impl PendingStart {
    fn cancel(&self) {
        self.cancelled.set(true);
        let pending = self.pending.borrow_mut().take();
        let waker = self.waker.borrow_mut().take();
        drop(pending);
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}
struct CaptureInner {
    provider: Rc<dyn CameraProvider>,
    source: u64,
    generation: Cell<u64>,
    budget: FrameBudget,
    pending: RefCell<Option<std::rc::Weak<PendingStart>>>,
}
impl Drop for CaptureInner {
    fn drop(&mut self) {
        if let Some(pending) = self
            .pending
            .get_mut()
            .take()
            .and_then(|weak| weak.upgrade())
        {
            pending.cancel();
        }
    }
}
#[derive(Clone)]
pub struct CameraCapture(Rc<CaptureInner>);
impl CameraCapture {
    pub fn new(provider: Rc<dyn CameraProvider>, budget: FrameBudget) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1 << 32);
        Self(Rc::new(CaptureInner {
            provider,
            source: NEXT.fetch_add(1, Ordering::Relaxed),
            generation: Cell::new(0),
            budget,
            pending: RefCell::new(None),
        }))
    }
    pub fn devices(&self) -> CameraFuture<Vec<CameraDevice>> {
        self.0.provider.devices()
    }
    pub fn permission(&self) -> CameraPermission {
        self.0.provider.permission()
    }
    pub fn cancel_pending_start(&self) {
        self.0.generation.set(
            self.0
                .generation
                .get()
                .checked_add(1)
                .expect("camera generation exhausted"),
        );
        let pending = self
            .0
            .pending
            .borrow_mut()
            .take()
            .and_then(|weak| weak.upgrade());
        if let Some(pending) = pending {
            pending.cancel();
        }
    }
    pub fn start(&self, options: CameraStartOptions) -> CameraFuture<CameraSession> {
        if options.selection.format.is_some_and(|format| {
            format.width == 0
                || format.height == 0
                || !format.frames_per_second.is_finite()
                || format.frames_per_second <= 0.
        }) {
            return Box::pin(async {
                Err(CameraError::new(
                    CameraErrorKind::InvalidRequest,
                    "Camera dimensions and frame rate must be positive",
                ))
            });
        }
        self.cancel_pending_start();
        let generation = self.0.generation.get();
        let source = self.0.source;
        let pending = self
            .0
            .provider
            .start(options, source, generation, self.0.budget.clone());
        let slot = Rc::new(PendingStart {
            pending: RefCell::new(Some(pending)),
            cancelled: Cell::new(false),
            waker: RefCell::new(None),
        });
        *self.0.pending.borrow_mut() = Some(Rc::downgrade(&slot));
        let weak = Rc::downgrade(&self.0);
        let pending = std::future::poll_fn(move |cx| {
            if slot.cancelled.get() {
                return std::task::Poll::Ready(Err(CameraError::new(
                    CameraErrorKind::Cancelled,
                    "Camera start was cancelled",
                )));
            }
            let Some(mut pending) = slot.pending.borrow_mut().take() else {
                return std::task::Poll::Ready(Err(CameraError::new(
                    CameraErrorKind::Cancelled,
                    "Camera start was cancelled",
                )));
            };
            *slot.waker.borrow_mut() = Some(cx.waker().clone());
            let result = pending.as_mut().poll(cx);
            if slot.cancelled.get() {
                if let std::task::Poll::Ready(Ok(native)) = result {
                    native.stop();
                }
                return std::task::Poll::Ready(Err(CameraError::new(
                    CameraErrorKind::Cancelled,
                    "Camera start was cancelled",
                )));
            }
            match result {
                std::task::Poll::Pending => {
                    *slot.pending.borrow_mut() = Some(pending);
                    std::task::Poll::Pending
                }
                std::task::Poll::Ready(result) => {
                    slot.waker.borrow_mut().take();
                    std::task::Poll::Ready(result)
                }
            }
        });
        Box::pin(async move {
            let native = pending.await?;
            if !weak
                .upgrade()
                .is_some_and(|c| c.generation.get() == generation)
            {
                native.stop();
                return Err(CameraError::new(
                    CameraErrorKind::Cancelled,
                    "Camera start was abandoned",
                ));
            }
            Ok(CameraSession(Rc::new(SessionInner {
                native,
                frames: Arc::new(FrameMailbox::new(source, generation)),
                stopped: Cell::new(false),
            })))
        })
    }
}
struct SessionInner {
    native: Rc<dyn NativeCameraSession>,
    frames: Arc<FrameMailbox>,
    stopped: Cell<bool>,
}
impl Drop for SessionInner {
    fn drop(&mut self) {
        if !self.stopped.replace(true) {
            self.frames.clear();
            self.native.stop();
        }
    }
}
#[derive(Clone)]
pub struct CameraSession(Rc<SessionInner>);
#[derive(Clone)]
pub struct WeakCameraSession(std::rc::Weak<SessionInner>);
impl WeakCameraSession {
    pub fn upgrade(&self) -> Option<CameraSession> { self.0.upgrade().map(CameraSession) }
}
impl CameraSession {
    pub fn downgrade(&self) -> WeakCameraSession { WeakCameraSession(Rc::downgrade(&self.0)) }
    pub fn is_stopped(&self) -> bool { self.0.stopped.get() }
    pub fn settings(&self) -> CameraSettings {
        self.0.native.settings()
    }
    pub fn frames(&self) -> Arc<FrameMailbox> {
        self.0.frames.clone()
    }
    pub fn poll(&self) -> Result<(), CameraError> {
        if self.0.stopped.get() {
            return Ok(());
        }
        match self.0.native.take_frame() {
            Ok(Some(frame)) => self
                .0
                .frames
                .publish(frame)
                .map_err(|e| CameraError::new(CameraErrorKind::NativeFailure, e.to_string())),
            Ok(None) => Ok(()),
            Err(error) => {
                self.stop();
                Err(error)
            }
        }
    }
    pub fn photo(&self) -> CameraFuture<CameraPhoto> {
        if self.0.stopped.get() {
            return Box::pin(async {
                Err(CameraError::new(
                    CameraErrorKind::Disconnected,
                    "Camera is stopped",
                ))
            });
        }
        let pending = self.0.native.photo();
        let session = Rc::downgrade(&self.0);
        Box::pin(async move {
            let photo = pending.await?;
            if !session.upgrade().is_some_and(|s| !s.stopped.get()) {
                return Err(CameraError::new(
                    CameraErrorKind::Cancelled,
                    "Camera stopped before photo capture completed",
                ));
            }
            Ok(photo)
        })
    }
    pub fn set_control(&self, control: CameraControl) -> Result<(), CameraError> {
        if self.0.stopped.get() {
            return Err(CameraError::new(
                CameraErrorKind::Disconnected,
                "Camera is stopped",
            ));
        }
        let valid = match control {
            CameraControl::Torch(_) => true,
            CameraControl::Zoom(zoom) => zoom.is_finite() && zoom > 0.,
            CameraControl::Focus(x, y) => {
                x.is_finite() && y.is_finite() && (0. ..=1.).contains(&x) && (0. ..=1.).contains(&y)
            }
            CameraControl::Exposure(exposure) => exposure.is_finite(),
        };
        if !valid {
            return Err(CameraError::new(
                CameraErrorKind::InvalidRequest,
                "Camera control requires finite values in its normalized range",
            ));
        }
        self.0.native.set_control(control)
    }
    pub fn stop(&self) {
        if !self.0.stopped.replace(true) {
            self.0.frames.clear();
            self.0.native.stop();
        }
    }
}

#[cfg(test)]
mod tests;
