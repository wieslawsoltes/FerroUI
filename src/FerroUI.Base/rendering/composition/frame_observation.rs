use super::Compositor;
use std::sync::{Arc, atomic::{AtomicU8, Ordering}};

/// What the next composition batch did in the native render pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum CompositionFrameStatus {
    Rendered = 2,
    NoSceneUpdate = 1,
    Discarded = 0,
}

impl Compositor {
    /// Observe the first scene drawn after registration. Call before showing a
    /// window to include the platform's paint during Show. The receipt carries
    /// the most recent batch the server applied before that scene.
    pub fn observe_first_scene(&self, callback: impl FnOnce(i64) + Send + 'static) {
        self.post_server_job(move |server| server.observe_scene(callback), false);
    }

    /// Observe the next batch from the UI thread. Its receipt identifies a
    /// native render pass and does not measure the display's scan-out time.
    /// Applications may share a batch across several updates.
    pub fn observe_next_frame(&self, callback: impl FnOnce(i64, CompositionFrameStatus) + Send + 'static) {
        let status = Arc::new(AtomicU8::new(CompositionFrameStatus::Discarded as u8));
        let observed = status.clone();
        self.post_server_job(move |server| {
            let result = if server.scene_was_drawn() {
                CompositionFrameStatus::Rendered
            } else {
                CompositionFrameStatus::NoSceneUpdate
            };
            observed.store(result as u8, Ordering::Release);
        }, true);
        let batch = self.request_commit_async();
        let sequence = batch.sequence_id();
        batch.rendered().on_completed(move || {
            let result = match status.load(Ordering::Acquire) {
                2 => CompositionFrameStatus::Rendered,
                1 => CompositionFrameStatus::NoSceneUpdate,
                _ => CompositionFrameStatus::Discarded,
            };
            callback(sequence, result);
        });
    }
}
