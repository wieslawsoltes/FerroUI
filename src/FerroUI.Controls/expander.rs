use crate::metadata::PseudoClassesAttribute;
use crate::primitives::{HeaderedContentControl, TemplatedControlImpl};
use crate::{ContentControlImpl, ControlImpl};
use ferroui_base::animation::IPageTransition;
use ferroui_base::data::{BindingMode, BindingPriority};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{
    CancelRoutedEventArgs, Interactive, InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutedEventHandlerToken,
    RoutingStrategies,
};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::threading::{
    CancellationTokenSource, Dispatcher, DispatcherPriority, FerroSynchronizationContext,
};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, ferro_routed_event, instantiate, FerroObject, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty,
    StyledPropertyOptions, Visual, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const PC_EXPANDED: &str = ":expanded";
const PC_UP: &str = ":up";
const PC_DOWN: &str = ":down";
const PC_LEFT: &str = ":left";
const PC_RIGHT: &str = ":right";

/// Direction in which an [`Expander`] control opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ExpandDirection {
    /// Opens down.
    Down = 0,
    /// Opens up.
    Up = 1,
    /// Opens left.
    Left = 2,
    /// Opens right.
    Right = 3,
}

/// A control with a header that has a collapsible content section.
#[repr(C)]
pub struct Expander {
    base: HeaderedContentControl,
    ignore_property_changed: Cell<bool>,
    last_transition_cts: RefCell<Option<CancellationTokenSource>>,
}

ferro_class! {
    Expander: HeaderedContentControl, virtuals ExpanderImpl: ContentControlImpl {
        /// Raises the `Collapsed` event.
        fn on_collapsed(this, event_args: &RoutedEventArgs);
        /// Raises the `Collapsing` event.
        fn on_collapsing(this, event_args: &CancelRoutedEventArgs);
        /// Raises the `Expanded` event.
        fn on_expanded(this, event_args: &RoutedEventArgs);
        /// Raises the `Expanding` event.
        fn on_expanding(this, event_args: &CancelRoutedEventArgs);
        /// Called when the `IsExpanded` property has to be coerced.
        fn on_coerce_is_expanded(this, value: bool) -> bool;
    }
}
ferroui_base::ferro_class_info!(Expander { new: Expander::new });

ferro_impl_classes!(
    Expander: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    TemplatedControlImpl,
    ContentControlImpl
);

impl ControlImpl for Expander {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::ExpanderAutomationPeer::new(this).upcast()
    }
}

impl FerroObjectImpl for Expander {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.update_pseudo_classes();
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if this.ignore_property_changed.get() {
            return;
        }

        if change.property() == Self::expand_direction_property().as_property() {
            this.update_pseudo_classes();
        } else if change.property() == Self::is_expanded_property().as_property() {
            // Expanded/Collapsed will be raised once transitions are complete.
            this.start_content_transition();

            this.update_pseudo_classes();
        }
    }
}

impl ExpanderImpl for Expander {
    fn on_collapsed(this: &Self, event_args: &RoutedEventArgs) {
        this.raise_event(event_args);
    }

    fn on_collapsing(this: &Self, event_args: &CancelRoutedEventArgs) {
        this.raise_event(event_args);
    }

    fn on_expanded(this: &Self, event_args: &RoutedEventArgs) {
        this.raise_event(event_args);
    }

    fn on_expanding(this: &Self, event_args: &CancelRoutedEventArgs) {
        this.raise_event(event_args);
    }

    fn on_coerce_is_expanded(this: &Self, value: bool) -> bool {
        let event_args = if value {
            let event_args = CancelRoutedEventArgs::with_event_and_source(Self::expanding_event(), this.to_ref());
            this.on_expanding(&event_args);
            event_args
        } else {
            let event_args = CancelRoutedEventArgs::with_event_and_source(Self::collapsing_event(), this.to_ref());
            this.on_collapsing(&event_args);
            event_args
        };

        if event_args.cancel() {
            // If the event was externally canceled we must still notify the
            // value has changed. This property changed notification will
            // update any external code observing this property that itself
            // may have set the new value. We are essentially reverting any
            // external state change along with ignoring the `IsExpanded`
            // property set. Remember `IsExpanded` is usually controlled by a
            // toggle button in the control theme and is also used for
            // animations.
            this.ignore_property_changed.set(true);

            this.raise_property_changed(
                Self::is_expanded_property().as_property(),
                Some(&value),
                &!value,
                BindingPriority::LocalValue,
                true,
            );

            this.ignore_property_changed.set(false);

            return !value;
        }

        value
    }
}

impl Expander {
    /// The pseudoclasses set by the class.
    pub const PSEUDO_CLASSES: PseudoClassesAttribute =
        PseudoClassesAttribute::new(&[PC_EXPANDED, PC_UP, PC_DOWN, PC_LEFT, PC_RIGHT]);
}

ferroui_base::ferro_properties! { impl Expander {
    ferro_property!(
        /// Defines the `ContentTransition` property.
        pub fn content_transition_property() -> StyledProperty<Option<Rc<dyn IPageTransition>>> {
            FerroProperty::register::<Expander, _>("ContentTransition", None)
        }
    );

    ferro_property!(
        /// Defines the `ExpandDirection` property.
        pub fn expand_direction_property() -> StyledProperty<ExpandDirection> {
            FerroProperty::register::<Expander, _>("ExpandDirection", ExpandDirection::Down)
        }
    );

    ferro_property!(
        /// Defines the `IsExpanded` property.
        pub fn is_expanded_property() -> StyledProperty<bool> {
            FerroProperty::register_with::<Expander, _>(
                "IsExpanded",
                StyledPropertyOptions::new(false)
                    .default_binding_mode(BindingMode::TwoWay)
                    .coerce(Expander::coerce_is_expanded),
            )
        }
    );
} }

