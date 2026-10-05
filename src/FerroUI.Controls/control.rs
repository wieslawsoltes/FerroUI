use crate::automation::peers::{AutomationPeer, NoneAutomationPeer};
use crate::primitives::{AdornerLayer, FlyoutBase};
use crate::templates::{DataTemplates, IDataTemplateHost, ITemplateOf};
use crate::Panel;
use crate::ContextMenu;
use ferroui_base::reactive::IDisposable;
use crate::{RequestBringIntoViewEventArgs, SizeChangedEventArgs};
use ferroui_base::input::{
    ContextRequestedEventArgs, FocusChangedEventArgs, InputElement, InputElementImpl, InputElementImplExt, KeyEventArgs,
    MouseButton, NavigationMethod,
    PointerReleasedEventArgs,
};
use ferroui_base::interactivity::{
    Interactive, InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutedEventHandlerToken, RoutingStrategies,
};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::styling::{ISetterValue, Setter};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, ferro_routed_event, instantiate, AnyValue, BoxedValue,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, IntoRef, Nullable, Rect,
    Ref, Size, StyledElement, StyledElementImpl, StyledProperty, Visual, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use std::cell::{Cell, OnceCell, RefCell};
use std::collections::{HashSet, VecDeque};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LoadState {
    Unloaded,
    Loaded,
    LoadPending,
}

// Note the following:
// `queue`:
//   Is the queue where any control will be added to indicate that its loaded
//   event should be scheduled and called later.
// `processing_queue`:
//   Contains a copied snapshot of `queue` at the time when processing starts
//   and individual events are being fired. This is needed because new
//   controls can be added in the Loaded event itself.
#[derive(Default)]
struct LoadedQueues {
    is_loaded_processing: bool,
    /// The queued controls in insertion order.
    queue: Vec<Ref<Control>>,
    /// The members of `queue`, for constant time lookups.
    queued: HashSet<Ref<Control>>,
    processing_queue: VecDeque<Ref<Control>>,
}

impl LoadedQueues {
    fn add(&mut self, control: Ref<Control>) -> bool {
        if self.queued.insert(control.clone()) {
            self.queue.push(control);
            true
        } else {
            false
        }
    }

    fn remove(&mut self, control: &Ref<Control>) {
        if self.queued.remove(control) {
            self.queue.retain(|c| c != control);
        }
    }
}

thread_local! {
    static LOADED: RefCell<LoadedQueues> = RefCell::new(LoadedQueues::default());
}

/// Base class for controls.
///
/// The control class extends [`InputElement`] and adds the following
/// features:
///
/// - A `Tag` property to allow user-defined data to be attached to the
///   control.
#[repr(C)]
pub struct Control {
    base: InputElement,
    load_state: Cell<LoadState>,
    data_templates: OnceCell<DataTemplates>,
    focus_adorner: RefCell<Option<Ref<Control>>>,
    context_flyout_handlers: Cell<Option<(RoutedEventHandlerToken, RoutedEventHandlerToken)>>,
    context_menu_handlers: Cell<Option<ContextMenuHandlers>>,
    automation_peer: RefCell<Option<Ref<AutomationPeer>>>,
}

/// The handlers the context menu support attaches to a control: the context
/// requested and context canceled handlers and the subscriptions to the
/// attachment to and the detachment from the visual tree.
pub(crate) type ContextMenuHandlers =
    (RoutedEventHandlerToken, RoutedEventHandlerToken, Rc<dyn IDisposable>, Rc<dyn IDisposable>);

ferro_class! {
    Control: InputElement, virtuals ControlImpl: InputElementImpl {
        /// Gets the element that receives the focus adorner.
        fn get_template_focus_target(this) -> Option<Ref<Control>>;
        /// Raises the `Loaded` event.
        fn on_loaded(this, e: &RoutedEventArgs);
        /// Raises the `Unloaded` event.
        fn on_unloaded(this, e: &RoutedEventArgs);
        /// Raises the `SizeChanged` event.
        fn on_size_changed(this, e: &SizeChangedEventArgs);
        /// Returns a new, type-specific automation peer implementation for
        /// the control.
        fn on_create_automation_peer(this) -> Ref<AutomationPeer>;
    }
}
ferroui_base::ferro_class_info!(Control { new: Control::new });

ferro_impl_classes!(Control: StyledElementImpl, LayoutableImpl, InteractiveImpl);

