//! Port of `ManagedDragDrop/ManagedDragDropService.cs`.

use crate::drag_drop_context_store::DragDropContextStore;
use ferroui_base::input::DragDropEffects;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{BoxedValue, PixelPoint, Point, Ref};
use ferroui_controls::TopLevel;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

ferroui_base::ferro_thread_local! {
    static INSTANCE: Rc<ManagedDragDropService> = Rc::new(ManagedDragDropService::new());
}

/// Provides a lightweight, managed drag-drop state shared within the application.
///
/// This service coordinates drag state, payload, and positions for behaviors that implement an
/// in-process drag experience without using the drag and drop of the system.
///
/// Deviation (DEVIATIONS.md, Behaviors): the singleton of the reference is shared by the threads
/// of the process; the objects of the framework belong to the thread that uses them, so there is
/// one service per thread.
pub struct ManagedDragDropService {
    is_dragging: Cell<bool>,
    payload: RefCell<Option<BoxedValue>>,
    data_format: RefCell<Option<String>>,
    payload_key: RefCell<Option<String>>,
    effects: Cell<DragDropEffects>,
    origin_top_level: RefCell<Option<Ref<TopLevel>>>,
    current_client_position: Cell<Point>,
    drag_started: Rc<HandlerList<dyn Fn()>>,
    drag_ended: Rc<HandlerList<dyn Fn()>>,
    drag_moved: Rc<HandlerList<dyn Fn()>>,
}

impl ManagedDragDropService {
    fn new() -> Self {
        Self {
            is_dragging: Cell::new(false),
            payload: RefCell::new(None),
            data_format: RefCell::new(None),
            payload_key: RefCell::new(None),
            effects: Cell::new(DragDropEffects::NONE),
            origin_top_level: RefCell::new(None),
            current_client_position: Cell::new(Point::default()),
            drag_started: Rc::new(HandlerList::new()),
            drag_ended: Rc::new(HandlerList::new()),
            drag_moved: Rc::new(HandlerList::new()),
        }
    }

    /// Gets the singleton instance of the service (of the thread).
    pub fn instance() -> Rc<ManagedDragDropService> {
        INSTANCE.with(Rc::clone)
    }

    /// Gets a value indicating whether a managed drag operation is in progress.
    pub fn is_dragging(&self) -> bool {
        self.is_dragging.get()
    }

    /// Gets the payload being dragged.
    pub fn payload(&self) -> Option<BoxedValue> {
        self.payload.borrow().clone()
    }

    /// Gets the data format associated with the [`payload`](Self::payload).
    pub fn data_format(&self) -> Option<String> {
        self.data_format.borrow().clone()
    }

    /// The key of the payload in the context store (an internal member of the reference).
    #[allow(dead_code)]
    pub(crate) fn payload_key(&self) -> Option<String> {
        self.payload_key.borrow().clone()
    }

    /// Gets the effective drag-drop effects requested by the drag initiator.
    pub fn effects(&self) -> DragDropEffects {
        self.effects.get()
    }

    /// Gets the top-level where the drag originated.
    pub fn origin_top_level(&self) -> Option<Ref<TopLevel>> {
        self.origin_top_level.borrow().clone()
    }

    /// Gets the current pointer position in client coordinates of the
    /// [`origin_top_level`](Self::origin_top_level).
    pub fn current_client_position(&self) -> Point {
        self.current_client_position.get()
    }

    /// Gets the current pointer position in screen pixel coordinates.
    pub fn screen_position(&self) -> PixelPoint {
        match self.origin_top_level() {
            Some(origin_top_level) => origin_top_level.point_to_screen(self.current_client_position()),
            None => PixelPoint::new(0, 0),
        }
    }

    /// Occurs when a managed drag operation begins. Disposing the returned handle unsubscribes.
    pub fn drag_started(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        Self::subscribe(&self.drag_started, handler)
    }

    /// Occurs when a managed drag operation ends. Disposing the returned handle unsubscribes.
    pub fn drag_ended(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        Self::subscribe(&self.drag_ended, handler)
    }

    /// Occurs when the pointer moves during a managed drag operation. Disposing the returned
    /// handle unsubscribes.
    pub fn drag_moved(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        Self::subscribe(&self.drag_moved, handler)
    }

    fn subscribe(list: &Rc<HandlerList<dyn Fn()>>, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = list.add(Rc::new(handler));
        let list = Rc::downgrade(list);
        Disposable::create(move || {
            if let Some(list) = list.upgrade() {
                list.remove(token);
            }
        })
    }

    fn raise(list: &HandlerList<dyn Fn()>) {
        for (_, handler) in list.snapshot().iter() {
            handler();
        }
    }

    /// Begins a managed drag operation from `origin` with `payload` in `data_format`, allowing
    /// `effects`; `start_client` is the starting pointer position in client coordinates.
    pub fn begin(
        &self,
        origin: &Ref<TopLevel>,
        payload: Option<BoxedValue>,
        data_format: Option<String>,
        effects: DragDropEffects,
        start_client: Point,
    ) {
        self.is_dragging.set(true);
        *self.origin_top_level.borrow_mut() = Some(origin.clone());
        *self.payload_key.borrow_mut() = DragDropContextStore::add(payload.as_ref());
        *self.payload.borrow_mut() = payload;
        *self.data_format.borrow_mut() = data_format;
        self.effects.set(effects);
        self.current_client_position.set(start_client);
        Self::raise(&self.drag_started);
    }

    /// Updates the current pointer position during a managed drag operation.
    pub fn move_(&self, client_position: Point) {
        if !self.is_dragging.get() {
            return;
        }
        self.current_client_position.set(client_position);
        Self::raise(&self.drag_moved);
    }

    /// Ends the current managed drag operation and resets state.
    pub fn end(&self) {
        if !self.is_dragging.get() {
            return;
        }
        self.is_dragging.set(false);
        Self::raise(&self.drag_ended);
        let payload_key = self.payload_key.borrow_mut().take();
        DragDropContextStore::remove(payload_key.as_deref());
        *self.origin_top_level.borrow_mut() = None;
        let payload = self.payload.borrow_mut().take();
        drop(payload);
        *self.data_format.borrow_mut() = None;
        self.effects.set(DragDropEffects::NONE);
        self.current_client_position.set(Point::default());
    }
}
