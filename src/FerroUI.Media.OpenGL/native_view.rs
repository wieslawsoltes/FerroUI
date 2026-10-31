use super::*;
use ferroui_opengl::{GlInterface, SharedGlTextureRetirementQueue};

type Release = Box<dyn FnOnce(Option<&GlInterface>) + Send>;

/// An immutable GL_TEXTURE_2D alias of retained native storage. The release
/// callback owns both the alias and its backing, and runs on the owning render
/// context after all consumer leases have finished. None means context loss.
#[must_use]
pub struct NativeTextureView {
    context: Rc<dyn IGlContext>,
    texture: u32,
    release: Option<Release>,
}
impl NativeTextureView {
    /// # Safety
    /// The texture must be complete RGBA8-compatible immutable storage, readable
    /// by the current context with native producer synchronization established.
    /// `release` must own the backing, delete the alias only on Some(context),
    /// and safely release native-only resources on None. No borrowed handles may
    /// escape the callback's owned captures. Never delete this texture elsewhere.
    pub unsafe fn new(
        context: Rc<dyn IGlContext>,
        texture: u32,
        release: impl FnOnce(Option<&GlInterface>) + Send + 'static,
    ) -> Self {
        Self {
            context,
            texture,
            release: Some(Box::new(release)),
        }
    }
}
impl Drop for NativeTextureView {
    fn drop(&mut self) {
        if let Some(release) = self.release.take() {
            // Rc makes this owner thread-bound. Roll back imports even if the
            // native job unwinds before committing its frame lease.
            let _current = CurrentContext(self.context.ensure_current());
            release(Some(&self.context.gl_interface()));
        }
    }
}
// An uncommitted view is always disposed inside the render job, where GL is
// current; after commitment its cleanup travels in NativeOwner below.
struct NativeOwner {
    queue: SharedGlTextureRetirementQueue,
    release: std::sync::Mutex<Option<Release>>,
    retirement: Option<FrameRetirement>,
    _wake: Arc<dyn Fn() + Send + Sync>,
}
impl Drop for NativeOwner {
    fn drop(&mut self) {
        let release = self.release.get_mut().unwrap().take().unwrap();
        let retirement = self.retirement.take().unwrap();
        self.queue.retire_native(move |context| {
            release(context);
            if context.is_none() {
                retirement.quarantine();
            }
            // Otherwise accounting is released only after actual native cleanup.
        });
    }
}
impl NativeGlFrameProducer {
    /// Creates a lease-backed alias without allocating/copying image storage.
    /// Admission reserves retained native bytes before importing. Saturation
    /// drops the captured native frame without queuing a render job.
    ///
    /// # Safety
    /// The callback must obey NativeTextureView::new's contract. Native producer
    /// storage must remain immutable until the callback releases it. Metadata
    /// must describe the sampled alias and count all retained native allocation.
    pub unsafe fn import_native(
        &self,
        metadata: FrameMetadata,
        import: impl FnOnce(&Rc<dyn IGlContext>) -> Result<NativeTextureView, MediaError>
            + Send
            + 'static,
    ) -> MediaFuture<Option<ReadyGlFrame>> {
        if metadata.device != Some(self.device())
            || metadata.device_generation != self.device_generation()
            || metadata.format != FrameFormat::Rgba8
            || metadata.alpha == FrameAlpha::Straight
        {
            return Box::pin(async { Err(unsupported("Incompatible native alias metadata")) });
        }
        let reservation = match self.budget.reserve(metadata.clone()) {
            Ok(v) => v,
            Err(FrameError::BudgetExceeded) => return Box::pin(async { Ok(None) }),
            Err(e) => return Box::pin(async move { Err(unsupported(e.to_string())) }),
        };
        let wake = self.retirement_wakeup();
        self.invoke(move |context| {
            let queue = context
                .shared_frame_retirement_queue()
                .ok_or_else(|| unsupported("Native alias has no retirement queue"))?;
            let mut view = import(context)?;
            if view.texture == 0 {
                view.release.take().unwrap()(Some(&context.gl_interface()));
                return Err(unsupported("Native alias import returned no texture"));
            }
            let owner = NativeOwner {
                _wake: queue.register_wakeup(wake),
                queue,
                retirement: Some(reservation.retirement()),
                release: std::sync::Mutex::new(view.release.take()),
            };
            let frame = reservation.lease(unsafe {
                SharedFrameTexture::new(
                    context.shared_frame_context_id().unwrap(),
                    metadata.device_generation,
                    view.texture,
                    owner,
                )
            });
            let completion = GpuCompletion::new();
            completion
                .retain(frame.clone())
                .map_err(|e| unsupported(e.to_string()))?;
            context.schedule_frame_completion(completion.clone());
            Ok(Some(ReadyGlFrame { frame, completion }))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    #[test]
    fn alias_cleanup_waits_for_consumer_and_keeps_accounting_through_native_retirement() {
        let budget = FrameBudget::new(Default::default());
        let metadata = FrameMetadata {
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
            top_left_origin: true,
            timestamp: std::time::Duration::ZERO,
            device: Some(DeviceIdentity::Context(1)),
            device_generation: 1,
            planes: vec![FramePlane {
                offset: 0,
                stride: 4,
                length: 4,
            }],
            retained_bytes: 4,
        };
        let reservation = budget.reserve(metadata).unwrap();
        let queue = SharedGlTextureRetirementQueue::default();
        let released = Arc::new(AtomicUsize::new(0));
        let observed = released.clone();
        let owner = NativeOwner {
            queue: queue.clone(),
            retirement: Some(reservation.retirement()),
            _wake: queue.register_wakeup(Arc::new(|| {})),
            release: std::sync::Mutex::new(Some(Box::new(move |gl| {
                assert!(gl.is_none()); // Simulate teardown of an unavailable context.
                observed.fetch_add(1, Ordering::SeqCst);
            }))),
        };
        let frame = reservation.lease(owner);
        let completion = GpuCompletion::new();
        completion.retain(frame.clone()).unwrap();
        drop(frame);
        assert_eq!(released.load(Ordering::SeqCst), 0);
        assert_eq!(budget.gpu_reads(), 1);
        completion.complete();
        assert_eq!(budget.gpu_reads(), 0);
        assert_eq!(released.load(Ordering::SeqCst), 0);
        assert_eq!(budget.usage().frames, 1); // queued alias/native teardown
        drop(queue);
        assert_eq!(released.load(Ordering::SeqCst), 1);
        assert_eq!(budget.usage().bytes, 4); // lost context quarantines credits
    }
}
