//! Port of `Collections/CollectionChangedTrigger.cs`.

use super::collection_changed;
use ferroui_base::data::model::CollectionChange;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty,
};
use ferroui_controls::ItemsSource;
use ferroui_xaml_interactivity::{
    Interaction, StyledElementBehaviorImpl, StyledElementBehaviorImplExt, StyledElementTrigger,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Executes associated actions whenever the bound collection raises a collection changed event.
///
/// The description of the change is the parameter of the actions.
#[repr(C)]
pub struct CollectionChangedTrigger {
    base: StyledElementTrigger,
    // The subscription to the changes of the collection.
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(CollectionChangedTrigger: StyledElementTrigger);
ferro_class_info!(CollectionChangedTrigger {
    new: CollectionChangedTrigger::new,
    markup: {
        property_attributes: [
            Collection: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(CollectionChangedTrigger: StyledElementImpl);

ferro_properties! {
    impl CollectionChangedTrigger {
        /// Identifies the `Collection` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the collection (`INotifyCollectionChanged` of
        /// the reference) is an untyped list of the framework (`ItemsSource`); a list that does
        /// not notify its changes is not observed.
        pub fn collection_property() -> StyledProperty<Option<ItemsSource>> {
            FerroProperty::register::<CollectionChangedTrigger, _>("Collection", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for CollectionChangedTrigger {
        fn on_attached(this: &Self) {
            Self::parent_on_attached(this);

            this.observe_collection(this.collection());
        }

        fn on_detaching(this: &Self) {
            this.observe_collection(None);

            Self::parent_on_detaching(this);
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for CollectionChangedTrigger {
        fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
            Self::parent_on_property_changed(this, change);

            // Deviation (DEVIATIONS.md, Behaviors): the reference subscribes to a new collection
            // whether the trigger is attached or not, and again when it is attached, so a
            // collection set before attaching runs the actions twice per change; here the trigger
            // observes the collection only while it is attached, once (defect corrected).
            if change.property() == Self::collection_property().as_property() && this.associated_object().is_some() {
                this.observe_collection(this.collection());
            }
        }
    }
}

impl CollectionChangedTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementTrigger::construct(), subscription: RefCell::new(None) }
    }

    /// Creates a trigger.
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

    /// Replaces the subscription to the changes of a collection with one to `collection`.
    fn observe_collection(&self, collection: Option<ItemsSource>) {
        let previous = self.subscription.borrow_mut().take();
        if let Some(previous) = previous {
            previous.dispose();
        }

        let Some(collection) = collection else {
            return;
        };

        let trigger = self.to_ref().downgrade();
        let subscription = collection_changed::subscribe(&collection, move |e: CollectionChange| {
            if let Some(trigger) = trigger.upgrade() {
                trigger.on_collection_changed(e);
            }
        });
        *self.subscription.borrow_mut() = subscription;
    }

    fn on_collection_changed(&self, e: CollectionChange) {
        if !self.is_enabled() {
            return;
        }

        let sender = self.associated_object().map(|associated_object| Rc::new(associated_object) as BoxedValue);
        let parameter: BoxedValue = Rc::new(e);
        Interaction::execute_actions(sender.as_ref(), self.actions().as_ref(), Some(&parameter));
    }
}
