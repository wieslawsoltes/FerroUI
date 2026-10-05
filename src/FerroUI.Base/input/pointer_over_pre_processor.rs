use super::raw::{IRawInputEventArgs, RawDragEvent, RawPointerEventArgs, RawPointerEventType};
use super::{
    IInputDevice, IInputRoot, IPointer, InputElement, KeyModifiers, PointerEventArgs, PointerPointProperties,
    PointerType,
};
use crate::reactive::IObserver;
use crate::{PixelPoint, Point, Rect, Ref};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Tracks which element the pointer is over for one input root, raising
/// the pointer entered and exited events as it changes.
///
/// It observes the raw input of the input manager (subscribe it to
/// [`IInputManager::pre_process`](super::IInputManager::pre_process)) and is
/// told by its host when the scene changed under a resting pointer.
pub struct PointerOverPreProcessor {
    last_active_pointer_device: RefCell<Option<Rc<dyn IInputDevice>>>,
    current_pointer: RefCell<Option<(Rc<dyn IPointer>, PixelPoint)>>,
    last_known_position: Cell<Option<PixelPoint>>,
    input_root: Rc<dyn IInputRoot>,
}

impl PointerOverPreProcessor {
    /// Creates the pre-processor for an input root.
    pub fn new(input_root: Rc<dyn IInputRoot>) -> Self {
        Self {
            last_active_pointer_device: RefCell::new(None),
            current_pointer: RefCell::new(None),
            last_known_position: Cell::new(None),
            input_root,
        }
    }

    /// The last known position of the pointer, in screen coordinates.
    pub fn last_position(&self) -> Option<PixelPoint> {
        self.last_known_position.get()
    }

    fn take_current_pointer(&self) -> Option<(Rc<dyn IPointer>, PixelPoint)> {
        self.current_pointer.borrow_mut().take()
    }

    fn current_pointer(&self) -> Option<(Rc<dyn IPointer>, PixelPoint)> {
        self.current_pointer.borrow().clone()
    }

    fn set_current_pointer(&self, value: Option<(Rc<dyn IPointer>, PixelPoint)>) {
        let old = self.current_pointer.replace(value);
        drop(old);
    }

    fn set_last_active_pointer_device(&self, value: Option<Rc<dyn IInputDevice>>) {
        let old = self.last_active_pointer_device.replace(value);
        drop(old);
    }

    fn is_this_root(&self, root: &Rc<dyn IInputRoot>) -> bool {
        std::ptr::addr_eq(Rc::as_ptr(root), Rc::as_ptr(&self.input_root))
    }

    fn process(&self, value: &dyn IRawInputEventArgs) {
        if let Some(drag_args) = value.downcast_ref::<RawDragEvent>() {
            // When a platform drag operation is in progress, the application
            // does not receive pointer move events until after the drop
            // event. This is a problem because if a popup is shown at the
            // pointer position in the drop event, it will be shown at the
            // position at which the drag was initiated, not the position at
            // which the drop occurred.
            //
            // Solve this by updating the last known pointer position when a
            // drag event occurs.
            self.last_known_position
                .set(Some(self.input_root.root_element().point_to_screen(drag_args.location())));
            return;
        }

        let Some(args) = value.downcast_ref::<RawPointerEventArgs>() else { return };

        if !self.is_this_root(args.root()) {
            return;
        }

        let device = value.device();
        let Some(pointer_device) = device.as_pointer_device() else { return };

        let is_last_active = self
            .last_active_pointer_device
            .borrow()
            .as_ref()
            .is_some_and(|last| std::ptr::addr_eq(Rc::as_ptr(last), Rc::as_ptr(device)));

        if !is_last_active {
            self.clear_pointer_over();

            // Set the last active device before processing input, because
            // clearing the pointer-over might be called and clear the last
            // device.
            self.set_last_active_pointer_device(Some(device.clone()));
        }

        let type_ = args.type_();
        let root = args.root();

        if matches!(
            type_,
            RawPointerEventType::LeaveWindow
                | RawPointerEventType::NonClientLeftButtonDown
                | RawPointerEventType::TouchCancel
                | RawPointerEventType::TouchEnd
        ) {
            if let Some((last_pointer, last_position)) = self.take_current_pointer() {
                self.clear_pointer_over_for(
                    &last_pointer,
                    root,
                    0,
                    Some(Self::point_to_client(root, last_position)),
                    PointerPointProperties::new(args.input_modifiers(), type_.to_update_kind()),
                    args.input_modifiers().to_key_modifiers(),
                );
            }
        } else if matches!(type_, RawPointerEventType::TouchBegin | RawPointerEventType::TouchUpdate) {
            self.last_known_position.set(Some(root.root_element().point_to_screen(args.position())));
        } else if let Some(pointer) = pointer_device.try_get_pointer(args) {
            if pointer.type_() != PointerType::Touch && type_ != RawPointerEventType::CancelCapture {
                let element =
                    Self::get_effective_pointer_over_element(args.input_hit_test_result().1, pointer.captured());

                self.set_pointer_over(
                    &pointer,
                    root,
                    element,
                    args.timestamp(),
                    args.position(),
                    PointerPointProperties::new(args.input_modifiers(), type_.to_update_kind()),
                    args.input_modifiers().to_key_modifiers(),
                );
            }
        }
    }

