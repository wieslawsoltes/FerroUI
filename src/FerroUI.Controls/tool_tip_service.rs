use crate::tool_tip::{is_tool_tip, same_value};
use crate::{Control, IToolTipService, ToolTip};
use ferroui_base::input::raw::{IRawInputEventArgs, RawPointerEventArgs, RawPointerEventType};
use ferroui_base::input::{IInputManager, IInputRoot, InputElement, PointerEventArgs};
use ferroui_base::reactive::{AnonymousObserver, CompositeDisposable, IDisposable};
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
use ferroui_base::{BoxedValue, FerroPropertyChangedEventArgs, Ref, Visual, WeakRef};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::Duration;

/// Handles [`ToolTip`] interaction with controls (internal upstream).
pub struct ToolTipService {
    this: Weak<ToolTipService>,
    subscriptions: CompositeDisposable,

    /// The control whose tooltip is shown or about to be shown. The service
    /// does not keep the control alive.
    tip_control: RefCell<Option<WeakRef<Control>>>,
    pointer_over_tip: Cell<bool>,
    pointer_over_owner: Cell<bool>,
    pending_close_id: Cell<i32>,
    /// The time the last tooltip closed, on the clock of the dispatcher (in
    /// milliseconds); `None` until a tooltip has closed.
    last_tip_close_time: Cell<Option<i64>>,
    timer: RefCell<Option<Rc<DispatcherTimer>>>,
}

impl ToolTipService {
    /// Creates the service over the input manager whose raw input it
    /// follows.
    pub fn new(input_manager: &Rc<dyn IInputManager>) -> Rc<ToolTipService> {
        let service = Rc::new_cyclic(|this: &Weak<ToolTipService>| ToolTipService {
            this: this.clone(),
            subscriptions: CompositeDisposable::new(),
            tip_control: RefCell::new(None),
            pointer_over_tip: Cell::new(false),
            pointer_over_owner: Cell::new(false),
            pending_close_id: Cell::new(0),
            last_tip_close_time: Cell::new(None),
            timer: RefCell::new(None),
        });

        let weak = Rc::downgrade(&service);
        service.subscriptions.add(input_manager.process().subscribe(Rc::new(AnonymousObserver::new({
            let weak = weak.clone();
            move |e: Rc<dyn IRawInputEventArgs>| {
                if let Some(this) = weak.upgrade() {
                    this.input_manager_on_process(&*e);
                }
            }
        }))));
        service.subscriptions.add(ToolTip::service_enabled_property().changed().subscribe({
            let weak = weak.clone();
            move |args| {
                if let Some(this) = weak.upgrade() {
                    this.service_enabled_changed(args);
                }
            }
        }));
        service.subscriptions.add(ToolTip::tip_property().changed().subscribe(move |e| {
            if let Some(this) = weak.upgrade() {
                this.tip_changed(e);
            }
        }));

        service
    }

    /// Stops the service: its timer, its pending close and its
    /// subscriptions.
    pub fn dispose(&self) {
        self.stop_timer();
        self.cancel_pending_close();
        self.subscriptions.dispose();
    }

