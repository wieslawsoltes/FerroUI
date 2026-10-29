//! Port of `Triggers/EventTriggerBase.cs`.

use crate::helpers::member_lookup::{find_plain_event, find_routed_event, object_of, subscribe_plain_event};
use crate::{
    AddEventHandlerRegistry, DisposableAction, Interaction, StyledElementBehaviorImpl, StyledElementBehaviorImplExt,
    StyledElementTrigger,
};
use ferroui_base::data::core::{ValueTypes, WeakValue};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RouteHandler};
use ferroui_base::metadata::{MarkupDelegate, MarkupValue};
use ferroui_base::reactive::IDisposable;
use ferroui_base::utilities::EventArgs;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, BoxedValue, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, StyledElementImpl, StyledProperty, WeakRef,
};
use ferroui_controls::Control;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const EVENT_NAME_DEFAULT_VALUE: &str = "AttachedToVisualTree";

/// A behavior that listens for a specified event on its source and executes its actions when that
/// event is fired.
///
/// The event is found by name: first in the [`AddEventHandlerRegistry`], then among the routed
/// events of the source and the plain events its markup metadata declares (the reflection of the
/// reference; see [`member_lookup`](crate::helpers::member_lookup)). The default event,
/// `AttachedToVisualTree`, fires when a control without a parent is attached to the visual tree.
#[repr(C)]
pub struct EventTriggerBase {
    base: StyledElementTrigger,
    // Deviation (DEVIATIONS.md, Behaviors): the source is held weakly when it is an object (it is
    // usually the host, which holds the trigger).
    resolved_source: RefCell<Option<WeakValue>>,
    event_handler: RefCell<Option<Rc<dyn IDisposable>>>,
    is_loaded_event_registered: Cell<bool>,
    loaded_event_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    try_register_event_handler_disposable: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class! {
    EventTriggerBase: StyledElementTrigger, virtuals EventTriggerBaseImpl: StyledElementBehaviorImpl {
        /// Raised when the event fires (by its name in the reference, the handler of the default
        /// event `AttachedToVisualTree`).
        fn attached_to_visual_tree(this, sender: Option<&BoxedValue>, event_args: &BoxedValue);
        /// Executes the actions associated with this behavior.
        fn on_event(this, event_args: Option<&BoxedValue>);
    }
}
ferro_class_info!(EventTriggerBase {
    markup: {
        property_attributes: [
            SourceObject: [ResolveByName],
        ],
    },
});

ferroui_base::ferro_impl_classes!(EventTriggerBase: StyledElementImpl);

impl EventTriggerBaseImpl for EventTriggerBase {
    fn attached_to_visual_tree(this: &Self, _sender: Option<&BoxedValue>, event_args: &BoxedValue) {
        this.on_event(Some(event_args));
    }

    fn on_event(this: &Self, event_args: Option<&BoxedValue>) {
        if !this.is_enabled() {
            return;
        }

        let resolved_source = this.resolved_source();
        Interaction::execute_actions(resolved_source.as_ref(), this.actions().as_ref(), event_args);
    }
}

ferro_properties! {
    impl EventTriggerBase {
        /// Identifies the `EventName` property.
        pub fn event_name_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<EventTriggerBase, _>("EventName", Some(EVENT_NAME_DEFAULT_VALUE.to_string()))
        }

        /// Identifies the `SourceObject` property.
        pub fn source_object_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<EventTriggerBase, _>("SourceObject", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for EventTriggerBase {
        fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
            Self::parent_on_property_changed(this, change);

            if change.property() == Self::event_name_property().as_property() {
                let resolved_source = this.resolved_source();
                let old_event_name = change.get_old_value::<Option<String>>().flatten();
                if let Some(old_event_name) = old_event_name {
                    this.unregister_event(Some(&old_event_name), resolved_source.as_ref());
                }

                let new_event_name = change.get_new_value::<Option<String>>();
                if let Some(new_event_name) = new_event_name {
                    this.register_event(Some(&new_event_name), resolved_source.as_ref());
                }
            } else if change.property() == Self::source_object_property().as_property() {
                this.set_resolved_source(this.event_name().as_deref(), this.source_object().or_else(|| this.associated_source()));
            }
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for EventTriggerBase {
        /// Called after the behavior is attached to the associated object.
        fn on_attached(this: &Self) {
            Self::parent_on_attached(this);

            this.set_resolved_source(this.event_name().as_deref(), this.source_object().or_else(|| this.associated_source()));
        }

        /// Called when the behavior is being detached from its associated object.
        fn on_detaching(this: &Self) {
            Self::parent_on_detaching(this);
            this.set_resolved_source(this.event_name().as_deref(), None);
        }
    }
}

/// Whether two sources are the same object (`==` of the reference: reference equality).
fn same_source(a: Option<&BoxedValue>, b: Option<&BoxedValue>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => match (object_of(a), object_of(b)) {
            (Some(a), Some(b)) => a.ptr_eq(&b),
            (None, None) => ValueTypes::identity_equals(Some(a), Some(b)),
            _ => false,
        },
        _ => false,
    }
}

impl EventTriggerBase {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElementTrigger::construct(),
            resolved_source: RefCell::new(None),
            event_handler: RefCell::new(None),
            is_loaded_event_registered: Cell::new(false),
            loaded_event_subscription: RefCell::new(None),
            try_register_event_handler_disposable: RefCell::new(None),
        }
    }