impl FerroObjectImpl for Control {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Visual::bounds_property().as_property() {
            let (old_value, new_value) = change.get_old_and_new_value::<Rect>();

            // Bounds is a rect with a position as well as a height and width.
            // This means it is possible for the rect to change position but
            // not size. Therefore, we want to explicitly check only the size
            // and raise an event only when that size has changed.
            if new_value.size() != old_value.size() {
                let size_changed_event_args = SizeChangedEventArgs::new(
                    Self::size_changed_event(),
                    this.to_ref(),
                    Size::new(old_value.width, old_value.height),
                    Size::new(new_value.width, new_value.height),
                );

                this.on_size_changed(&size_changed_event_args);
            }
        }
    }

    fn update_data_validation(
        this: &Self,
        property: &'static ferroui_base::FerroProperty,
        state: ferroui_base::data::BindingValueType,
        error: Option<&ferroui_base::data::BindingError>,
    ) {
        crate::DataValidationErrors::set_error(this, error);
        Self::parent_update_data_validation(this, property, state, error);
    }
}

impl VisualImpl for Control {
    fn on_attached_to_visual_tree_core(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree_core(this, e);

        this.initialize_if_needed();

        this.schedule_on_loaded_core();
    }

    fn on_detached_from_visual_tree_core(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree_core(this, e);

        this.on_unloaded_core();
    }

    fn ensure_initialized_for_visual_brush(this: &Self) {
        this.ensure_initialized();
    }
}

impl InputElementImpl for Control {
    fn on_got_focus(this: &Self, e: &FocusChangedEventArgs) {
        Self::parent_on_got_focus(this, e);

        if this.is_focused()
            && (e.navigation_method == NavigationMethod::Tab || e.navigation_method == NavigationMethod::Directional)
        {
            if let Some(adorner_layer) = AdornerLayer::get_adorner_layer(this) {
                if this.focus_adorner.borrow().is_none() {
                    let template = if this.is_set(Self::focus_adorner_property().as_property()) {
                        this.get_value(Self::focus_adorner_property())
                    } else {
                        adorner_layer.default_focus_adorner()
                    };

                    if let Some(template) = template {
                        *this.focus_adorner.borrow_mut() = template.build_typed();
                    }
                }

                let focus_adorner = this.focus_adorner.borrow().clone();
                if let (Some(focus_adorner), Some(target)) = (focus_adorner, this.get_template_focus_target()) {
                    AdornerLayer::set_adorned_element(&focus_adorner, target);
                    adorner_layer.children().add(focus_adorner);
                }
            }
        }
    }

    fn on_lost_focus(this: &Self, e: &FocusChangedEventArgs) {
        Self::parent_on_lost_focus(this, e);

        let focus_adorner = this.focus_adorner.borrow().clone();
        if let Some(focus_adorner) = focus_adorner {
            if let Some(adorner_layer) = focus_adorner.parent().and_then(|parent| parent.cast::<Panel>()) {
                adorner_layer.children().remove(focus_adorner);
                *this.focus_adorner.borrow_mut() = None;
            }
        }
    }

    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, e);

        if e.is_source(this) && !e.handled() && e.initial_press_mouse_button() == MouseButton::Right {
            let args = ContextRequestedEventArgs::from_pointer_event_args(e);
            this.raise_event(&args);
            e.set_handled(args.handled());
        }
    }

    fn on_key_up(this: &Self, e: &KeyEventArgs) {
        Self::parent_on_key_up(this, e);

        if e.is_source(this) && !e.handled() {
            let Some(settings) = this.get_platform_settings() else { return };
            let configuration = settings.hotkey_configuration();
            let keymap = &configuration.open_context_menu;

            let matches = keymap.iter().any(|key| key.matches(Some(e)));

            if matches {
                let args = ContextRequestedEventArgs::new();
                this.raise_event(&args);
                e.set_handled(args.handled());
            }
        }
    }
}

impl ControlImpl for Control {
    fn get_template_focus_target(this: &Self) -> Option<Ref<Control>> {
        Some(this.to_ref())
    }

    fn on_loaded(this: &Self, e: &RoutedEventArgs) {
        this.raise_event(e);
    }

    fn on_unloaded(this: &Self, e: &RoutedEventArgs) {
        this.raise_event(e);
    }

    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        NoneAutomationPeer::new(this).upcast()
    }

    fn on_size_changed(this: &Self, e: &SizeChangedEventArgs) {
        this.raise_event(e);
    }
}

