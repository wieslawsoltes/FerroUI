//! The frame of a video or a camera a 3D material samples (#608, step D7 of
//! `docs/porting/media3d.md`, section 3.11).
//!
//! A [`Scene3DFrameSlot`] is what a snapshot of a scene holds for such a material. The scene does
//! not change when a new frame arrives: the render thread puts the frame into the slot, and the
//! renderer samples what the slot holds when it draws. A new frame is therefore a new render of
//! the output, with no mesh or record upload.

use crate::platform::SharedBitmapImpl;
use crate::reactive::Subscription;
use crate::rendering::composition::frames::FrameMetadata;
use crate::utilities::RefCounted;
use crate::{Ref, Visual};
use std::cell::RefCell;
use std::rc::{Rc, Weak};
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

/// An element that shows the frames of a video or a camera and can give them to a 3D material: a
/// `VisualBrush` over it samples the frame instead of rendering the element (step D7).
pub trait IScene3DFrameProvider {
    /// The slot of the element. The first call starts filling it, with the frame shown now.
    fn frame_slot(&self) -> Arc<Scene3DFrameSlot>;

    /// Calls `handler` on the UI thread when the slot has a new frame, until the returned
    /// subscription is dropped or disposed.
    fn subscribe_frames(&self, handler: Rc<dyn Fn()>) -> Subscription;
}

/// The most visuals [`find_frame_provider`] looks at: a brush over a large tree records it instead.
const MAX_VISITED: usize = 256;

crate::ferro_thread_local! {
    /// The elements with frames on this thread, by the address of their visual. Most applications
    /// have none, so a look through it costs them nothing.
    static PROVIDERS: RefCell<Vec<(usize, Weak<dyn IScene3DFrameProvider>)>> = const { RefCell::new(Vec::new()) };
}

/// Registers `provider` as the frames of `visual`, for as long as the provider lives.
pub fn register_frame_provider(visual: &Visual, provider: &Rc<dyn IScene3DFrameProvider>) {
    let address = std::ptr::from_ref(visual) as usize;
    PROVIDERS.with(|providers| {
        let mut providers = providers.borrow_mut();
        providers.retain(|(other, weak)| *other != address && weak.strong_count() > 0);
        providers.push((address, Rc::downgrade(provider)));
    });
}

/// The provider of the frames `visual` shows: its own, or that of the first of its descendants
/// that has one. `None` for a visual that shows none.
pub fn find_frame_provider(visual: &Ref<Visual>) -> Option<Rc<dyn IScene3DFrameProvider>> {
    PROVIDERS.with(|providers| {
        let providers = providers.borrow();
        if providers.is_empty() {
            return None;
        }
        let find = |visual: &Visual| {
            let address = std::ptr::from_ref(visual) as usize;
            providers.iter().find(|(other, _)| *other == address).and_then(|(_, weak)| weak.upgrade())
        };
        let mut stack: Vec<Ref<Visual>> = vec![visual.clone()];
        let mut visited = 0;
        while let Some(current) = stack.pop() {
            if let Some(provider) = find(&current) {
                return Some(provider);
            }
            visited += 1;
            if visited >= MAX_VISITED {
                return None;
            }
            if let Some(children) = current.visual_children_snapshot() {
                stack.extend(children.iter().rev().cloned());
            }
        }
        None
    })
}