    fn tip_control(&self) -> Option<Ref<Control>> {
        self.tip_control.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    fn set_tip_control(&self, value: Option<&Ref<Control>>) {
        *self.tip_control.borrow_mut() = value.map(Ref::downgrade);
    }

    fn input_manager_on_process(&self, e: &dyn IRawInputEventArgs) {
        let Some(pointer_event) = e.downcast_ref::<RawPointerEventArgs>() else { return };

        let tip_control = self.tip_control();
        let tip_visual_root = tip_control.as_ref().and_then(|control| control.visual_root());
        let current_tip = tip_control.as_ref().and_then(|control| control.get_value(ToolTip::tool_tip_property()));

        // The input root of the tip itself, for popup windows only, not overlays.
        let tip_popup_input_root = current_tip
            .and_then(|tip| tip.popup_host())
            .and_then(|host| host.as_control().get_input_root())
            .filter(|root| root_element_visual(root) != tip_visual_root);

        let is_tip_event = tip_popup_input_root.as_ref().is_some_and(|root| same_input_root(e.root(), root));
        let is_tip_owner_window_event = root_element_visual(e.root()) == tip_visual_root;

        if is_tip_event || is_tip_owner_window_event {
            // The pointer is on one of the two windows (tip or owner) involved: remember which one, and cancel any
            // close scheduled by a previous LeaveWindow on the other one.
            if pointer_event.type_() != RawPointerEventType::LeaveWindow {
                self.pointer_over_tip.set(is_tip_event);
                self.pointer_over_owner.set(is_tip_owner_window_event);
                self.cancel_pending_close();
            } else if is_tip_event {
                self.pointer_over_tip.set(false);
            } else {
                self.pointer_over_owner.set(false);
            }
        }

        match pointer_event.type_() {
            RawPointerEventType::Move => {
                let candidate = pointer_event.input_hit_test_result().0.map(|element| element.upcast::<Visual>());
                IToolTipService::update(self, pointer_event.root(), candidate);
            }

            RawPointerEventType::LeaveWindow => {
                if (is_tip_event && !self.pointer_over_owner.get())
                    || (is_tip_owner_window_event && !self.pointer_over_tip.get())
                {
                    if tip_popup_input_root.is_some() {
                        // The pointer is leaving one of the two windows, but there's a chance it's going to
                        // the other one. Schedule a close and cancel it if we receive another event.
                        self.schedule_pending_close();
                    } else {
                        self.close_current_tip();
                    }
                }
            }

            RawPointerEventType::LeftButtonDown
            | RawPointerEventType::RightButtonDown
            | RawPointerEventType::MiddleButtonDown
            | RawPointerEventType::XButton1Down
            | RawPointerEventType::XButton2Down => {
                // clear the tip
                self.stop_timer();
                if let Some(tip_control) = self.tip_control() {
                    tip_control.clear_value(ToolTip::is_open_property());
                }
                self.pointer_over_tip.set(false);
            }

            _ => {}
        }
    }

    fn schedule_pending_close(&self) {
        let control = self.tip_control();
        let close_id = self.pending_close_id.get().wrapping_add(1);
        self.pending_close_id.set(close_id);

        // Post below input priority: any pointer event is processed before this callback and gets a chance to cancel it.
        let weak = self.this.clone();
        Dispatcher::ui_thread().post_local(
            move || {
                let Some(this) = weak.upgrade() else { return };
                if this.pending_close_id.get() == close_id && this.tip_control() == control {
                    this.close_current_tip();
                }
            },
            DispatcherPriority::BACKGROUND,
        );
    }

    fn cancel_pending_close(&self) {
        self.pending_close_id.set(self.pending_close_id.get().wrapping_add(1));
    }

    fn close_current_tip(&self) {
        self.stop_timer();
        if let Some(tip_control) = self.tip_control() {
            tip_control.clear_value(ToolTip::is_open_property());
        }
        self.set_tip_control(None);
        self.pointer_over_tip.set(false);
        self.pointer_over_owner.set(false);
    }

    fn service_enabled_changed(&self, args: &FerroPropertyChangedEventArgs<'_>) {
        let Some(tip_control) = self.tip_control() else { return };
        let sender = args.sender().downcast_ref::<Control>();

        if sender.is_some_and(|sender| sender.to_ref() == tip_control) && !ToolTip::get_service_enabled(&tip_control) {
            self.stop_timer();
        }
    }

    /// Called when the `Tip` property changes on a control.
    fn tip_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let control = e.sender().downcast_ref::<Control>().expect("the tooltip properties are attached to controls");
        let (old_value, new_value) = e.get_old_and_new_value::<Option<BoxedValue>>();

        let changed = match (&old_value, &new_value) {
            (None, None) => false,
            (Some(old_value), Some(new_value)) => !same_value(old_value, new_value),
            _ => true,
        };

        if ToolTip::get_is_open(control) && changed && !new_value.as_ref().is_some_and(is_tool_tip) {
            match new_value {
                None => self.close(control),
                Some(new_value) => {
                    if let Some(tip) = control.get_value(ToolTip::tool_tip_property()) {
                        tip.set_content(Some(new_value));
                    }
                }
            }
        }
    }

