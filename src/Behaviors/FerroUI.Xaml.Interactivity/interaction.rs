//! Port of `Interaction.cs`.

use crate::{ActionCollection, BehaviorCollection, IAction};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_properties, ferro_static_type, AttachedProperty, BoxedValue, FerroObject, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, StyledElement, Visual, WeakRef,
};
use ferroui_controls::{Control, TopLevel};
use std::cell::RefCell;
use std::rc::Rc;

/// Defines a [`BehaviorCollection`] attached property and provides a method for executing an
/// [`ActionCollection`].
pub struct Interaction;

ferro_static_type!(Interaction);

ferro_properties! {
    impl Interaction {
        /// Gets or sets the [`BehaviorCollection`] associated with a specified object.
        pub fn behaviors_property() -> AttachedProperty<Option<BehaviorCollection>> {
            let property = FerroProperty::register_attached::<Interaction, FerroObject, Option<BehaviorCollection>>(
                "Behaviors",
                None,
            );
            // The static constructor of the reference: `BehaviorsProperty.Changed.Subscribe(..)`.
            let _ = property.changed().subscribe(Interaction::behaviors_changed);
            property
        }
    }
}

/// The owner of the handlers [`Interaction`] installs on a host.
///
/// Deviation (DEVIATIONS.md, Behaviors): the reference installs static event handlers on the
/// host, which it removes and adds again by delegate identity (`-=` then `+=`). A subscription of
/// the framework is a disposable, so the subscriptions of a host are kept with the host, in an
/// attached property of this private type, and replaced as a set.
struct InteractionHostHandlers;

ferro_static_type!(InteractionHostHandlers);

ferro_properties! {
    impl InteractionHostHandlers {
        fn host_handlers_property() -> AttachedProperty<Option<HostHandlers>> {
            FerroProperty::register_attached::<InteractionHostHandlers, FerroObject, Option<HostHandlers>>(
                "InteractionHostHandlers",
                None,
            )
        }
    }
}

/// The subscriptions [`Interaction`] made on a host.
#[derive(Clone)]
struct HostHandlers(Rc<RefCell<Vec<Rc<dyn IDisposable>>>>);

impl PartialEq for HostHandlers {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// Which set of handlers is installed on a host: the set of a collection created by the getter
/// (markup's `<Interaction.Behaviors>`), or the set of a collection assigned to the property.
#[derive(Clone, Copy, PartialEq, Eq)]
enum HandlerSet {
    FromGetter,
    FromChangedEvent,
}

impl Interaction {
    /// Gets the [`BehaviorCollection`] associated with a specified object.
    ///
    /// A collection is created and set if the object has none: the collection of markup's
    /// property element syntax (`<Interaction.Behaviors>`), which adds the behaviors to the
    /// collection the getter returns.
    pub fn get_behaviors(obj: &FerroObject) -> BehaviorCollection {
        let behavior_collection = obj.get_value(Self::behaviors_property());
        match behavior_collection {
            Some(behavior_collection) => behavior_collection,
            None => {
                let behavior_collection = BehaviorCollection::new();
                obj.set_value(Self::behaviors_property(), Some(behavior_collection.clone()));
                Self::set_visual_tree_event_handlers_from_getter(obj);
                behavior_collection
            }
        }
    }

    fn get_existing_behaviors(obj: &FerroObject) -> Option<BehaviorCollection> {
        obj.get_value(Self::behaviors_property())
    }

    /// Sets the [`BehaviorCollection`] associated with a specified object.
    pub fn set_behaviors(obj: &FerroObject, value: Option<BehaviorCollection>) {
        obj.set_value(Self::behaviors_property(), value);
    }

    /// Executes all actions in the [`ActionCollection`] and returns their results.
    ///
    /// `sender` is passed on to the actions; the value of `parameter` is determined by the calling
    /// behavior. Returns the results of the actions that returned one, in the order of the
    /// actions.
    pub fn execute_actions(
        sender: Option<&BoxedValue>,
        actions: Option<&ActionCollection>,
        parameter: Option<&BoxedValue>,
    ) -> Vec<BoxedValue> {
        let Some(actions) = actions else {
            return Vec::new();
        };

        let mut results = Vec::new();

        for ferro_object in actions.snapshot().iter() {
            let Some(action) = <dyn IAction>::of(ferro_object) else {
                continue;
            };

            if let Some(result) = action.execute(sender, parameter) {
                results.push(result);
            }
        }

        results
    }