    /// Called by the host when the part of the scene within `dirty_rect`
    /// changed: re-evaluates which element a resting pointer is over.
    pub fn scene_invalidated(&self, dirty_rect: Rect) {
        let Some((pointer, position)) = self.current_pointer() else { return };

        let root_element = self.input_root.root_element();
        let client_point = Self::point_to_client(&self.input_root, position);

        if dirty_rect.contains(client_point) {
            let element = Self::get_effective_pointer_over_element(
                root_element.input_hit_test(client_point),
                pointer.captured(),
            );

            self.set_pointer_over(
                &pointer,
                &self.input_root,
                element,
                0,
                client_point,
                PointerPointProperties::NONE,
                KeyModifiers::NONE,
            );
        } else if !root_element.bounds().contains(client_point) {
            self.clear_pointer_over_for(
                &pointer,
                &self.input_root,
                0,
                Some(client_point),
                PointerPointProperties::NONE,
                KeyModifiers::NONE,
            );
        }
    }

    fn get_effective_pointer_over_element(
        hit_test_element: Option<Ref<InputElement>>,
        captured: Option<Ref<InputElement>>,
    ) -> Option<Ref<InputElement>> {
        if captured.is_some() && hit_test_element != captured {
            None
        } else {
            hit_test_element
        }
    }

    fn clear_pointer_over(&self) {
        if let Some((pointer, position)) = self.current_pointer() {
            let client_point = Self::point_to_client(&self.input_root, position);
            self.clear_pointer_over_for(
                &pointer,
                &self.input_root,
                0,
                Some(client_point),
                PointerPointProperties::NONE,
                KeyModifiers::NONE,
            );
        }

        self.set_current_pointer(None);
        self.set_last_active_pointer_device(None);
    }

    fn clear_pointer_over_for(
        &self,
        pointer: &Rc<dyn IPointer>,
        root: &Rc<dyn IInputRoot>,
        timestamp: u64,
        position: Option<Point>,
        properties: PointerPointProperties,
        input_modifiers: KeyModifiers,
    ) {
        let Some(element) = root.pointer_over_element() else { return };

        let root_element = root.root_element();

        // Do not pass the root visual when we have an unknown position, so
        // `get_position` won't return invalid values.
        let e = PointerEventArgs::new(
            Some(InputElement::pointer_exited_event()),
            &element,
            pointer.clone(),
            position.map(|_| &***root_element as &crate::Visual),
            position.unwrap_or_default(),
            timestamp,
            properties,
            input_modifiers,
        );

        if !element.is_attached_to_visual_tree() {
            // The element has been removed from the visual tree so do a top
            // down cleanup.
            if root_element.is_pointer_over() {
                Self::clear_children_pointer_over(&e, &root_element, true);
            }
        }

        let mut current = Some(element);

        while let Some(element) = current {
            e.set_source(&element);
            e.set_handled(false);
            element.raise_event(&e);
            current = Self::get_visual_parent(&element);
        }

        root.set_pointer_over_element(None);
        root.set_cursor_element(pointer.captured());

        self.set_last_active_pointer_device(None);
        self.set_current_pointer(None);
    }