ferroui_base::ferro_properties! { impl Control {
    ferro_property!(
        /// Defines the `FocusAdorner` property.
        pub fn focus_adorner_property() -> StyledProperty<Option<Rc<dyn ITemplateOf<Option<Ref<Control>>>>>> {
            FerroProperty::register::<Control, _>("FocusAdorner", None)
        }
    );

    ferro_property!(
        /// Defines the `Tag` property.
        pub fn tag_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<Control, _>("Tag", None)
        }
    );

    ferro_property!(
        /// Defines the `ContextMenu` property.
        pub fn context_menu_property() -> StyledProperty<Option<Ref<ContextMenu>>> {
            FerroProperty::register::<Control, _>("ContextMenu", None)
        }
    );

    ferro_property!(
        /// Defines the `ContextFlyout` property.
        pub fn context_flyout_property() -> StyledProperty<Option<Ref<FlyoutBase>>> {
            FerroProperty::register::<Control, _>("ContextFlyout", None)
        }
    );
} }

impl Control {
    ferro_routed_event!(
        /// Event raised when an element wishes to be scrolled into view.
        pub fn request_bring_into_view_event() -> RoutedEvent<RequestBringIntoViewEventArgs> {
            RoutedEvent::register::<Control, _>("RequestBringIntoView", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `Loaded` event.
        pub fn loaded_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<Control, _>("Loaded", RoutingStrategies::DIRECT)
        }
    );

    ferro_routed_event!(
        /// Defines the `Unloaded` event.
        pub fn unloaded_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<Control, _>("Unloaded", RoutingStrategies::DIRECT)
        }
    );

    ferro_routed_event!(
        /// Defines the `SizeChanged` event.
        pub fn size_changed_event() -> RoutedEvent<SizeChangedEventArgs> {
            RoutedEvent::register::<Control, _>("SizeChanged", RoutingStrategies::DIRECT)
        }
    );

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: InputElement::construct(), load_state: Cell::new(LoadState::Unloaded), data_templates: OnceCell::new(), focus_adorner: RefCell::new(None), context_flyout_handlers: Cell::new(None), context_menu_handlers: Cell::new(None), automation_peer: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the automation peer of the control, if it has been created.
    pub(crate) fn get_automation_peer(&self) -> Option<Ref<AutomationPeer>> {
        self.verify_access();
        self.automation_peer.borrow().clone()
    }

    /// Gets the automation peer of the control, creating it with
    /// `on_create_automation_peer` if it does not exist yet.
    pub(crate) fn get_or_create_automation_peer(&self) -> Ref<AutomationPeer> {
        self.verify_access();

        if let Some(peer) = self.automation_peer.borrow().clone() {
            return peer;
        }

        let peer = self.on_create_automation_peer();
        *self.automation_peer.borrow_mut() = Some(peer.clone());
        peer
    }

    /// The control's focus adorner.
    pub fn focus_adorner(&self) -> Option<Rc<dyn ITemplateOf<Option<Ref<Control>>>>> {
        self.get_value(Self::focus_adorner_property())
    }

    pub fn set_focus_adorner(&self, value: Option<Rc<dyn ITemplateOf<Option<Ref<Control>>>>>) {
        self.set_value(Self::focus_adorner_property(), value)
    }

    /// The data templates for the control.
    ///
    /// Each control may define data templates which are applied to the
    /// control itself and its children.
    pub fn data_templates(&self) -> DataTemplates {
        self.data_templates.get_or_init(DataTemplates::new).clone()
    }

    /// Whether the data templates collection has been created.
    pub fn is_data_templates_initialized(&self) -> bool {
        self.data_templates.get().is_some()
    }

    /// A context menu to the control.
    pub fn context_menu(&self) -> Option<Ref<ContextMenu>> {
        self.get_value(Self::context_menu_property())
    }

    pub fn set_context_menu(&self, value: impl Into<Nullable<ContextMenu>>) {
        self.set_value(Self::context_menu_property(), value.into().0)
    }

    /// The handlers the context menu support attached to this control.
    pub(crate) fn context_menu_handlers(&self) -> &Cell<Option<ContextMenuHandlers>> {
        &self.context_menu_handlers
    }

    /// A context flyout to the control.
    pub fn context_flyout(&self) -> Option<Ref<FlyoutBase>> {
        self.get_value(Self::context_flyout_property())
    }