impl Expander {
    ferro_routed_event!(
        /// Defines the `Collapsed` event.
        pub fn collapsed_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<Expander, _>("Collapsed", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `Collapsing` event.
        pub fn collapsing_event() -> RoutedEvent<CancelRoutedEventArgs> {
            RoutedEvent::register::<Expander, _>("Collapsing", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `Expanded` event.
        pub fn expanded_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<Expander, _>("Expanded", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `Expanding` event.
        pub fn expanding_event() -> RoutedEvent<CancelRoutedEventArgs> {
            RoutedEvent::register::<Expander, _>("Expanding", RoutingStrategies::BUBBLE)
        }
    );

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: HeaderedContentControl::construct(),
            ignore_property_changed: Cell::new(false),
            last_transition_cts: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The transition used when expanding or collapsing the content.
    pub fn content_transition(&self) -> Option<Rc<dyn IPageTransition>> {
        self.get_value(Self::content_transition_property())
    }

    pub fn set_content_transition(&self, value: Option<Rc<dyn IPageTransition>>) {
        self.set_value(Self::content_transition_property(), value)
    }

    /// The direction in which the expander opens.
    pub fn expand_direction(&self) -> ExpandDirection {
        self.get_value(Self::expand_direction_property())
    }

    pub fn set_expand_direction(&self, value: ExpandDirection) {
        self.set_value(Self::expand_direction_property(), value)
    }

    /// Whether the content area of the expander is open and visible.
    pub fn is_expanded(&self) -> bool {
        self.get_value(Self::is_expanded_property())
    }

    pub fn set_is_expanded(&self, value: bool) {
        self.set_value(Self::is_expanded_property(), value)
    }

    /// Occurs after the content area has closed and only the header is
    /// visible.
    pub fn collapsed(&self, handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static) -> RoutedEventHandlerToken {
        self.add_handler(Self::collapsed_event(), handler)
    }

    /// Occurs as the content area is closing.
    ///
    /// The `cancel` property of the event args may be set to true to cancel
    /// the event and keep the control open (expanded).
    pub fn collapsing(
        &self,
        handler: impl Fn(&Interactive, &CancelRoutedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::collapsing_event(), handler)
    }

    /// Occurs after the expander has opened to display both its header and
    /// content.
    pub fn expanded(&self, handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static) -> RoutedEventHandlerToken {
        self.add_handler(Self::expanded_event(), handler)
    }

    /// Occurs as the content area is opening.
    ///
    /// The `cancel` property of the event args may be set to true to cancel
    /// the event and keep the control closed (collapsed).
    pub fn expanding(
        &self,
        handler: impl Fn(&Interactive, &CancelRoutedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::expanding_event(), handler)
    }

    /// Starts the content transition (if set) and invokes the `Expanded` and
    /// `Collapsed` events when completed.
    fn start_content_transition(&self) {
        let this = self.to_ref();

        // Expanded/Collapsed events are invoked asynchronously to ensure
        // other events, such as Click, have time to complete first.
        let post_completed = move || {
            Dispatcher::ui_thread().post_local(
                move || {
                    if this.is_expanded() {
                        this.on_expanded(&RoutedEventArgs::with_event_and_source(Self::expanded_event(), this.clone()));
                    } else {
                        this.on_collapsed(&RoutedEventArgs::with_event_and_source(
                            Self::collapsed_event(),
                            this.clone(),
                        ));
                    }
                },
                DispatcherPriority::DEFAULT,
            );
        };

        let content_transition = self.content_transition();
        let visual_content: Option<Ref<Visual>> = self.presenter().map(Ref::upcast);

        if let (Some(_), Some(content_transition), Some(visual_content)) =
            (self.content(), content_transition, visual_content)
        {
            let expand_direction = self.expand_direction();
            let forward = expand_direction == ExpandDirection::Left || expand_direction == ExpandDirection::Up;

            let cts = CancellationTokenSource::new();
            let token = cts.token();
            let last = self.last_transition_cts.replace(Some(cts));
            if let Some(last) = last {
                last.cancel();
            }

            let task = if self.is_expanded() {
                content_transition.start(None, Some(&visual_content), forward, token)
            } else {
                content_transition.start(Some(&visual_content), None, !forward, token)
            };

            let context = FerroSynchronizationContext::current().unwrap_or_else(|| {
                FerroSynchronizationContext::with_dispatcher(
                    &Dispatcher::current_dispatcher(),
                    DispatcherPriority::NORMAL,
                )
            });
            drop(context.to_task_scheduler().start_local(async move {
                if task.await.is_ok() {
                    post_completed();
                }
            }));
        } else {
            post_completed();
        }
    }

    /// Updates the visual state of the control by applying the latest
    /// pseudoclasses.
    fn update_pseudo_classes(&self) {
        let expand_direction = self.expand_direction();

        self.pseudo_classes().set(PC_UP, expand_direction == ExpandDirection::Up);
        self.pseudo_classes().set(PC_DOWN, expand_direction == ExpandDirection::Down);
        self.pseudo_classes().set(PC_LEFT, expand_direction == ExpandDirection::Left);
        self.pseudo_classes().set(PC_RIGHT, expand_direction == ExpandDirection::Right);

        self.pseudo_classes().set(PC_EXPANDED, self.is_expanded());
    }

    /// Coerces/validates the `IsExpanded` property value.
    fn coerce_is_expanded(instance: &FerroObject, value: bool) -> bool {
        if let Some(expander) = instance.downcast_ref::<Expander>() {
            return expander.on_coerce_is_expanded(value);
        }

        value
    }
}