    /// Gets or sets the name of the event to listen for.
    pub fn event_name(&self) -> Option<String> {
        self.get_value(Self::event_name_property())
    }

    pub fn set_event_name(&self, value: Option<String>) {
        self.set_value(Self::event_name_property(), value)
    }

    /// Gets or sets the source object from which this behavior listens for events. If it is not
    /// set, the source defaults to the associated object. Markup resolves a name given for it to
    /// the named element.
    pub fn source_object(&self) -> Option<BoxedValue> {
        self.get_value(Self::source_object_property())
    }

    pub fn set_source_object(&self, value: Option<BoxedValue>) {
        self.set_value(Self::source_object_property(), value)
    }

    /// The source the trigger listens to.
    pub fn resolved_source(&self) -> Option<BoxedValue> {
        self.resolved_source.borrow().as_ref().and_then(WeakValue::upgrade)
    }

    /// The associated object as an untyped value.
    fn associated_source(&self) -> Option<BoxedValue> {
        self.associated_object().map(|associated_object| Rc::new(associated_object) as BoxedValue)
    }

    fn set_resolved_source(&self, event_name: Option<&str>, new_source: Option<BoxedValue>) {
        let resolved_source = self.resolved_source();
        if same_source(resolved_source.as_ref(), new_source.as_ref()) {
            return;
        }

        if resolved_source.is_some() {
            self.unregister_event(event_name, resolved_source.as_ref());
        }

        *self.resolved_source.borrow_mut() = new_source.as_ref().map(WeakValue::new);

        if new_source.is_some() {
            self.register_event(event_name, new_source.as_ref());
        }
    }

    fn register_event(&self, event_name: Option<&str>, resolved_source: Option<&BoxedValue>) {
        let Some(event_name) = event_name.filter(|name| !name.is_empty()) else {
            return;
        };

        if event_name != EVENT_NAME_DEFAULT_VALUE {
            self.register_user_event(event_name, resolved_source);
        } else if !self.is_loaded_event_registered.get() {
            self.register_default_event(resolved_source);
        }
    }

    fn unregister_event(&self, event_name: Option<&str>, resolved_source: Option<&BoxedValue>) {
        let Some(event_name) = event_name.filter(|name| !name.is_empty()) else {
            return;
        };

        if event_name != EVENT_NAME_DEFAULT_VALUE {
            self.unregister_user_event(event_name, resolved_source);
        } else if self.is_loaded_event_registered.get() {
            self.unregister_default_event(resolved_source);
        }
    }

    /// The handler of the event: calls [`attached_to_visual_tree`](Self::attached_to_visual_tree)
    /// on the trigger while it is alive.
    fn handler(&self) -> Rc<dyn Fn(Option<&BoxedValue>, &BoxedValue)> {
        let this: WeakRef<EventTriggerBase> = self.to_ref().downgrade();
        Rc::new(move |sender: Option<&BoxedValue>, event_args: &BoxedValue| {
            if let Some(this) = this.upgrade() {
                this.attached_to_visual_tree(sender, event_args);
            }
        })
    }