    pub fn set_context_flyout(&self, value: impl Into<Nullable<FlyoutBase>>) {
        self.set_value(Self::context_flyout_property(), value.into().0)
    }

    /// The handlers the context flyout support attached to this control
    /// for its context requested and context canceled events.
    pub(crate) fn context_flyout_handlers(&self) -> &Cell<Option<(RoutedEventHandlerToken, RoutedEventHandlerToken)>> {
        &self.context_flyout_handlers
    }

    /// Whether the control is fully constructed in the visual tree and both
    /// layout and render are complete.
    ///
    /// This is set to true while raising the `Loaded` event.
    pub fn is_loaded(&self) -> bool {
        self.load_state.get() == LoadState::Loaded
    }

    /// A user-defined object attached to the control.
    pub fn tag(&self) -> Option<BoxedValue> {
        self.get_value(Self::tag_property())
    }

    pub fn set_tag(&self, value: Option<BoxedValue>) {
        self.set_value(Self::tag_property(), value)
    }

    /// Occurs when the control has been fully constructed in the visual tree
    /// and both layout and render are complete.
    ///
    /// This event is guaranteed to occur after the control template is
    /// applied and references to objects created after the template is
    /// applied are available. This makes it different from
    /// `on_attached_to_visual_tree` which doesn't have these references. This
    /// event occurs at the latest possible time in the control creation
    /// life-cycle.
    pub fn loaded(&self, handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static) -> RoutedEventHandlerToken {
        self.add_handler(Self::loaded_event(), handler)
    }

    /// Occurs when the control is removed from the visual tree.
    ///
    /// This is API symmetrical with `loaded` and exists for compatibility
    /// with other XAML frameworks; however, it behaves the same as
    /// `on_detached_from_visual_tree`.
    pub fn unloaded(&self, handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static) -> RoutedEventHandlerToken {
        self.add_handler(Self::unloaded_event(), handler)
    }

    /// Occurs when the bounds (actual size) of the control have changed.
    pub fn size_changed(
        &self,
        handler: impl Fn(&Interactive, &SizeChangedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::size_changed_event(), handler)
    }

    /// The boxing convention for a control held in an untyped value (a
    /// `Content`, a `Tag`, an item): the value holds a `Ref<Control>`.
    pub fn boxed(control: impl IntoRef<Control>) -> BoxedValue {
        Rc::new(control.into_ref())
    }

    /// The control held by an untyped value, if it holds one. See
    /// [`boxed`](Self::boxed).
    pub fn from_boxed(value: &BoxedValue) -> Option<Ref<Control>> {
        let value: &dyn AnyValue = &**value;
        value.downcast_ref::<Ref<Control>>().cloned()
    }

    /// The logical element held by an untyped value, if it holds one: a
    /// `Ref<Control>` (see [`boxed`](Self::boxed)) or a
    /// `Ref<StyledElement>`.
    pub fn logical_from_boxed(value: &BoxedValue) -> Option<Ref<StyledElement>> {
        let value: &dyn AnyValue = &**value;
        if let Some(control) = value.downcast_ref::<Ref<Control>>() {
            return Some(control.clone().upcast());
        }
        value.downcast_ref::<Ref<StyledElement>>().cloned()
    }

    /// Ensures that the control is initialized and laid out when it is used
    /// as the source of a visual brush while not attached to a visual root.
    pub fn ensure_initialized(&self) {
        if self.visual_root().is_none() {
            if !self.is_initialized() {
                for i in self.get_self_and_visual_descendants() {
                    if let Some(c) = i.downcast_ref::<Control>() {
                        if !c.is_initialized() {
                            c.begin_init();
                            c.end_init();
                        }
                    }
                }
            }

            if !self.is_arrange_valid() {
                self.measure(Size::INFINITY);
                self.arrange(Rect::from_size(self.desired_size()));
            }
        }
    }

