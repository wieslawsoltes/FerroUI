//! The drop target of a window: turns the calls of OLE during a drag over
//! the window into raw drag events of the framework.
//!
//! The conversions between the effects and the key states of the system
//! and of the framework are functions of values, tested on every host; the
//! target itself (`imp`) is for Windows.

use crate::interop::unmanaged_methods::ModifierKeys;
use crate::win32_com::DropEffect;
use ferroui_base::input::RawInputModifiers;
use ferroui_base::input::DragDropEffects;

/// The effects of the framework as effects of OLE.
pub(crate) fn convert_drop_effect_to_ole(operation: DragDropEffects) -> DropEffect {
    let mut result = DropEffect::None.0;
    if operation.contains(DragDropEffects::COPY) {
        result |= DropEffect::Copy.0;
    }
    if operation.contains(DragDropEffects::MOVE) {
        result |= DropEffect::Move.0;
    }
    if operation.contains(DragDropEffects::LINK) {
        result |= DropEffect::Link.0;
    }
    DropEffect(result)
}

/// The effects of OLE as effects of the framework.
pub(crate) fn convert_drop_effect(effect: DropEffect) -> DragDropEffects {
    let has = |flag: DropEffect| (effect.0 & flag.0) == flag.0;
    let mut result = DragDropEffects::NONE;
    if has(DropEffect::Copy) {
        result |= DragDropEffects::COPY;
    }
    if has(DropEffect::Move) {
        result |= DragDropEffects::MOVE;
    }
    if has(DropEffect::Link) {
        result |= DragDropEffects::LINK;
    }
    result
}

/// The key state of a drag as modifiers of raw input.
pub(crate) fn convert_key_state(grf_key_state: i32) -> RawInputModifiers {
    let mut modifiers = RawInputModifiers::NONE;
    let state = ModifierKeys::from_bits_retain(grf_key_state);

    if state.contains(ModifierKeys::MK_LBUTTON) {
        modifiers |= RawInputModifiers::LEFT_MOUSE_BUTTON;
    }
    if state.contains(ModifierKeys::MK_MBUTTON) {
        modifiers |= RawInputModifiers::MIDDLE_MOUSE_BUTTON;
    }
    if state.contains(ModifierKeys::MK_RBUTTON) {
        modifiers |= RawInputModifiers::RIGHT_MOUSE_BUTTON;
    }
    if state.contains(ModifierKeys::MK_SHIFT) {
        modifiers |= RawInputModifiers::SHIFT;
    }
    if state.contains(ModifierKeys::MK_CONTROL) {
        modifiers |= RawInputModifiers::CONTROL;
    }
    if state.contains(ModifierKeys::MK_ALT) {
        modifiers |= RawInputModifiers::ALT;
    }
    modifiers
}

#[cfg(windows)]
pub(crate) use imp::*;

#[cfg(windows)]
mod imp {
    use super::*;
    use crate::data_transfer_to_ole_data_object_wrapper::DataTransferToOleDataObjectWrapper;
    use crate::interop::unmanaged_methods::POINT;
    use crate::ole_data_object_to_data_transfer_wrapper::OleDataObjectToDataTransferWrapper;
    use crate::win32_com::{IDataObject, IDropTarget, IDropTargetImpl};
    use crate::window_impl::WindowImpl;
    use crate::wnd_proc_guard::guard;
    use ferroui_base::input::platform::PlatformDataTransfer;
    use ferroui_base::input::raw::{IDragDropDevice, RawDragEvent, RawDragEventType};
    use ferroui_base::input::{IDataTransfer, IInputRoot};
    use ferroui_controls::platform::ITopLevelImpl;
    use ferroui_base::reactive::IDisposable;
    use ferroui_base::{PixelPoint, Point};
    use ferroui_microcom::{ComPtr, HResult};
    use std::cell::RefCell;
    use std::rc::{Rc, Weak};

    /// A data transfer for a data object of the system, and the wrapper
    /// that was made for it when the object is not one of this process:
    /// the wrapper keeps a COM reference, so it is disposed by the code
    /// that asked for it.
    pub(crate) struct OleDataTransfer {
        pub data_transfer: Rc<dyn IDataTransfer>,
        pub wrapper: Option<Rc<PlatformDataTransfer>>,
    }

