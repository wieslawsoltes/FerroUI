//! The frame of a video or a camera a 3D material samples (#608, step D7 of
//! `docs/porting/media3d.md`, section 3.11).
//!
//! A [`Scene3DFrameSlot`] is what a snapshot of a scene holds for such a material. The scene does
//! not change when a new frame arrives: the render thread puts the frame into the slot, and the
//! renderer samples what the slot holds when it draws. A new frame is therefore a new render of
//! the output, with no mesh or record upload.

use crate::platform::SharedBitmapImpl;
use crate::rendering::composition::frames::FrameMetadata;
use crate::utilities::RefCounted;
use std::sync::{Arc, Mutex};

/// A frame in a slot: the bitmap of the frame on the device of the renderer, and its metadata
/// (the crop, the rotation and the mirroring a sampler applies).
struct SlotFrame {
    bitmap: RefCounted<SharedBitmapImpl>,
    metadata: FrameMetadata,
}

/// The frame a 3D material samples now: see the documentation of the module.
#[derive(Default)]
pub struct Scene3DFrameSlot {
    frame: Mutex<Option<SlotFrame>>,
}

impl Scene3DFrameSlot {
    /// An empty slot: a material paints nothing until a frame arrives.
    pub fn new() -> Arc<Scene3DFrameSlot> {
        Arc::new(Scene3DFrameSlot::default())
    }

    /// The bitmap and metadata of the frame the slot holds.
    pub fn current(&self) -> Option<(Arc<SharedBitmapImpl>, FrameMetadata)> {
        let frame = self.frame.lock().unwrap_or_else(|e| e.into_inner());
        frame.as_ref().map(|frame| (frame.bitmap.item(), frame.metadata.clone()))
    }

    /// Puts a frame into the slot, on the render thread. The reference to the frame it held is
    /// released there.
    pub fn publish(&self, bitmap: RefCounted<SharedBitmapImpl>, metadata: FrameMetadata) {
        let previous = {
            let mut frame = self.frame.lock().unwrap_or_else(|e| e.into_inner());
            frame.replace(SlotFrame { bitmap, metadata })
        };
        drop(previous);
    }

    /// Empties the slot, on the render thread.
    pub fn clear(&self) {
        let previous = self.frame.lock().unwrap_or_else(|e| e.into_inner()).take();
        drop(previous);
    }
}
