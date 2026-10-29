//! Port of `Collections/BehaviorCollection.cs`.

use crate::{IBehavior, IBehaviorEventsHandler};
use ferroui_base::collections::{FerroList, NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};
use ferroui_base::{FerroObject, Nullable, Ref, StyledElement, Visual, WeakRef};
use ferroui_controls::{Control, TopLevel};
use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::rc::{Rc, Weak};

/// Represents a collection of [`IBehavior`]s with a shared
/// [`associated_object`](BehaviorCollection::associated_object).
///
/// The collection is a notifying list of objects (`FerroList<FerroObject>`); every item must be
/// a behavior. It is a reference object: clones refer to the same collection and compare by
/// identity.
#[derive(Clone)]
pub struct BehaviorCollection(Rc<BehaviorCollectionState>);

struct BehaviorCollectionState {
    list: FerroList<Ref<FerroObject>>,
    // Deviation (DEVIATIONS.md, Behaviors): the reference holds its host strongly; the host
    // holds the collection (the value of `Interaction.Behaviors`), so the collection refers to
    // it weakly, as the framework refers to parents and templated parents.
    associated_object: RefCell<Option<WeakRef<FerroObject>>>,
    // After a collection change we need to compare the current state of the collection with the
    // old collection so that we can call detach on all removed items.
    old_collection: RefCell<Vec<Ref<FerroObject>>>,
    pending_synchronizations: RefCell<Vec<Ref<FerroObject>>>,
    is_attaching_collection: Cell<bool>,
    is_synchronizing_collection: Cell<bool>,
    has_observed_initialized: Cell<bool>,
    has_observed_logical_attachment: Cell<bool>,
    has_observed_visual_attachment: Cell<bool>,
    has_observed_loaded: Cell<bool>,
}

impl PartialEq for BehaviorCollection {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Deref for BehaviorCollection {
    type Target = FerroList<Ref<FerroObject>>;

    fn deref(&self) -> &FerroList<Ref<FerroObject>> {
        &self.0.list
    }
}

impl Default for BehaviorCollection {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for BehaviorCollection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BehaviorCollection({})", self.0.list.count())
    }
}

/// The object as a behavior.
fn behavior_of(item: &FerroObject) -> Option<&dyn IBehavior> {
    <dyn IBehavior>::of(item)
}

/// The object as a handler of the lifecycle callbacks of its host.
fn events_handler_of(item: &FerroObject) -> Option<&dyn IBehaviorEventsHandler> {
    <dyn IBehaviorEventsHandler>::of(item)
}

/// Whether the object is a top level that is open (`TopLevel { IsLoaded: true }`).
fn is_open_top_level(associated_object: Option<&Ref<FerroObject>>) -> bool {
    associated_object.and_then(|o| o.downcast_ref::<TopLevel>()).is_some_and(|top_level| top_level.is_loaded())
}

/// `associatedObject is StyledElement { IsInitialized: true }`.
fn is_initialized(associated_object: Option<&Ref<FerroObject>>) -> bool {
    associated_object.and_then(|o| o.downcast_ref::<StyledElement>()).is_some_and(StyledElement::is_initialized)
}

/// `associatedObject is StyledElement styledElement && (styledElement.IsAttachedToLogicalTree || isOpenTopLevel)`.
fn is_logically_attached(associated_object: Option<&Ref<FerroObject>>, is_open_top_level: bool) -> bool {
    associated_object
        .and_then(|o| o.downcast_ref::<StyledElement>())
        .is_some_and(|styled_element| styled_element.is_attached_to_logical_tree() || is_open_top_level)
}

/// `associatedObject is Visual visual && (visual.IsAttachedToVisualTree() || isOpenTopLevel)`.
fn is_visually_attached(associated_object: Option<&Ref<FerroObject>>, is_open_top_level: bool) -> bool {
    associated_object
        .and_then(|o| o.downcast_ref::<Visual>())
        .is_some_and(|visual| visual.is_attached_to_visual_tree() || is_open_top_level)
}

