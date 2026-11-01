use crate::{unsupported, NativeGlFrameProducer};
use ferroui_media::{frames::*, MediaError, MediaFuture, RendererViewport, TextureRenderer};
use ferroui_opengl::IGlContext;
use std::{
    rc::Rc,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

/// Client code called on the owning compositor render thread with its GL
/// context current. Preserve all GL state changed by the rendering library,
/// write premultiplied RGBA8, and sample inputs without mutating their storage.
pub trait GlTextureRender: Send + Sync {
    fn draw(
        &self,
        context: &Rc<dyn IGlContext>,
        output: u32,
        viewport: RendererViewport,
        inputs: &[FrameLease],
    ) -> Result<(), MediaError>;
}
/// Renderer-neutral adapter: scenes, native libraries and hit testing remain
/// client-owned. Native completion retains both output and all sampled inputs.
pub struct GlTextureRenderer {
    producer: NativeGlFrameProducer,
    source: u64,
    sequence: AtomicU64,
    client: Arc<dyn GlTextureRender>,
}
impl GlTextureRenderer {
    pub fn new(
        producer: NativeGlFrameProducer,
        source: u64,
        client: Arc<dyn GlTextureRender>,
    ) -> Self {
        Self {
            producer,
            source,
            sequence: AtomicU64::new(1),
            client,
        }
    }
}
// Own this in the queued draw closure, not the UI future: cancellation may
// abandon a future while its native render job is still queued to execute.
struct Submission {
    completion: GpuCompletion,
    recording: bool,
    scheduled: bool,
}
impl Submission {
    fn begin(&mut self) {
        self.recording = true;
    }
    fn scheduled(&mut self) {
        self.scheduled = true;
    }
}
impl Drop for Submission {
    fn drop(&mut self) {
        if !self.recording {
            self.completion.complete();
        } else if !self.scheduled {
            self.completion.mark_indeterminate();
        }
    }
}
impl TextureRenderer for GlTextureRenderer {
    fn render(
        &self,
        viewport: RendererViewport,
        inputs: Vec<FrameLease>,
        completion: GpuCompletion,
    ) -> MediaFuture<FrameLease> {
        if self.producer.budget().quarantined_reads() > 0 {
            return Box::pin(async {
                Err(unsupported("Renderer GPU completion is indeterminate"))
            });
        }
        let bytes = viewport.width as u64 * viewport.height as u64 * 4;
        let metadata = FrameMetadata {
            identity: FrameIdentity {
                source: self.source,
                generation: viewport.generation,
                sequence: self
                    .sequence
                    .fetch_update(Ordering::AcqRel, Ordering::Acquire, |v| v.checked_add(1))
                    .expect("renderer sequence exhausted"),
            },
            width: viewport.width,
            height: viewport.height,
            visible: FrameRegion {
                x: 0,
                y: 0,
                width: viewport.width,
                height: viewport.height,
            },
            format: FrameFormat::Rgba8,
            color: FrameColor::Srgb,
            alpha: FrameAlpha::Premultiplied,
            rotation: FrameRotation::None,
            mirrored: false,
            top_left_origin: false,
            timestamp: Duration::ZERO,
            device: Some(self.producer.device()),
            device_generation: self.producer.device_generation(),
            planes: vec![FramePlane {
                offset: 0,
                stride: viewport.width as u64 * 4,
                length: bytes,
            }],
            retained_bytes: bytes,
        };
        let mut submission = Submission {
            completion,
            recording: false,
            scheduled: false,
        };
        let client = self.client.clone();
        let draw_inputs = inputs;
        let pending = self.producer.produce(metadata, move |context, output| {
            if draw_inputs.iter().any(|v| !v.is_current()) {
                return Err(MediaError::cancelled());
            }
            if draw_inputs.iter().any(|v| {
                v.metadata().device
                    != context
                        .shared_frame_context_id()
                        .map(DeviceIdentity::Context)
                    || v.metadata().device_generation != context.shared_frame_context_generation()
            }) {
                return Err(unsupported(
                    "Incompatible renderer input context/generation",
                ));
            }
            submission.begin();
            let result = client.draw(context, output, viewport, &draw_inputs);
            // A failed client draw may still have recorded GPU reads/writes.
            context.schedule_frame_completion(submission.completion.clone());
            submission.scheduled();
            result
        });
        Box::pin(async move {
            let ready = pending.await?.ok_or_else(MediaError::cancelled)?;
            match ready.completion().wait().await {
                Ok(()) => ready
                    .take_if_ready()?
                    .ok_or_else(|| unsupported("Native renderer completion missing")),
                Err(error) => Err(unsupported(error.to_string())),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn reserved() -> (FrameBudget, GpuCompletion) {
        let budget = FrameBudget::new(Default::default());
        let frame = budget
            .lease(
                FrameMetadata {
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
                    timestamp: Duration::ZERO,
                    device: Some(DeviceIdentity::Context(1)),
                    device_generation: 1,
                    planes: vec![FramePlane {
                        offset: 0,
                        stride: 4,
                        length: 4,
                    }],
                    retained_bytes: 4,
                },
                (),
            )
            .unwrap();
        let completion = GpuCompletion::new();
        completion.retain(frame).unwrap();
        (budget, completion)
    }
    #[test]
    fn dropping_an_unexecuted_native_job_rolls_back_read_credits() {
        let (budget, completion) = reserved();
        let mut submission = Submission {
            completion: completion.clone(),
            recording: false,
            scheduled: false,
        };
        let queued = move || {
            submission.begin();
        };
        completion.mark_indeterminate(); // UI cancellation before queued job disposal.
        drop(queued);
        assert_eq!(completion.status(), GpuCompletionStatus::Complete);
        assert_eq!(budget.gpu_reads(), 0);
        assert_eq!(budget.quarantined_reads(), 0);
        assert_eq!(budget.usage().frames, 0);
    }
    #[test]
    fn abandoned_recording_is_quarantined_but_a_scheduled_read_waits_for_its_fence() {
        let (budget, completion) = reserved();
        let mut submission = Submission {
            completion: completion.clone(),
            recording: false,
            scheduled: false,
        };
        submission.begin();
        drop(submission);
        assert_eq!(completion.status(), GpuCompletionStatus::Indeterminate);
        assert_eq!(budget.gpu_reads(), 1);
        assert_eq!(budget.quarantined_reads(), 1);
        completion.complete();
        assert_eq!(budget.usage().frames, 0);
        let (budget, completion) = reserved();
        let mut submission = Submission {
            completion: completion.clone(),
            recording: false,
            scheduled: false,
        };
        submission.begin();
        submission.scheduled();
        drop(submission);
        assert_eq!(completion.status(), GpuCompletionStatus::Pending);
        assert_eq!(budget.usage().frames, 1);
        completion.complete();
        assert_eq!(budget.usage().frames, 0);
    }
}
