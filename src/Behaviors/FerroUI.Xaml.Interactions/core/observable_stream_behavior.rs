//! Port of `Core/ObservableStreamBehavior.cs`.

use super::{attach_actions_to_logical_tree, detach_actions_from_logical_tree};
use ferroui_base::logical_tree::LogicalTreeAttachmentEventArgs;
use ferroui_base::reactive::{AnonymousObserver, IDisposable, IObservable, ObservableError};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledElementImplExt, StyledProperty,
    WeakRef,
};
use ferroui_xaml_interactivity::{ActionCollection, Interaction, StyledElementBehavior, StyledElementBehaviorImpl};
use std::cell::RefCell;
use std::rc::Rc;

/// A behavior that subscribes to an observable and executes actions on its values, its error and
/// its completion.
///
/// The [`source`](Self::source) is an observable of untyped values
/// (`Rc<dyn IObservable<Option<BoxedValue>>>` or `Rc<dyn IObservable<BoxedValue>>`, the
/// `IObservable<object>` of the reference). The subscription lives while the behavior is in the
/// logical tree; the actions run in jobs of the dispatcher.
#[repr(C)]
pub struct ObservableStreamBehavior {
    base: StyledElementBehavior,
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(ObservableStreamBehavior: StyledElementBehavior);
ferro_class_info!(ObservableStreamBehavior {
    new: ObservableStreamBehavior::new,
    markup: {
        content: Actions,
    },
});
ferroui_base::ferro_impl_classes!(ObservableStreamBehavior: StyledElementBehaviorImpl);

ferro_properties! {
    impl ObservableStreamBehavior {
        /// Identifies the `Source` property.
        pub fn source_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<ObservableStreamBehavior, _>("Source", None)
        }

        /// Identifies the `Actions` property.
        pub fn actions_property() -> StyledProperty<Option<ActionCollection>> {
            FerroProperty::register::<ObservableStreamBehavior, _>("Actions", None)
        }

        /// Identifies the `ErrorActions` property.
        pub fn error_actions_property() -> StyledProperty<Option<ActionCollection>> {
            FerroProperty::register::<ObservableStreamBehavior, _>("ErrorActions", None)
        }

        /// Identifies the `CompletedActions` property.
        pub fn completed_actions_property() -> StyledProperty<Option<ActionCollection>> {
            FerroProperty::register::<ObservableStreamBehavior, _>("CompletedActions", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for ObservableStreamBehavior {
        /// The constructor body: the behavior starts with empty collections of actions.
        fn constructed(this: &Self) {
            Self::parent_constructed(this);
            this.set_current_value(Self::actions_property(), Some(ActionCollection::new()));
            this.set_current_value(Self::error_actions_property(), Some(ActionCollection::new()));
            this.set_current_value(Self::completed_actions_property(), Some(ActionCollection::new()));
        }

        fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
            Self::parent_on_property_changed(this, change);

            let property = change.property();
            if property == Self::source_property().as_property() {
                this.update_subscription();
            } else if property == Self::actions_property().as_property()
                || property == Self::error_actions_property().as_property()
                || property == Self::completed_actions_property().as_property()
            {
                this.handle_action_collection_changed(change);
            }
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementImpl for ObservableStreamBehavior {
        fn on_attached_to_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
            Self::parent_on_attached_to_logical_tree(this, e);
            attach_actions_to_logical_tree(this.actions().as_ref(), this);
            attach_actions_to_logical_tree(this.error_actions().as_ref(), this);
            attach_actions_to_logical_tree(this.completed_actions().as_ref(), this);
            this.update_subscription();
        }

        fn on_detached_from_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
            let subscription = this.subscription.borrow_mut().take();
            if let Some(subscription) = subscription {
                subscription.dispose();
            }
            detach_actions_from_logical_tree(this.actions().as_ref(), this);
            detach_actions_from_logical_tree(this.error_actions().as_ref(), this);
            detach_actions_from_logical_tree(this.completed_actions().as_ref(), this);
            Self::parent_on_detached_from_logical_tree(this, e);
        }
    }
}

/// The observable an untyped source is (`Source is IObservable<object> observable`): its values
/// are given to the actions as untyped values.
fn observable_of(source: &BoxedValue) -> Option<Rc<dyn IObservable<Option<BoxedValue>>>> {
    if let Some(observable) = source.downcast_ref::<Rc<dyn IObservable<Option<BoxedValue>>>>() {
        return Some(observable.clone());
    }

    if let Some(observable) = source.downcast_ref::<Rc<dyn IObservable<BoxedValue>>>() {
        let observable = observable.clone();
        return Some(ferroui_base::reactive::Observable::create(move |observer| {
            observable.subscribe(Rc::new(ForwardingObserver { target: observer }))
        }));
    }

    None
}

/// Gives the values of an observable of untyped values to an observer of optional untyped values.
struct ForwardingObserver {
    target: Rc<dyn ferroui_base::reactive::IObserver<Option<BoxedValue>>>,
}

impl ferroui_base::reactive::IObserver<BoxedValue> for ForwardingObserver {
    fn on_next(&self, value: BoxedValue) {
        self.target.on_next(Some(value));
    }

    fn on_error(&self, error: ObservableError) {
        self.target.on_error(error);
    }

    fn on_completed(&self) {
        self.target.on_completed();
    }
}

impl ObservableStreamBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementBehavior::construct(), subscription: RefCell::new(None) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the source observable. This is a property of the object model.
    pub fn source(&self) -> Option<BoxedValue> {
        self.get_value(Self::source_property())
    }

    pub fn set_source(&self, value: Option<BoxedValue>) {
        self.set_value(Self::source_property(), value)
    }

    /// Gets or sets the collection of actions to execute when the observable emits a value. This
    /// is a property of the object model.
    pub fn actions(&self) -> Option<ActionCollection> {
        self.get_value(Self::actions_property())
    }

    pub fn set_actions(&self, value: Option<ActionCollection>) {
        self.set_value(Self::actions_property(), value)
    }

    /// Gets or sets the collection of actions to execute when the observable emits an error. This
    /// is a property of the object model.
    pub fn error_actions(&self) -> Option<ActionCollection> {
        self.get_value(Self::error_actions_property())
    }

    pub fn set_error_actions(&self, value: Option<ActionCollection>) {
        self.set_value(Self::error_actions_property(), value)
    }

    /// Gets or sets the collection of actions to execute when the observable completes. This is a
    /// property of the object model.
    pub fn completed_actions(&self) -> Option<ActionCollection> {
        self.get_value(Self::completed_actions_property())
    }

    pub fn set_completed_actions(&self, value: Option<ActionCollection>) {
        self.set_value(Self::completed_actions_property(), value)
    }

    fn handle_action_collection_changed(&self, change: &FerroPropertyChangedEventArgs<'_>) {
        let old_actions = change.get_old_value::<Option<ActionCollection>>().flatten();
        let new_actions = change.get_new_value::<Option<ActionCollection>>();

        if let Some(old_actions) = &old_actions {
            if self.is_attached_to_logical_tree() {
                detach_actions_from_logical_tree(Some(old_actions), self);
            }
        }

        if let Some(new_actions) = &new_actions {
            if self.is_attached_to_logical_tree() {
                attach_actions_to_logical_tree(Some(new_actions), self);
            }
        }
    }

    fn update_subscription(&self) {
        let subscription = self.subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }

        let Some(observable) = self.source().as_ref().and_then(observable_of) else {
            return;
        };

        let on_next = {
            let behavior: WeakRef<ObservableStreamBehavior> = self.to_ref().downgrade();
            move |value: Option<BoxedValue>| {
                let behavior = behavior.clone();
                Dispatcher::ui_thread().post_local(
                    move || {
                        if let Some(behavior) = behavior.upgrade() {
                            let sender = behavior.associated_sender();
                            Interaction::execute_actions(sender.as_ref(), behavior.actions().as_ref(), value.as_ref());
                        }
                    },
                    DispatcherPriority::DEFAULT,
                );
            }
        };

        let on_error = {
            let behavior: WeakRef<ObservableStreamBehavior> = self.to_ref().downgrade();
            move |error: ObservableError| {
                let behavior = behavior.clone();
                // Deviation (DEVIATIONS.md, Behaviors): the error actions are given the message of
                // the error (an error of the observable is not an untyped value).
                let error: BoxedValue = Rc::new(error.to_string());
                Dispatcher::ui_thread().post_local(
                    move || {
                        if let Some(behavior) = behavior.upgrade() {
                            let sender = behavior.associated_sender();
                            Interaction::execute_actions(sender.as_ref(), behavior.error_actions().as_ref(), Some(&error));
                        }
                    },
                    DispatcherPriority::DEFAULT,
                );
            }
        };

        let on_completed = {
            let behavior: WeakRef<ObservableStreamBehavior> = self.to_ref().downgrade();
            move || {
                let behavior = behavior.clone();
                Dispatcher::ui_thread().post_local(
                    move || {
                        if let Some(behavior) = behavior.upgrade() {
                            let sender = behavior.associated_sender();
                            Interaction::execute_actions(sender.as_ref(), behavior.completed_actions().as_ref(), None);
                        }
                    },
                    DispatcherPriority::DEFAULT,
                );
            }
        };

        let subscription =
            observable.subscribe(Rc::new(AnonymousObserver::new_with_error_and_completed(on_next, on_error, on_completed)));
        *self.subscription.borrow_mut() = Some(subscription);
    }

    /// The associated object as the sender of the actions.
    fn associated_sender(&self) -> Option<BoxedValue> {
        self.associated_object().map(|associated_object| Rc::new(associated_object) as BoxedValue)
    }
}