    fn loaded_processing_action() {
        // Copy the loaded queue for processing. Two collections are used so
        // that while one is being processed the other accepts adding new
        // controls to process next: new controls can be added within the
        // Loaded callback/event itself.
        LOADED.with(|loaded| {
            let mut loaded = loaded.borrow_mut();
            let queue = std::mem::take(&mut loaded.queue);
            loaded.queued.clear();
            loaded.processing_queue.extend(queue);
        });

        // A panic from `on_loaded_core` (a Loaded handler or override)
        // propagates to the dispatcher as usual, but the processing machinery
        // must recover: controls left unprocessed are requeued and a new
        // dispatcher job is posted for them, so a single faulty handler
        // cannot stop Loaded from ever being raised again.
        struct Finally;

        impl Drop for Finally {
            fn drop(&mut self) {
                let restart = LOADED.with(|loaded| {
                    let mut loaded = loaded.borrow_mut();
                    let remaining = std::mem::take(&mut loaded.processing_queue);
                    for control in remaining {
                        loaded.add(control);
                    }
                    loaded.is_loaded_processing = false;

                    // Restart if any controls were added to the queue while
                    // processing.
                    if !loaded.queue.is_empty() {
                        loaded.is_loaded_processing = true;
                        true
                    } else {
                        false
                    }
                });

                if restart {
                    Dispatcher::ui_thread().post_local(Control::loaded_processing_action, DispatcherPriority::LOADED);
                }
            }
        }

        let _finally = Finally;

        loop {
            let next = LOADED.with(|loaded| loaded.borrow_mut().processing_queue.pop_front());
            match next {
                Some(control) => control.on_loaded_core(),
                None => break,
            }
        }
    }

    /// Schedules `on_loaded_core` to be called for this control. For
    /// performance, it will be queued with other controls.
    pub(crate) fn schedule_on_loaded_core(&self) {
        if self.load_state.get() == LoadState::Unloaded {
            let post = LOADED.with(|loaded| {
                let mut loaded = loaded.borrow_mut();
                let is_added = loaded.add(self.to_ref());
                if is_added && !loaded.is_loaded_processing {
                    loaded.is_loaded_processing = true;
                    true
                } else {
                    false
                }
            });
            self.load_state.set(LoadState::LoadPending);

            if post {
                Dispatcher::ui_thread().post_local(Self::loaded_processing_action, DispatcherPriority::LOADED);
            }
        }
    }

    /// Invoked as the first step of marking the control as loaded and
    /// raising the `Loaded` event.
    pub(crate) fn on_loaded_core(&self) {
        if self.load_state.get() == LoadState::LoadPending && self.is_attached_to_logical_tree() {
            self.load_state.set(LoadState::Loaded);

            self.on_loaded(&RoutedEventArgs::with_event_and_source(Self::loaded_event(), self.to_ref()));
        } else {
            // We somehow got here while being detached?
            self.load_state.set(LoadState::Unloaded);
        }
    }

    /// Invoked as the first step of marking the control as unloaded and
    /// raising the `Unloaded` event.
    pub(crate) fn on_unloaded_core(&self) {
        match self.load_state.get() {
            LoadState::Loaded => {
                self.load_state.set(LoadState::Unloaded);

                self.on_unloaded(&RoutedEventArgs::with_event_and_source(Self::unloaded_event(), self.to_ref()));
            }
            LoadState::LoadPending => {
                // Remove from the loaded event queue here as a failsafe in
                // case the control is detached before the dispatcher runs
                // the Loaded jobs.
                let this = self.to_ref();
                LOADED.with(|loaded| loaded.borrow_mut().remove(&this));

                self.load_state.set(LoadState::Unloaded);
            }
            LoadState::Unloaded => {}
        }
    }

    /// Clears the queue of controls waiting for their `Loaded` event. Since
    /// tests reset the dispatcher instance, the callback might never arrive.
    #[doc(hidden)]
    pub fn reset_loaded_queue_for_unit_tests() {
        LOADED.with(|loaded| {
            let mut loaded = loaded.borrow_mut();
            loaded.queue.clear();
            loaded.queued.clear();
            loaded.processing_queue.clear();
            loaded.is_loaded_processing = false;
        });
    }
}

impl IDataTemplateHost for Control {
    fn data_templates(&self) -> DataTemplates {
        Control::data_templates(self)
    }

    fn is_data_templates_initialized(&self) -> bool {
        Control::is_data_templates_initialized(self)
    }
}

impl ISetterValue for Control {
    fn initialize(&self, setter: &Setter) {
        // A context menu implements the contract itself.
        if let Some(context_menu) = self.downcast_ref::<ContextMenu>() {
            return context_menu.initialize(setter);
        }

        if setter.property() == Some(Self::context_flyout_property().as_property()) {
            return; // Allow ContextFlyout to not need wrapping in a template
        }

        panic!("Cannot use a control as a Setter value. Wrap the control in a <Template>.");
    }
}
