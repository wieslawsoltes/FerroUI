//! Popups: windows without a frame that are owned by the window they
//! belong to, are shown without being activated and are placed by the
//! managed popup positioner.

use crate::interop::unmanaged_methods::{
    create_window_ex, get_ancestor, get_class_long_ptr, get_focus, set_class_long_ptr, set_focus, ClassLongIndex,
    ClassStyles, HitTestValues, MouseActivate, WindowStyles, WindowsMessage, CW_USEDEFAULT, MONITOR,
};
use crate::window_impl::{WindowImpl, WindowKind, WindowProperties};
use ferroui_base::{PixelPoint, Size};
use ferroui_controls::platform::{IPopupImpl, ITopLevelImpl, IWindowBaseImpl, IWindowImpl};
use ferroui_controls::primitives::popup_positioning::{
    IPopupPositioner, ManagedPopupPositioner, ManagedPopupPositionerPopupImplHelper,
};
use ferroui_controls::{WindowDecorations, WindowResizeReason, WindowState};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// `GA_ROOT` of `GetAncestor`: the root window of a chain of parents.
const GA_ROOT: u32 = 2;

/// A popup of the Windows backend.
///
/// In the reference this is a class that derives from the window
/// implementation; here a popup is a window implementation of the popup
/// kind, and this type creates it.
pub struct PopupImpl;

impl PopupImpl {
    /// Creates a popup that belongs to `parent`.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(parent: Rc<dyn IWindowBaseImpl>) -> Rc<WindowImpl> {
        let parent_handle = parent.handle().map_or(0, |handle| handle.handle());
        let state = PopupState {
            parent: parent.clone(),
            parent_handle,
            drop_shadow_hint: Cell::new(true),
            is_hit_test_visible: Cell::new(true),
            max_auto_size: Cell::new(None),
            popup_positioner: RefCell::new(None),
        };

        let window = WindowImpl::create(
            WindowKind::Popup(state),
            WindowProperties {
                show_in_taskbar: false,
                is_resizable: false,
                is_minimizable: false,
                is_maximizable: false,
                decorations: WindowDecorations::None,
                is_full_screen: false,
                window_state: WindowState::Normal,
            },
        );

        if let WindowKind::Popup(state) = window.kind() {
            let weak = Rc::downgrade(&window);
            let parent: Rc<dyn ITopLevelImpl> = parent;
            let helper = ManagedPopupPositionerPopupImplHelper::new(
                parent,
                Rc::new(move |position: PixelPoint, size: Size, _scaling: f64| {
                    if let Some(window) = weak.upgrade() {
                        window.move_(position);
                        window.resize(size, WindowResizeReason::Layout);
                        //TODO: We ignore the scaling override for now
                    }
                }),
            );
            let positioner: Rc<dyn IPopupPositioner> = Rc::new(ManagedPopupPositioner::new(Rc::new(helper)));
            *state.popup_positioner.borrow_mut() = Some(positioner);
        }

        window
    }
}

/// What a popup has beside what every window has.
pub(crate) struct PopupState {
    parent: Rc<dyn IWindowBaseImpl>,
    parent_handle: isize,
    drop_shadow_hint: Cell<bool>,
    is_hit_test_visible: Cell<bool>,
    max_auto_size: Cell<Option<Size>>,
    popup_positioner: RefCell<Option<Rc<dyn IPopupPositioner>>>,
}

impl PopupState {
    pub(crate) fn create_window(&self, atom: u16) -> isize {
        let style = WindowStyles::WS_POPUP | WindowStyles::WS_CLIPSIBLINGS | WindowStyles::WS_CLIPCHILDREN;

        let ex_style = WindowStyles::WS_EX_TOOLWINDOW;

        let result = create_window_ex(
            ex_style.bits(),
            atom,
            style.bits(),
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            self.parent_handle,
        );

        enable_box_shadow(result, self.drop_shadow_hint.get());

        result
    }

    /// The messages a popup answers itself; `None` for the messages the
    /// procedure of a window handles.
    pub(crate) fn wnd_proc(&self, msg: u32) -> Option<isize> {
        match msg {
            WindowsMessage::WM_DISPLAYCHANGE => {
                self.max_auto_size.set(None);
                None
            }
            WindowsMessage::WM_MOUSEACTIVATE => Some(MouseActivate::MA_NOACTIVATE as isize),
            WindowsMessage::WM_NCHITTEST if !self.is_hit_test_visible.get() => Some(HitTestValues::HTTRANSPARENT as isize),
            _ => None,
        }
    }

    pub(crate) fn reset_max_auto_size(&self) {
        self.max_auto_size.set(None);
    }

    /// The largest size a popup that sizes itself may take: the working
    /// area of the screen it is on.
    pub(crate) fn max_auto_size_hint(&self, window: &WindowImpl) -> Size {
        if self.max_auto_size.get().is_none() {
            let screen = window.screen().screen_from_hwnd(window.hwnd(), MONITOR::MONITOR_DEFAULTTONEAREST);

            if let Some(screen) = screen {
                self.max_auto_size.set(Some((*screen).as_ref().working_area().to_rect(window.render_scaling()).size()));
            }
        }

        self.max_auto_size.get().unwrap_or(Size::new(f64::INFINITY, f64::INFINITY))
    }

    pub(crate) fn popup_positioner(&self) -> Option<Rc<dyn IPopupPositioner>> {
        self.popup_positioner.borrow().clone()
    }

    pub(crate) fn set_window_manager_add_shadow_hint(&self, hwnd: isize, enabled: bool) {
        self.drop_shadow_hint.set(enabled);

        enable_box_shadow(hwnd, enabled);
    }

    pub(crate) fn set_hit_test_visible(&self, is_hit_test_visible: bool) {
        self.is_hit_test_visible.set(is_hit_test_visible);
    }

    /// Gives the keyboard focus back to the window the chain of popups
    /// belongs to, if the focus is inside that window.
    pub(crate) fn take_focus(&self) {
        let mut parent: Rc<dyn IWindowBaseImpl> = self.parent.clone();

        loop {
            let next = match parent.as_any().downcast_ref::<WindowImpl>().map(WindowImpl::kind) {
                Some(WindowKind::Popup(popup)) => popup.parent.clone(),
                _ => break,
            };
            parent = next;
        }

        let Some(parent_handle) = parent.handle().map(|handle| handle.handle()) else {
            return;
        };

        let focus_owner = get_focus();
        if focus_owner != 0 && get_ancestor(focus_owner, GA_ROOT) == parent_handle {
            set_focus(parent_handle);
        }
    }
}

fn enable_box_shadow(hwnd: isize, enabled: bool) {
    let mut classes = get_class_long_ptr(hwnd, ClassLongIndex::GCL_STYLE) as u32;

    if enabled {
        classes |= ClassStyles::CS_DROPSHADOW.bits();
    } else {
        classes &= !ClassStyles::CS_DROPSHADOW.bits();
    }

    set_class_long_ptr(hwnd, ClassLongIndex::GCL_STYLE, classes as isize);
}

// A popup is a window implementation, so it has the window contract as
// well as the popup contract; this states what the popup contract adds.
const _: fn(&WindowImpl) -> (&dyn IPopupImpl, &dyn IWindowImpl) = |window| (window, window);