/// `associatedObject is Control { IsLoaded: true }`.
fn is_loaded(associated_object: Option<&Ref<FerroObject>>) -> bool {
    associated_object.and_then(|o| o.downcast_ref::<Control>()).is_some_and(Control::is_loaded)
}

/// Resets a flag when dropped: the `finally` of a `try` that sets it.
struct ResetFlag<'a>(&'a Cell<bool>, bool);

impl Drop for ResetFlag<'_> {
    fn drop(&mut self) {
        self.0.set(self.1);
    }
}

impl BehaviorCollection {
    /// Initializes a new instance of the [`BehaviorCollection`] class.
    pub fn new() -> Self {
        let state = Rc::new(BehaviorCollectionState {
            list: FerroList::new(),
            associated_object: RefCell::new(None),
            old_collection: RefCell::new(Vec::new()),
            pending_synchronizations: RefCell::new(Vec::new()),
            is_attaching_collection: Cell::new(false),
            is_synchronizing_collection: Cell::new(false),
            has_observed_initialized: Cell::new(false),
            has_observed_logical_attachment: Cell::new(false),
            has_observed_visual_attachment: Cell::new(false),
            has_observed_loaded: Cell::new(false),
        });
        let weak: Weak<BehaviorCollectionState> = Rc::downgrade(&state);
        state.list.add_collection_changed(Rc::new(move |e: &NotifyCollectionChangedEventArgs<'_, Ref<FerroObject>>| {
            if let Some(state) = weak.upgrade() {
                BehaviorCollection(state).behavior_collection_collection_changed(e);
            }
        }));
        Self(state)
    }

    /// The list of the collection.
    pub fn list(&self) -> &FerroList<Ref<FerroObject>> {
        &self.0.list
    }

    /// Gets the object to which the [`BehaviorCollection`] is attached.
    pub fn associated_object(&self) -> Option<Ref<FerroObject>> {
        self.0.associated_object.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    /// The behaviors of the collection (`this.OfType<IBehavior>().ToList()`).
    fn behaviors(&self) -> Vec<Ref<FerroObject>> {
        self.0.list.snapshot().iter().filter(|item| behavior_of(item).is_some()).cloned().collect()
    }

    /// Attaches the collection of behaviors to the specified object.
    ///
    /// # Panics
    /// Panics if the [`BehaviorCollection`] is already attached to a different object.
    pub fn attach(&self, associated_object: impl Into<Nullable<FerroObject>>) {
        let associated_object = associated_object.into().0;
        let current = self.associated_object();
        if associated_object == current {
            return;
        }

        if current.is_some() {
            panic!("An instance of a behavior cannot be attached to more than one object at a time.");
        }

        debug_assert!(associated_object.is_some(), "The previous checks should keep us from ever setting null here.");
        let Some(associated_object) = associated_object else {
            return;
        };
        *self.0.associated_object.borrow_mut() = Some(associated_object.downgrade());
        let behaviors = self.behaviors();
        self.0.is_attaching_collection.set(true);
        {
            let _finally = ResetFlag(&self.0.is_attaching_collection, false);
            for behavior in &behaviors {
                let Some(associated_object_for_attach) = self.associated_object() else {
                    break;
                };

                if !self.0.list.contains(behavior) {
                    continue;
                }

                if let Some(behavior) = behavior_of(behavior) {
                    behavior.attach(Some(associated_object_for_attach));
                }
            }
        }

        self.synchronize_behavior_events(self.behaviors());
        self.capture_current_lifecycle_state();
    }

    /// Detaches the collection of behaviors from the
    /// [`associated_object`](Self::associated_object).
    pub fn detach(&self) {
        for item in self.0.list.snapshot().iter() {
            if let Some(behavior_item) = behavior_of(item) {
                if behavior_item.associated_object().is_some() {
                    behavior_item.detach();
                }
            }
        }

        *self.0.associated_object.borrow_mut() = None;
        self.0.old_collection.borrow_mut().clear();
        self.0.pending_synchronizations.borrow_mut().clear();
        self.0.has_observed_initialized.set(false);
        self.0.has_observed_logical_attachment.set(false);
        self.0.has_observed_visual_attachment.set(false);
        self.0.has_observed_loaded.set(false);
    }

    /// Dispatches the attachment of the host to the visual tree.
    pub(crate) fn attached_to_visual_tree(&self) {
        self.0.has_observed_visual_attachment.set(true);
        self.dispatch_behavior_event(|handler| handler.attached_to_visual_tree_event_handler());
    }

    /// Dispatches the detachment of the host from the visual tree.
    pub(crate) fn detached_from_visual_tree(&self) {
        self.0.has_observed_visual_attachment.set(false);
        self.dispatch_behavior_event(|handler| handler.detached_from_visual_tree_event_handler());
    }

    /// Dispatches the attachment of the host to the logical tree.
    pub(crate) fn attached_to_logical_tree(&self) {
        self.0.has_observed_logical_attachment.set(true);
        self.dispatch_behavior_event(|handler| handler.attached_to_logical_tree_event_handler());
    }

    /// Dispatches the detachment of the host from the logical tree.
    pub(crate) fn detached_from_logical_tree(&self) {
        self.0.has_observed_logical_attachment.set(false);
        self.dispatch_behavior_event(|handler| handler.detached_from_logical_tree_event_handler());
    }

    /// Dispatches the `Loaded` event of the host.
    pub(crate) fn loaded(&self) {
        self.0.has_observed_loaded.set(true);
        self.dispatch_behavior_event(|handler| handler.loaded_event_handler());
    }

    /// Dispatches the `Unloaded` event of the host.
    pub(crate) fn unloaded(&self) {
        self.0.has_observed_loaded.set(false);
        self.dispatch_behavior_event(|handler| handler.unloaded_event_handler());
    }

    /// Dispatches the `Initialized` event of the host.
    pub(crate) fn initialized(&self) {
        self.0.has_observed_initialized.set(true);
        self.dispatch_behavior_event(|handler| handler.initialized_event_handler());
    }

    /// Dispatches the `DataContextChanged` event of the host.
    pub(crate) fn data_context_changed(&self) {
        self.dispatch_behavior_event(|handler| handler.data_context_changed_event_handler());
    }

    /// Dispatches the `ResourcesChanged` event of the host.
    pub(crate) fn resources_changed(&self) {
        self.dispatch_behavior_event(|handler| handler.resources_changed_event_handler());
    }

    /// Dispatches the `ActualThemeVariantChanged` event of the host.
    pub(crate) fn actual_theme_variant_changed(&self) {
        self.dispatch_behavior_event(|handler| handler.actual_theme_variant_changed_event_handler());
    }

    /// Dispatches the `Opened` event of a top level host: its attachment to the visual and the
    /// logical tree, which a top level does not raise.
    pub(crate) fn opened(&self) {
        let was_synchronizing_collection = self.0.is_synchronizing_collection.replace(true);
        let finally = FinishDispatch { collection: self, was_synchronizing_collection };
        self.attached_to_visual_tree();
        self.attached_to_logical_tree();
        drop(finally);
    }

    fn dispatch_behavior_event(&self, dispatch: fn(&dyn IBehaviorEventsHandler)) {
        let was_synchronizing_collection = self.0.is_synchronizing_collection.replace(true);
        let finally = FinishDispatch { collection: self, was_synchronizing_collection };
        for item in self.0.list.snapshot().iter() {
            let (Some(behavior_events_handler), Some(behavior)) = (events_handler_of(item), behavior_of(item)) else {
                continue;
            };
            if behavior.associated_object().is_some() && !self.is_pending(item) {
                dispatch(behavior_events_handler);
            }
        }
        drop(finally);
    }

    /// The end of a dispatch (its `finally`): restores the flag and replays the behaviors that
    /// were added while the outermost dispatch ran.
    fn finish_dispatch(&self, was_synchronizing_collection: bool) {
        self.0.is_synchronizing_collection.set(was_synchronizing_collection);
        if !was_synchronizing_collection && !self.0.pending_synchronizations.borrow().is_empty() {
            let pending = std::mem::take(&mut *self.0.pending_synchronizations.borrow_mut());
            self.synchronize_behavior_events(pending);
        }
    }

    fn is_pending(&self, behavior: &Ref<FerroObject>) -> bool {
        self.0.pending_synchronizations.borrow().iter().any(|pending| pending == behavior)
    }

    fn behavior_collection_collection_changed(&self, event_args: &NotifyCollectionChangedEventArgs<'_, Ref<FerroObject>>) {
        if event_args.action == NotifyCollectionChangedAction::Reset {
            let old_collection = std::mem::take(&mut *self.0.old_collection.borrow_mut());
            for behavior in &old_collection {
                if let Some(behavior) = behavior_of(behavior) {
                    if behavior.associated_object().is_some() {
                        behavior.detach();
                    }
                }
            }

            let mut attached_behaviors = Vec::with_capacity(self.0.list.count());
            for new_item in self.0.list.snapshot().iter() {
                self.verified_attach(new_item);
                self.0.old_collection.borrow_mut().push(new_item.clone());
                attached_behaviors.push(new_item.clone());
            }

            if !self.0.is_attaching_collection.get() {
                self.queue_or_synchronize_behavior_events_many(attached_behaviors);
            }
            #[cfg(debug_assertions)]
            self.verify_old_collection_integrity();
            return;
        }

        match event_args.action {
            NotifyCollectionChangedAction::Add => {
                let event_index = event_args.new_starting_index.max(0) as usize;
                let Some(changed_item) = event_args.new_items.first().cloned() else {
                    return;
                };
                self.verified_attach(&changed_item);
                {
                    let mut old_collection = self.0.old_collection.borrow_mut();
                    let index = event_index.min(old_collection.len());
                    old_collection.insert(index, changed_item.clone());
                }
                if !self.0.is_attaching_collection.get() {
                    self.queue_or_synchronize_behavior_events(changed_item);
                }
            }

            NotifyCollectionChangedAction::Replace => {
                let event_index = event_args.old_starting_index;
                let event_index = if event_index == -1 { 0 } else { event_index as usize };

                let Some(changed_item) = event_args.new_items.first().cloned() else {
                    return;
                };

                let old_item = self.0.old_collection.borrow()[event_index].clone();
                if let Some(old_item) = behavior_of(&old_item) {
                    if old_item.associated_object().is_some() {
                        old_item.detach();
                    }
                }

                self.verified_attach(&changed_item);
                self.0.old_collection.borrow_mut()[event_index] = changed_item.clone();
                if !self.0.is_attaching_collection.get() {
                    self.queue_or_synchronize_behavior_events(changed_item);
                }
            }

            NotifyCollectionChangedAction::Remove => {
                let event_index = event_args.old_starting_index.max(0) as usize;

                let old_item = self.0.old_collection.borrow()[event_index].clone();
                if let Some(old_item) = behavior_of(&old_item) {
                    if old_item.associated_object().is_some() {
                        old_item.detach();
                    }
                }

                self.0.old_collection.borrow_mut().remove(event_index);
            }

            _ => {
                debug_assert!(false, "Unsupported collection operation attempted.");
            }
        }
        #[cfg(debug_assertions)]
        self.verify_old_collection_integrity();
    }

    fn verified_attach(&self, item: &Ref<FerroObject>) {
        let Some(behavior) = behavior_of(item) else {
            panic!("Only IBehavior types are supported in a BehaviorCollection.");
        };

        if self.0.old_collection.borrow().iter().any(|old| old == item) {
            panic!("Cannot add an instance of a behavior to a BehaviorCollection more than once.");
        }

        if let Some(associated_object) = self.associated_object() {
            behavior.attach(Some(associated_object));
        }
    }

    fn queue_or_synchronize_behavior_events(&self, behavior: Ref<FerroObject>) {
        if self.0.is_synchronizing_collection.get() || self.has_pending_host_lifecycle_event() {
            self.queue_pending_synchronization(behavior);
            return;
        }

        self.synchronize_behavior_events(vec![behavior]);
    }

    fn queue_or_synchronize_behavior_events_many(&self, behaviors: Vec<Ref<FerroObject>>) {
        if self.0.is_synchronizing_collection.get() || self.has_pending_host_lifecycle_event() {
            for behavior in behaviors {
                self.queue_pending_synchronization(behavior);
            }

            return;
        }

        self.synchronize_behavior_events(behaviors);
    }

    fn queue_pending_synchronization(&self, behavior: Ref<FerroObject>) {
        if !self.is_pending(&behavior) {
            self.0.pending_synchronizations.borrow_mut().push(behavior);
        }
    }

    fn has_pending_host_lifecycle_event(&self) -> bool {
        let associated_object = self.associated_object();
        let Some(associated) = associated_object.as_ref() else {
            return false;
        };
        let associated_object = Some(associated);

        if !self.0.has_observed_initialized.get() && is_initialized(associated_object) {
            return true;
        }

        let is_open_top_level = is_open_top_level(associated_object);
        if !self.0.has_observed_logical_attachment.get() && is_logically_attached(associated_object, is_open_top_level) {
            return true;
        }

        if !self.0.has_observed_visual_attachment.get() && is_visually_attached(associated_object, is_open_top_level) {
            return true;
        }

        !self.0.has_observed_loaded.get() && is_loaded(associated_object)
    }

    fn capture_current_lifecycle_state(&self) {
        let associated_object = self.associated_object();
        let associated_object = associated_object.as_ref();
        let is_open_top_level = is_open_top_level(associated_object);
        self.0.has_observed_initialized.set(is_initialized(associated_object));
        self.0.has_observed_logical_attachment.set(is_logically_attached(associated_object, is_open_top_level));
        self.0.has_observed_visual_attachment.set(is_visually_attached(associated_object, is_open_top_level));
        self.0.has_observed_loaded.set(is_loaded(associated_object));
    }

    fn synchronize_behavior_events(&self, behaviors: Vec<Ref<FerroObject>>) {
        self.0.is_synchronizing_collection.set(true);
        let _finally = FinishSynchronization(self);
        let mut current_batch = behaviors;
        for phase in 0..4 {
            let mut index = 0;
            while index < current_batch.len() {
                let behavior = current_batch[index].clone();
                Self::synchronize_phase(&behavior, phase);
                self.drain_pending_synchronizations(&mut current_batch, phase);
                index += 1;
            }
        }
    }

    fn drain_pending_synchronizations(&self, current_batch: &mut Vec<Ref<FerroObject>>, current_phase: i32) {
        let mut catch_up_batch: Vec<(Ref<FerroObject>, i32)> = Vec::new();
        loop {
            if !self.0.pending_synchronizations.borrow().is_empty() {
                let pending = std::mem::take(&mut *self.0.pending_synchronizations.borrow_mut());
                for behavior in pending {
                    if !current_batch.contains(&behavior) {
                        current_batch.push(behavior.clone());
                    }

                    let is_already_queued = catch_up_batch.iter().any(|(queued, _)| queued.ptr_eq(&behavior));
                    if !is_already_queued {
                        catch_up_batch.push((behavior, 0));
                    }
                }
            }

            let mut next_index: Option<usize> = None;
            let mut next_phase = i32::MAX;
            for (index, (_, candidate_phase)) in catch_up_batch.iter().enumerate() {
                if *candidate_phase <= current_phase && *candidate_phase < next_phase {
                    next_index = Some(index);
                    next_phase = *candidate_phase;
                }
            }

            let Some(next_index) = next_index else {
                return;
            };

            let (behavior, phase) = catch_up_batch[next_index].clone();
            Self::synchronize_phase(&behavior, phase);
            catch_up_batch[next_index] = (behavior, phase + 1);
        }
    }

    fn synchronize_phase(behavior: &Ref<FerroObject>, phase: i32) {
        match phase {
            0 => Self::synchronize_initialized(behavior),
            1 => Self::synchronize_logical_attachment(behavior),
            2 => Self::synchronize_visual_attachment(behavior),
            3 => Self::synchronize_loaded(behavior),
            _ => {}
        }
    }

    fn synchronize_initialized(behavior: &Ref<FerroObject>) {
        let (Some(events_handler), Some(behavior)) = (events_handler_of(behavior), behavior_of(behavior)) else {
            return;
        };
        if is_initialized(behavior.associated_object().as_ref()) {
            events_handler.initialized_event_handler();
        }
    }

    fn synchronize_logical_attachment(behavior: &Ref<FerroObject>) {
        let (Some(events_handler), Some(behavior)) = (events_handler_of(behavior), behavior_of(behavior)) else {
            return;
        };
        let associated_object = behavior.associated_object();
        let is_open_top_level = is_open_top_level(associated_object.as_ref());
        if is_logically_attached(associated_object.as_ref(), is_open_top_level) {
            events_handler.attached_to_logical_tree_event_handler();
        }
    }

    fn synchronize_visual_attachment(behavior: &Ref<FerroObject>) {
        let (Some(events_handler), Some(behavior)) = (events_handler_of(behavior), behavior_of(behavior)) else {
            return;
        };
        let associated_object = behavior.associated_object();
        let is_open_top_level = is_open_top_level(associated_object.as_ref());
        if is_visually_attached(associated_object.as_ref(), is_open_top_level) {
            events_handler.attached_to_visual_tree_event_handler();
        }
    }

    fn synchronize_loaded(behavior: &Ref<FerroObject>) {
        let (Some(events_handler), Some(behavior)) = (events_handler_of(behavior), behavior_of(behavior)) else {
            return;
        };
        if is_loaded(behavior.associated_object().as_ref()) {
            events_handler.loaded_event_handler();
        }
    }

    #[cfg(debug_assertions)]
    fn verify_old_collection_integrity(&self) {
        let items = self.0.list.snapshot();
        let old_collection = self.0.old_collection.borrow();
        let is_valid = items.len() == old_collection.len() && items.iter().zip(old_collection.iter()).all(|(a, b)| a == b);

        debug_assert!(is_valid, "Referential integrity of the collection has been compromised.");
    }
}

/// The `finally` of a dispatch of a host event ([`BehaviorCollection::finish_dispatch`]). While a
/// panic unwinds, only the flag is restored: no behavior is called back.
struct FinishDispatch<'a> {
    collection: &'a BehaviorCollection,
    was_synchronizing_collection: bool,
}

impl Drop for FinishDispatch<'_> {
    fn drop(&mut self) {
        if std::thread::panicking() {
            self.collection.0.is_synchronizing_collection.set(self.was_synchronizing_collection);
            return;
        }
        self.collection.finish_dispatch(self.was_synchronizing_collection);
    }
}

/// The `finally` of a replay of the lifecycle ([`BehaviorCollection::synchronize_behavior_events`]).
struct FinishSynchronization<'a>(&'a BehaviorCollection);

impl Drop for FinishSynchronization<'_> {
    fn drop(&mut self) {
        (self.0).0.is_synchronizing_collection.set(false);
        let pending = std::mem::take(&mut *(self.0).0.pending_synchronizations.borrow_mut());
        drop(pending);
    }
}
