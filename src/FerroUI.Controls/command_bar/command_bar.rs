use super::{
    CommandBarButton, CommandBarDefaultLabelPosition, CommandBarOverflowButtonVisibility, CommandBarSeparator,
    CommandBarToggleButton, ICommandBarElement,
};
use crate::items_source::{IItemsList, ItemsChangedEventArgs, ItemsChangedHandler, ItemsSource, ItemsView};
use crate::primitives::{
    Popup, TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
};
use crate::{Button, ContentControl, Control, ControlImpl, ItemsControl};
use ferroui_base::collections::{
    CollectionChangedHandler, FerroList, NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs,
};
use ferroui_base::input::{
    FocusChangedEventArgs, InputElement, InputElementImpl, Key, KeyEventArgs, KeyModifiers, NavigationMethod,
};
use ferroui_base::interactivity::{
    Interactive, InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutedEventHandlerToken, RoutingStrategies,
};
use ferroui_base::layout::{LayoutableImpl, LayoutableImplExt};
use ferroui_base::reactive::{CompositeDisposable, IDisposable, ObservableExt};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, ferro_routed_event, instantiate, BoxedValue,
    DirectProperty, FerroObjectExtensions, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, Size, StyledElementImpl, StyledProperty, Visual, VisualImpl, VisualImplExt,
    VisualTreeAttachmentEventArgs,
};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A list of command bar elements: the type of the `PrimaryCommands` and
/// `SecondaryCommands` collections.
pub type CommandBarElementList = FerroList<Rc<dyn ICommandBarElement>>;

type ElementChangedHandler = CollectionChangedHandler<Rc<dyn ICommandBarElement>>;

/// The read-only, notifying collection of command bar elements that a
/// [`CommandBar`] exposes as `VisiblePrimaryCommands` and `OverflowItems`
/// (the read-only observable collection of the reference).
///
/// The collection is a reference object: the value is a shared handle,
/// clones refer to the same collection and handles compare by identity.
/// Only the command bar that owns it changes it. As an items source its
/// elements that are controls are items boxed as controls, so that an
/// items control presents them as their own containers.
#[derive(Clone)]
pub struct CommandBarElementCollection(Rc<CommandBarElementCollectionData>);

struct CommandBarElementCollectionData {
    items: RefCell<Rc<Vec<Rc<dyn ICommandBarElement>>>>,
    collection_changed: HandlerList<ElementChangedHandler>,
}

impl PartialEq for CommandBarElementCollection {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl std::fmt::Debug for CommandBarElementCollection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CommandBarElementCollection(count = {})", self.count())
    }
}

impl CommandBarElementCollection {
    fn new() -> Self {
        Self(Rc::new(CommandBarElementCollectionData {
            items: RefCell::new(Rc::new(Vec::new())),
            collection_changed: HandlerList::new(),
        }))
    }

    /// The number of elements.
    pub fn count(&self) -> usize {
        self.0.items.borrow().len()
    }

    /// Whether the collection has no elements.
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    /// The element at `index`. Panics if out of range.
    pub fn get(&self, index: usize) -> Rc<dyn ICommandBarElement> {
        self.0.items.borrow()[index].clone()
    }

    /// The elements as they are now. The snapshot shares storage with the
    /// collection until the collection changes.
    pub fn snapshot(&self) -> Rc<Vec<Rc<dyn ICommandBarElement>>> {
        self.0.items.borrow().clone()
    }

    /// A copy of the elements.
    pub fn to_vec(&self) -> Vec<Rc<dyn ICommandBarElement>> {
        self.snapshot().to_vec()
    }

    /// Whether the collection contains `element`.
    pub fn contains(&self, element: &Rc<dyn ICommandBarElement>) -> bool {
        self.index_of(element).is_some()
    }

    /// The index of `element`, if the collection contains it.
    pub fn index_of(&self, element: &Rc<dyn ICommandBarElement>) -> Option<usize> {
        self.0.items.borrow().iter().position(|item| item == element)
    }

    /// Subscribes to the changes of the collection. Returns the token for
    /// [`remove_collection_changed`](Self::remove_collection_changed).
    pub fn add_collection_changed(&self, handler: Rc<ElementChangedHandler>) -> u64 {
        self.0.collection_changed.add(handler)
    }

    /// Unsubscribes a handler.
    pub fn remove_collection_changed(&self, token: u64) -> bool {
        self.0.collection_changed.remove(token)
    }

    /// The collection as the source of the items of an items control.
    pub fn as_items_source(&self) -> ItemsSource {
        let list: Rc<dyn IItemsList> = self.0.clone();
        ItemsSource::new(list)
    }

    /// Removes every element. Notifies of a reset even when the collection
    /// is empty, as the observable collection of the reference does.
    fn clear(&self) {
        *self.0.items.borrow_mut() = Rc::new(Vec::new());
        self.0.notify(&NotifyCollectionChangedEventArgs {
            action: NotifyCollectionChangedAction::Reset,
            new_items: &[],
            old_items: &[],
            new_starting_index: -1,
            old_starting_index: -1,
        });
    }

    fn add(&self, element: Rc<dyn ICommandBarElement>) {
        let index = {
            let mut items = self.0.items.borrow_mut();
            let items = Rc::make_mut(&mut items);
            items.push(element.clone());
            items.len() - 1
        };
        self.0.notify(&NotifyCollectionChangedEventArgs {
            action: NotifyCollectionChangedAction::Add,
            new_items: std::slice::from_ref(&element),
            old_items: &[],
            new_starting_index: index as i32,
            old_starting_index: -1,
        });
    }
}

