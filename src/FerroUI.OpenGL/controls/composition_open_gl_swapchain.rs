use crate::composition::task_support::start;
use crate::composition::{
    CompositionGlTextureInfo, ICompositionGlContext, ICompositionGlTexture, ICompositionGlTextureLease,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::{CompositionDrawingSurface, ServerJobTask};
use ferroui_base::threading::DispatcherTask;
use ferroui_base::PixelSize;
use std::cell::RefCell;
use std::rc::Rc;

/// The textures a control draws its frames into, one after another, while the compositor
/// still uses the previous ones.
pub(crate) struct CompositionOpenGlSwapchain {
    context: Rc<dyn ICompositionGlContext>,
    surface: CompositionDrawingSurface,
    entries: Rc<RefCell<Vec<Rc<Entry>>>>,
}

pub(crate) struct Entry {
    texture: Rc<dyn ICompositionGlTexture>,
    last_present: RefCell<Option<ServerJobTask<()>>>,
}

impl CompositionOpenGlSwapchain {
    pub(crate) fn new(context: Rc<dyn ICompositionGlContext>, surface: CompositionDrawingSurface) -> Self {
        Self { context, surface, entries: Rc::new(RefCell::new(Vec::new())) }
    }

    fn cleanup_and_find_next_entry(&self, size: PixelSize) -> Option<Rc<Entry>> {
        let mut first_found: Option<Rc<Entry>> = None;
        let mut found_multiple = false;

        let mut c = self.entries.borrow().len();
        while c > 0 {
            c -= 1;
            let entry = self.entries.borrow()[c].clone();
            let ready = entry.texture.is_ready_for_draw();
            let matches = entry.texture.size() == size;
            let broken = entry
                .last_present
                .borrow()
                .as_ref()
                .is_some_and(|task| task.is_completed() && !task.is_completed_successfully());
            if broken || (!matches && ready) {
                drop(entry.texture.dispose_async());
                self.entries.borrow_mut().remove(c);
            }

            if matches && ready {
                if first_found.is_none() {
                    first_found = Some(entry);
                } else {
                    found_multiple = true;
                }
            }
        }

        // We are making sure that there was at least one texture of the same size in flight
        // Otherwise we might encounter UI thread lockups
        if found_multiple {
            first_found
        } else {
            None
        }
    }

    /// Starts a frame: the lease of the frame and the texture to draw it into.
    pub(crate) fn begin_draw(&self, size: PixelSize) -> (Lease, CompositionGlTextureInfo) {
        let entry = match self.cleanup_and_find_next_entry(size) {
            Some(entry) => entry,
            None => {
                let entry = Rc::new(Entry {
                    texture: self.context.create_texture(&self.surface, size),
                    last_present: RefCell::new(None),
                });
                self.entries.borrow_mut().push(entry.clone());
                entry
            }
        };

        let lease = entry.texture.begin_draw();
        let texture_info = lease.texture_info();
        (Lease { entry, lease: RefCell::new(Some(lease)) }, texture_info)
    }

    pub(crate) fn dispose_async(&self) -> DispatcherTask<()> {
        let entries = self.entries.clone();
        start(async move {
            // Snapshot, since awaiting the disposal can pump the dispatcher
            let snapshot = entries.borrow().clone();
            for entry in snapshot {
                let _ = entry.texture.dispose_async().await;
            }
            entries.borrow_mut().clear();
        })
    }
}

/// A frame being drawn: disposing the lease presents the frame, discarding it does not.
pub(crate) struct Lease {
    entry: Rc<Entry>,
    lease: RefCell<Option<Rc<dyn ICompositionGlTextureLease>>>,
}

impl Lease {
    pub(crate) fn discard(&self) {
        let lease = self.lease.borrow_mut().take();
        if let Some(lease) = lease {
            lease.dispose();
        }
    }
}

impl IDisposable for Lease {
    fn dispose(&self) {
        let lease = self.lease.borrow_mut().take();
        if let Some(lease) = lease {
            *self.entry.last_present.borrow_mut() = Some(lease.present_async());
        }
    }
}
