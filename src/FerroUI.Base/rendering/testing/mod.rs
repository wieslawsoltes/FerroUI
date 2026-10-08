//! Test doubles for the render contracts: a platform drawing context that
//! records what is drawn, render targets and a render interface built on
//! it. Available to tests of this crate and, with the `testing` feature, to
//! other crates.

mod mock_drawing_context_impl;
mod mock_platform_render_interface;

pub use mock_drawing_context_impl::{
    DrawingLog, MockDrawingContextImpl, MockDrawingContextLayerImpl, MockRenderTargetBitmapImpl,
};
pub use mock_platform_render_interface::{
    MockGeometryImpl, MockGlyphRunImpl, MockPlatformRenderInterface, MockPlatformRenderInterfaceContext, MockRegion,
    MockRenderTarget, MockStreamGeometryImpl,
};

use crate::media::BoxShadow;
use crate::rendering::composition::drawing::{
    DrawBitmapPayload, DrawCustomPayload, DrawEllipsePayload, DrawGeometryPayload, DrawGlyphRunPayload,
    DrawLinePayload, DrawRectanglePayload, PushClipPayload, PushEffectPayload, PushGeometryClipPayload,
    PushOpacityMaskPayload, PushOpacityPayload, PushRenderOptionsPayload, PushTextOptionsPayload, PushTransformPayload,
    RenderDataOpcode, RenderDataReader, RenderDataStream,
};
use crate::rendering::{IRenderLoop, IRenderLoopTask};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// A render loop that never ticks by itself: tests drive frames explicitly.
#[derive(Default)]
pub struct ManualRenderLoop {
    tasks: Mutex<Vec<Arc<dyn IRenderLoopTask>>>,
    runs_in_background: bool,
    frames_out_of_turn: AtomicUsize,
}

impl ManualRenderLoop {
    pub fn new() -> Arc<ManualRenderLoop> {
        Arc::new(ManualRenderLoop::default())
    }

    /// A loop that a test ticks from a thread of its own: it reports that it
    /// runs in the background.
    pub fn background() -> Arc<ManualRenderLoop> {
        Arc::new(ManualRenderLoop { runs_in_background: true, ..ManualRenderLoop::default() })
    }

    /// Runs every registered task once, on the calling thread.
    pub fn tick(&self) {
        let tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner()).clone();
        for task in tasks {
            task.render();
        }
    }

    /// The number of registered tasks.
    pub fn task_count(&self) -> usize {
        self.tasks.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    /// How often a frame out of turn was asked for. The loop does not
    /// render one: whoever ticks it does.
    pub fn frames_requested_out_of_turn(&self) -> usize {
        self.frames_out_of_turn.load(Ordering::SeqCst)
    }
}

impl IRenderLoop for ManualRenderLoop {
    fn add(&self, i: Arc<dyn IRenderLoopTask>) {
        self.tasks.lock().unwrap_or_else(|e| e.into_inner()).push(i);
    }

    fn remove(&self, i: &Arc<dyn IRenderLoopTask>) {
        self.tasks.lock().unwrap_or_else(|e| e.into_inner()).retain(|t| !Arc::ptr_eq(t, i));
    }

    fn runs_in_background(&self) -> bool {
        self.runs_in_background
    }

    fn wakeup(&self) {}

    fn request_frame_out_of_turn(&self) {
        self.frames_out_of_turn.fetch_add(1, Ordering::SeqCst);
    }
}

/// The opcodes recorded in a stream, in order (decoded with the payload
/// sizes, so that a test can state the operations it expects).
pub fn recorded_opcodes(stream: &RenderDataStream) -> Vec<RenderDataOpcode> {
    let mut reader = RenderDataReader::new(stream.opcodes());
    let mut opcodes = Vec::new();
    while !reader.is_at_end() {
        let opcode = reader.peek::<RenderDataOpcode>();
        match opcode {
            RenderDataOpcode::DrawLine => drop(reader.read_payload::<DrawLinePayload>()),
            RenderDataOpcode::DrawRectangle => {
                let payload = reader.read_payload::<DrawRectanglePayload>();
                for _ in 0..payload.box_shadow_count {
                    reader.read::<BoxShadow>();
                }
            }
            RenderDataOpcode::DrawEllipse => drop(reader.read_payload::<DrawEllipsePayload>()),
            RenderDataOpcode::DrawGeometry => drop(reader.read_payload::<DrawGeometryPayload>()),
            RenderDataOpcode::DrawGlyphRun => drop(reader.read_payload::<DrawGlyphRunPayload>()),
            RenderDataOpcode::DrawBitmap => drop(reader.read_payload::<DrawBitmapPayload>()),
            RenderDataOpcode::DrawCustom => drop(reader.read_payload::<DrawCustomPayload>()),
            RenderDataOpcode::PushClip => drop(reader.read_payload::<PushClipPayload>()),
            RenderDataOpcode::PushGeometryClip => drop(reader.read_payload::<PushGeometryClipPayload>()),
            RenderDataOpcode::PushOpacity => drop(reader.read_payload::<PushOpacityPayload>()),
            RenderDataOpcode::PushOpacityMask => drop(reader.read_payload::<PushOpacityMaskPayload>()),
            RenderDataOpcode::PushTransform => drop(reader.read_payload::<PushTransformPayload>()),
            RenderDataOpcode::PushRenderOptions => drop(reader.read_payload::<PushRenderOptionsPayload>()),
            RenderDataOpcode::PushTextOptions => drop(reader.read_payload::<PushTextOptionsPayload>()),
            RenderDataOpcode::PushEffect => drop(reader.read_payload::<PushEffectPayload>()),
            RenderDataOpcode::Pop => drop(reader.read::<RenderDataOpcode>()),
            RenderDataOpcode::Invalid => panic!("the stream holds an invalid opcode"),
        }
        opcodes.push(opcode);
    }
    opcodes
}
