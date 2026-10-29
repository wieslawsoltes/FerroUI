//! Port of `Core/RoutedEventTriggerBehavior.cs`.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::interactivity::{
    IRoutedEventArgs, Interactive, RouteHandler, RoutedEvent, RoutedEventHandlerToken, RoutingStrategies,
};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObject, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Nullable, Ref, StaticType, StyledElementImpl,
    StyledProperty, WeakRef,
};
use ferroui_controls::TopLevel;
use ferroui_xaml_interactivity::{
    Interaction, StyledElementBehaviorImpl, StyledElementBehaviorImplExt, StyledElementTriggerOfT,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The handler a [`RoutedEventTriggerBehavior`] added: the element, the event and the token.
struct AddedHandler {
    interactive: WeakRef<Interactive>,
    routed_event: RoutedEvent,
    token: RoutedEventHandlerToken,
}

/// A behavior that listens for a [`RoutedEvent`] event on its source and executes its actions
/// when that event is fired.
#[repr(C)]
pub struct RoutedEventTriggerBehavior {
    base: StyledElementTriggerOfT,
    is_initialized: Cell<bool>,
    is_attached: Cell<bool>,
    // The handler added to the source (the reference removes its delegate).
    added_handler: RefCell<Option<AddedHandler>>,
}

ferro_class!(RoutedEventTriggerBehavior: StyledElementTriggerOfT);
ferro_class_info!(RoutedEventTriggerBehavior {
    new: RoutedEventTriggerBehavior::new,
    markup: {
        property_attributes: [
            SourceInteractive: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(RoutedEventTriggerBehavior: StyledElementImpl);

ferro_properties! {
    impl RoutedEventTriggerBehavior {
        /// Identifies the `RoutedEvent` property.
        pub fn routed_event_property() -> StyledProperty<Option<RoutedEvent>> {
            FerroProperty::register::<RoutedEventTriggerBehavior, _>("RoutedEvent", None)
        }

        /// Identifies the `RoutingStrategies` property.
        pub fn routing_strategies_property() -> StyledProperty<RoutingStrategies> {
            FerroProperty::register::<RoutedEventTriggerBehavior, _>(
                "RoutingStrategies",
                RoutingStrategies::DIRECT | RoutingStrategies::BUBBLE,
            )
        }

        /// Identifies the `SourceInteractive` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the behavior does not own its source, so the
        /// value is an element reference.
        pub fn source_interactive_property() -> StyledProperty<Option<ElementRef<Interactive>>> {
            ValueTypes::register_element_ref::<Interactive>();
            FerroProperty::register::<RoutedEventTriggerBehavior, _>("SourceInteractive", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for RoutedEventTriggerBehavior {
        fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
            Self::parent_on_property_changed(this, change);

            let property = change.property();

            if property == Self::routed_event_property().as_property() {
                this.on_value_changed(change);
            }

            if property == Self::routing_strategies_property().as_property() {
                this.on_value_changed(change);
            }

            if property == Self::source_interactive_property().as_property() {
                this.on_value_changed(change);
            }
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for RoutedEventTriggerBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            this.is_attached.set(true);
            this.add_handler();
        }

        fn on_detached_from_visual_tree(this: &Self) {
            this.is_attached.set(false);

            let associated_object_is_top_level =
                this.associated_object().is_some_and(|associated_object| associated_object.is::<TopLevel>());
            let source_is_top_level =
                this.compute_resolved_source_interactive().is_some_and(|source| source.is::<TopLevel>());
            if !associated_object_is_top_level || !source_is_top_level {
                this.remove_handler();
            }
        }

        fn on_detaching(this: &Self) {
            this.is_attached.set(false);
            this.remove_handler();
            Self::parent_on_detaching(this);
        }
    }
}

impl RoutedEventTriggerBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElementTriggerOfT::construct(<Interactive as StaticType>::TYPE),
            is_initialized: Cell::new(false),
            is_attached: Cell::new(false),
            added_handler: RefCell::new(None),
        }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets routing event to listen for. This is a property of the object model.
    pub fn routed_event(&self) -> Option<RoutedEvent> {
        self.get_value(Self::routed_event_property())
    }

    pub fn set_routed_event(&self, value: Option<RoutedEvent>) {
        self.set_value(Self::routed_event_property(), value)
    }

    /// Gets or sets the routing event [`RoutingStrategies`]. This is a property of the object
    /// model.
    pub fn routing_strategies(&self) -> RoutingStrategies {
        self.get_value(Self::routing_strategies_property())
    }

    pub fn set_routing_strategies(&self, value: RoutingStrategies) {
        self.set_value(Self::routing_strategies_property(), value)
    }

    /// Gets or sets the source object from which this behavior listens for events. If it is not
    /// set, the source will default to the associated object. This is a property of the object
    /// model; markup resolves a name given for it to the named element.
    pub fn source_interactive(&self) -> Option<Ref<Interactive>> {
        ElementRef::resolve(&self.get_value(Self::source_interactive_property()))
    }

    pub fn set_source_interactive(&self, value: impl Into<Nullable<Interactive>>) {
        self.set_value(Self::source_interactive_property(), ElementRef::from_nullable(value.into().0))
    }

    fn on_value_changed(&self, change: &FerroPropertyChangedEventArgs<'_>) {
        let Some(behavior) = change.sender().downcast_ref::<RoutedEventTriggerBehavior>() else {
            return;
        };
        if behavior.associated_object().is_none() {
            return;
        }

        if behavior.is_initialized.get() && behavior.is_attached.get() {
            behavior.remove_handler();
            behavior.add_handler();
        }
    }

    fn add_handler(&self) {
        if self.is_initialized.get() {
            return;
        }

        let interactive = self.compute_resolved_source_interactive();
        if let (Some(interactive), Some(routed_event)) = (interactive, self.routed_event()) {
            let behavior: WeakRef<RoutedEventTriggerBehavior> = self.to_ref().downgrade();
            let handler: RouteHandler = Rc::new(move |_sender: &Interactive, e: &dyn IRoutedEventArgs| {
                if let Some(behavior) = behavior.upgrade() {
                    behavior.handler(e);
                }
            });
            let token = interactive.add_route_handler(&routed_event, handler, self.routing_strategies(), false);
            *self.added_handler.borrow_mut() =
                Some(AddedHandler { interactive: interactive.downgrade(), routed_event, token });
            self.is_initialized.set(true);
        }
    }

    fn remove_handler(&self) {
        let interactive = self.compute_resolved_source_interactive();
        if interactive.is_some() && self.routed_event().is_some() && self.is_initialized.get() {
            // Deviation (DEVIATIONS.md, Behaviors): the handler is removed from the element and
            // the event it was added to. The reference removes it from the current source and
            // event, so after a change of either the old handler stays (defect corrected).
            let added_handler = self.added_handler.borrow_mut().take();
            if let Some(added_handler) = added_handler {
                if let Some(interactive) = added_handler.interactive.upgrade() {
                    interactive.remove_handler(&added_handler.routed_event, added_handler.token);
                }
            }
            self.is_initialized.set(false);
        }
    }

    fn compute_resolved_source_interactive(&self) -> Option<Ref<Interactive>> {
        if self.get_value(Self::source_interactive_property()).is_some() {
            self.source_interactive()
        } else {
            self.associated_object().and_then(|associated_object| associated_object.cast::<Interactive>())
        }
    }

    fn handler(&self, e: &dyn IRoutedEventArgs) {
        // The args are the parameter of the actions (a shared handle to the live args).
        let parameter: BoxedValue = Rc::new(e.share());
        self.execute(Some(&parameter));

        if !self.is_attached.get() {
            self.remove_handler();
        }
    }

    fn execute(&self, parameter: Option<&BoxedValue>) {
        if !self.is_enabled() {
            return;
        }

        if let Some(interactive) = self.compute_resolved_source_interactive() {
            let sender: BoxedValue = Rc::new(interactive.upcast::<FerroObject>());
            Interaction::execute_actions(Some(&sender), self.actions().as_ref(), parameter);
        }
    }
}