    fn on_tip_control_changed(&self, old_value: Option<&Ref<Control>>, new_value: Option<&Ref<Control>>) {
        self.stop_timer();

        let mut closed_previous_tip = false; // avoid race conditions by remembering whether we closed a tooltip in the current call.

        if let Some(old_value) = old_value {
            if ToolTip::get_is_open(old_value) {
                self.close(old_value);
                closed_previous_tip = true;
            }
        }

        if let Some(new_value) = new_value {
            if !ToolTip::get_is_open(new_value) {
                let between_show_delay = ToolTip::get_between_show_delay(new_value);

                let closed_recently = || {
                    self.last_tip_close_time.get().is_some_and(|last_tip_close_time| {
                        Dispatcher::ui_thread().now().wrapping_sub(last_tip_close_time) <= i64::from(between_show_delay)
                    })
                };

                let show_delay = if between_show_delay >= 0 && (closed_previous_tip || closed_recently()) {
                    0
                } else {
                    ToolTip::get_show_delay(new_value)
                };

                if show_delay == 0 {
                    self.open(new_value);
                } else {
                    self.start_show_timer(show_delay, new_value);
                }
            }
        }
    }

    fn tool_tip_closed(&self) {
        self.last_tip_close_time.set(Some(Dispatcher::ui_thread().now()));
    }

    fn tool_tip_pointer_exited(&self, tool_tip: &ToolTip) {
        // The pointer has exited the tooltip. Close the tooltip unless the current tooltip source is still the
        // adorned control.
        if let Some(control) = tool_tip.adorned_control() {
            if Some(&control) != self.tip_control().as_ref() {
                self.close(&control);
            }
        }
    }

    fn start_show_timer(&self, show_delay: i32, control: &Ref<Control>) {
        let Ok(show_delay) = u64::try_from(show_delay) else {
            panic!("The show delay of a tooltip must not be negative.");
        };

        let timer = DispatcherTimer::new();
        timer.set_interval(Duration::from_millis(show_delay));
        timer.set_tag(Some(Control::boxed(control)));
        // The timer belongs to the service, which stops and drops it.
        let _ = timer.tick({
            let weak = self.this.clone();
            let control = control.downgrade();
            move |_| {
                let (Some(this), Some(control)) = (weak.upgrade(), control.upgrade()) else { return };
                if this.timer.borrow().is_some() {
                    this.open(&control);
                }
            }
        });
        *self.timer.borrow_mut() = Some(timer.clone());
        timer.start();
    }

    fn open(&self, control: &Ref<Control>) {
        self.stop_timer();

        if control.is_attached_to_visual_tree() {
            ToolTip::set_is_open(control, true);

            // Value can be coerced back to false, need to double check.
            if ToolTip::get_is_open(control) {
                if let Some(tool_tip) = control.get_value(ToolTip::tool_tip_property()) {
                    self.subscribe_to_tool_tip(&tool_tip);
                }
            }
        }
    }

