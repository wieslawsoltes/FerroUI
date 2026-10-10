//! The input queue of a backend that handles input asynchronously (the
//! port of `Shared/RawEventGrouping.cs`, a source file the reference
//! compiles into every backend that uses it).
//!
//! While queueing, it groups Move and TouchUpdate events so that the
//! intermediate points of a move can be provided.

use ferroui_base::input::raw::{
    IRawInputEventArgs, RawPointerEventArgs, RawPointerEventType, RawPointerPoint, RawTouchEventArgs,
};
use ferroui_base::input::IntermediatePoints;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use std::cell::{Cell, LazyCell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::rc::{Rc, Weak};
use std::sync::Arc;

/// A raw input event.
pub type RawEvent = Rc<dyn IRawInputEventArgs>;
/// What handles a queued event.
pub type RawEventHandler = Rc<dyn Fn(RawEvent)>;

/// Where a grouper queues its events.
pub trait IRawEventGrouperDispatchQueue {
    fn add(&self, args: RawEvent, handler: RawEventHandler);
}

/// A queue that is emptied by whoever owns it (the event loop of the
/// platform).
#[derive(Default)]
pub struct ManualRawEventGrouperDispatchQueue {
    input_queue: RefCell<VecDeque<(RawEvent, RawEventHandler)>>,
}

impl ManualRawEventGrouperDispatchQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn has_jobs(&self) -> bool {
        !self.input_queue.borrow().is_empty()
    }

    pub fn dispatch_next(&self) {
        let next = self.input_queue.borrow_mut().pop_front();
        let Some((args, handler)) = next else {
            return;
        };
        handler(args);
    }
}

impl IRawEventGrouperDispatchQueue for ManualRawEventGrouperDispatchQueue {
    fn add(&self, args: RawEvent, handler: RawEventHandler) {
        self.input_queue.borrow_mut().push_back((args, handler));
    }
}

/// A queue that empties itself through the dispatcher, at input priority.
pub struct AutomaticRawEventGrouperDispatchQueue {
    this: Weak<AutomaticRawEventGrouperDispatchQueue>,
    input_queue: RefCell<VecDeque<(RawEvent, RawEventHandler)>>,
    dispatcher: Arc<Dispatcher>,
}

impl AutomaticRawEventGrouperDispatchQueue {
    pub fn new(dispatcher: Option<Arc<Dispatcher>>) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            input_queue: RefCell::new(VecDeque::new()),
            dispatcher: dispatcher.unwrap_or_else(Dispatcher::current_dispatcher),
        })
    }

    fn post_dispatch(&self) {
        let this = self.this.clone();
        self.dispatcher.post_local(
            move || {
                if let Some(this) = this.upgrade() {
                    this.dispatch_from_queue();
                }
            },
            DispatcherPriority::INPUT,
        );
    }

    fn dispatch_from_queue(&self) {
        loop {
            let next = self.input_queue.borrow_mut().pop_front();
            let Some((args, handler)) = next else {
                return;
            };

            handler(args);

            // The priority above input (`DispatcherPriority.Input + 1`) is the default one.
            if self.dispatcher.has_jobs_with_priority(DispatcherPriority::DEFAULT) {
                self.post_dispatch();
                return;
            }
        }
    }
}

impl IRawEventGrouperDispatchQueue for AutomaticRawEventGrouperDispatchQueue {
    fn add(&self, args: RawEvent, handler: RawEventHandler) {
        let count = {
            let mut queue = self.input_queue.borrow_mut();
            queue.push_back((args, handler));
            queue.len()
        };

        if count == 1 {
            self.post_dispatch();
        }
    }
}

type MergedPoints = Rc<RefCell<Vec<RawPointerPoint>>>;

struct GrouperState {
    event_callback: RawEventHandler,
    last_touch_points: RefCell<HashMap<i64, RawEvent>>,
    last_event: RefCell<Option<RawEvent>>,
    // The points that were merged into a queued event, by the address of
    // the event (which the queue keeps alive until it is dispatched). The
    // reference appends to the pooled list behind the lazy value of the
    // event; the lazy value of the port cannot be appended to, so the
    // list lives here and the lazy value reads it when it is first asked
    // for.
    merged_points: RefCell<HashMap<usize, MergedPoints>>,
    disposed: Cell<bool>,
}

/// Queues raw input events and merges consecutive pointer moves.
pub struct RawEventGrouper {
    state: Rc<GrouperState>,
    queue: Rc<dyn IRawEventGrouperDispatchQueue>,
    dispatch: RawEventHandler,
}

fn same_event(a: &RawEvent, b: &RawEvent) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}

fn as_pointer_event(args: &RawEvent) -> Option<&RawPointerEventArgs> {
    args.downcast_ref::<RawPointerEventArgs>()
}

fn as_touch_event(args: &RawEvent) -> Option<&RawTouchEventArgs> {
    args.downcast_ref::<RawTouchEventArgs>()
}

fn event_key(event: &RawPointerEventArgs) -> usize {
    event as *const RawPointerEventArgs as usize
}

