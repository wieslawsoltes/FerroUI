//! The screens of the platform (the port of `SnapshotScreensImpl.cs`).

use super::i_wayland_outputs_sink::IWaylandOutputsSink;
use super::wayland_output_snapshot::{WaylandOutputId, WaylandOutputSnapshot, WaylandOutputsSnapshot};
use ferroui_base::PixelRect;
use ferroui_controls::platform::{
    IPlatformHandle, IScreenImpl, ITopLevelImpl, PlatformScreen, Screen, ScreenHelper, ScreensBase, ScreensBaseImpl,
    ScreensBaseImplExt,
};
use std::any::Any;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// UI-thread screens backed by the most recent output snapshot pushed
/// from the wayland worker.
pub struct SnapshotScreensImpl {
    this: Weak<SnapshotScreensImpl>,
    base: ScreensBase<WaylandOutputId, WaylandSnapshotScreen>,
    latest: RefCell<WaylandOutputsSnapshot>,
}

impl SnapshotScreensImpl {
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            base: ScreensBase::new(),
            latest: RefCell::new(WaylandOutputsSnapshot::default()),
        })
    }

    pub(crate) fn lookup(&self, id: WaylandOutputId) -> Option<WaylandOutputSnapshot> {
        self.latest.borrow().outputs.iter().find(|output| output.id == id).cloned()
    }

    /// The screen of the last of `ids` that is a screen: the last-entered output is the
    /// "current" one, so the list is walked in reverse to prefer the freshest enter.
    pub(crate) fn screen_from_output_ids(&self, ids: &[WaylandOutputId]) -> Option<Rc<Screen>> {
        // The screens are brought up to date with the snapshot before they are asked for.
        let _ = self.all_platform_screens();
        ids.iter().rev().find_map(|id| self.try_get_screen(id)).map(|screen| screen.base.screen().clone())
    }
}

impl IWaylandOutputsSink for SnapshotScreensImpl {
    fn on_outputs_changed(&self, snapshot: WaylandOutputsSnapshot) {
        *self.latest.borrow_mut() = snapshot;
        if let Some(this) = self.this.upgrade() {
            this.on_changed();
        }
    }
}

impl ScreensBaseImpl for SnapshotScreensImpl {
    type Key = WaylandOutputId;
    type Screen = WaylandSnapshotScreen;

    fn screens_base(&self) -> &ScreensBase<WaylandOutputId, WaylandSnapshotScreen> {
        &self.base
    }

    fn get_all_screen_keys(&self) -> Vec<WaylandOutputId> {
        self.latest.borrow().outputs.iter().map(|output| output.id).collect()
    }

    fn create_screen_from_key(&self, key: &WaylandOutputId) -> Rc<WaylandSnapshotScreen> {
        Rc::new(WaylandSnapshotScreen::new(*key))
    }

    fn screen_added(&self, screen: &Rc<WaylandSnapshotScreen>) {
        screen.refresh(self);
        // The base implementation calls `screen_changed`, which refreshes again, as the
        // reference does.
        self.screen_changed(screen);
    }

    fn screen_changed(&self, screen: &Rc<WaylandSnapshotScreen>) {
        screen.refresh(self);
    }

    fn screen_from_top_level_core(&self, top_level: &dyn ITopLevelImpl) -> Option<Rc<Screen>> {
        #[cfg(target_os = "linux")]
        if let Some(window) = top_level.as_any().downcast_ref::<crate::window_impl::WindowImpl>() {
            if let Some(screen) = self.screen_from_output_ids(&window.base().current_output_ids()) {
                return Some(screen);
            }
        }

        match top_level.as_window_impl() {
            Some(window) => ScreenHelper::screen_from_window(window, &IScreenImpl::all_screens(self)),
            None => None,
        }
    }
}

/// A screen of an output.
pub struct WaylandSnapshotScreen {
    base: PlatformScreen,
    id: WaylandOutputId,
}

impl WaylandSnapshotScreen {
    fn new(id: WaylandOutputId) -> Self {
        Self { base: PlatformScreen::new(Rc::new(WaylandScreenHandle::new(id))), id }
    }

    pub fn refresh(&self, owner: &SnapshotScreensImpl) {
        let Some(snap) = owner.lookup(self.id) else {
            return;
        };

        self.base.set_display_name(snap.name.or(snap.description).or(snap.model));
        // Per design: screen-level scale is always 1; per-window scaling
        // is owned by WSurface (preferred_buffer_scale + wp_fractional_scale).
        self.base.set_scaling(1.0);
        let bounds = PixelRect::new(
            snap.logical_position.x,
            snap.logical_position.y,
            snap.logical_size.width,
            snap.logical_size.height,
        );
        self.base.set_bounds(bounds);
        // Wayland gives us no panel/strut info; treat the full bounds as
        // working area (matches Mutter / KWin client expectations).
        self.base.set_working_area(bounds);
    }
}

impl AsRef<PlatformScreen> for WaylandSnapshotScreen {
    fn as_ref(&self) -> &PlatformScreen {
        &self.base
    }
}

/// The platform handle of a screen: the identity of its output.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WaylandScreenHandle {
    id: WaylandOutputId,
}

impl WaylandScreenHandle {
    pub fn new(id: WaylandOutputId) -> Self {
        Self { id }
    }

    pub fn id(&self) -> WaylandOutputId {
        self.id
    }
}

impl IPlatformHandle for WaylandScreenHandle {
    fn handle(&self) -> isize {
        0
    }

    fn handle_descriptor(&self) -> Option<&str> {
        Some("WaylandOutput")
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
