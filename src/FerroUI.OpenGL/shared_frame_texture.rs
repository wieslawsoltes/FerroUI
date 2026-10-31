use ferroui_base::rendering::composition::frames::FrameRetirement;
use ferroui_base::{
    platform::{
        IPlatformHandle, KnownPlatformGraphicsExternalImageHandleTypes, LeasedPlatformHandle,
    },
    rendering::composition::frames::{FrameError, FrameLease},
};
use std::{
    any::Any,
    rc::Rc,
    sync::{Arc, Mutex, Weak},
};

type Wake = dyn Fn() + Send + Sync;
type NativeRetirement = Box<dyn FnMut(Option<&crate::GlInterface>) -> bool + Send>;
#[derive(Default)]
struct RetirementQueueState {
    closed: bool,
    textures: Vec<(u32, FrameRetirement)>,
    wakeups: Vec<Weak<Wake>>,
    native: Vec<NativeRetirement>,
}
impl Drop for RetirementQueueState {
    fn drop(&mut self) {
        for (_, retirement) in self.textures.drain(..) {
            retirement.quarantine();
        }
        for mut release in self.native.drain(..) {
            release(None);
        }
    }
}
/// Cross-thread destruction requests, drained only with the owning GL context
/// current. Byte credits remain charged until native deletion has happened.
#[derive(Clone, Default)]
pub struct SharedGlTextureRetirementQueue(Arc<Mutex<RetirementQueueState>>);
impl SharedGlTextureRetirementQueue {
    /// Retires producer GL objects on their owner thread. After context loss,
    /// `None` permits releasing non-GL native ownership without issuing GL calls.
    pub fn retire_native(
        &self,
        release: impl FnOnce(Option<&crate::GlInterface>) + Send + 'static,
    ) {
        let mut release = Some(release);
        self.defer_native(move |context| {
            release.take().unwrap()(context);
            true
        });
    }
    /// Returns false while a native producer still owns a surface. Called once
    /// per tick without waiting. With None, arrange native-only cleanup after
    /// producer completion and return true; no future GL access is possible.
    pub fn defer_native(
        &self,
        release: impl FnMut(Option<&crate::GlInterface>) -> bool + Send + 'static,
    ) {
        let mut release: Option<NativeRetirement> = Some(Box::new(release));
        let wakeups = {
            let mut state = self.0.lock().unwrap();
            if !state.closed {
                state.native.push(release.take().unwrap());
            }
            state
                .wakeups
                .iter()
                .filter_map(Weak::upgrade)
                .collect::<Vec<_>>()
        };
        if let Some(mut release) = release {
            release(None);
        }
        for wake in wakeups {
            wake();
        }
    }
    pub(crate) fn take_native(&self) -> Vec<NativeRetirement> {
        std::mem::take(&mut self.0.lock().unwrap().native)
    }
    pub(crate) fn poll_native(&self, gl: &crate::GlInterface) -> bool {
        let mut pending = Vec::new();
        for mut release in self.take_native() {
            if !release(Some(gl)) {
                pending.push(release);
            }
        }
        let mut state = self.0.lock().unwrap();
        state.native.extend(pending);
        !state.native.is_empty()
    }
    /// Keep the returned registration alive while producing or retaining frames.
    pub fn register_wakeup(&self, wake: Arc<Wake>) -> Arc<Wake> {
        let mut state = self.0.lock().unwrap();
        state.wakeups.retain(|v| v.strong_count() != 0);
        state.wakeups.push(Arc::downgrade(&wake));
        wake
    }
    pub fn retire(&self, texture: u32, retirement: FrameRetirement) {
        let wakeups = {
            let mut state = self.0.lock().unwrap();
            if state.closed {
                retirement.quarantine();
                return;
            }
            state.textures.push((texture, retirement));
            state
                .wakeups
                .iter()
                .filter_map(Weak::upgrade)
                .collect::<Vec<_>>()
        };
        for wake in wakeups {
            wake();
        }
    }
    pub(crate) fn take(&self) -> Vec<(u32, FrameRetirement)> {
        std::mem::take(&mut self.0.lock().unwrap().textures)
    }
    pub(crate) fn has_pending(&self) -> bool {
        let state = self.0.lock().unwrap();
        !state.textures.is_empty() || !state.native.is_empty()
    }
    pub(crate) fn close(&self) {
        let pending = {
            let mut state = self.0.lock().unwrap();
            state.closed = true;
            std::mem::take(&mut state.textures)
        };
        for (_, retirement) in pending {
            retirement.quarantine();
        }
        for mut release in self.take_native() {
            release(None);
        }
    }
    /// Creates an owner whose final drop queues deletion; it does not delete GL
    /// objects on an arbitrary thread. The wake registration travels with it.
    pub fn owner(
        &self,
        texture: u32,
        retirement: FrameRetirement,
        wake: Arc<Wake>,
    ) -> SharedGlTextureOwner {
        SharedGlTextureOwner {
            queue: self.clone(),
            texture,
            retirement: Some(retirement),
            _wake: wake,
        }
    }
}
pub struct SharedGlTextureOwner {
    queue: SharedGlTextureRetirementQueue,
    texture: u32,
    retirement: Option<FrameRetirement>,
    _wake: Arc<Wake>,
}
impl Drop for SharedGlTextureOwner {
    fn drop(&mut self) {
        self.queue
            .retire(self.texture, self.retirement.take().unwrap());
    }
}