impl RawEventGrouper {
    pub fn new(
        event_callback: impl Fn(RawEvent) + 'static,
        queue: Option<Rc<dyn IRawEventGrouperDispatchQueue>>,
    ) -> Self {
        let state = Rc::new(GrouperState {
            event_callback: Rc::new(event_callback),
            last_touch_points: RefCell::new(HashMap::new()),
            last_event: RefCell::new(None),
            merged_points: RefCell::new(HashMap::new()),
            disposed: Cell::new(false),
        });
        let queue = queue.unwrap_or_else(|| AutomaticRawEventGrouperDispatchQueue::new(None));
        let dispatch: RawEventHandler = {
            let state = state.clone();
            Rc::new(move |ev| Self::dispatch(&state, ev))
        };
        Self { state, queue, dispatch }
    }

    fn add_to_queue(&self, args: RawEvent) {
        *self.state.last_event.borrow_mut() = Some(args.clone());
        self.queue.add(args, self.dispatch.clone());
    }

    fn dispatch(state: &Rc<GrouperState>, ev: RawEvent) {
        if !state.disposed.get() {
            let is_last = state.last_event.borrow().as_ref().is_some_and(|last| same_event(last, &ev));
            if is_last {
                *state.last_event.borrow_mut() = None;
            }

            if let Some(touch_update) = as_touch_event(&ev) {
                if touch_update.type_() == RawPointerEventType::TouchUpdate {
                    state.last_touch_points.borrow_mut().remove(&touch_update.raw_pointer_id());
                }
            }

            (state.event_callback)(ev.clone());
        }

        // The merged points of the event are released with it (the
        // reference returns its pooled list here).
        if let Some(pointer) = as_pointer_event(&ev) {
            state.merged_points.borrow_mut().remove(&event_key(pointer));
        }
    }

    pub fn handle_event(&self, args: RawEvent) {
        /*
         Try to update already enqueued events if
         1) they are still not handled (_lastEvent and _lastTouchPoints shouldn't contain said event in that case)
         2) previous event belongs to the same "event block", events in the same block:
           - belong from the same device
           - are pointer move events (Move/TouchUpdate)
           - have the same type
           - have same modifiers

         Even if nothing is updated and the event is actually enqueued, we need to update the relevant tracking info
        */
        let last_event = self.state.last_event.borrow().clone();
        let merged = (|| {
            let pointer_event = as_pointer_event(&args)?;
            let last_event = last_event.as_ref()?;
            let last_pointer_event = as_pointer_event(last_event)?;
            let same_block = Rc::ptr_eq(last_event.device(), args.device())
                && last_pointer_event.input_modifiers() == pointer_event.input_modifiers()
                && last_pointer_event.type_() == pointer_event.type_()
                && matches!(last_pointer_event.type_(), RawPointerEventType::Move | RawPointerEventType::TouchUpdate);
            if !same_block {
                return None;
            }

            if let Some(touch_event) = as_touch_event(&args) {
                let last_touch_event =
                    self.state.last_touch_points.borrow().get(&touch_event.raw_pointer_id()).cloned();
                match last_touch_event {
                    Some(last_touch_event) => {
                        let last = as_pointer_event(&last_touch_event).expect("a touch event is a pointer event");
                        Self::merge_events(&self.state, last, pointer_event);
                    }
                    None => {
                        self.state.last_touch_points.borrow_mut().insert(touch_event.raw_pointer_id(), args.clone());
                        self.add_to_queue(args.clone());
                    }
                }
            } else {
                Self::merge_events(&self.state, last_pointer_event, pointer_event);
            }

            Some(())
        })();
        if merged.is_some() {
            return;
        }

        self.state.last_touch_points.borrow_mut().clear();
        if let Some(touch_event) = as_touch_event(&args) {
            if touch_event.type_() == RawPointerEventType::TouchUpdate {
                self.state.last_touch_points.borrow_mut().insert(touch_event.raw_pointer_id(), args.clone());
            }
        }
        self.add_to_queue(args);
    }

    fn merge_events(state: &GrouperState, last: &RawPointerEventArgs, current: &RawPointerEventArgs) {
        let list = state.merged_points.borrow_mut().entry(event_key(last)).or_default().clone();
        if last.intermediate_points().is_none() {
            let list = list.clone();
            let points: IntermediatePoints = Rc::new(LazyCell::new(Box::new(move || Some(list.borrow().clone()))));
            last.set_intermediate_points(Some(points));
        }
        let point = last.point();
        let mut merged = RawPointerPoint::new();
        merged.position = last.position();
        merged.pressure = point.pressure;
        merged.set_contact_rect(point.contact_rect());
        merged.twist = point.twist;
        merged.x_tilt = point.x_tilt;
        merged.y_tilt = point.y_tilt;
        list.borrow_mut().push(merged);
        last.set_position(current.position());
        last.set_timestamp(current.timestamp());
        last.set_input_modifiers(current.input_modifiers());
    }

    pub fn dispose(&self) {
        self.state.disposed.set(true);
        *self.state.last_event.borrow_mut() = None;
        self.state.last_touch_points.borrow_mut().clear();
        self.state.merged_points.borrow_mut().clear();
    }
}
