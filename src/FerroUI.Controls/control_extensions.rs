use crate::{Control, RequestBringIntoViewEventArgs};
use ferroui_base::controls::NameScopeExtensions;
use ferroui_base::layout::{BringIntoViewRequest, Layoutable};
use ferroui_base::reactive::{AnonymousObserver, IDisposable, IObservable};
use ferroui_base::{ObjectType, Rect, Ref};
use std::rc::Rc;

impl Control {
    /// Tries to bring the control into view.
    pub fn bring_into_view(&self) {
        self.bring_into_view_core(None);
    }

    /// Tries to bring the control into view.
    ///
    /// `rect` is the area of the control to being into view.
    pub fn bring_into_view_rect(&self, rect: Rect) {
        self.bring_into_view_core(Some(rect));
    }

    fn bring_into_view_core(&self, rect: Option<Rect>) {
        if self.try_execute_bring_into_view(rect) {
            return;
        }

        if let Some(layout_root) = self.get_layout_root() {
            let layout_manager = layout_root.layout_manager();
            if let Some(layout_manager) = layout_manager.as_bring_into_view_layout_manager() {
                layout_manager
                    .enqueue_bring_into_view(Rc::new(ControlBringIntoViewRequest { target: self.to_ref(), rect }));
            }
        }
    }

    fn try_execute_bring_into_view(&self, rect: Option<Rect>) -> bool {
        if !self.is_effectively_visible() {
            return true; // BringIntoView on an invisible control just does nothing.
        }

        if !self.is_measure_valid() || !self.is_arrange_valid() {
            return false;
        }

        let mut ev = RequestBringIntoViewEventArgs::with_event(Control::request_bring_into_view_event());
        ev.target_object = Some(self.to_ref().upcast());
        ev.set_target_rect(rect.unwrap_or_else(|| Rect::from_size(self.bounds().size())));

        self.raise_event(&ev);
        true
    }

    /// Finds the named control in the scope of the specified control.
    ///
    /// Returns the control or `None` if not found; panics if the control has
    /// no parent name scope or the named control is of a different class.
    pub fn find_control<T: ObjectType>(&self, name: &str) -> Option<Ref<T>> {
        match NameScopeExtensions::find_name_scope(self) {
            Some(name_scope) => name_scope.find_as::<T>(name),
            None => panic!("Could not find parent name scope."),
        }
    }

    /// Gets the named control in the scope of the specified control.
    ///
    /// Panics if the control has no parent name scope or the named control
    /// is not found.
    pub fn get_control<T: ObjectType>(&self, name: &str) -> Ref<T> {
        match self.find_control::<T>(name) {
            Some(control) => control,
            None => panic!("Could not find control named '{name}'."),
        }
    }

    /// Sets a pseudoclass of the control depending on an observable trigger.
    ///
    /// Returns a disposable used to cancel the subscription.
    pub fn set_pseudo_class_from(&self, name: &str, trigger: &dyn IObservable<bool>) -> Rc<dyn IDisposable> {
        let name = name.to_string();
        let weak = self.to_ref().downgrade();
        trigger.subscribe(Rc::new(AnonymousObserver::new(move |x: bool| {
            if let Some(this) = weak.upgrade() {
                this.pseudo_classes().set(&name, x);
            }
        })))
    }
}

struct ControlBringIntoViewRequest {
    target: Ref<Control>,
    rect: Option<Rect>,
}

impl BringIntoViewRequest for ControlBringIntoViewRequest {
    fn target(&self) -> Ref<Layoutable> {
        self.target.clone().upcast()
    }

    fn try_execute(&self) -> bool {
        self.target.try_execute_bring_into_view(self.rect)
    }
}