impl CommandBarElementCollectionData {
    fn notify(&self, e: &NotifyCollectionChangedEventArgs<'_, Rc<dyn ICommandBarElement>>) {
        if self.collection_changed.is_empty() {
            return;
        }
        for (_, handler) in self.collection_changed.snapshot().iter() {
            handler(e);
        }
    }
}

/// The item an items control is given for `element`: the control itself
/// (boxed as a control) when the element is one.
fn element_item(element: &Rc<dyn ICommandBarElement>) -> Option<BoxedValue> {
    match element.as_object().and_then(|object| object.downcast_ref::<Control>()) {
        Some(control) => Some(Control::boxed(control.to_ref())),
        None => Some(Rc::new(element.clone())),
    }
}

impl IItemsList for CommandBarElementCollectionData {
    fn count(&self) -> usize {
        self.items.borrow().len()
    }

    fn get_at(&self, index: usize) -> Option<BoxedValue> {
        let element = self.items.borrow()[index].clone();
        element_item(&element)
    }

    fn is_notifying(&self) -> bool {
        true
    }

    fn add_collection_changed(&self, handler: Rc<ItemsChangedHandler>) -> Option<u64> {
        Some(self.collection_changed.add(Rc::new(
            move |e: &NotifyCollectionChangedEventArgs<'_, Rc<dyn ICommandBarElement>>| {
                let new_items: Vec<Option<BoxedValue>> = e.new_items.iter().map(element_item).collect();
                let old_items: Vec<Option<BoxedValue>> = e.old_items.iter().map(element_item).collect();
                handler(&ItemsChangedEventArgs {
                    action: e.action,
                    new_items: ItemsView::from_slice(&new_items),
                    old_items: ItemsView::from_slice(&old_items),
                    new_starting_index: e.new_starting_index,
                    old_starting_index: e.old_starting_index,
                });
            },
        )))
    }

    fn remove_collection_changed(&self, token: u64) {
        self.collection_changed.remove(token);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl From<CommandBarElementCollection> for ItemsSource {
    fn from(value: CommandBarElementCollection) -> Self {
        value.as_items_source()
    }
}

/// Resets a flag when dropped.
struct Reset<'a>(&'a Cell<bool>);

impl Drop for Reset<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

/// The object behind `element` viewed as class `T`, if it is one.
fn element_as<T: ferroui_base::ObjectType>(element: &dyn ICommandBarElement) -> Option<&T> {
    element.as_object().and_then(|object| object.downcast_ref::<T>())
}

fn is_separator(element: &dyn ICommandBarElement) -> bool {
    element_as::<CommandBarSeparator>(element).is_some()
}

/// A command bar that provides primary commands displayed inline and
/// secondary commands accessible via an overflow menu.
#[repr(C)]
pub struct CommandBar {
    base: TemplatedControl,
    has_secondary_commands: Cell<bool>,
    is_overflow_button_visible: Cell<bool>,

    overflow_button: RefCell<Option<Ref<Button>>>,
    overflow_popup: RefCell<Option<Ref<Popup>>>,
    overflow_presenter: RefCell<Option<Ref<ItemsControl>>>,
    content_presenter: RefCell<Option<Ref<Control>>>,
    // The handlers added to the template parts: click, got focus, key down
    // and pointer pressed of the overflow button; key down of the overflow
    // presenter; opened of the overflow popup.
    overflow_button_handlers: Cell<
        Option<(RoutedEventHandlerToken, RoutedEventHandlerToken, RoutedEventHandlerToken, RoutedEventHandlerToken)>,
    >,
    overflow_presenter_handler: Cell<Option<RoutedEventHandlerToken>>,
    overflow_popup_opened: RefCell<Option<Rc<dyn IDisposable>>>,
    // The subscriptions to the changes of the current command lists.
    primary_commands_changed: Cell<Option<u64>>,
    secondary_commands_changed: Cell<Option<u64>>,

    visible_primary_commands: CommandBarElementCollection,
    overflow_items: CommandBarElementCollection,
    overflow_primary_secondary_separator: Rc<dyn ICommandBarElement>,
    secondary_command_visibility_subscriptions: CompositeDisposable,
    is_dynamic_update_in_progress: Cell<bool>,
    is_opening_overflow_popup: Cell<bool>,
    has_deferred_dynamic_overflow_update: Cell<bool>,
    constraint_width: Cell<f64>,
    opened_via_keyboard: Cell<bool>,
}

ferro_class!(CommandBar: TemplatedControl);

ferro_class_info!(CommandBar {
    new: CommandBar::new,
    markup: {
        content: PrimaryCommands,
        properties: [
            VisiblePrimaryCommands: CommandBarElementCollection { get: CommandBar::visible_primary_commands },
            OverflowItems: CommandBarElementCollection { get: CommandBar::overflow_items },
        ],
        attributes: [
            TemplatePart("PART_OverflowButton", type(Ref<Button>)),
            TemplatePart("PART_OverflowPopup", type(Ref<Popup>)),
            TemplatePart("PART_OverflowPresenter", type(Ref<ItemsControl>)),
            TemplatePart("PART_ContentPresenter", type(Ref<Control>)),
        ],
    },
});

ferro_impl_classes!(CommandBar: StyledElementImpl, InteractiveImpl, InputElementImpl, ControlImpl);

impl FerroObjectImpl for CommandBar {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        let primary_commands = CommandBarElementList::new();
        this.set_current_value(Self::primary_commands_property(), Some(primary_commands));

        let secondary_commands = CommandBarElementList::new();
        this.set_current_value(Self::secondary_commands_property(), Some(secondary_commands));

        this.rebuild_secondary_command_visibility_subscriptions();