    fn behaviors_changed(e: &FerroPropertyChangedEventArgs<'_>) {
        let old_collection = e.get_old_value::<Option<BehaviorCollection>>().flatten();
        let new_collection = e.get_new_value::<Option<BehaviorCollection>>();

        if old_collection == new_collection {
            return;
        }

        let sender = e.sender().to_ref();
        let is_attached_to_visual_tree =
            sender.downcast_ref::<Visual>().is_some_and(|visual| visual.is_attached_to_visual_tree());

        if let Some(old_collection) = &old_collection {
            if old_collection.associated_object().is_some() {
                if is_attached_to_visual_tree {
                    old_collection.detached_from_visual_tree();
                }

                old_collection.detach();
            }
        }

        if let Some(new_collection) = &new_collection {
            Self::set_visual_tree_event_handlers_from_changed_event(&sender);
            new_collection.attach(&sender);

            if is_attached_to_visual_tree {
                new_collection.attached_to_visual_tree();
            }
        }
    }

    fn set_visual_tree_event_handlers_from_getter(obj: &FerroObject) {
        Self::set_visual_tree_event_handlers(obj, HandlerSet::FromGetter);
    }

    fn set_visual_tree_event_handlers_from_changed_event(obj: &FerroObject) {
        Self::set_visual_tree_event_handlers(obj, HandlerSet::FromChangedEvent);
    }

    /// Installs a set of handlers on the host, after removing the set installed before (the
    /// reference removes the handlers of both sets and adds the ones of `set`, event by event,
    /// which puts them last on each event, as here).
    fn set_visual_tree_event_handlers(obj: &FerroObject, set: HandlerSet) {
        let property = InteractionHostHandlers::host_handlers_property();
        let handlers = match obj.get_value(property) {
            Some(handlers) => handlers,
            None => {
                let handlers = HostHandlers(Rc::new(RefCell::new(Vec::new())));
                obj.set_value(property, Some(handlers.clone()));
                handlers
            }
        };
        let previous = std::mem::take(&mut *handlers.0.borrow_mut());
        for subscription in previous {
            subscription.dispose();
        }

        let host = obj.to_weak();
        let mut subscriptions: Vec<Rc<dyn IDisposable>> = Vec::new();

        if let Some(visual) = obj.downcast_ref::<Visual>() {
            // AttachedToVisualTree / DetachedFromVisualTree

            let weak = host.clone();
            subscriptions.push(visual.attached_to_visual_tree(move |_| {
                if let Some(d) = weak.upgrade() {
                    match set {
                        HandlerSet::FromGetter => Self::visual_attached_to_visual_tree_from_getter(&d),
                        HandlerSet::FromChangedEvent => Self::visual_attached_to_visual_tree_from_changed_event(&d),
                    }
                }
            }));
            let weak = host.clone();
            subscriptions.push(visual.detached_from_visual_tree(move |_| {
                if let Some(d) = weak.upgrade() {
                    match set {
                        HandlerSet::FromGetter => Self::visual_detached_from_visual_tree_from_getter(&d),
                        HandlerSet::FromChangedEvent => Self::visual_detached_from_visual_tree_from_changed_event(&d),
                    }
                }
            }));
        }

        if let Some(styled_element) = obj.downcast_ref::<StyledElement>() {
            // AttachedToLogicalTree / DetachedFromLogicalTree

            let weak = host.clone();
            subscriptions.push(styled_element.attached_to_logical_tree(move |_| {
                Self::with_existing_behaviors(&weak, BehaviorCollection::attached_to_logical_tree);
            }));
            let weak = host.clone();
            subscriptions.push(styled_element.detached_from_logical_tree(move |_| {
                Self::with_existing_behaviors(&weak, BehaviorCollection::detached_from_logical_tree);
            }));

            // Initialized

            let weak = host.clone();
            subscriptions.push(styled_element.initialized(move || {
                Self::with_existing_behaviors(&weak, BehaviorCollection::initialized);
            }));

            // DataContextChanged

            let weak = host.clone();
            subscriptions.push(styled_element.data_context_changed(move || {
                Self::with_existing_behaviors(&weak, BehaviorCollection::data_context_changed);
            }));

            // ResourcesChanged

            let weak = host.clone();
            subscriptions.push(styled_element.resources_changed(move |_| {
                Self::with_existing_behaviors(&weak, BehaviorCollection::resources_changed);
            }));

            // ActualThemeVariantChanged

            let weak = host.clone();
            subscriptions.push(styled_element.actual_theme_variant_changed(move || {
                Self::with_existing_behaviors(&weak, BehaviorCollection::actual_theme_variant_changed);
            }));
        }

        if let Some(control) = obj.downcast_ref::<Control>() {
            // Loaded / Unloaded

            let weak = host.clone();
            let token = control.loaded(move |_, _| {
                Self::with_existing_behaviors(&weak, BehaviorCollection::loaded);
            });
            let control_weak = control.to_ref().downgrade();
            subscriptions.push(Disposable::create(move || {
                if let Some(control) = control_weak.upgrade() {
                    control.remove_handler(Control::loaded_event(), token);
                }
            }));

            let weak = host.clone();
            let token = control.unloaded(move |_, _| {
                Self::with_existing_behaviors(&weak, BehaviorCollection::unloaded);
            });
            let control_weak = control.to_ref().downgrade();
            subscriptions.push(Disposable::create(move || {
                if let Some(control) = control_weak.upgrade() {
                    control.remove_handler(Control::unloaded_event(), token);
                }
            }));
        }

        if let Some(top_level) = obj.downcast_ref::<TopLevel>() {
            let weak = host.clone();
            subscriptions.push(top_level.opened(move || {
                if let Some(d) = weak.upgrade() {
                    match set {
                        HandlerSet::FromGetter => Self::top_level_opened_from_getter(&d),
                        HandlerSet::FromChangedEvent => Self::top_level_opened_from_changed_event(&d),
                    }
                }
            }));
        }

        *handlers.0.borrow_mut() = subscriptions;
    }

