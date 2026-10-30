//! Port of `Collections/CollectionChangedBehavior.cs`.

use super::collection_changed;
use ferroui_base::collections::NotifyCollectionChangedAction;
use ferroui_base::data::model::CollectionChange;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, DirectProperty, FerroObject,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StaticType,
    StyledElementImpl, StyledProperty,
};
use ferroui_controls::ItemsSource;
use ferroui_xaml_interactivity::{
    ActionCollection, DisposableAction, DisposingBehavior, DisposingBehaviorImpl, Interaction,
    StyledElementBehaviorImpl,
};
use std::cell::{OnceCell, RefCell};
use std::rc::Rc;

/// Executes different sets of actions when the observed collection changes: the
/// [`added_actions`](Self::added_actions) when items are added, the
/// [`removed_actions`](Self::removed_actions) when items are removed and the
/// [`reset_actions`](Self::reset_actions) when the collection is reset.
///
/// The description of the change is the parameter of the actions.
#[repr(C)]
pub struct CollectionChangedBehavior {
    base: DisposingBehavior,
    added_actions: OnceCell<ActionCollection>,
    removed_actions: OnceCell<ActionCollection>,
    reset_actions: OnceCell<ActionCollection>,
    // The subscription to the changes of the collection.
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(CollectionChangedBehavior: DisposingBehavior);
ferro_class_info!(CollectionChangedBehavior {
    new: CollectionChangedBehavior::new,
    markup: {
        property_attributes: [
            Collection: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(CollectionChangedBehavior: StyledElementImpl, StyledElementBehaviorImpl);

ferro_properties! {
    impl CollectionChangedBehavior {
        /// Identifies the `Collection` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the collection (`INotifyCollectionChanged` of
        /// the reference) is an untyped list of the framework (`ItemsSource`); a list that does
        /// not notify its changes is not observed.
        pub fn collection_property() -> StyledProperty<Option<ItemsSource>> {
            FerroProperty::register::<CollectionChangedBehavior, _>("Collection", None)
        }

        /// Identifies the `AddedActions` property.
        pub fn added_actions_property() -> DirectProperty<CollectionChangedBehavior, ActionCollection> {
            FerroProperty::register_direct::<CollectionChangedBehavior, _>(
                "AddedActions",
                |behavior| behavior.added_actions(),
                None,
                ActionCollection::new(),
            )
        }

        /// Identifies the `RemovedActions` property.
        pub fn removed_actions_property() -> DirectProperty<CollectionChangedBehavior, ActionCollection> {
            FerroProperty::register_direct::<CollectionChangedBehavior, _>(
                "RemovedActions",
                |behavior| behavior.removed_actions(),
                None,
                ActionCollection::new(),
            )
        }

        /// Identifies the `ResetActions` property.
        pub fn reset_actions_property() -> DirectProperty<CollectionChangedBehavior, ActionCollection> {
            FerroProperty::register_direct::<CollectionChangedBehavior, _>(
                "ResetActions",
                |behavior| behavior.reset_actions(),
                None,
                ActionCollection::new(),
            )
        }
    }
}

ferroui_base::ferro_overrides! {
    impl DisposingBehaviorImpl for CollectionChangedBehavior {
        fn on_attached_override(this: &Self) -> Rc<dyn IDisposable> {
            this.observe_collection(this.collection());

            let behavior = this.to_ref().downgrade();
            DisposableAction::create(move || {
                if let Some(behavior) = behavior.upgrade() {
                    behavior.observe_collection(None);
                }
            })
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for CollectionChangedBehavior {
        fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
            Self::parent_on_property_changed(this, change);

            // Deviation (DEVIATIONS.md, Behaviors): the reference subscribes to a new collection
            // whether the behavior is attached or not, and again when it is attached, so a
            // collection set before attaching runs the actions twice per change; here the
            // behavior observes the collection only while it is attached, once (defect
            // corrected).
            if change.property() == Self::collection_property().as_property() && this.associated_object().is_some() {
                this.observe_collection(this.collection());
            }
        }
    }
}

impl CollectionChangedBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: DisposingBehavior::construct(<FerroObject as StaticType>::TYPE),
            added_actions: OnceCell::new(),
            removed_actions: OnceCell::new(),
            reset_actions: OnceCell::new(),
            subscription: RefCell::new(None),
        }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the collection to observe. Markup resolves a name given for it to the named
    /// element.
    pub fn collection(&self) -> Option<ItemsSource> {
        self.get_value(Self::collection_property())
    }

    pub fn set_collection(&self, value: Option<ItemsSource>) {
        self.set_value(Self::collection_property(), value)
    }

    /// Actions invoked when items are added to the collection.
    pub fn added_actions(&self) -> ActionCollection {
        self.added_actions.get_or_init(ActionCollection::new).clone()
    }

    /// Actions invoked when items are removed from the collection.
    pub fn removed_actions(&self) -> ActionCollection {
        self.removed_actions.get_or_init(ActionCollection::new).clone()
    }

    /// Actions invoked when the collection is reset.
    pub fn reset_actions(&self) -> ActionCollection {
        self.reset_actions.get_or_init(ActionCollection::new).clone()
    }

    /// Replaces the subscription to the changes of a collection with one to `collection`.
    fn observe_collection(&self, collection: Option<ItemsSource>) {
        let previous = self.subscription.borrow_mut().take();
        if let Some(previous) = previous {
            previous.dispose();
        }

        let Some(collection) = collection else {
            return;
        };

        let behavior = self.to_ref().downgrade();
        let subscription = collection_changed::subscribe(&collection, move |e: CollectionChange| {
            if let Some(behavior) = behavior.upgrade() {
                behavior.on_collection_changed(e);
            }
        });
        *self.subscription.borrow_mut() = subscription;
    }

    fn on_collection_changed(&self, e: CollectionChange) {
        if !self.is_enabled() {
            return;
        }

        let actions = match e.action {
            NotifyCollectionChangedAction::Add => Some(self.added_actions()),
            NotifyCollectionChangedAction::Remove => Some(self.removed_actions()),
            NotifyCollectionChangedAction::Reset => Some(self.reset_actions()),
            _ => None,
        };

        if let Some(actions) = actions {
            let sender = self.associated_object().map(|associated_object| Rc::new(associated_object) as BoxedValue);
            let parameter: BoxedValue = Rc::new(e);
            Interaction::execute_actions(sender.as_ref(), Some(&actions), Some(&parameter));
        }
    }
}