        let weak = this.to_ref().downgrade();
        this.size_changed(move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.command_bar_size_changed();
            }
        });
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        let property = change.property();

        if property == Self::is_open_property().as_property() {
            let is_open = change.get_new_value::<bool>();
            if is_open {
                this.raise_event(&RoutedEventArgs::with_event(Self::opening_event()));
                let overflow_popup = this.overflow_popup.borrow().clone();
                if let Some(overflow_popup) = overflow_popup {
                    {
                        this.is_opening_overflow_popup.set(true);
                        let _reset = Reset(&this.is_opening_overflow_popup);
                        overflow_popup.set_is_open(true);
                    }

                    this.apply_deferred_dynamic_overflow_update();
                }
                this.raise_event(&RoutedEventArgs::with_event(Self::opened_event()));
            } else {
                this.raise_event(&RoutedEventArgs::with_event(Self::closing_event()));
                let overflow_popup = this.overflow_popup.borrow().clone();
                if let Some(overflow_popup) = overflow_popup {
                    overflow_popup.set_is_open(false);
                }
                this.raise_event(&RoutedEventArgs::with_event(Self::closed_event()));
            }
        } else if property == Self::default_label_position_property().as_property() {
            this.apply_label_position_to_children();
            if this.is_dynamic_overflow_enabled() {
                this.request_dynamic_overflow_update();
            }
        } else if property == Self::overflow_button_visibility_property().as_property() {
            this.update_overflow_button_visibility();
        } else if property == Self::is_sticky_property().as_property() {
            this.update_sticky_behavior();
        } else if property == Self::is_dynamic_overflow_enabled_property().as_property() {
            this.request_dynamic_overflow_update();
        } else if property == Self::primary_commands_property().as_property() {
            let (old_primary, new_primary) = change.get_old_and_new_value::<Option<CommandBarElementList>>();
            if let (Some(old_primary), Some(token)) = (old_primary, this.primary_commands_changed.take()) {
                old_primary.remove_collection_changed(token);
            }
            if let Some(new_primary) = new_primary {
                let weak = this.to_ref().downgrade();
                let token = new_primary.add_collection_changed(Rc::new(
                    move |e: &NotifyCollectionChangedEventArgs<'_, Rc<dyn ICommandBarElement>>| {
                        if let Some(this) = weak.upgrade() {
                            this.on_primary_commands_changed(e);
                        }
                    },
                ));
                this.primary_commands_changed.set(Some(token));
            }
            this.apply_label_position_to_children();
            this.request_dynamic_overflow_update();
        } else if property == Self::secondary_commands_property().as_property() {
            let (old_secondary, new_secondary) = change.get_old_and_new_value::<Option<CommandBarElementList>>();
            if let (Some(old_secondary), Some(token)) = (old_secondary, this.secondary_commands_changed.take()) {
                old_secondary.remove_collection_changed(token);
            }
            if let Some(new_secondary) = new_secondary {
                let weak = this.to_ref().downgrade();
                let token = new_secondary.add_collection_changed(Rc::new(
                    move |_: &NotifyCollectionChangedEventArgs<'_, Rc<dyn ICommandBarElement>>| {
                        if let Some(this) = weak.upgrade() {
                            this.on_secondary_commands_changed();
                        }
                    },
                ));
                this.secondary_commands_changed.set(Some(token));
            }
            this.rebuild_secondary_command_visibility_subscriptions();
            this.request_dynamic_overflow_update();
        }
    }
}

impl VisualImpl for CommandBar {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        this.rebuild_secondary_command_visibility_subscriptions();
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        this.secondary_command_visibility_subscriptions.clear();
        Self::parent_on_detached_from_visual_tree(this, e);
    }
}

impl LayoutableImpl for CommandBar {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let new_constraint = if available_size.width.is_finite() { available_size.width } else { f64::INFINITY };

        if new_constraint != this.constraint_width.get() {
            this.constraint_width.set(new_constraint);
            this.request_dynamic_overflow_update();
        }

        Self::parent_measure_override(this, available_size)
    }
}

