//! The transparency level of a window (the port of
//! `TransparencyHelper.cs`).

use crate::x11_globals::X11Globals;
use crate::x11_info::X11Info;
use crate::xlib::{self, PropertyMode, XID};
use ferroui_controls::WindowTransparencyLevel;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// Whether a transparency level can be had (`IsSupported`): none without
/// a compositing manager, transparency with one, blur with the one of KDE.
pub(crate) fn is_supported(level: &WindowTransparencyLevel, is_composition_enabled: bool, wm_name: Option<&str>) -> bool {
    // None is suppported when composition is disabled.
    if *level == WindowTransparencyLevel::none() {
        return !is_composition_enabled;
    }

    // Transparent is suppported when composition is enabled.
    if *level == WindowTransparencyLevel::transparent() {
        return is_composition_enabled;
    }

    // Blur is supported when composition is enabled and KWin is used.
    if *level == WindowTransparencyLevel::blur() {
        return is_composition_enabled && wm_name == Some("KWin");
    }

    false
}

/// The level a request resolves to (`SetTransparencyRequest`): the first
/// supported one, or the default for the state of composition.
pub(crate) fn resolve_level(
    levels: &[WindowTransparencyLevel],
    is_composition_enabled: bool,
    wm_name: Option<&str>,
) -> WindowTransparencyLevel {
    for level in levels {
        if !is_supported(level, is_composition_enabled, wm_name) {
            continue;
        }

        return level.clone();
    }

    // If we get here, we didn't find a supported level. Use the defualt of Transparent or
    // None, depending on whether composition is enabled.
    if is_composition_enabled {
        WindowTransparencyLevel::transparent()
    } else {
        WindowTransparencyLevel::none()
    }
}

/// Follows the compositing manager and the window manager for one window.
pub struct TransparencyHelper {
    x11: Rc<X11Info>,
    window: XID,
    globals: Rc<X11Globals>,
    current_level: RefCell<WindowTransparencyLevel>,
    requested_levels: RefCell<Option<Vec<WindowTransparencyLevel>>>,
    blur_atoms_are_set: Cell<bool>,
    transparency_level_changed: RefCell<Option<Rc<dyn Fn(WindowTransparencyLevel)>>>,
    subscriptions: Cell<(u64, u64)>,
}

impl TransparencyHelper {
    pub fn new(x11: &Rc<X11Info>, window: XID, globals: &Rc<X11Globals>) -> Rc<Self> {
        let this = Rc::new(Self {
            x11: x11.clone(),
            window,
            globals: globals.clone(),
            current_level: RefCell::new(WindowTransparencyLevel::none()),
            requested_levels: RefCell::new(None),
            blur_atoms_are_set: Cell::new(false),
            transparency_level_changed: RefCell::new(None),
            subscriptions: Cell::new((0, 0)),
        });
        let update = |weak: Weak<Self>| {
            move |()| {
                if let Some(this) = weak.upgrade() {
                    this.update_transparency();
                }
            }
        };
        let composition = globals.composition_changed.subscribe(update(Rc::downgrade(&this)));
        let window_manager = globals.window_manager_changed.subscribe(update(Rc::downgrade(&this)));
        this.subscriptions.set((composition, window_manager));
        this
    }

    pub fn transparency_level_changed(&self) -> Option<Rc<dyn Fn(WindowTransparencyLevel)>> {
        self.transparency_level_changed.borrow().clone()
    }

    pub fn set_transparency_level_changed(&self, value: Option<Rc<dyn Fn(WindowTransparencyLevel)>>) {
        let previous = self.transparency_level_changed.replace(value);
        drop(previous);
    }

    pub fn current_level(&self) -> WindowTransparencyLevel {
        self.current_level.borrow().clone()
    }

    pub fn set_current_level(&self, value: WindowTransparencyLevel) {
        if *self.current_level.borrow() != value {
            *self.current_level.borrow_mut() = value.clone();
            if let Some(changed) = self.transparency_level_changed() {
                changed(value);
            }
        }
    }

    pub fn set_transparency_request(&self, levels: &[WindowTransparencyLevel]) {
        *self.requested_levels.borrow_mut() = Some(levels.to_vec());

        let level = resolve_level(levels, self.globals.is_composition_enabled(), self.globals.wm_name().as_deref());
        self.set_blur(level == WindowTransparencyLevel::blur());
        self.set_current_level(level);
    }

    fn update_transparency(&self) {
        let levels = self.requested_levels.borrow().clone().unwrap_or_default();
        self.set_transparency_request(&levels);
    }

    fn set_blur(&self, blur: bool) {
        let atoms = self.x11.atoms();
        if blur {
            if !self.blur_atoms_are_set.get() {
                xlib::x_change_property_longs(
                    self.x11.display(),
                    self.window,
                    atoms._KDE_NET_WM_BLUR_BEHIND_REGION,
                    atoms.CARDINAL,
                    PropertyMode::Replace,
                    &[0],
                );
                self.blur_atoms_are_set.set(true);
            }
        } else if self.blur_atoms_are_set.get() {
            xlib::x_delete_property(self.x11.display(), self.window, atoms._KDE_NET_WM_BLUR_BEHIND_REGION);
            self.blur_atoms_are_set.set(false);
        }
    }

    pub fn dispose(&self) {
        let (composition, window_manager) = self.subscriptions.get();
        self.globals.window_manager_changed.unsubscribe(window_manager);
        self.globals.composition_changed.unsubscribe(composition);
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    fn none() -> WindowTransparencyLevel {
        WindowTransparencyLevel::none()
    }
    fn transparent() -> WindowTransparencyLevel {
        WindowTransparencyLevel::transparent()
    }
    fn blur() -> WindowTransparencyLevel {
        WindowTransparencyLevel::blur()
    }

    #[test]
    fn the_levels_depend_on_the_compositing_manager() {
        assert!(is_supported(&none(), false, None));
        assert!(!is_supported(&none(), true, None));
        assert!(is_supported(&transparent(), true, None));
        assert!(!is_supported(&transparent(), false, None));
        assert!(is_supported(&blur(), true, Some("KWin")));
        assert!(!is_supported(&blur(), true, Some("Mutter")));
        assert!(!is_supported(&blur(), false, Some("KWin")));
        assert!(!is_supported(&WindowTransparencyLevel::acrylic_blur(), true, Some("KWin")));
        assert!(!is_supported(&WindowTransparencyLevel::mica(), true, Some("KWin")));
    }

    #[test]
    fn the_first_supported_level_of_a_request_is_taken() {
        assert_eq!(resolve_level(&[blur(), transparent()], true, Some("KWin")), blur());
        assert_eq!(resolve_level(&[blur(), transparent()], true, Some("Mutter")), transparent());
        assert_eq!(resolve_level(&[blur(), transparent(), none()], false, None), none());
    }

    #[test]
    fn without_a_supported_level_the_default_follows_composition() {
        assert_eq!(resolve_level(&[], true, None), transparent());
        assert_eq!(resolve_level(&[], false, None), none());
        assert_eq!(resolve_level(&[none()], true, None), transparent());
        assert_eq!(resolve_level(&[transparent(), blur()], false, Some("KWin")), none());
    }
}