    fn clear_children_pointer_over(e: &PointerEventArgs, element: &Ref<InputElement>, clear_root: bool) {
        if let Some(children) = element.visual_children_snapshot() {
            for el in children.iter() {
                if let Some(child) = el.cast::<InputElement>() {
                    if child.is_pointer_over() {
                        Self::clear_children_pointer_over(e, &child, true);
                        break;
                    }
                }
            }
        }

        if clear_root {
            e.set_source(element);
            e.set_handled(false);
            element.raise_event(e);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn set_pointer_over(
        &self,
        pointer: &Rc<dyn IPointer>,
        root: &Rc<dyn IInputRoot>,
        element: Option<Ref<InputElement>>,
        timestamp: u64,
        position: Point,
        properties: PointerPointProperties,
        input_modifiers: KeyModifiers,
    ) {
        let pointer_over_element = root.pointer_over_element();
        let screen_position = root.root_element().point_to_screen(position);
        self.last_known_position.set(Some(screen_position));

        if element != pointer_over_element {
            match &element {
                Some(element) => self.set_pointer_over_to_element(
                    pointer,
                    root,
                    element,
                    timestamp,
                    position,
                    properties,
                    input_modifiers,
                ),
                None => self.clear_pointer_over_for(
                    pointer,
                    root,
                    timestamp,
                    Some(position),
                    properties,
                    input_modifiers,
                ),
            }
        }

        self.set_current_pointer(Some((pointer.clone(), screen_position)));
    }

    #[allow(clippy::too_many_arguments)]
    fn set_pointer_over_to_element(
        &self,
        pointer: &Rc<dyn IPointer>,
        root: &Rc<dyn IInputRoot>,
        element: &Ref<InputElement>,
        timestamp: u64,
        position: Point,
        properties: PointerPointProperties,
        input_modifiers: KeyModifiers,
    ) {
        let mut branch: Option<Ref<InputElement>> = None;
        let mut el = Some(element.clone());

        while let Some(current) = el {
            if current.is_pointer_over() {
                branch = Some(current);
                break;
            }
            el = Self::get_visual_parent(&current);
        }

        el = root.pointer_over_element();

        let root_element = root.root_element();
        let e = PointerEventArgs::new(
            Some(InputElement::pointer_exited_event()),
            el.clone().map(Ref::upcast::<crate::FerroObject>),
            pointer.clone(),
            Some(&root_element),
            position,
            timestamp,
            properties,
            input_modifiers,
        );

        if let (Some(old), Some(branch)) = (&el, &branch) {
            if !old.is_attached_to_visual_tree() {
                Self::clear_children_pointer_over(&e, branch, false);
            }
        }

        while let Some(current) = el {
            if Some(&current) == branch.as_ref() {
                break;
            }
            e.set_source(&current);
            e.set_handled(false);
            current.raise_event(&e);
            el = Self::get_visual_parent(&current);
        }

        root.set_pointer_over_element(Some(element.clone()));
        el = Some(element.clone());
        root.set_cursor_element(pointer.captured().or_else(|| Some(element.clone())));

        e.set_routed_event(Some(InputElement::pointer_entered_event()));

        while let Some(current) = el {
            if Some(&current) == branch.as_ref() {
                break;
            }
            e.set_source(&current);
            e.set_handled(false);
            current.raise_event(&e);
            el = Self::get_visual_parent(&current);
        }
    }

    fn get_visual_parent(e: &InputElement) -> Option<Ref<InputElement>> {
        e.visual_parent().and_then(|parent| parent.downcast::<InputElement>().ok())
    }

    fn point_to_client(root: &Rc<dyn IInputRoot>, p: PixelPoint) -> Point {
        root.root_element().point_to_client(p)
    }
}

impl IObserver<Rc<dyn IRawInputEventArgs>> for PointerOverPreProcessor {
    fn on_next(&self, value: Rc<dyn IRawInputEventArgs>) {
        self.process(&*value);
    }

    fn on_completed(&self) {
        self.clear_pointer_over();
    }
}