impl TemplatedControlImpl for CommandBar {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);

        let overflow_button = this.overflow_button.borrow().clone();
        if let (Some(overflow_button), Some((click, got_focus, key_down, pointer_pressed))) =
            (overflow_button, this.overflow_button_handlers.take())
        {
            overflow_button.remove_handler(Button::click_event(), click);
            overflow_button.remove_handler(InputElement::got_focus_event(), got_focus);
            overflow_button.remove_handler(InputElement::key_down_event(), key_down);
            overflow_button.remove_handler(InputElement::pointer_pressed_event(), pointer_pressed);
        }
        let overflow_presenter = this.overflow_presenter.borrow().clone();
        if let (Some(overflow_presenter), Some(key_down)) =
            (overflow_presenter, this.overflow_presenter_handler.take())
        {
            overflow_presenter.remove_handler(InputElement::key_down_event(), key_down);
        }
        let overflow_popup_opened = this.overflow_popup_opened.take();
        if let Some(overflow_popup_opened) = overflow_popup_opened {
            overflow_popup_opened.dispose();
        }

        let overflow_button = e.name_scope().find_as::<Button>("PART_OverflowButton");
        let overflow_popup = e.name_scope().find_as::<Popup>("PART_OverflowPopup");
        let overflow_presenter = e.name_scope().find_as::<ItemsControl>("PART_OverflowPresenter");
        let content_presenter = e.name_scope().find_as::<Control>("PART_ContentPresenter");
        *this.overflow_button.borrow_mut() = overflow_button.clone();
        *this.overflow_popup.borrow_mut() = overflow_popup.clone();
        *this.overflow_presenter.borrow_mut() = overflow_presenter.clone();
        *this.content_presenter.borrow_mut() = content_presenter;

        let weak = this.to_ref().downgrade();

        if let Some(overflow_button) = overflow_button {
            let click = overflow_button.click({
                let weak = weak.clone();
                move |_, _| {
                    if let Some(this) = weak.upgrade() {
                        this.on_overflow_button_click();
                    }
                }
            });
            let got_focus = overflow_button.add_handler(InputElement::got_focus_event(), {
                let weak = weak.clone();
                move |_: &Interactive, e: &FocusChangedEventArgs| {
                    if let Some(this) = weak.upgrade() {
                        this.on_overflow_button_got_focus(e);
                    }
                }
            });
            let key_down = overflow_button.add_handler_with(
                InputElement::key_down_event(),
                {
                    let weak = weak.clone();
                    move |_: &Interactive, e: &KeyEventArgs| {
                        if let Some(this) = weak.upgrade() {
                            this.on_overflow_button_key_down(e);
                        }
                    }
                },
                Interactive::DEFAULT_ROUTES,
                true,
            );
            let pointer_pressed = overflow_button.add_handler_with(
                InputElement::pointer_pressed_event(),
                {
                    let weak = weak.clone();
                    move |_: &Interactive, _| {
                        if let Some(this) = weak.upgrade() {
                            this.on_overflow_button_pointer_pressed();
                        }
                    }
                },
                Interactive::DEFAULT_ROUTES,
                true,
            );
            this.overflow_button_handlers.set(Some((click, got_focus, key_down, pointer_pressed)));
        }
        if let Some(overflow_presenter) = overflow_presenter {
            let key_down = overflow_presenter.add_handler(InputElement::key_down_event(), {
                let weak = weak.clone();
                move |_: &Interactive, e: &KeyEventArgs| {
                    if let Some(this) = weak.upgrade() {
                        this.on_overflow_presenter_key_down(e);
                    }
                }
            });
            this.overflow_presenter_handler.set(Some(key_down));
        }
        if let Some(overflow_popup) = overflow_popup {
            let opened = overflow_popup.opened(move || {
                if let Some(this) = weak.upgrade() {
                    this.on_overflow_popup_opened();
                }
            });
            *this.overflow_popup_opened.borrow_mut() = Some(opened);
        }

        this.apply_label_position_to_children();
        this.update_overflow_button_visibility();
        this.update_sticky_behavior();
        this.request_dynamic_overflow_update();
    }
}

ferro_properties! {
    impl CommandBar {
        /// Defines the `PrimaryCommands` property.
        pub fn primary_commands_property() -> StyledProperty<Option<CommandBarElementList>> {
            FerroProperty::register::<CommandBar, _>("PrimaryCommands", None)
        }

        /// Defines the `SecondaryCommands` property.
        pub fn secondary_commands_property() -> StyledProperty<Option<CommandBarElementList>> {
            FerroProperty::register::<CommandBar, _>("SecondaryCommands", None)
        }

        /// Defines the `Content` property.
        pub fn content_property() -> StyledProperty<Option<BoxedValue>> {
            ContentControl::content_property().add_owner::<CommandBar>()
        }

        /// Defines the `DefaultLabelPosition` property.
        pub fn default_label_position_property() -> StyledProperty<CommandBarDefaultLabelPosition> {
            FerroProperty::register::<CommandBar, _>("DefaultLabelPosition", CommandBarDefaultLabelPosition::Bottom)
        }

        /// Defines the `IsDynamicOverflowEnabled` property.
        pub fn is_dynamic_overflow_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<CommandBar, _>("IsDynamicOverflowEnabled", false)
        }

        /// Defines the `OverflowButtonVisibility` property.
        pub fn overflow_button_visibility_property() -> StyledProperty<CommandBarOverflowButtonVisibility> {
            FerroProperty::register::<CommandBar, _>(
                "OverflowButtonVisibility",
                CommandBarOverflowButtonVisibility::Auto,
            )
        }

        /// Defines the `IsOpen` property.
        pub fn is_open_property() -> StyledProperty<bool> {
            FerroProperty::register::<CommandBar, _>("IsOpen", false)
        }

        /// Defines the `IsSticky` property.
        pub fn is_sticky_property() -> StyledProperty<bool> {
            FerroProperty::register::<CommandBar, _>("IsSticky", false)
        }

        /// Defines the `ItemWidthBottom` property.
        pub fn item_width_bottom_property() -> StyledProperty<f64> {
            FerroProperty::register::<CommandBar, _>("ItemWidthBottom", 70.0)
        }

        /// Defines the `ItemWidthRight` property.
        pub fn item_width_right_property() -> StyledProperty<f64> {
            FerroProperty::register::<CommandBar, _>("ItemWidthRight", 102.0)
        }

        /// Defines the `ItemWidthCollapsed` property.
        pub fn item_width_collapsed_property() -> StyledProperty<f64> {
            FerroProperty::register::<CommandBar, _>("ItemWidthCollapsed", 42.0)
        }

        /// Defines the `HasSecondaryCommands` property.
        pub fn has_secondary_commands_property() -> DirectProperty<CommandBar, bool> {
            FerroProperty::register_direct::<CommandBar, _>(
                "HasSecondaryCommands",
                |o| o.has_secondary_commands(),
                None,
                false,
            )
        }

        /// Defines the `IsOverflowButtonVisible` property.
        pub fn is_overflow_button_visible_property() -> DirectProperty<CommandBar, bool> {
            FerroProperty::register_direct::<CommandBar, _>(
                "IsOverflowButtonVisible",
                |o| o.is_overflow_button_visible(),
                None,
                false,
            )
        }
    }
}

