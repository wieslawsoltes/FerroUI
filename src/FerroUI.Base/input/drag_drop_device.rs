use super::raw::{IDragDropDevice, IRawInputEventArgs, RawDragEvent, RawDragEventType};
use super::{DragDrop, DragDropEffects, DragEventArgs, IDataTransfer, IInputDevice, IInputRoot, KeyModifiers};
use crate::interactivity::{Interactive, RoutedEvent};
use crate::{Point, Ref};
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    static INSTANCE: Rc<DragDropDevice> = Rc::new(DragDropDevice { last_target: RefCell::new(None) });
}

/// The input device that turns the raw drag events of the platform into
/// the routed drag-and-drop events of [`DragDrop`].
///
/// This is an implementation detail of the platform backends.
pub struct DragDropDevice {
    last_target: RefCell<Option<Ref<Interactive>>>,
}

impl DragDropDevice {
    /// The drag-and-drop device of the current thread.
    pub fn instance() -> Rc<DragDropDevice> {
        INSTANCE.with(Rc::clone)
    }

    fn last_target(&self) -> Option<Ref<Interactive>> {
        self.last_target.borrow().clone()
    }

    fn set_last_target(&self, value: Option<Ref<Interactive>>) {
        let old = self.last_target.replace(value);
        drop(old);
    }

    fn get_target(root: &Rc<dyn IInputRoot>, local: Point) -> Option<Ref<Interactive>> {
        let hit = root.root_element().input_hit_test(local)?;
        let target = hit.get_self_and_visual_ancestors().find_map(|visual| visual.downcast::<Interactive>().ok())?;

        if DragDrop::get_allow_drop(&target) {
            Some(target)
        } else {
            None
        }
    }

    fn raise_drag_event(
        target: Option<&Ref<Interactive>>,
        input_root: &Rc<dyn IInputRoot>,
        point: Point,
        routed_event: &RoutedEvent<DragEventArgs>,
        operation: DragDropEffects,
        data_transfer: &Rc<dyn IDataTransfer>,
        modifiers: KeyModifiers,
    ) -> DragDropEffects {
        let Some(target) = target else {
            return DragDropEffects::NONE;
        };

        let Some(p) = input_root.root_element().translate_point(point, target) else {
            return DragDropEffects::NONE;
        };

        let args = DragEventArgs::new(Some(routed_event), data_transfer.clone(), target.clone(), p, modifiers);
        args.set_drag_effects(operation);

        target.raise_event(&args);
        args.drag_effects()
    }

    fn drag_enter(
        &self,
        input_root: &Rc<dyn IInputRoot>,
        point: Point,
        data: &Rc<dyn IDataTransfer>,
        effects: DragDropEffects,
        modifiers: KeyModifiers,
    ) -> DragDropEffects {
        let target = Self::get_target(input_root, point);
        self.set_last_target(target.clone());
        Self::raise_drag_event(
            target.as_ref(),
            input_root,
            point,
            DragDrop::drag_enter_event(),
            effects,
            data,
            modifiers,
        )
    }

    fn drag_over(
        &self,
        input_root: &Rc<dyn IInputRoot>,
        point: Point,
        data: &Rc<dyn IDataTransfer>,
        effects: DragDropEffects,
        modifiers: KeyModifiers,
    ) -> DragDropEffects {
        let target = Self::get_target(input_root, point);
        let last_target = self.last_target();

        if target == last_target {
            return Self::raise_drag_event(
                target.as_ref(),
                input_root,
                point,
                DragDrop::drag_over_event(),
                effects,
                data,
                modifiers,
            );
        }

        // The last target is updated even when a handler panics.
        let _finally = Finally(|| self.set_last_target(target.clone()));

        if last_target.is_some() {
            Self::raise_drag_event(
                last_target.as_ref(),
                input_root,
                point,
                DragDrop::drag_leave_event(),
                effects,
                data,
                modifiers,
            );
        }

        Self::raise_drag_event(
            target.as_ref(),
            input_root,
            point,
            DragDrop::drag_enter_event(),
            effects,
            data,
            modifiers,
        )
    }

    fn drag_leave(
        &self,
        input_root: &Rc<dyn IInputRoot>,
        point: Point,
        data: &Rc<dyn IDataTransfer>,
        effects: DragDropEffects,
        modifiers: KeyModifiers,
    ) {
        let last_target = self.last_target();
        if last_target.is_none() {
            return;
        }

        let _finally = Finally(|| self.set_last_target(None));

        Self::raise_drag_event(
            last_target.as_ref(),
            input_root,
            point,
            DragDrop::drag_leave_event(),
            effects,
            data,
            modifiers,
        );
    }

    fn drop(
        &self,
        input_root: &Rc<dyn IInputRoot>,
        point: Point,
        data: &Rc<dyn IDataTransfer>,
        effects: DragDropEffects,
        modifiers: KeyModifiers,
    ) -> DragDropEffects {
        let last_target = self.last_target();
        let _finally = Finally(|| self.set_last_target(None));

        Self::raise_drag_event(last_target.as_ref(), input_root, point, DragDrop::drop_event(), effects, data, modifiers)
    }

    fn process_raw_drag_event(&self, e: &RawDragEvent) {
        let (root, location, data, effects, modifiers) =
            (e.root(), e.location(), e.data_transfer(), e.effects(), e.key_modifiers());

        match e.type_() {
            RawDragEventType::DragEnter => e.set_effects(self.drag_enter(root, location, data, effects, modifiers)),
            RawDragEventType::DragOver => e.set_effects(self.drag_over(root, location, data, effects, modifiers)),
            RawDragEventType::DragLeave => self.drag_leave(root, location, data, effects, modifiers),
            RawDragEventType::Drop => e.set_effects(self.drop(root, location, data, effects, modifiers)),
        }
    }
}

/// Runs a closure when dropped: the `finally` block of the reference
/// implementation.
struct Finally<F: FnMut()>(F);

impl<F: FnMut()> Drop for Finally<F> {
    fn drop(&mut self) {
        (self.0)()
    }
}

impl IInputDevice for DragDropDevice {
    fn process_raw_event(&self, e: &dyn IRawInputEventArgs) {
        if !e.handled() {
            if let Some(margs) = e.downcast_ref::<RawDragEvent>() {
                self.process_raw_drag_event(margs);
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IDragDropDevice for DragDropDevice {}