    /// The data transfer of a data object: the one a data object of this
    /// process wraps (`None` when it was released), or a wrapper of the
    /// object.
    pub(crate) fn try_get_data_transfer_from_ole_data_object(p_data_obj: &IDataObject) -> Option<OleDataTransfer> {
        if let Some(data_object) = DataTransferToOleDataObjectWrapper::try_unwrap(p_data_obj) {
            return data_object.data_transfer().map(|data_transfer| OleDataTransfer { data_transfer, wrapper: None });
        }

        let wrapper = OleDataObjectToDataTransferWrapper::new(p_data_obj);
        Some(OleDataTransfer { data_transfer: wrapper.clone(), wrapper: Some(wrapper) })
    }

    /// The drop target of a window.
    pub(crate) struct OleDropTarget {
        target: Rc<dyn IInputRoot>,
        top_level: Weak<WindowImpl>,
        drag_device: Rc<dyn IDragDropDevice>,

        current_drag: RefCell<Option<OleDataTransfer>>,
    }

    impl OleDropTarget {
        /// The COM object of the drop target of a window.
        pub(crate) fn new(
            top_level: Weak<WindowImpl>,
            target: Rc<dyn IInputRoot>,
            drag_device: Rc<dyn IDragDropDevice>,
        ) -> ComPtr<IDropTarget> {
            IDropTarget::from_impl(OleDropTarget { top_level, target, drag_device, current_drag: RefCell::new(None) })
        }

        fn set_data_object(&self, p_data_obj: Option<&IDataObject>) {
            let new_drag = p_data_obj.and_then(try_get_data_transfer_from_ole_data_object);
            let same = match (&*self.current_drag.borrow(), &new_drag) {
                (Some(current), Some(new)) => Rc::ptr_eq(&current.data_transfer, &new.data_transfer),
                (None, None) => true,
                _ => false,
            };
            if !same {
                self.release_data_object();
                *self.current_drag.borrow_mut() = new_drag;
            }
        }

        fn release_data_object(&self) {
            // OleDataObjectToDataTransferWrapper keeps COM reference, so it should be disposed.
            let current_drag = self.current_drag.borrow_mut().take();
            if let Some(OleDataTransfer { wrapper: Some(ole_drag_source), .. }) = current_drag {
                ole_drag_source.dispose();
            }
        }

        fn current_drag(&self) -> Option<Rc<dyn IDataTransfer>> {
            self.current_drag.borrow().as_ref().map(|current| current.data_transfer.clone())
        }

        /// The point of a drag, which the system gives in the coordinates
        /// of the screen, in the coordinates of the window. (The reference
        /// asks the root element, which asks its top-level.)
        fn get_drag_location(&self, top_level: &WindowImpl, drag_point: POINT) -> Point {
            let screen_pt = PixelPoint::new(drag_point.x, drag_point.y);
            top_level.point_to_client_impl(screen_pt)
        }

        /// Sends a drag event of the kind given for the current drag and
        /// returns the effects the framework answered.
        fn dispatch(
            &self,
            type_: RawDragEventType,
            grf_key_state: i32,
            pt: POINT,
            effects: DropEffect,
        ) -> DropEffect {
            let Some(top_level) = self.top_level.upgrade() else {
                return DropEffect::None;
            };
            let (Some(dispatch), Some(current_drag)) = (top_level.input(), self.current_drag()) else {
                return DropEffect::None;
            };

            let args = Rc::new(RawDragEvent::new(
                self.drag_device.clone(),
                type_,
                self.target.clone(),
                self.get_drag_location(&top_level, pt),
                current_drag,
                convert_drop_effect(effects),
                convert_key_state(grf_key_state),
            ));
            dispatch(args.clone());
            convert_drop_effect_to_ole(args.effects())
        }
    }

    impl Drop for OleDropTarget {
        fn drop(&mut self) {
            self.release_data_object();
        }
    }

    /// Reads the effects a caller allows; none for a null pointer.
    fn read_effect(pdw_effect: *mut DropEffect) -> DropEffect {
        if pdw_effect.is_null() {
            return DropEffect::None;
        }
        // SAFETY: a non-null effect of the caller.
        unsafe { pdw_effect.read() }
    }

