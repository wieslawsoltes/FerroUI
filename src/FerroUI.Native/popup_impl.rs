use crate::helpers::ComResultExt;
use crate::interop::*;
use crate::top_level_impl::{
    top_level_of, impl_top_level_contract, MacOSTopLevelHandle, TopLevelEvents, TopLevelImpl, TopLevelParent,
};
use crate::window_impl::WindowImpl;
use crate::window_impl_base::{impl_window_base_contract, WindowBaseImpl, WindowBaseParent, WindowEventsParent};
use ferroui_base::{PixelPoint, Size};
use ferroui_controls::platform::{IPopupImpl, ITopLevelImpl, IWindowBaseImpl, PlatformThemeVariant};
use ferroui_controls::primitives::popup_positioning::{
    IPopupPositioner, ManagedPopupPositioner, ManagedPopupPositionerPopupImplHelper,
};
use ferroui_controls::WindowResizeReason;
use ferroui_microcom::ComPtr;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// A macOS popup window.
pub struct PopupImpl {
    weak_self: Weak<PopupImpl>,
    base: WindowBaseImpl,
    parent: Rc<dyn ITopLevelImpl>,
    /// The native popup; `None` until it is created and once it is
    /// disposed.
    native: RefCell<Option<ComPtr<IFrnPopup>>>,
    popup_positioner: RefCell<Option<Rc<dyn IPopupPositioner>>>,
}

/// Walks up from a popup's parent to the top-level that is not a popup.
fn root_parent(mut parent: Rc<dyn ITopLevelImpl>) -> Rc<dyn ITopLevelImpl> {
    loop {
        let next = match parent.as_any().downcast_ref::<PopupImpl>() {
            Some(popup) => popup.parent.clone(),
            None => return parent,
        };
        parent = next;
    }
}

impl PopupImpl {
    pub(crate) fn new(factory: ComPtr<IFerroNativeFactory>, parent: Rc<dyn ITopLevelImpl>) -> Rc<PopupImpl> {
        let this = Rc::new_cyclic(|weak_self| PopupImpl {
            weak_self: weak_self.clone(),
            base: WindowBaseImpl::new(factory.clone()),
            parent: parent.clone(),
            native: RefCell::new(None),
            popup_positioner: RefCell::new(None),
        });

        let e = IFrnWindowEvents::from_impl(TopLevelEvents(this.clone()));
        let native = factory.create_popup(Some(&e)).check().expect("the native popup");
        *this.native.borrow_mut() = Some(native.clone());
        this.base.init(MacOSTopLevelHandle::from_window_base(ComPtr::<IFrnWindowBase>::from_ref(&native)));

        let weak = Rc::downgrade(&this);
        let positioner_helper = ManagedPopupPositionerPopupImplHelper::new(
            parent.clone(),
            Rc::new(move |position, size, scaling| {
                if let Some(this) = weak.upgrade() {
                    this.move_resize(position, size, scaling);
                }
            }),
        );
        let positioner: Rc<dyn IPopupPositioner> = Rc::new(ManagedPopupPositioner::new(Rc::new(positioner_helper)));
        *this.popup_positioner.borrow_mut() = Some(positioner);

        let parent = root_parent(parent);

        if let Some(window) = parent.as_any().downcast_ref::<WindowImpl>() {
            let parent_native = window.window_base().native();
            native.set_parent(parent_native.as_deref()).check();
        }

        // Use the parent's input context to process events
        if let Some(parent_top_level) = top_level_of(&*parent) {
            this.top_level().set_input_method(parent_top_level.input_method());
        }

        this
    }

    pub(crate) fn top_level(&self) -> &Rc<TopLevelImpl> {
        self.base.top_level()
    }

    fn move_resize(&self, position: PixelPoint, size: Size, _scaling: f64) {
        self.base.set_position(position);
        self.base.resize(size, WindowResizeReason::Layout);
    }

    /// The native popup; `None` once the popup is disposed.
    pub fn native(&self) -> Option<ComPtr<IFrnPopup>> {
        self.native.borrow().clone()
    }
}

impl TopLevelParent for PopupImpl {
    fn top_level(&self) -> &Rc<TopLevelImpl> {
        self.base.top_level()
    }

    fn dispose_top_level(&self) {
        if let Some(native) = self.base.native() {
            native.set_parent(None).check();
        }

        self.base.dispose();
        let native = self.native.borrow_mut().take();
        drop(native);
    }

    fn create_popup_core(&self) -> Option<Rc<dyn IPopupImpl>> {
        let this: Rc<dyn ITopLevelImpl> = self.weak_self.upgrade()?;
        Some(PopupImpl::new(self.top_level().factory().clone(), this))
    }

    fn set_frame_theme_variant_core(&self, theme_variant: Option<PlatformThemeVariant>) {
        self.base.set_frame_theme_variant(theme_variant);
    }
}

impl WindowBaseParent for PopupImpl {
    fn window_base(&self) -> &WindowBaseImpl {
        &self.base
    }
}

impl WindowEventsParent for PopupImpl {
    fn on_closing(&self) -> bool {
        true
    }

    fn on_window_state_changed(&self, _state: FrnWindowState) {}

    fn on_got_input_when_disabled(&self) {
        // NOP on Popup
    }
}

impl_top_level_contract!(PopupImpl {
    fn as_window_base_impl(&self) -> Option<&dyn IWindowBaseImpl> {
        Some(self)
    }

    fn as_popup_impl(&self) -> Option<&dyn IPopupImpl> {
        Some(self)
    }
});

impl_window_base_contract!(PopupImpl);

impl IPopupImpl for PopupImpl {
    fn popup_positioner(&self) -> Option<Rc<dyn IPopupPositioner>> {
        self.popup_positioner.borrow().clone()
    }

    fn set_window_manager_add_shadow_hint(&self, _enabled: bool) {}

    fn take_focus(&self) {
        let parent = root_parent(self.parent.clone());

        if let Some(window) = parent.as_any().downcast_ref::<WindowImpl>() {
            if let Some(native) = window.native() {
                native.take_focus_from_children().check();
            }
        }
    }

    fn set_hit_test_visible(&self, is_hit_test_visible: bool) {
        if let Some(native) = self.native() {
            native.set_hit_test_visible(is_hit_test_visible).check();
        }
    }
}
