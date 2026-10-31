use crate::{
    frames::{FrameAlpha, FrameError, FrameLease, FrameMailbox, GpuCompletion},
    MediaError, MediaErrorKind, MediaFuture,
};
use std::{cell::Cell, rc::Rc, sync::Arc};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RenderSchedule {
    #[default]
    OnInvalidation,
    Continuous,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RendererViewport {
    pub width: u32,
    pub height: u32,
    pub generation: u64,
}
pub trait TextureRenderer {
    /// Outputs immutable color storage using the admitted device. Private depth,
    /// stencil and MSAA resources remain the client renderer's responsibility.
    /// Before submitting GPU writes, the backend must retain its output in
    /// `completion`; inputs are already retained by the host. Signal only after
    /// actual GPU completion, even when the returned future is abandoned.
    fn render(
        &self,
        viewport: RendererViewport,
        inputs: Vec<FrameLease>,
        completion: GpuCompletion,
    ) -> MediaFuture<FrameLease>;
}
struct HostInner {
    renderer: Rc<dyn TextureRenderer>,
    viewport: Cell<RendererViewport>,
    active: Cell<bool>,
    dirty: Cell<bool>,
    busy: Cell<bool>,
    schedule: Cell<RenderSchedule>,
    output: Arc<FrameMailbox>,
    source: u64,
}
impl Drop for HostInner {
    fn drop(&mut self) {
        self.output.clear();
    }
}
#[derive(Clone)]
pub struct TextureRendererHost(Rc<HostInner>);
struct RenderGuard {
    host: std::rc::Weak<HostInner>,
    completed: bool,
}
impl Drop for RenderGuard {
    fn drop(&mut self) {
        if let Some(host) = self.host.upgrade() {
            host.busy.set(false);
            if !self.completed {
                host.dirty.set(true);
            }
        }
    }
}
impl TextureRendererHost {
    pub fn new(renderer: Rc<dyn TextureRenderer>, source: u64) -> Self {
        Self(Rc::new(HostInner {
            renderer,
            viewport: Cell::new(RendererViewport {
                width: 0,
                height: 0,
                generation: 1,
            }),
            active: Cell::new(false),
            dirty: Cell::new(true),
            busy: Cell::new(false),
            schedule: Cell::new(RenderSchedule::OnInvalidation),
            output: Arc::new(FrameMailbox::new(source, 1)),
            source,
        }))
    }
    pub fn output(&self) -> Arc<FrameMailbox> {
        self.0.output.clone()
    }
    pub fn set_active(&self, active: bool) {
        self.0.active.set(active);
        if active {
            self.invalidate();
        } else {
            self.0.output.clear();
        }
    }
    pub fn set_schedule(&self, schedule: RenderSchedule) {
        self.0.schedule.set(schedule);
        self.invalidate();
    }
    pub fn invalidate(&self) {
        self.0.dirty.set(true);
    }
    pub fn resize(
        &self,
        width: f64,
        height: f64,
        scale: f64,
        max_dimension: u32,
    ) -> Result<(), MediaError> {
        if !width.is_finite()
            || !height.is_finite()
            || !scale.is_finite()
            || width < 0.
            || height < 0.
            || scale <= 0.
        {
            return Err(MediaError::new(
                MediaErrorKind::Unsupported,
                "Invalid renderer bounds or scale",
            ));
        }
        let (w, h) = ((width * scale).ceil(), (height * scale).ceil());
        if w > max_dimension as f64 || h > max_dimension as f64 || !w.is_finite() || !h.is_finite()
        {
            return Err(MediaError::new(
                MediaErrorKind::Unsupported,
                "Renderer dimensions exceed device limits",
            ));
        }
        let old = self.0.viewport.get();
        if (old.width, old.height) == (w as u32, h as u32) {
            return Ok(());
        }
        let generation = old
            .generation
            .checked_add(1)
            .expect("renderer generation exhausted");
        self.0.viewport.set(RendererViewport {
            width: w as u32,
            height: h as u32,
            generation,
        });
        self.0
            .output
            .replace_generation(generation)
            .expect("increasing generation");
        self.invalidate();
        Ok(())
    }
    /// The existing render scheduler calls this once per presentation opportunity.
    /// Hidden, zero-sized, clean or already-busy hosts do not schedule work.
    pub fn render(&self, inputs: Vec<FrameLease>) -> Option<MediaFuture<()>> {
        let viewport = self.0.viewport.get();
        if !self.0.active.get()
            || viewport.width == 0
            || viewport.height == 0
            || self.0.busy.get()
            || inputs.iter().any(|input| !input.is_current())
            || (!self.0.dirty.get() && self.0.schedule.get() == RenderSchedule::OnInvalidation)
        {
            return None;
        }
        self.0.busy.set(true);
        self.0.dirty.set(false);
        let guard = RenderGuard {
            host: Rc::downgrade(&self.0),
            completed: false,
        };
        let completion = GpuCompletion::new();
        for input in &inputs {
            if completion.retain(input.clone()).is_err() {
                // No backend has recorded work yet. Roll back earlier input
                // reservations and keep the invalidation for a later frame.
                completion.complete();
                drop(guard);
                return None;
            }
        }
        let pending = self
            .0
            .renderer
            .render(viewport, inputs.clone(), completion.clone());
        let weak = Rc::downgrade(&self.0);
        Some(Box::pin(async move {
            let mut guard = guard;
            let frame = pending.await?;
            match completion.retain(frame.clone()) {
                Ok(()) | Err(FrameError::CompletionAlreadySignaled) => {}
                Err(error) => {
                    return Err(MediaError::new(
                        MediaErrorKind::NativeFailure,
                        error.to_string(),
                    ))
                }
            }
            completion.wait().await.map_err(|error| {
                MediaError::new(MediaErrorKind::NativeFailure, error.to_string())
            })?;
            if inputs.iter().any(|input| !input.is_current()) {
                return Err(MediaError::cancelled());
            }
            let host = weak
                .upgrade()
                .filter(|h| h.active.get() && h.viewport.get() == viewport)
                .ok_or_else(MediaError::cancelled)?;
            let m = frame.metadata();
            if m.identity.source != host.source
                || m.identity.generation != viewport.generation
                || (m.width, m.height) != (viewport.width, viewport.height)
                || !matches!(m.alpha, FrameAlpha::Opaque | FrameAlpha::Premultiplied)
                || inputs.iter().any(|input| input.same_storage(&frame))
            {
                return Err(MediaError::new(
                    MediaErrorKind::Unsupported,
                    "Invalid renderer output or cyclic texture dependency",
                ));
            }
            host.output
                .publish(frame)
                .map_err(|e| MediaError::new(MediaErrorKind::NativeFailure, e.to_string()))?;
            guard.completed = true;
            Ok(())
        }))
    }
    pub fn viewport_point(
        &self,
        x: f64,
        y: f64,
        logical_width: f64,
        logical_height: f64,
    ) -> Option<(f64, f64)> {
        if ![x, y, logical_width, logical_height]
            .iter()
            .all(|v| v.is_finite())
            || logical_width <= 0.
            || logical_height <= 0.
        {
            return None;
        }
        let v = self.0.viewport.get();
        Some((
            x / logical_width * v.width as f64,
            y / logical_height * v.height as f64,
        ))
    }
}
/// Projects a planar texture corner with a client-supplied column-major matrix.
/// Points on/behind the viewing plane and non-finite inputs are rejected.
pub fn project_texture_point(matrix: &[f64; 16], u: f64, v: f64) -> Option<(f64, f64)> {
    if !matrix.iter().chain([&u, &v]).all(|x| x.is_finite()) {
        return None;
    }
    let w = matrix[3] * u + matrix[7] * v + matrix[15];
    if w <= 0. {
        return None;
    }
    let x = (matrix[0] * u + matrix[4] * v + matrix[12]) / w;
    let y = (matrix[1] * u + matrix[5] * v + matrix[13]) / w;
    (x.is_finite() && y.is_finite()).then_some((x, y))
}

#[cfg(test)]
#[path = "renderer_tests.rs"]
mod tests;