    fn write_effect(pdw_effect: *mut DropEffect, effect: DropEffect) {
        if !pdw_effect.is_null() {
            // SAFETY: a non-null effect of the caller.
            unsafe { pdw_effect.write(effect) };
        }
    }


    impl IDropTargetImpl for OleDropTarget {
        fn drag_enter(
            &self,
            p_data_obj: Option<&IDataObject>,
            grf_key_state: i32,
            pt: POINT,
            pdw_effect: *mut DropEffect,
        ) -> Result<(), HResult> {
            let allowed = read_effect(pdw_effect);
            let effect = guard(DropEffect::None, || {
                self.set_data_object(p_data_obj);

                // Can happen if the DataTransferToOleDataObjectWrapper was somehow disposed
                self.dispatch(RawDragEventType::DragEnter, grf_key_state, pt, allowed)
            });
            write_effect(pdw_effect, effect);
            Ok(())
        }

        fn drag_over(&self, grf_key_state: i32, pt: POINT, pdw_effect: *mut DropEffect) -> Result<(), HResult> {
            let allowed = read_effect(pdw_effect);
            let effect = guard(DropEffect::None, || self.dispatch(RawDragEventType::DragOver, grf_key_state, pt, allowed));
            write_effect(pdw_effect, effect);
            Ok(())
        }

        fn drag_leave(&self) -> Result<(), HResult> {
            guard((), || {
                self.dispatch(RawDragEventType::DragLeave, 0, POINT::default(), DropEffect::None);
            });
            self.release_data_object();
            Ok(())
        }

        fn drop(
            &self,
            p_data_obj: Option<&IDataObject>,
            grf_key_state: i32,
            pt: POINT,
            pdw_effect: *mut DropEffect,
        ) -> Result<(), HResult> {
            let allowed = read_effect(pdw_effect);
            let effect = guard(DropEffect::None, || {
                self.set_data_object(p_data_obj);

                // Can happen if the DataTransferToOleDataObjectWrapper was somehow disposed
                self.dispatch(RawDragEventType::Drop, grf_key_state, pt, allowed)
            });
            write_effect(pdw_effect, effect);
            self.release_data_object();
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effects_convert_both_ways() {
        let all = DragDropEffects::COPY | DragDropEffects::MOVE | DragDropEffects::LINK;
        assert_eq!(convert_drop_effect_to_ole(DragDropEffects::NONE).0, 0);
        assert_eq!(convert_drop_effect_to_ole(DragDropEffects::COPY).0, 1);
        assert_eq!(convert_drop_effect_to_ole(DragDropEffects::MOVE).0, 2);
        assert_eq!(convert_drop_effect_to_ole(DragDropEffects::LINK).0, 4);
        assert_eq!(convert_drop_effect_to_ole(all).0, 7);
        for bits in 0..8 {
            let effects = DragDropEffects::from_bits_retain(bits);
            assert_eq!(convert_drop_effect(convert_drop_effect_to_ole(effects)), effects);
        }
        // The scroll flag of the system is not an effect of the framework.
        assert_eq!(convert_drop_effect(DropEffect(i32::MIN | 1)), DragDropEffects::COPY);
    }

    #[test]
    fn the_key_state_of_a_drag_becomes_modifiers() {
        assert_eq!(convert_key_state(0), RawInputModifiers::NONE);
        assert_eq!(convert_key_state(0x0001), RawInputModifiers::LEFT_MOUSE_BUTTON);
        assert_eq!(convert_key_state(0x0002), RawInputModifiers::RIGHT_MOUSE_BUTTON);
        assert_eq!(convert_key_state(0x0010), RawInputModifiers::MIDDLE_MOUSE_BUTTON);
        assert_eq!(convert_key_state(0x0004), RawInputModifiers::SHIFT);
        assert_eq!(convert_key_state(0x0008), RawInputModifiers::CONTROL);
        assert_eq!(convert_key_state(0x0020), RawInputModifiers::ALT);
        assert_eq!(
            convert_key_state(0x0001 | 0x0004 | 0x0008),
            RawInputModifiers::LEFT_MOUSE_BUTTON | RawInputModifiers::SHIFT | RawInputModifiers::CONTROL
        );
    }
}