    /// Calls `f` with the collection of the host, if the host is alive and has one. The handlers
    /// of the logical tree, `Loaded` / `Unloaded`, `Initialized`, `DataContextChanged`,
    /// `ResourcesChanged` and `ActualThemeVariantChanged` are the same in both sets.
    fn with_existing_behaviors(host: &WeakRef<FerroObject>, f: fn(&BehaviorCollection)) {
        let Some(d) = host.upgrade() else {
            return;
        };

        if let Some(behaviors) = Self::get_existing_behaviors(&d) {
            f(&behaviors);
        }
    }

    // AttachedToVisualTree / DetachedFromVisualTree

    fn visual_attached_to_visual_tree_from_getter(d: &Ref<FerroObject>) {
        let behaviors = Self::get_existing_behaviors(d);
        if let Some(behaviors) = &behaviors {
            behaviors.attach(d);
            behaviors.attached_to_visual_tree();
        }
    }

    fn visual_detached_from_visual_tree_from_getter(d: &Ref<FerroObject>) {
        let Some(behaviors) = Self::get_existing_behaviors(d) else {
            return;
        };

        behaviors.detached_from_visual_tree();

        if let Some(top_level) = d.cast::<TopLevel>() {
            Self::schedule_top_level_behavior_detach(top_level, behaviors);
        } else {
            behaviors.detach();
        }
    }

    fn visual_attached_to_visual_tree_from_changed_event(d: &Ref<FerroObject>) {
        if let Some(behaviors) = Self::get_existing_behaviors(d) {
            behaviors.attached_to_visual_tree();
        }
    }

    fn visual_detached_from_visual_tree_from_changed_event(d: &Ref<FerroObject>) {
        let Some(behaviors) = Self::get_existing_behaviors(d) else {
            return;
        };

        behaviors.detached_from_visual_tree();

        if let Some(top_level) = d.cast::<TopLevel>() {
            Self::schedule_top_level_behavior_detach(top_level, behaviors);
        }
    }

    fn schedule_top_level_behavior_detach(top_level: Ref<TopLevel>, behaviors: BehaviorCollection) {
        Dispatcher::ui_thread().post_local(
            move || {
                if !top_level.is_attached_to_visual_tree()
                    && top_level.get_value(Self::behaviors_property()).as_ref() == Some(&behaviors)
                    && behaviors.associated_object().is_some()
                {
                    behaviors.detach();
                }
            },
            DispatcherPriority::DEFAULT,
        );
    }

    // TopLevel Opened

    fn top_level_opened_from_getter(d: &Ref<FerroObject>) {
        let behaviors = Self::get_existing_behaviors(d);
        if let Some(behaviors) = &behaviors {
            behaviors.attach(d);
            behaviors.opened();
        }
    }

    fn top_level_opened_from_changed_event(d: &Ref<FerroObject>) {
        if let Some(behaviors) = Self::get_existing_behaviors(d) {
            behaviors.opened();
        }
    }
}