impl CommandBar {
    ferro_routed_event!(
        /// Defines the `Opening` routed event.
        pub fn opening_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<CommandBar, _>("Opening", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `Opened` routed event.
        pub fn opened_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<CommandBar, _>("Opened", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `Closing` routed event.
        pub fn closing_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<CommandBar, _>("Closing", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `Closed` routed event.
        pub fn closed_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<CommandBar, _>("Closed", RoutingStrategies::BUBBLE)
        }
    );

    fn static_constructor() {
        // A binding that delivers one of the read-only collections to an
        // items source property (the bindings of the control themes).
        ItemsSource::register_binding_conversion::<CommandBarElementCollection>();
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            has_secondary_commands: Cell::new(false),
            is_overflow_button_visible: Cell::new(false),
            overflow_button: RefCell::new(None),
            overflow_popup: RefCell::new(None),
            overflow_presenter: RefCell::new(None),
            content_presenter: RefCell::new(None),
            overflow_button_handlers: Cell::new(None),
            overflow_presenter_handler: Cell::new(None),
            overflow_popup_opened: RefCell::new(None),
            primary_commands_changed: Cell::new(None),
            secondary_commands_changed: Cell::new(None),
            visible_primary_commands: CommandBarElementCollection::new(),
            overflow_items: CommandBarElementCollection::new(),
            overflow_primary_secondary_separator: CommandBarSeparator::new().as_command_bar_element(),
            secondary_command_visibility_subscriptions: CompositeDisposable::new(),
            is_dynamic_update_in_progress: Cell::new(false),
            is_opening_overflow_popup: Cell::new(false),
            has_deferred_dynamic_overflow_update: Cell::new(false),
            constraint_width: Cell::new(f64::INFINITY),
            opened_via_keyboard: Cell::new(false),
        }
    }

    /// Creates a command bar.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the read-only collection of primary commands currently visible
    /// in the bar (may be a subset when dynamic overflow is active).
    pub fn visible_primary_commands(&self) -> CommandBarElementCollection {
        self.visible_primary_commands.clone()
    }

    /// Gets the read-only collection of items shown in the overflow popup
    /// (secondary commands plus any primary commands moved to overflow by
    /// dynamic overflow).
    pub fn overflow_items(&self) -> CommandBarElementCollection {
        self.overflow_items.clone()
    }

    /// Gets the collection of primary commands displayed in the bar. A bar
    /// without a collection is given a new one.
    pub fn primary_commands(&self) -> CommandBarElementList {
        match self.get_value(Self::primary_commands_property()) {
            Some(value) => value,
            None => {
                let list = CommandBarElementList::new();
                self.set_current_value(Self::primary_commands_property(), Some(list.clone()));
                list
            }
        }
    }

    /// Sets the collection of primary commands displayed in the bar.
    pub fn set_primary_commands(&self, value: Option<CommandBarElementList>) {
        self.set_value(Self::primary_commands_property(), value)
    }

    /// Gets the collection of secondary commands shown in the overflow
    /// menu. A bar without a collection is given a new one.
    pub fn secondary_commands(&self) -> CommandBarElementList {
        match self.get_value(Self::secondary_commands_property()) {
            Some(value) => value,
            None => {
                let list = CommandBarElementList::new();
                self.set_current_value(Self::secondary_commands_property(), Some(list.clone()));
                list
            }
        }
    }

    /// Sets the collection of secondary commands shown in the overflow
    /// menu.
    pub fn set_secondary_commands(&self, value: Option<CommandBarElementList>) {
        self.set_value(Self::secondary_commands_property(), value)
    }

    /// Gets the custom content displayed at the start (left) of the bar.
    pub fn content(&self) -> Option<BoxedValue> {
        self.get_value(Self::content_property())
    }

    /// Sets the custom content displayed at the start (left) of the bar.
    pub fn set_content(&self, value: Option<BoxedValue>) {
        self.set_value(Self::content_property(), value)
    }

    /// Gets how labels are positioned on command buttons.
    pub fn default_label_position(&self) -> CommandBarDefaultLabelPosition {
        self.get_value(Self::default_label_position_property())
    }

    /// Sets how labels are positioned on command buttons.
    pub fn set_default_label_position(&self, value: CommandBarDefaultLabelPosition) {
        self.set_value(Self::default_label_position_property(), value)
    }

    /// Gets whether primary commands are automatically moved to the
    /// overflow menu when there is not enough space to display them.
    pub fn is_dynamic_overflow_enabled(&self) -> bool {
        self.get_value(Self::is_dynamic_overflow_enabled_property())
    }

    /// Sets whether primary commands are automatically moved to the
    /// overflow menu when there is not enough space to display them.
    pub fn set_is_dynamic_overflow_enabled(&self, value: bool) {
        self.set_value(Self::is_dynamic_overflow_enabled_property(), value)
    }

    /// Gets the visibility of the overflow button.
    pub fn overflow_button_visibility(&self) -> CommandBarOverflowButtonVisibility {
        self.get_value(Self::overflow_button_visibility_property())
    }

    /// Sets the visibility of the overflow button.
    pub fn set_overflow_button_visibility(&self, value: CommandBarOverflowButtonVisibility) {
        self.set_value(Self::overflow_button_visibility_property(), value)
    }

    /// Gets whether the overflow menu is open.
    pub fn is_open(&self) -> bool {
        self.get_value(Self::is_open_property())
    }

    /// Sets whether the overflow menu is open.
    pub fn set_is_open(&self, value: bool) {
        self.set_value(Self::is_open_property(), value)
    }

    /// Gets whether the overflow menu stays open after a command is invoked
    /// (disables light-dismiss behavior).
    pub fn is_sticky(&self) -> bool {
        self.get_value(Self::is_sticky_property())
    }

    /// Sets whether the overflow menu stays open after a command is invoked
    /// (disables light-dismiss behavior).
    pub fn set_is_sticky(&self, value: bool) {
        self.set_value(Self::is_sticky_property(), value)
    }

    /// Gets the estimated item width used in dynamic-overflow calculations
    /// when the default label position is
    /// [`CommandBarDefaultLabelPosition::Bottom`].
    pub fn item_width_bottom(&self) -> f64 {
        self.get_value(Self::item_width_bottom_property())
    }

    /// Sets the estimated item width used when the labels are at the
    /// bottom.
    pub fn set_item_width_bottom(&self, value: f64) {
        self.set_value(Self::item_width_bottom_property(), value)
    }

    /// Gets the estimated item width used in dynamic-overflow calculations
    /// when the default label position is
    /// [`CommandBarDefaultLabelPosition::Right`].
    pub fn item_width_right(&self) -> f64 {
        self.get_value(Self::item_width_right_property())
    }

    /// Sets the estimated item width used when the labels are on the right.
    pub fn set_item_width_right(&self, value: f64) {
        self.set_value(Self::item_width_right_property(), value)
    }

    /// Gets the estimated item width used in dynamic-overflow calculations
    /// when the default label position is
    /// [`CommandBarDefaultLabelPosition::Collapsed`].
    pub fn item_width_collapsed(&self) -> f64 {
        self.get_value(Self::item_width_collapsed_property())
    }

    /// Sets the estimated item width used when the labels are collapsed.
    pub fn set_item_width_collapsed(&self, value: f64) {
        self.set_value(Self::item_width_collapsed_property(), value)
    }

    /// Gets whether there are any commands (secondary or overflowed
    /// primary) in the overflow menu.
    pub fn has_secondary_commands(&self) -> bool {
        self.has_secondary_commands.get()
    }

    fn set_has_secondary_commands(&self, value: bool) {
        self.set_and_raise_cell(Self::has_secondary_commands_property(), &self.has_secondary_commands, value);
    }

    /// Gets whether the overflow button is currently visible.
    pub fn is_overflow_button_visible(&self) -> bool {
        self.is_overflow_button_visible.get()
    }

    fn set_is_overflow_button_visible(&self, value: bool) {
        self.set_and_raise_cell(Self::is_overflow_button_visible_property(), &self.is_overflow_button_visible, value);
    }

    /// Occurs when the overflow menu is about to open.
    pub fn opening(&self, handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static) -> RoutedEventHandlerToken {
        self.add_handler(Self::opening_event(), handler)
    }

    /// Occurs when the overflow menu has opened.
    pub fn opened(&self, handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static) -> RoutedEventHandlerToken {
        self.add_handler(Self::opened_event(), handler)
    }

    /// Occurs when the overflow menu is about to close.
    pub fn closing(&self, handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static) -> RoutedEventHandlerToken {
        self.add_handler(Self::closing_event(), handler)
    }

    /// Occurs when the overflow menu has closed.
    pub fn closed(&self, handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static) -> RoutedEventHandlerToken {
        self.add_handler(Self::closed_event(), handler)
    }

    fn command_bar_size_changed(&self) {
        self.request_dynamic_overflow_update();
    }

    fn on_overflow_button_click(&self) {
        self.set_current_value(Self::is_open_property(), !self.is_open());
    }

    fn on_overflow_button_got_focus(&self, e: &FocusChangedEventArgs) {
        self.opened_via_keyboard
            .set(matches!(e.navigation_method, NavigationMethod::Directional | NavigationMethod::Tab));
    }

    fn on_overflow_button_key_down(&self, e: &KeyEventArgs) {
        if matches!(e.key, Key::Enter | Key::Space) {
            self.opened_via_keyboard.set(true);
        }
    }

    fn on_overflow_button_pointer_pressed(&self) {
        self.opened_via_keyboard.set(false);
    }

    fn on_overflow_presenter_key_down(&self, e: &KeyEventArgs) {
        match e.key {
            Key::Up => {
                self.navigate_overflow(false);
                e.set_handled(true);
            }
            Key::Down => {
                self.navigate_overflow(true);
                e.set_handled(true);
            }
            Key::Home => {
                self.focus_overflow_item(true, NavigationMethod::Directional);
                e.set_handled(true);
            }
            Key::End => {
                self.focus_overflow_item(false, NavigationMethod::Directional);
                e.set_handled(true);
            }
            Key::Escape => {
                self.set_current_value(Self::is_open_property(), false);
                let overflow_button = self.overflow_button.borrow().clone();
                if let Some(overflow_button) = overflow_button {
                    overflow_button.focus_with(NavigationMethod::Unspecified, KeyModifiers::NONE);
                }
                e.set_handled(true);
            }
            _ => {}
        }
    }

    fn on_overflow_popup_opened(&self) {
        let method =
            if self.opened_via_keyboard.get() { NavigationMethod::Directional } else { NavigationMethod::Pointer };

        let weak = self.to_ref().downgrade();
        Dispatcher::ui_thread().post_local(
            move || {
                if let Some(this) = weak.upgrade() {
                    if this.is_open() {
                        this.focus_overflow_item(true, method);
                    }
                }
            },
            DispatcherPriority::LOADED,
        );
    }

    fn navigate_overflow(&self, forward: bool) {
        let items = self.get_focusable_overflow_items();
        if items.is_empty() {
            return;
        }

        let current = items.iter().position(|item| item.is_focused() || item.is_keyboard_focus_within());

        let next = match current {
            None => {
                if forward {
                    0
                } else {
                    items.len() - 1
                }
            }
            Some(current) => {
                if forward {
                    (current + 1) % items.len()
                } else {
                    (current + items.len() - 1) % items.len()
                }
            }
        };

        items[next].focus_with(NavigationMethod::Directional, KeyModifiers::NONE);
    }

    fn focus_overflow_item(&self, first: bool, method: NavigationMethod) {
        let items = self.get_focusable_overflow_items();
        if !items.is_empty() {
            items[if first { 0 } else { items.len() - 1 }].focus_with(method, KeyModifiers::NONE);
        }
    }

    fn get_focusable_overflow_items(&self) -> Vec<Ref<Control>> {
        let mut result = Vec::new();
        for item in self.overflow_items.snapshot().iter() {
            if let Some(control) = element_as::<Control>(&**item) {
                if control.is_enabled() && control.is_visible() && control.focusable() && !is_separator(&**item) {
                    result.push(control.to_ref());
                }
            }
        }
        result
    }

    fn on_primary_commands_changed(&self, e: &NotifyCollectionChangedEventArgs<'_, Rc<dyn ICommandBarElement>>) {
        for element in e.new_items {
            self.apply_label_position_to_element(&**element);
        }
        self.request_dynamic_overflow_update();
    }

    fn on_secondary_commands_changed(&self) {
        self.rebuild_secondary_command_visibility_subscriptions();
        self.request_dynamic_overflow_update();
    }

    fn request_dynamic_overflow_update(&self) {
        if self.is_opening_overflow_popup.get() {
            self.has_deferred_dynamic_overflow_update.set(true);
            return;
        }

        self.update_dynamic_overflow();
    }

    fn apply_deferred_dynamic_overflow_update(&self) {
        if !self.has_deferred_dynamic_overflow_update.get() {
            return;
        }

        self.has_deferred_dynamic_overflow_update.set(false);
        self.update_dynamic_overflow();
    }

    fn update_dynamic_overflow(&self) {
        // Reading the collections gives a bar without one a new one, which
        // runs this method from the change of the property, as in the
        // reference.
        let primary_commands = self.primary_commands().snapshot();
        let secondary_commands = self.secondary_commands().snapshot();
        if self.is_dynamic_update_in_progress.get() {
            return;
        }
        self.is_dynamic_update_in_progress.set(true);
        let _reset = Reset(&self.is_dynamic_update_in_progress);

        self.visible_primary_commands.clear();
        self.overflow_items.clear();
        self.set_overflow_mode(&*self.overflow_primary_secondary_separator, false);

        let mut overflowed_primary_commands: Vec<Rc<dyn ICommandBarElement>> = Vec::new();

        let constraint_width = self.constraint_width.get();
        let available_width = if constraint_width.is_finite() { constraint_width } else { self.bounds().width };

        if !self.is_dynamic_overflow_enabled() || available_width <= 0.0 {
            for item in primary_commands.iter() {
                self.set_overflow_mode(&**item, false);
                self.visible_primary_commands.add(item.clone());
            }
        } else {
            let content_presenter = self.content_presenter.borrow().clone();
            let content_width = content_presenter.map_or(0.0, |presenter| presenter.desired_size().width);

            if available_width < 50.0 {
                for item in primary_commands.iter() {
                    self.set_overflow_mode(&**item, false);
                    self.visible_primary_commands.add(item.clone());
                }
            } else {
                let item_width = match self.default_label_position() {
                    CommandBarDefaultLabelPosition::Right => self.item_width_right(),
                    CommandBarDefaultLabelPosition::Collapsed => self.item_width_collapsed(),
                    CommandBarDefaultLabelPosition::Bottom => self.item_width_bottom(),
                };

                let primary_non_sep_count = primary_commands.iter().filter(|item| !is_separator(&***item)).count();

                const OVERFLOW_BUTTON_WIDTH: f64 = 48.0;
                let padding = self.padding();
                let padding_h = padding.left + padding.right;
                let base_available = available_width - content_width - padding_h;
                let all_fit_without_button = secondary_commands.is_empty()
                    && base_available + 2.0 >= primary_non_sep_count as f64 * item_width;

                let max_items = if all_fit_without_button {
                    primary_non_sep_count
                } else {
                    let available_for_items = base_available - OVERFLOW_BUTTON_WIDTH;
                    let mut max_items = ((available_for_items / item_width) as i32).max(0) as usize;
                    if max_items == 0 && !primary_commands.is_empty() && available_for_items > 32.0 {
                        max_items = 1;
                    }
                    max_items
                };

                // Lower DynamicOverflowOrder = higher priority (stays visible longer).
                let mut prioritized: Vec<(usize, i32)> = primary_commands
                    .iter()
                    .enumerate()
                    .map(|(index, item)| (index, Self::get_dynamic_overflow_order(&**item)))
                    .collect();

                prioritized.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));

                // The set of the visible indices: one flag for each command.
                let mut visible_indices = vec![false; primary_commands.len()];
                let mut non_separator_count = 0;
                for &(index, _) in &prioritized {
                    if is_separator(&*primary_commands[index]) {
                        visible_indices[index] = true;
                    } else if non_separator_count < max_items {
                        visible_indices[index] = true;
                        non_separator_count += 1;
                    }
                }

                if non_separator_count == 0 {
                    visible_indices.fill(false);
                }

                Self::trim_orphaned_separators_from_visible_commands(&primary_commands, &mut visible_indices);

                for (index, item) in primary_commands.iter().enumerate() {
                    if visible_indices[index] {
                        self.set_overflow_mode(&**item, false);
                        self.visible_primary_commands.add(item.clone());
                    } else if is_separator(&**item) {
                        self.set_overflow_mode(&**item, false);
                    } else {
                        self.set_overflow_mode(&**item, true);
                        overflowed_primary_commands.push(item.clone());
                    }
                }
            }
        }

