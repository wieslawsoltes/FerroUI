use crate::presentation_source::PresentationSource;
use ferroui_base::input::platform::IPlatformDragSource;
use ferroui_base::input::raw::{
    IDragDropDevice, IRawInputEventArgs, RawDragEvent, RawDragEventType, RawKeyEventArgs, RawKeyEventType,
    RawPointerEventArgs, RawPointerEventType,
};
use ferroui_base::input::{
    Cursor, DragDropEffects, IDataTransfer, IInputManager, IInputRoot, Key, LocalBoxFuture, PointerPressedEventArgs,
    RawInputModifiers, StandardCursorType,
};
use ferroui_base::reactive::{AnonymousObserver, IObserver};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{FerroLocator, LocatorExtensions, Point};
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

const MOUSE_INPUTMODIFIERS: RawInputModifiers = RawInputModifiers::LEFT_MOUSE_BUTTON
    .union(RawInputModifiers::MIDDLE_MOUSE_BUTTON)
    .union(RawInputModifiers::RIGHT_MOUSE_BUTTON);

/// The outcome of the drag-and-drop operation, set once.
#[derive(Default)]
struct DragResult {
    effect: Cell<Option<DragDropEffects>>,
    waker: RefCell<Option<Waker>>,
}

impl DragResult {
    fn on_next(&self, effect: DragDropEffects) {
        if self.effect.get().is_none() {
            self.effect.set(Some(effect));
        }
        let waker = self.waker.borrow_mut().take();
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

struct DragResultFuture(Rc<DragResult>);

impl Future for DragResultFuture {
    type Output = DragDropEffects;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<DragDropEffects> {
        match self.0.effect.get() {
            Some(effect) => Poll::Ready(effect),
            None => {
                *self.0.waker.borrow_mut() = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

struct State {
    drag_drop: Rc<dyn IDragDropDevice>,
    input_manager: Rc<dyn IInputManager>,
    result: Rc<DragResult>,
    allowed_effects: Cell<DragDropEffects>,
    dragged_data: RefCell<Option<Rc<dyn IDataTransfer>>>,
    last_source: RefCell<Option<Rc<PresentationSource>>>,
    last_position: Cell<Point>,
    last_cursor_type: Cell<Option<StandardCursorType>>,
    initial_input_modifiers: Cell<Option<RawInputModifiers>>,
}

/// The drag source used when the platform has none: runs the
/// drag-and-drop operation inside the application, from the raw input of
/// its own windows.
pub struct InProcessDragSource {
    state: Rc<State>,
}

impl InProcessDragSource {
    /// Creates a drag source over the input manager and the drag-and-drop
    /// device of the current services.
    ///
    /// # Panics
    /// Panics when one of the two services is not registered.
    pub fn new() -> Rc<Self> {
        let locator = FerroLocator::current();
        Rc::new(Self {
            state: Rc::new(State {
                input_manager: locator.get_required_service::<dyn IInputManager>(),
                drag_drop: locator.get_required_service::<dyn IDragDropDevice>(),
                result: Rc::new(DragResult::default()),
                allowed_effects: Cell::new(DragDropEffects::NONE),
                dragged_data: RefCell::new(None),
                last_source: RefCell::new(None),
                last_position: Cell::new(Point::default()),
                last_cursor_type: Cell::new(None),
                initial_input_modifiers: Cell::new(None),
            }),
        })
    }
}

impl IPlatformDragSource for InProcessDragSource {
    fn do_drag_drop_async(
        &self,
        trigger_event: &PointerPressedEventArgs,
        data_transfer: Rc<dyn IDataTransfer>,
        allowed_effects: DragDropEffects,
    ) -> LocalBoxFuture<DragDropEffects> {
        Dispatcher::ui_thread().verify_access();
        trigger_event.pointer().capture(None);

        let state = &self.state;
        if state.dragged_data.borrow().is_none() {
            *state.dragged_data.borrow_mut() = Some(data_transfer.clone());
            *state.last_source.borrow_mut() = None;
            state.last_position.set(Point::default());
            state.allowed_effects.set(allowed_effects);

            let observer_state = state.clone();
            let input_observer: Rc<dyn IObserver<Rc<dyn IRawInputEventArgs>>> =
                Rc::new(AnonymousObserver::new(move |arg: Rc<dyn IRawInputEventArgs>| {
                    if let Some(pointer_event_args) = arg.downcast_ref::<RawPointerEventArgs>() {
                        observer_state.process_mouse_events(pointer_event_args);
                    } else if let Some(key_event_args) = arg.downcast_ref::<RawKeyEventArgs>() {
                        observer_state.process_key_events(key_event_args);
                    }
                }));

            let subscription = state.input_manager.pre_process().subscribe(input_observer);
            let result = DragResultFuture(state.result.clone());
            return Box::pin(async move {
                let effect = result.await;
                subscription.dispose();
                data_transfer.dispose();
                effect
            });
        }

        Box::pin(std::future::ready(DragDropEffects::NONE))
    }
}

impl State {
    fn raise_event_and_update_cursor(
        &self,
        type_: RawDragEventType,
        root: &Rc<dyn IInputRoot>,
        pt: Point,
        modifiers: RawInputModifiers,
    ) -> DragDropEffects {
        self.last_position.set(pt);

        let dragged_data = self.dragged_data.borrow().clone().expect("a drag-and-drop operation is running");
        let raw_event = Rc::new(RawDragEvent::new(
            self.drag_drop.clone(),
            type_,
            root.clone(),
            pt,
            dragged_data,
            self.allowed_effects.get(),
            modifiers,
        ));

        let source = PresentationSource::from_root_visual(&root.root_element());
        let input = source.as_ref().and_then(|source| source.platform_impl()).and_then(|platform_impl| platform_impl.input());
        if let Some(input) = input {
            input(raw_event.clone());
        }

        let effect = Self::get_preferred_effect(raw_event.effects() & self.allowed_effects.get(), modifiers);
        self.update_cursor(source, effect);
        effect
    }

    fn get_preferred_effect(effect: DragDropEffects, modifiers: RawInputModifiers) -> DragDropEffects {
        if effect == DragDropEffects::COPY
            || effect == DragDropEffects::MOVE
            || effect == DragDropEffects::LINK
            || effect == DragDropEffects::NONE
        {
            return effect; // No need to check for the modifiers.
        }
        if effect.contains(DragDropEffects::LINK) && modifiers.contains(RawInputModifiers::ALT) {
            return DragDropEffects::LINK;
        }
        if effect.contains(DragDropEffects::COPY) && modifiers.contains(RawInputModifiers::CONTROL) {
            return DragDropEffects::COPY;
        }
        DragDropEffects::MOVE
    }

    fn get_cursor_for_drop_effect(effects: DragDropEffects) -> StandardCursorType {
        if effects.contains(DragDropEffects::COPY) {
            return StandardCursorType::DragCopy;
        }
        if effects.contains(DragDropEffects::MOVE) {
            return StandardCursorType::DragMove;
        }
        if effects.contains(DragDropEffects::LINK) {
            return StandardCursorType::DragLink;
        }
        StandardCursorType::No
    }

    fn update_cursor(&self, root: Option<Rc<PresentationSource>>, effect: DragDropEffects) {
        let last_source = self.last_source.borrow().clone();
        if !same_source(last_source.as_ref(), root.as_ref()) {
            if let Some(last_source) = last_source {
                last_source.set_cursor_override(None);
            }
            *self.last_source.borrow_mut() = root.clone();
            self.last_cursor_type.set(None);
        }

        if let Some(root) = root {
            let ct = Self::get_cursor_for_drop_effect(effect);
            if self.last_cursor_type.get() != Some(ct) {
                self.last_cursor_type.set(Some(ct));
                root.set_cursor_override(Some(Cursor::new(ct)));
            }
        }
    }

    fn last_root(&self) -> Option<Rc<dyn IInputRoot>> {
        self.last_source.borrow().clone().map(|source| source as Rc<dyn IInputRoot>)
    }

    fn cancel_dragging(&self) {
        if let Some(last_source) = self.last_root() {
            self.raise_event_and_update_cursor(
                RawDragEventType::DragLeave,
                &last_source,
                self.last_position.get(),
                RawInputModifiers::NONE,
            );
        }
        self.update_cursor(None, DragDropEffects::NONE);
        self.result.on_next(DragDropEffects::NONE);
    }

    fn process_key_events(&self, e: &RawKeyEventArgs) {
        if e.type_() == RawKeyEventType::KeyDown && e.key() == Key::Escape {
            if let Some(last_source) = self.last_root() {
                self.raise_event_and_update_cursor(
                    RawDragEventType::DragLeave,
                    &last_source,
                    self.last_position.get(),
                    e.modifiers(),
                );
            }
            self.update_cursor(None, DragDropEffects::NONE);
            self.result.on_next(DragDropEffects::NONE);
            e.set_handled(true);
        } else if matches!(e.key(), Key::LeftCtrl | Key::RightCtrl | Key::LeftAlt | Key::RightAlt) {
            if let Some(last_source) = self.last_root() {
                self.raise_event_and_update_cursor(
                    RawDragEventType::DragOver,
                    &last_source,
                    self.last_position.get(),
                    e.modifiers(),
                );
            }
        }
    }

    fn check_dragging_accepted(&self, e: &RawPointerEventArgs, changed_mouse_button: RawInputModifiers) {
        if self.initial_input_modifiers.get().is_some_and(|initial| initial.contains(changed_mouse_button)) {
            let result =
                self.raise_event_and_update_cursor(RawDragEventType::Drop, e.root(), e.position(), e.input_modifiers());
            self.update_cursor(None, DragDropEffects::NONE);
            self.result.on_next(result);
        } else {
            self.cancel_dragging();
        }
        e.set_handled(true);
    }

    fn process_mouse_events(&self, e: &RawPointerEventArgs) {
        if self.initial_input_modifiers.get().is_none() {
            self.initial_input_modifiers.set(Some(e.input_modifiers() & MOUSE_INPUTMODIFIERS));
        }

        match e.type_() {
            RawPointerEventType::LeftButtonDown
            | RawPointerEventType::RightButtonDown
            | RawPointerEventType::MiddleButtonDown
            | RawPointerEventType::NonClientLeftButtonDown => {
                self.cancel_dragging();
                e.set_handled(true);
            }
            RawPointerEventType::LeaveWindow => {
                self.raise_event_and_update_cursor(RawDragEventType::DragLeave, e.root(), e.position(), e.input_modifiers());
            }
            RawPointerEventType::LeftButtonUp => self.check_dragging_accepted(e, RawInputModifiers::LEFT_MOUSE_BUTTON),
            RawPointerEventType::MiddleButtonUp => self.check_dragging_accepted(e, RawInputModifiers::MIDDLE_MOUSE_BUTTON),
            RawPointerEventType::RightButtonUp => self.check_dragging_accepted(e, RawInputModifiers::RIGHT_MOUSE_BUTTON),
            RawPointerEventType::Move => {
                e.set_handled(true);
                let mods = e.input_modifiers() & MOUSE_INPUTMODIFIERS;
                if self.initial_input_modifiers.get() != Some(mods) {
                    self.cancel_dragging();
                    return;
                }

                let last_source = self.last_source.borrow().clone();
                let is_last_source =
                    last_source.as_ref().is_some_and(|last| std::ptr::addr_eq(Rc::as_ptr(e.root()), Rc::as_ptr(last)));
                if !is_last_source {
                    if let Some(last_source) = last_source {
                        if let Some(lr) = last_source.root_visual() {
                            let r = e.root().root_element();
                            let last_root: Rc<dyn IInputRoot> = last_source;
                            self.raise_event_and_update_cursor(
                                RawDragEventType::DragLeave,
                                &last_root,
                                lr.point_to_client(r.point_to_screen(e.position())),
                                e.input_modifiers(),
                            );
                        }
                    }
                    self.raise_event_and_update_cursor(RawDragEventType::DragEnter, e.root(), e.position(), e.input_modifiers());
                } else {
                    self.raise_event_and_update_cursor(RawDragEventType::DragOver, e.root(), e.position(), e.input_modifiers());
                }
            }
            _ => {}
        }
    }
}

fn same_source(a: Option<&Rc<PresentationSource>>, b: Option<&Rc<PresentationSource>>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        (None, None) => true,
        _ => false,
    }
}