    /// Follows `tool_tip` until its popup closes: the close time is
    /// recorded, and the pointer leaving the tooltip closes it.
    fn subscribe_to_tool_tip(&self, tool_tip: &Ref<ToolTip>) {
        let pointer_exited = tool_tip.add_handler(InputElement::pointer_exited_event(), {
            let weak = self.this.clone();
            let weak_tool_tip = tool_tip.downgrade();
            move |_, _: &PointerEventArgs| {
                if let (Some(this), Some(tool_tip)) = (weak.upgrade(), weak_tool_tip.upgrade()) {
                    this.tool_tip_pointer_exited(&tool_tip);
                }
            }
        });

        let closed: Rc<RefCell<Option<Rc<dyn IDisposable>>>> = Rc::new(RefCell::new(None));
        let subscription = tool_tip.closed({
            let weak = self.this.clone();
            let closed = closed.clone();
            move |tool_tip| {
                if let Some(this) = weak.upgrade() {
                    this.tool_tip_closed();
                }

                let subscription = closed.borrow_mut().take();
                if let Some(subscription) = subscription {
                    subscription.dispose();
                }
                tool_tip.remove_handler(InputElement::pointer_exited_event(), pointer_exited);
            }
        });
        *closed.borrow_mut() = Some(subscription);
    }

    fn close(&self, control: &Control) {
        ToolTip::set_is_open(control, false);
    }

    fn stop_timer(&self) {
        let timer = self.timer.borrow_mut().take();
        if let Some(timer) = timer {
            timer.stop();
        }
    }
}

impl IToolTipService for ToolTipService {
    fn update(&self, root: &Rc<dyn IInputRoot>, candidate_tool_tip_host: Option<Ref<Visual>>) {
        let tip_control = self.tip_control();
        let current_tool_tip =
            tip_control.as_ref().and_then(|control| control.get_value(ToolTip::tool_tip_property()));

        let tool_tip_input_root = current_tool_tip
            .as_ref()
            .and_then(|tool_tip| tool_tip.popup_host())
            .and_then(|host| host.hosted_visual_tree_root())
            .and_then(|hosted_root| hosted_root.get_input_root());
        if tool_tip_input_root.is_some_and(|tool_tip_input_root| same_input_root(root, &tool_tip_input_root)) {
            // Don't update while the pointer is over a tooltip
            return;
        }

        let current_tool_tip_visual: Option<Ref<Visual>> = current_tool_tip.map(Ref::upcast);
        let mut candidate_tool_tip_host = candidate_tool_tip_host;

        while let Some(candidate) = candidate_tool_tip_host.clone() {
            // when OverlayPopupHost is in use, the tooltip is in the same window as the host control
            if Some(&candidate) == current_tool_tip_visual.as_ref() {
                return;
            }

            if let Some(control) = candidate.downcast_ref::<Control>() {
                if !ToolTip::get_service_enabled(control) {
                    return;
                }

                if ToolTip::get_tip(control).is_some()
                    && (control.is_effectively_enabled() || ToolTip::get_show_on_disabled(control))
                {
                    break;
                }
            }

            candidate_tool_tip_host = candidate.visual_parent();
        }

        let new_control = candidate_tool_tip_host.and_then(|candidate| candidate.cast::<Control>());

        if new_control == tip_control {
            return;
        }

        self.on_tip_control_changed(tip_control.as_ref(), new_control.as_ref());
        self.set_tip_control(new_control.as_ref());
        self.pointer_over_tip.set(false);
        self.pointer_over_owner.set(false);
    }

    fn as_tool_tip_service(&self) -> Option<&ToolTipService> {
        Some(self)
    }
}

impl IDisposable for ToolTipService {
    fn dispose(&self) {
        ToolTipService::dispose(self)
    }
}

/// The root element of an input root, as a visual: `None` for a root that
/// has closed, as the event that closed a popup is still processed with the
/// popup as its root (the reference reads a null `RootElement` there).
fn root_element_visual(root: &Rc<dyn IInputRoot>) -> Option<Ref<Visual>> {
    root.try_root_element().map(Ref::upcast)
}

/// Whether two input root handles are the same input root.
fn same_input_root(a: &Rc<dyn IInputRoot>, b: &Rc<dyn IInputRoot>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}