        self.add_overflow_items(&overflowed_primary_commands, &secondary_commands);
        self.set_has_secondary_commands(!self.overflow_items.is_empty());
        self.update_overflow_button_visibility();
    }

    fn set_overflow_mode(&self, element: &dyn ICommandBarElement, in_overflow: bool) {
        element.set_is_in_overflow(in_overflow);
        self.apply_label_position_to_element(element);
    }

    fn get_dynamic_overflow_order(element: &dyn ICommandBarElement) -> i32 {
        if let Some(button) = element_as::<CommandBarButton>(element) {
            button.dynamic_overflow_order()
        } else if let Some(toggle_button) = element_as::<CommandBarToggleButton>(element) {
            toggle_button.dynamic_overflow_order()
        } else {
            0
        }
    }

    fn apply_label_position_to_children(&self) {
        for command in self.primary_commands().snapshot().iter() {
            self.apply_label_position_to_element(&**command);
        }
        for command in self.secondary_commands().snapshot().iter() {
            self.apply_label_position_to_element(&**command);
        }

        self.apply_label_position_to_element(&*self.overflow_primary_secondary_separator);
    }

    fn apply_label_position_to_element(&self, element: &dyn ICommandBarElement) {
        let label_position = if element.is_in_overflow() {
            CommandBarDefaultLabelPosition::Right
        } else {
            self.default_label_position()
        };

        element.set_is_compact(label_position == CommandBarDefaultLabelPosition::Collapsed);

        if let Some(button) = element_as::<CommandBarButton>(element) {
            button.set_label_position(label_position);
        } else if let Some(toggle_button) = element_as::<CommandBarToggleButton>(element) {
            toggle_button.set_label_position(label_position);
        }
    }

    fn update_sticky_behavior(&self) {
        let overflow_popup = self.overflow_popup.borrow().clone();
        if let Some(overflow_popup) = overflow_popup {
            overflow_popup.set_is_light_dismiss_enabled(!self.is_sticky());
        }
    }

    fn update_overflow_button_visibility(&self) {
        self.set_is_overflow_button_visible(match self.overflow_button_visibility() {
            CommandBarOverflowButtonVisibility::Visible => true,
            CommandBarOverflowButtonVisibility::Collapsed => false,
            CommandBarOverflowButtonVisibility::Auto => self.has_secondary_commands(),
        });
    }

    fn add_overflow_items(
        &self,
        overflowed_primary_commands: &[Rc<dyn ICommandBarElement>],
        secondary_commands: &[Rc<dyn ICommandBarElement>],
    ) {
        for item in overflowed_primary_commands {
            self.overflow_items.add(item.clone());
        }

        if !overflowed_primary_commands.is_empty() && Self::has_visible_elements(secondary_commands) {
            self.set_overflow_mode(&*self.overflow_primary_secondary_separator, true);
            self.overflow_items.add(self.overflow_primary_secondary_separator.clone());
        }

        for item in secondary_commands {
            self.set_overflow_mode(&**item, true);
            self.overflow_items.add(item.clone());
        }
    }

    fn rebuild_secondary_command_visibility_subscriptions(&self) {
        self.secondary_command_visibility_subscriptions.clear();

        for command in self.secondary_commands().snapshot().iter() {
            if let Some(visual) = command.as_object().filter(|object| object.is::<Visual>()) {
                let is_initial_value = Cell::new(true);
                let weak = self.to_ref().downgrade();
                let subscription = FerroObjectExtensions::get_observable(visual, Visual::is_visible_property())
                    .subscribe_fn(move |_| {
                        if is_initial_value.replace(false) {
                            return;
                        }

                        if let Some(this) = weak.upgrade() {
                            this.request_dynamic_overflow_update();
                        }
                    });
                self.secondary_command_visibility_subscriptions.add(subscription);
            }
        }
    }

    fn has_visible_elements(commands: &[Rc<dyn ICommandBarElement>]) -> bool {
        commands.iter().any(|command| element_as::<Visual>(&**command).is_some_and(|visual| visual.is_visible()))
    }

    fn trim_orphaned_separators_from_visible_commands(
        commands: &[Rc<dyn ICommandBarElement>],
        visible_indices: &mut [bool],
    ) {
        let mut to_remove = Vec::new();
        for i in 0..commands.len() {
            if !visible_indices[i] || !is_separator(&*commands[i]) {
                continue;
            }

            let has_non_separator_before =
                Self::find_non_separator_in_visible_commands(commands, visible_indices, false, i as isize - 1)
                    .is_some();
            let has_non_separator_after =
                Self::find_non_separator_in_visible_commands(commands, visible_indices, true, i as isize + 1)
                    .is_some();

            if !has_non_separator_before || !has_non_separator_after {
                to_remove.push(i);
            }
        }

        for index in to_remove {
            visible_indices[index] = false;
        }

        let mut previous_was_separator = false;
        for i in 0..commands.len() {
            if !visible_indices[i] {
                continue;
            }

            if is_separator(&*commands[i]) {
                if previous_was_separator {
                    visible_indices[i] = false;
                } else {
                    previous_was_separator = true;
                }
            } else {
                previous_was_separator = false;
            }
        }
    }

    /// The index of the first visible command that is not a separator,
    /// searching from `start_index` in the given direction.
    fn find_non_separator_in_visible_commands(
        commands: &[Rc<dyn ICommandBarElement>],
        visible_indices: &[bool],
        forward: bool,
        start_index: isize,
    ) -> Option<usize> {
        let mut i = start_index;
        while if forward { i < commands.len() as isize } else { i >= 0 } {
            let index = i as usize;
            if visible_indices[index] && !is_separator(&*commands[index]) {
                return Some(index);
            }
            i += if forward { 1 } else { -1 };
        }
        None
    }
}