/// Immutable storage owned by a producer, sampled in its admitted GL context.
/// Importing creates a borrowed view; it never adopts or copies this texture.
pub struct SharedFrameTexture {
    context_id: u64,
    generation: u64,
    texture: u32,
    _owner: Box<dyn Any + Send + Sync>,
}
impl SharedFrameTexture {
    /// # Safety
    /// `texture` must name RGBA8 GL_TEXTURE_2D storage in the given context or
    /// proven share group. The owner must keep it alive and immutable after
    /// producer readiness until the last frame lease/GPU completion retires.
    /// Native destruction must use the owning context, retaining allocation
    /// accounting until any queued destruction actually completes.
    pub unsafe fn new<T: Any + Send + Sync>(
        context_id: u64,
        generation: u64,
        texture: u32,
        owner: T,
    ) -> Self {
        Self {
            context_id,
            generation,
            texture,
            _owner: Box::new(owner),
        }
    }
    pub fn context_id(&self) -> u64 {
        self.context_id
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn texture(&self) -> u32 {
        self.texture
    }
    pub fn handle(frame: &FrameLease) -> Result<Rc<dyn IPlatformHandle>, FrameError> {
        let source = frame
            .resource::<Self>()
            .ok_or(FrameError::MissingImporter)?;
        Ok(Rc::new(LeasedPlatformHandle::new(
            source.texture as isize,
            Some(KnownPlatformGraphicsExternalImageHandleTypes::SHARED_GL_TEXTURE),
            frame.clone(),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::rendering::composition::frames::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    fn reservation() -> (FrameBudget, FrameReservation) {
        let budget = FrameBudget::new(FrameBudgetLimits {
            frames: 1,
            bytes: 4,
            max_dimension: 1,
            gpu_reads: 1,
        });
        let reservation = budget
            .reserve(FrameMetadata {
                identity: FrameIdentity {
                    source: 1,
                    generation: 1,
                    sequence: 1,
                },
                width: 1,
                height: 1,
                visible: FrameRegion {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                },
                format: FrameFormat::Rgba8,
                color: FrameColor::Srgb,
                alpha: FrameAlpha::Opaque,
                rotation: FrameRotation::None,
                mirrored: false,
                top_left_origin: false,
                timestamp: std::time::Duration::ZERO,
                device: Some(DeviceIdentity::Context(1)),
                device_generation: 1,
                planes: vec![FramePlane {
                    offset: 0,
                    stride: 4,
                    length: 4,
                }],
                retained_bytes: 4,
            })
            .unwrap();
        (budget, reservation)
    }
    #[test]
    fn queued_deletion_keeps_credit_and_wakes_outside_the_queue_lock() {
        let (budget, reservation) = reservation();
        let queue = SharedGlTextureRetirementQueue::default();
        let observed = queue.clone();
        let wakes = Arc::new(AtomicUsize::new(0));
        let count = wakes.clone();
        let wake = queue.register_wakeup(Arc::new(move || {
            assert!(observed.0.try_lock().is_ok());
            count.fetch_add(1, Ordering::SeqCst);
        }));
        let retirement = reservation.retirement();
        let frame = reservation.lease(queue.owner(17, retirement, wake));
        drop(frame);
        assert_eq!(budget.usage().frames, 1);
        assert_eq!(wakes.load(Ordering::SeqCst), 1);
        let native = queue.take();
        assert_eq!(native[0].0, 17);
        assert_eq!(budget.usage().frames, 1);
        drop(native);
        assert_eq!(budget.usage().frames, 0);
    }
    #[test]
    fn rejected_cleanup_quarantines_allocation_credit() {
        let (budget, reservation) = reservation();
        let queue = SharedGlTextureRetirementQueue::default();
        let wake = queue.register_wakeup(Arc::new(|| {}));
        let owner = queue.owner(23, reservation.retirement(), wake);
        let frame = reservation.lease(owner);
        queue.close();
        drop(frame);
        assert_eq!(budget.usage().bytes, 4);
        assert!(queue.take().is_empty());
    }
    #[test]
    fn native_surface_ownership_keeps_retirement_ticks_active() {
        let queue = SharedGlTextureRetirementQueue::default();
        let count = Arc::new(AtomicUsize::new(0));
        let observed = count.clone();
        queue.defer_native(move |context| {
            assert!(context.is_some());
            observed.fetch_add(1, Ordering::SeqCst) >= 1
        });
        let gl = crate::testing::FakeGl::install(crate::GlVersion::new(
            crate::GlProfileType::OpenGLES,
            3,
            0,
        ))
        .interface();
        assert!(queue.poll_native(&gl));
        assert!(queue.has_pending());
        assert!(!queue.poll_native(&gl));
        assert!(!queue.has_pending());
        assert_eq!(count.load(Ordering::SeqCst), 2);
    }
}
