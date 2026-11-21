//! Native modal ownership, input/focus scope and overlay lifetime for real ribbon
//! dialogs. No ContentDialog/Task/runtime emulation; concrete hosts own outcomes.
use super::{RibbonDialogHost, ribbon_dialog_host::InputPaneAdjustment};
use crate::compat::ContentDialogButton;
use ferroui_base::{
    Ref, WeakRef,
    input::{InputElement, Key, KeyModifiers, NavigationMethod},
    interactivity::{RoutedEventHandlerToken, RoutingStrategies},
    reactive::IDisposable,
    threading::{Dispatcher, DispatcherPriority},
};
use ferroui_controls::{
    Control, TopLevel,
    platform::IInputPane,
    primitives::{PopupOverlayLayer, PopupRoot},
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RibbonDialogOpenError {
    AlreadyOpen,
    RootNotVisible,
    MissingOverlay,
}
type Action = Rc<dyn Fn(ContentDialogButton)>;
ferroui_base::ferro_thread_local! {static ACTIVE:RefCell<Vec<Rc<RibbonDialogSession>>>=const{RefCell::new(Vec::new())};}
/// Active registry owns the visible host. Callers that are themselves the host
/// store Weak<Self> here to avoid host -> session -> host ownership cycles.
pub(crate) struct RibbonDialogSession {
    owner: WeakRef<TopLevel>,
    layer: Ref<PopupOverlayLayer>,
    host: Ref<RibbonDialogHost>,
    previous: Option<WeakRef<InputElement>>,
    action: RefCell<Option<Action>>,
    closing: Cell<bool>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    key: Cell<Option<RoutedEventHandlerToken>>,
    focus: Cell<Option<RoutedEventHandlerToken>>,
    input_pane: Option<DialogInputPane>,
}

struct DialogInputPane {
    pane: Rc<dyn IInputPane>,
    pending: Cell<bool>,
    adjustment: RefCell<Option<InputPaneAdjustment>>,
}
impl RibbonDialogSession {
    pub fn preflight(
        mut root: Ref<TopLevel>,
    ) -> Result<(Ref<TopLevel>, Ref<PopupOverlayLayer>), RibbonDialogOpenError> {
        while let Some(parent) = root.cast::<PopupRoot>().and_then(|p| p.parent_top_level()) {
            root = parent;
        }
        if !root.is_visible() {
            return Err(RibbonDialogOpenError::RootNotVisible);
        }
        if ACTIVE.with_borrow(|all| {
            all.iter()
                .any(|session| session.owner.upgrade().as_ref() == Some(&root))
        }) {
            return Err(RibbonDialogOpenError::AlreadyOpen);
        }
        let layer = PopupOverlayLayer::get_popup_overlay_layer(&root)
            .ok_or(RibbonDialogOpenError::MissingOverlay)?;
        Ok((root, layer))
    }
    /// Reserve before attachment. The concrete controller stores its session
    /// handle before show() can run Loaded/focus/layout callbacks and reenter.
    pub fn new(
        root: Ref<TopLevel>,
        host: Ref<RibbonDialogHost>,
        action: impl Fn(ContentDialogButton) + 'static,
    ) -> Result<Rc<Self>, RibbonDialogOpenError> {
        if host.is_dialog_showing() {
            return Err(RibbonDialogOpenError::AlreadyOpen);
        }
        let (owner, layer) = Self::preflight(root)?;
        let previous = owner
            .focus_manager()
            .get_focused_element()
            .map(|e| e.downgrade());
        let this = Rc::new(Self {
            owner: owner.downgrade(),
            layer,
            host,
            previous,
            action: RefCell::new(Some(Rc::new(action))),
            closing: Cell::new(false),
            subscriptions: RefCell::default(),
            key: Cell::new(None),
            focus: Cell::new(None),
            input_pane: owner.input_pane().map(|pane| DialogInputPane {
                pane,
                pending: Cell::new(false),
                adjustment: RefCell::default(),
            }),
        });
        let weak = Rc::downgrade(&this);
        this.host.on_button_action(Some(Rc::new(move |primary| {
            if let Some(session) = weak.upgrade() {
                session.request(primary);
            }
        })));
        let weak = Rc::downgrade(&this);
        this.key.set(Some(owner.add_handler_with(
            InputElement::key_down_event(),
            move |_, e| {
                if let Some(session) = weak.upgrade() {
                    if e.key == Key::Escape {
                        session.host.process_button(ContentDialogButton::Close);
                    } else if e.key == Key::Enter && !e.handled() {
                        match session.host.default_button() {
                            ContentDialogButton::Primary => {
                                session.host.process_button(ContentDialogButton::Primary)
                            }
                            ContentDialogButton::Secondary => {
                                session.host.process_button(ContentDialogButton::Secondary)
                            }
                            ContentDialogButton::Close => {
                                session.host.process_button(ContentDialogButton::Close)
                            }
                            _ => {}
                        }
                    }
                }
            },
            RoutingStrategies::BUBBLE,
            false,
        )));
        let weak = Rc::downgrade(&this);
        this.focus.set(Some(owner.add_handler(
            InputElement::got_focus_event(),
            move |_, _| {
                if let Some(session) = weak.upgrade()
                    && !session.closing.get()
                    && !session.contains_focus()
                {
                    session.host.focus_dialog();
                }
            },
        )));
        let weak = Rc::downgrade(&this);
        this.subscriptions
            .borrow_mut()
            .push(owner.property_changed(move |e| {
                if e.property() == TopLevel::client_size_property().as_property()
                    && let Some(session) = weak.upgrade()
                {
                    session.resize();
                    session.schedule_input_pane_adjustment();
                }
            }));
        let weak = Rc::downgrade(&this);
        this.subscriptions.borrow_mut().push(
            owner
                .closed(move || {
                    if let Some(session) = weak.upgrade()
                        && !session.closing.get()
                    {
                        session.host.force_close_dialog();
                    }
                })
                .detach(),
        );
        if let Some(input_pane) = &this.input_pane {
            let weak = Rc::downgrade(&this);
            let subscription = input_pane.pane.state_changed(Rc::new(move |_| {
                if let Some(session) = weak.upgrade() {
                    session.schedule_input_pane_adjustment();
                }
            }));
            this.subscriptions.borrow_mut().push(subscription);
        }
        this.host.begin_dialog();
        ACTIVE.with_borrow_mut(|all| all.push(this.clone()));
        Ok(this)
    }
    pub fn for_host(host: &RibbonDialogHost) -> Option<Rc<Self>> {
        ACTIVE.with_borrow(|all| {
            all.iter()
                .find(|session| session.host.ptr_eq(&host.to_ref()) && session.is_open())
                .cloned()
        })
    }
    pub fn show(self: &Rc<Self>) {
        if self.closing.get() {
            return;
        }
        self.resize();
        if self.closing.get() {
            return;
        }
        self.layer
            .children()
            .add(self.host.clone().upcast::<Control>());
        if self.closing.get() {
            return;
        }
        if let Some(owner) = self.owner.upgrade() {
            owner.update_layout();
        }
        if !self.closing.get() {
            self.host.focus_dialog();
            if !self.closing.get() {
                self.host.notify_dialog_opened();
                self.schedule_input_pane_adjustment();
            }
        }
    }
    pub(super) fn clear_input_pane_adjustment(&self) {
        if let Some(input_pane) = &self.input_pane {
            drop(input_pane.adjustment.take());
        }
    }

    pub(super) fn schedule_input_pane_adjustment(self: &Rc<Self>) {
        let Some(input_pane) = &self.input_pane else {
            return;
        };
        if self.closing.get() || input_pane.pending.replace(true) {
            return;
        }
        let weak = Rc::downgrade(self);
        Dispatcher::ui_thread().post_local(
            move || {
                let Some(session) = weak.upgrade() else {
                    return;
                };
                session.update_input_pane_adjustment();
            },
            DispatcherPriority::RENDER,
        );
    }

    fn update_input_pane_adjustment(&self) {
        let Some(input_pane) = &self.input_pane else {
            return;
        };
        input_pane.pending.set(false);
        let Some(owner) = self.owner.upgrade().filter(|_| self.is_open()) else {
            return;
        };
        let pane = input_pane.pane.occluded_rect();
        if pane.height <= 0. && input_pane.adjustment.borrow().is_none() {
            return;
        }
        // Compare the pane to the normal template arrangement each time.
        // Measuring the already moved dialog can make the overlap disappear
        // and cause the two states to alternate.
        let previous = input_pane.adjustment.take();
        let scroll_offset = previous
            .as_ref()
            .map(|adjustment| (adjustment.scroll.clone(), adjustment.scroll.offset()));
        drop(previous);
        owner.update_layout();
        if self.is_open() {
            let adjustment = self.host.adjust_for_input_pane(&owner, pane);
            if self.is_open() {
                input_pane.adjustment.replace(adjustment);
                owner.update_layout();
                if self.is_open()
                    && let Some((scroll, offset)) = scroll_offset
                    && input_pane
                        .adjustment
                        .borrow()
                        .as_ref()
                        .is_some_and(|current| current.scroll == scroll)
                {
                    scroll.set_offset(offset);
                }
            }
        }
    }

    fn request(&self, primary: ContentDialogButton) {
        if !self.closing.get() {
            let action = self.action.borrow().clone();
            if let Some(action) = action {
                action(primary);
            }
        }
    }
    fn resize(&self) {
        if let Some(owner) = self.owner.upgrade() {
            let size = owner.client_size();
            self.host.set_width(size.width);
            self.host.set_height(size.height);
        }
    }
    pub fn owner(&self) -> Option<Ref<TopLevel>> {
        self.owner.upgrade()
    }
    pub fn contains_focus(&self) -> bool {
        self.owner
            .upgrade()
            .and_then(|owner| owner.focus_manager().get_focused_element())
            .is_some_and(|e| {
                e.get_self_and_visual_ancestors()
                    .any(|v| v == self.host.clone().upcast::<ferroui_base::Visual>())
            })
    }
    pub fn is_open(&self) -> bool {
        !self.closing.get()
    }
    pub fn dismiss(&self) -> bool {
        if self.closing.replace(true) {
            return false;
        }
        self.host.end_dialog();
        self.host.on_button_action(None);
        self.action.replace(None);
        for subscription in self.subscriptions.take() {
            subscription.dispose();
        }
        if let Some(owner) = self.owner.upgrade() {
            if let Some(token) = self.key.take() {
                owner.remove_handler(InputElement::key_down_event(), token);
            }
            if let Some(token) = self.focus.take() {
                owner.remove_handler(InputElement::got_focus_event(), token);
            }
        }
        self.layer
            .children()
            .remove(self.host.clone().upcast::<Control>());
        self.clear_input_pane_adjustment();
        ACTIVE.with_borrow_mut(|all| all.retain(|session| !std::ptr::eq(&**session, self)));
        if let Some(previous) = self.previous.as_ref().and_then(WeakRef::upgrade) {
            previous.focus_with(NavigationMethod::Unspecified, KeyModifiers::NONE);
        }
        true
    }
}