    fn register_user_event(&self, event_name: &str, resolved_source: Option<&BoxedValue>) {
        let Some(resolved_source) = resolved_source else {
            return;
        };

        let disposable = AddEventHandlerRegistry::try_register_event_handler(resolved_source, event_name, self.handler());
        if let Some(disposable) = disposable {
            *self.try_register_event_handler_disposable.borrow_mut() = Some(disposable);
            return;
        }

        self.add_event_handler(event_name, resolved_source);
    }

    fn unregister_user_event(&self, _event_name: &str, resolved_source: Option<&BoxedValue>) {
        let disposable = self.try_register_event_handler_disposable.borrow_mut().take();
        if let Some(disposable) = disposable {
            disposable.dispose();
            return;
        }

        if self.event_handler.borrow().is_none() {
            return;
        }

        let event_handler = self.event_handler.borrow_mut().take();
        if resolved_source.is_some() {
            if let Some(event_handler) = event_handler {
                event_handler.dispose();
            }
        }
    }

    /// Subscribes to the event of the source by name: a routed event of the class of the source
    /// (or of a base class), else a plain event its markup metadata declares (`GetRuntimeEvent`
    /// and `EventInfo.AddEventHandler` of the reference). Does nothing when the source has no
    /// such event.
    fn add_event_handler(&self, event_name: &str, resolved_source: &BoxedValue) {
        let handler = self.handler();

        if let Some(interactive) = object_of(resolved_source).and_then(|object| object.cast::<Interactive>()) {
            if let Some(routed_event) = find_routed_event(interactive.get_type(), event_name) {
                let route_handler: RouteHandler = Rc::new(move |sender: &Interactive, event_args: &dyn IRoutedEventArgs| {
                    let sender: BoxedValue = Rc::new(sender.to_ref());
                    let event_args: BoxedValue = Rc::new(event_args.share());
                    handler(Some(&sender), &event_args);
                });
                let token =
                    interactive.add_route_handler(&routed_event, route_handler, Interactive::DEFAULT_ROUTES, false);
                let interactive = interactive.downgrade();
                *self.event_handler.borrow_mut() = Some(DisposableAction::create(move || {
                    if let Some(interactive) = interactive.upgrade() {
                        interactive.remove_handler(&routed_event, token);
                    }
                }));
                return;
            }
        }

        let Some(event_info) = find_plain_event(&**resolved_source, event_name) else {
            return;
        };

        let delegate = MarkupDelegate::new(move |arguments: &[MarkupValue]| -> MarkupValue {
            let (sender, event_args) = match arguments {
                [] => (None, None),
                [event_args] => (None, event_args.clone()),
                [sender, event_args, ..] => (sender.clone(), event_args.clone()),
            };
            let event_args = event_args.unwrap_or_else(|| Rc::new(EventArgs::EMPTY) as BoxedValue);
            handler(sender.as_ref(), &event_args);
            None
        });
        *self.event_handler.borrow_mut() = subscribe_plain_event(event_info, resolved_source, delegate);
    }

    fn register_default_event(&self, resolved_source: Option<&BoxedValue>) {
        let Some(element) = resolved_source.and_then(object_of).and_then(|object| object.cast::<Control>()) else {
            return;
        };

        if !Self::is_element_loaded(&element) {
            self.is_loaded_event_registered.set(true);
            let handler = self.handler();
            let sender = element.downgrade();
            let subscription = element.attached_to_visual_tree(move |e| {
                let sender: Option<BoxedValue> = sender.upgrade().map(|sender| Rc::new(sender) as BoxedValue);
                let event_args: BoxedValue = Rc::new(e.clone());
                handler(sender.as_ref(), &event_args);
            });
            *self.loaded_event_subscription.borrow_mut() = Some(subscription);
        }
    }

    fn unregister_default_event(&self, resolved_source: Option<&BoxedValue>) {
        self.is_loaded_event_registered.set(false);
        let subscription = self.loaded_event_subscription.borrow_mut().take();
        let is_control = resolved_source.and_then(object_of).is_some_and(|object| object.is::<Control>());
        if let (true, Some(subscription)) = (is_control, subscription) {
            subscription.dispose();
        }
    }

    fn is_element_loaded(element: &Control) -> bool {
        element.parent().is_some()
    }
}
