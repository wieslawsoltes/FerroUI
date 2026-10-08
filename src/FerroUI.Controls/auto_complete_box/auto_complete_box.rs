// (c) Copyright Microsoft Corporation.
// This source is subject to the Microsoft Public License (Ms-PL).
// Please see https://go.microsoft.com/fwlink/?LinkID=131993 for details.
// All other rights reserved.
//
// Ported from the Silverlight Toolkit sources as adapted by the upstream
// project; the license text is in the `NOTICE.md` of this crate.
//
// This file holds both parts of the upstream partial class: the control and
// its properties.

use super::{AutoCompleteFilterMode, PopulatedEventArgs, PopulatingEventArgs};
use crate::primitives::{
    Popup, SelectingItemsControl, TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl,
    TemplatedControlImplExt,
};
use crate::templates::{FuncDataTemplate, IDataTemplate};
use crate::utils::{BindingEvaluator, ISelectionAdapter, SelectingItemsControlSelectionAdapter};
use crate::{
    AssignedBinding, ContentControl, ControlImpl, ItemsChangedEventArgs, ItemsChangedHandler, ItemsSource,
    SelectionChangedEventArgs, TextBlock, TextBox, TextChangedEventArgs,
};
use ferroui_base::animation::TimeSpan;
use ferroui_base::collections::{FerroList, NotifyCollectionChangedAction};
use ferroui_base::controls::NameScopeRef;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::BindingMode;
use ferroui_base::input::navigation::XYFocusHelpers;
use ferroui_base::input::{
    FocusChangedEventArgs, InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs,
};
use ferroui_base::interactivity::{
    Interactive, InteractiveImpl, RoutedEvent, RoutedEventHandlerToken, RoutingStrategies,
};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::IBrush;
use ferroui_base::reactive::{Disposable, IDisposable, ObservableExt};
use ferroui_base::threading::{
    CancellationToken, CancellationTokenSource, Dispatcher, DispatcherPriority, DispatcherTimer,
    FerroSynchronizationContext, OperationCanceledError,
};
use ferroui_base::utilities::{CancelEventArgs, HandlerList, StringComparison};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, ferro_routed_event, instantiate, BoxedValue,
    DirectProperty, FerroObject, FerroObjectExtensions, FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs,
    Ref, StyledElementImpl, StyledProperty, StyledPropertyMetadata, StyledPropertyOptions, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

/// Represents the filter used by the [`AutoCompleteBox`] control to
/// determine whether an item is a possible match for the specified text.
///
/// The function takes the string used as the basis for filtering and the
/// item that is compared with it, and returns `true` to indicate the item
/// is a possible match. `T` is the type used for filtering: the text form
/// of an item (`Option<String>`) or the item itself
/// (`Option<BoxedValue>`).
///
/// A filter is a reference: copies compare equal to what they were copied
/// from, two filters made from the same function do not.
pub struct AutoCompleteFilterPredicate<T: 'static>(Rc<dyn Fn(Option<&str>, &T) -> bool>);

impl<T: 'static> AutoCompleteFilterPredicate<T> {
    /// Wraps a filter function.
    pub fn new(filter: impl Fn(Option<&str>, &T) -> bool + 'static) -> Self {
        Self(Rc::new(filter))
    }

    /// Calls the filter.
    pub fn invoke(&self, search: Option<&str>, item: &T) -> bool {
        (self.0)(search, item)
    }
}

impl<T: 'static> Clone for AutoCompleteFilterPredicate<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: 'static> PartialEq for AutoCompleteFilterPredicate<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// Represents the selector used by the [`AutoCompleteBox`] control to
/// determine how the specified text should be modified with an item.
///
/// The function takes the string used as the basis for filtering and the
/// selected item that should be combined with it, and returns the modified
/// text that will be used by the control. `T` is the type used for
/// selecting: the text form of an item (`Option<String>`) or the item
/// itself (`BoxedValue`).
///
/// A selector is a reference: copies compare equal to what they were copied
/// from, two selectors made from the same function do not.
pub struct AutoCompleteSelector<T: 'static>(Rc<dyn Fn(Option<&str>, &T) -> String>);

impl<T: 'static> AutoCompleteSelector<T> {
    /// Wraps a selector function.
    pub fn new(selector: impl Fn(Option<&str>, &T) -> String + 'static) -> Self {
        Self(Rc::new(selector))
    }

    /// Calls the selector.
    pub fn invoke(&self, search: Option<&str>, item: &T) -> String {
        (self.0)(search, item)
    }
}

impl<T: 'static> Clone for AutoCompleteSelector<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: 'static> PartialEq for AutoCompleteSelector<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// The pending result of an [`AutoCompleteAsyncPopulator`]: the items for
/// the drop-down, or the notice that the population was cancelled.
pub type AutoCompletePopulation = Pin<Box<dyn Future<Output = Result<Vec<Option<BoxedValue>>, OperationCanceledError>>>>;

/// The asynchronous populator of an [`AutoCompleteBox`]: given the search
/// text and a cancellation token it produces the items for the drop-down.
/// The future is driven by the dispatcher of the control.
///
/// A populator is a reference: copies compare equal to what they were
/// copied from.
#[derive(Clone)]
pub struct AutoCompleteAsyncPopulator(Rc<dyn Fn(Option<String>, CancellationToken) -> AutoCompletePopulation>);

impl AutoCompleteAsyncPopulator {
    /// Wraps a populator function.
    pub fn new(populator: impl Fn(Option<String>, CancellationToken) -> AutoCompletePopulation + 'static) -> Self {
        Self(Rc::new(populator))
    }

    /// Calls the populator.
    pub fn invoke(&self, search_text: Option<String>, cancellation_token: CancellationToken) -> AutoCompletePopulation {
        (self.0)(search_text, cancellation_token)
    }
}

impl PartialEq for AutoCompleteAsyncPopulator {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// Specifies the name of the selection adapter template part.
const ELEMENT_SELECTION_ADAPTER: &str = "PART_SelectionAdapter";

/// Specifies the name of the selector template part.
const ELEMENT_SELECTOR: &str = "PART_SelectingItemsControl";

/// Specifies the name of the popup template part.
const ELEMENT_POPUP: &str = "PART_Popup";

/// The name for the text box part.
const ELEMENT_TEXT_BOX: &str = "PART_TextBox";

const PC_DROPDOWN_OPEN: &str = ":dropdownopen";

/// Resets a flag when dropped (the `finally` of the reference).
struct Reset<'a>(&'a Cell<bool>);

impl Drop for Reset<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

/// The length of a text in UTF-16 code units, the unit of the positions of
/// a text box.
fn utf16_length(text: Option<&str>) -> i32 {
    text.map_or(0, |text| text.encode_utf16().count() as i32)
}

/// The first `length` UTF-16 code units of a text.
fn utf16_prefix(text: &str, length: i32) -> String {
    let units: Vec<u16> = text.encode_utf16().take(length.max(0) as usize).collect();
    String::from_utf16_lossy(&units)
}

/// Represents a control that provides a text box for user input and a
/// drop-down that contains possible matches based on the input in the text
/// box.
#[repr(C)]
pub struct AutoCompleteBox {
    base: TemplatedControl,

    /// A local cached copy of the items data.
    items: RefCell<Option<Rc<Vec<Option<BoxedValue>>>>>,

    /// The observable collection that contains references to all of the
    /// items in the generated view of data that is provided to the
    /// selection-style control adapter.
    view: Rc<FerroList<Option<BoxedValue>>>,

    /// The view as the items source handed to the selection adapter.
    view_source: ItemsSource,

    /// A counter that tracks how many pending `Text` property change
    /// notifications should be ignored. This is used when the `Text`
    /// property is set programmatically and we want to suppress the
    /// corresponding change handlers. The counter is decremented after each
    /// ignored notification.
    ///
    /// The counter is important because the text changed event of the text
    /// box does not fire immediately; using a counter allows nested
    /// property changes to be ignored safely.
    ignore_text_property_change: Cell<i32>,

    /// A counter that tracks how many pending text changed notifications of
    /// the text box should be ignored. This is distinct from
    /// `ignore_text_property_change` and is used when the notification of
    /// the text box needs to be suppressed (e.g., during programmatic
    /// updates to the text box content). The counter is decremented after
    /// each ignored event.
    ignore_text_box_text_change: Cell<i32>,

    /// A value indicating whether to ignore calling a pending change
    /// handlers.
    ignore_property_change: Cell<bool>,

    /// A value indicating whether to ignore the selection changed event.
    ignore_text_selection_change: Cell<bool>,

    /// A value indicating whether to skip the text update processing when
    /// the selected item is updated.
    skip_selected_item_text_update: Cell<bool>,

    /// The last observed text box selection start location.
    text_selection_start: Cell<i32>,

    /// A value indicating whether the user initiated the current populate
    /// call.
    user_called_populate: Cell<bool>,

    /// A value indicating whether the popup has been opened at least once.
    popup_has_opened: Cell<bool>,

    /// The dispatcher timer used for the `MinimumPopulateDelay` condition
    /// for auto completion, with the subscription to its tick.
    delay_timer: RefCell<Option<(Rc<DispatcherTimer>, Rc<dyn IDisposable>)>>,

    /// A value indicating whether a read-only property change handler
    /// should allow the value to be set. This is used to ensure that
    /// read-only properties cannot be changed via `set_value`, etc.
    allow_write: Cell<bool>,

    /// A boolean indicating if a cancellation was requested.
    cancel_requested: Cell<bool>,

    /// A boolean indicating if filtering is in action.
    filter_in_action: Cell<bool>,

    /// The text box template part.
    text_box: RefCell<Option<Ref<TextBox>>>,
    text_box_subscriptions: RefCell<Option<Rc<dyn IDisposable>>>,

    /// The selection adapter, with the subscriptions to its events.
    adapter: RefCell<Option<Rc<dyn ISelectionAdapter>>>,
    adapter_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,

    /// A control that can provide updated string values from a binding.
    value_member_binding_evaluator: RefCell<Option<Ref<BindingEvaluator>>>,

    /// A weak subscription for the collection changed event: the collection
    /// and the token of the handler.
    collection_change_subscription: RefCell<Option<(ItemsSource, u64)>>,

    population_cancellation_token_source: RefCell<Option<CancellationTokenSource>>,

    item_template_is_from_value_member_binding: Cell<bool>,
    setting_item_template_from_value_member_binding: Cell<bool>,

    search_text: RefCell<Option<String>>,

    /// The drop down popup control, with the subscription to its closed
    /// event.
    drop_down_popup: RefCell<Option<(Ref<Popup>, Rc<dyn IDisposable>)>>,

    populating: HandlerList<dyn Fn(&PopulatingEventArgs)>,
    populated: HandlerList<dyn Fn(&PopulatedEventArgs)>,
    drop_down_opening: HandlerList<dyn Fn(&CancelEventArgs)>,
    drop_down_opened: HandlerList<dyn Fn()>,
    drop_down_closing: HandlerList<dyn Fn(&CancelEventArgs)>,
    drop_down_closed: HandlerList<dyn Fn()>,
}

ferro_class! {
    AutoCompleteBox: TemplatedControl, virtuals AutoCompleteBoxImpl: TemplatedControlImpl {
        /// Returns the [`ISelectionAdapter`] part, if possible. Otherwise,
        /// `None`.
        fn get_selection_adapter_part(this, name_scope: &NameScopeRef) -> Option<Rc<dyn ISelectionAdapter>>;
        /// Raises the `Populating` event.
        fn on_populating(this, e: &PopulatingEventArgs);
        /// Raises the `Populated` event.
        fn on_populated(this, e: &PopulatedEventArgs);
        /// Raises the `SelectionChanged` event.
        fn on_selection_changed(this, e: &SelectionChangedEventArgs);
        /// Raises the `DropDownOpening` event.
        fn on_drop_down_opening(this, e: &CancelEventArgs);
        /// Raises the `DropDownOpened` event.
        fn on_drop_down_opened(this);
        /// Raises the `DropDownClosing` event.
        fn on_drop_down_closing(this, e: &CancelEventArgs);
        /// Raises the `DropDownClosed` event.
        fn on_drop_down_closed(this);
        /// Raises the `TextChanged` event.
        fn on_text_changed(this, e: &TextChangedEventArgs);
        /// Converts the specified object to a string by using the binding
        /// specified by the `ValueMemberBinding` property.
        ///
        /// Override this method to provide a custom string conversion.
        fn format_value(this, value: &Option<BoxedValue>) -> Option<String>;
    }
}

ferro_class_info!(AutoCompleteBox {
    new: AutoCompleteBox::new,
    markup: {
        property_attributes: [ValueMemberBinding: [InheritDataTypeFromItems("ItemsSource")]],
        attributes: [
            TemplatePart("PART_Popup", type(Ref<Popup>)),
            TemplatePart("PART_SelectingItemsControl", type(Ref<SelectingItemsControl>)),
            TemplatePart("PART_SelectionAdapter", type(Rc<dyn ISelectionAdapter>)),
            TemplatePart("PART_TextBox", type(Ref<TextBox>)),
            PseudoClasses(":dropdownopen"),
        ],
    },
});

ferro_impl_classes!(AutoCompleteBox: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl);

impl ControlImpl for AutoCompleteBox {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::AutoCompleteBoxAutomationPeer::new(this).upcast()
    }
}

impl FerroObjectImpl for AutoCompleteBox {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::value_member_binding_property().as_property() {
            this.on_value_member_binding_changed(change.get_new_value::<Option<AssignedBinding>>());
        }
    }
}

impl InputElementImpl for AutoCompleteBox {
    /// Provides handling for the key down event.
    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        Self::parent_on_key_down(this, e);

        if e.handled() || !this.is_enabled() {
            return;
        }

        // The drop down is open, pass along the key event arguments to the
        // selection adapter. If it isn't handled by the adapter's logic,
        // then we handle some simple navigation scenarios for controlling
        // the drop down.
        if this.is_drop_down_open() {
            if let Some(adapter) = this.selection_adapter() {
                adapter.handle_key_down(e);
                if e.handled() {
                    return;
                }
            }

            if e.key == Key::Escape {
                this.on_adapter_selection_canceled();
                e.set_handled(true);
            }
        } else {
            // The drop down is not open, the Down key will toggle it open.
            // Ignore key buttons, if they are used for XY focus.
            if e.key == Key::Down && !XYFocusHelpers::is_allowed_xy_navigation_mode(this, Some(e.key_device_type)) {
                this.set_current_value(Self::is_drop_down_open_property(), true);
                e.set_handled(true);
            }
        }

        // Standard drop down navigation.
        match e.key {
            Key::F4 => {
                this.set_current_value(Self::is_drop_down_open_property(), !this.is_drop_down_open());
                e.set_handled(true);
            }
            Key::Enter => {
                if this.is_drop_down_open() {
                    this.on_adapter_selection_complete();
                    e.set_handled(true);
                }
            }
            _ => {}
        }
    }

    fn on_got_focus(this: &Self, e: &FocusChangedEventArgs) {
        if let Some(text_box) = this.text_box() {
            if this.text_box_selection_length() <= 0 {
                text_box.focus();
                text_box.select_all();
            }
        }

        Self::parent_on_got_focus(this, e);
    }

    fn on_lost_focus(this: &Self, e: &FocusChangedEventArgs) {
        this.set_current_value(Self::is_drop_down_open_property(), false);

        this.user_called_populate.set(false);

        let text_box = this.text_box();
        let text_box_context_menu_is_open = text_box.as_ref().is_some_and(|text_box| {
            text_box.context_flyout().is_some_and(|flyout| flyout.is_open())
                || text_box.context_menu().is_some_and(|menu| menu.is_open())
        });
        let context_menu_is_open = this.context_flyout().is_some_and(|flyout| flyout.is_open())
            || this.context_menu().is_some_and(|menu| menu.is_open());

        if !text_box_context_menu_is_open && !context_menu_is_open && this.clear_selection_on_lost_focus() {
            this.clear_text_box_selection();
        }

        Self::parent_on_lost_focus(this, e);
    }
}

impl TemplatedControlImpl for AutoCompleteBox {
    /// Builds the visual tree for the [`AutoCompleteBox`] control when a
    /// new template is applied.
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        if let Some((_, closed)) = this.drop_down_popup.take() {
            closed.dispose();
        }

        // Set the template parts. Individual part setters remove and add
        // any event handlers.
        let popup = e.name_scope().find_as::<Popup>(ELEMENT_POPUP);
        if let Some(popup) = &popup {
            let weak = this.to_ref().downgrade();
            let closed = popup.closed(move || {
                if let Some(this) = weak.upgrade() {
                    this.drop_down_popup_closed();
                }
            });
            *this.drop_down_popup.borrow_mut() = Some((popup.clone(), closed));
        }

        this.set_selection_adapter(this.get_selection_adapter_part(e.name_scope()));
        this.set_text_box(e.name_scope().find_as::<TextBox>(ELEMENT_TEXT_BOX));

        // If the drop down property indicates that the popup is open, flip
        // its value to invoke the changed handler.
        if this.is_drop_down_open() && popup.is_some_and(|popup| !popup.is_open()) {
            this.opening_drop_down(false);
        }

        Self::parent_on_apply_template(this, e);
    }
}

impl AutoCompleteBoxImpl for AutoCompleteBox {
    fn get_selection_adapter_part(_this: &Self, name_scope: &NameScopeRef) -> Option<Rc<dyn ISelectionAdapter>> {
        let mut adapter: Option<Rc<dyn ISelectionAdapter>> = None;
        if let Some(selector) = name_scope.find_as::<SelectingItemsControl>(ELEMENT_SELECTOR) {
            // Check if it is already a selection adapter.
            adapter = name_scope.find(ELEMENT_SELECTOR).and_then(|object| <dyn ISelectionAdapter>::from_object(&object));
            if adapter.is_none() {
                // Built in support for wrapping a selector control.
                adapter = Some(SelectingItemsControlSelectionAdapter::with_selector(selector));
            }
        }
        if adapter.is_none() {
            adapter = name_scope.find(ELEMENT_SELECTION_ADAPTER).map(|object| {
                <dyn ISelectionAdapter>::from_object(&object).unwrap_or_else(|| {
                    panic!(
                        "Expected control '{ELEMENT_SELECTION_ADAPTER}' to be 'ISelectionAdapter' but it was '{}'.",
                        object.get_type().name()
                    )
                })
            });
        }
        adapter
    }

    fn on_populating(this: &Self, e: &PopulatingEventArgs) {
        for (_, handler) in this.populating.snapshot().iter() {
            handler(e);
        }
    }

    fn on_populated(this: &Self, e: &PopulatedEventArgs) {
        for (_, handler) in this.populated.snapshot().iter() {
            handler(e);
        }
    }

    fn on_selection_changed(this: &Self, e: &SelectionChangedEventArgs) {
        this.raise_event(e);
    }

    fn on_drop_down_opening(this: &Self, e: &CancelEventArgs) {
        for (_, handler) in this.drop_down_opening.snapshot().iter() {
            handler(e);
        }
    }

    fn on_drop_down_opened(this: &Self) {
        for (_, handler) in this.drop_down_opened.snapshot().iter() {
            handler();
        }
    }

    fn on_drop_down_closing(this: &Self, e: &CancelEventArgs) {
        for (_, handler) in this.drop_down_closing.snapshot().iter() {
            handler(e);
        }
    }

    fn on_drop_down_closed(this: &Self) {
        for (_, handler) in this.drop_down_closed.snapshot().iter() {
            handler();
        }
    }

    fn on_text_changed(this: &Self, e: &TextChangedEventArgs) {
        this.raise_event(e);
    }

    fn format_value(this: &Self, value: &Option<BoxedValue>) -> Option<String> {
        if let Some(value_member_binding) = this.value_member_binding() {
            let evaluator = this.value_member_binding_evaluator.borrow().clone();
            let evaluator = evaluator.unwrap_or_else(|| {
                let evaluator = BindingEvaluator::new();
                *this.value_member_binding_evaluator.borrow_mut() = Some(evaluator.clone());
                evaluator
            });
            evaluator.update_binding(&value_member_binding);
            let result = evaluator.evaluate_string(value).unwrap_or_default();
            evaluator.clear_data_context();
            return Some(result);
        }

        Some(ValueTypes::to_display_string(value.as_ref()))
    }
}

ferro_properties! {
    impl AutoCompleteBox {
        /// Defines the `CaretIndex` property.
        pub fn caret_index_property() -> StyledProperty<i32> {
            TextBox::caret_index_property().add_owner_with::<AutoCompleteBox>(
                StyledPropertyMetadata::new(Some(0)).with_default_binding_mode(BindingMode::TwoWay),
            )
        }

        /// Defines the `PlaceholderText` property.
        pub fn placeholder_text_property() -> StyledProperty<Option<String>> {
            TextBox::placeholder_text_property().add_owner::<AutoCompleteBox>()
        }

        /// Defines the `PlaceholderForeground` property.
        pub fn placeholder_foreground_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            TextBox::placeholder_foreground_property().add_owner::<AutoCompleteBox>()
        }

        /// Identifies the `MinimumPrefixLength` property.
        pub fn minimum_prefix_length_property() -> StyledProperty<i32> {
            FerroProperty::register_with::<AutoCompleteBox, _>(
                "MinimumPrefixLength",
                StyledPropertyOptions::new(1).validate(AutoCompleteBox::is_valid_minimum_prefix_length),
            )
        }

        /// Identifies the `MinimumPopulateDelay` property.
        pub fn minimum_populate_delay_property() -> StyledProperty<TimeSpan> {
            FerroProperty::register_with::<AutoCompleteBox, _>(
                "MinimumPopulateDelay",
                StyledPropertyOptions::new(TimeSpan::ZERO).validate(AutoCompleteBox::is_valid_minimum_populate_delay),
            )
        }

        /// Identifies the `MaxDropDownHeight` property.
        pub fn max_drop_down_height_property() -> StyledProperty<f64> {
            FerroProperty::register_with::<AutoCompleteBox, _>(
                "MaxDropDownHeight",
                StyledPropertyOptions::new(f64::INFINITY).validate(AutoCompleteBox::is_valid_max_drop_down_height),
            )
        }

        /// Identifies the `IsTextCompletionEnabled` property.
        pub fn is_text_completion_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<AutoCompleteBox, _>("IsTextCompletionEnabled", false)
        }

        /// Identifies the `ItemTemplate` property.
        pub fn item_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register::<AutoCompleteBox, _>("ItemTemplate", None)
        }

        /// Defines the `ClearSelectionOnLostFocus` property.
        pub fn clear_selection_on_lost_focus_property() -> StyledProperty<bool> {
            TextBox::clear_selection_on_lost_focus_property().add_owner::<AutoCompleteBox>()
        }

        /// Identifies the `IsDropDownOpen` property.
        pub fn is_drop_down_open_property() -> StyledProperty<bool> {
            FerroProperty::register::<AutoCompleteBox, _>("IsDropDownOpen", false)
        }

        /// Identifies the `SelectedItem` property.
        pub fn selected_item_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register_with::<AutoCompleteBox, _>(
                "SelectedItem",
                StyledPropertyOptions::new(None).default_binding_mode(BindingMode::TwoWay).enable_data_validation(true),
            )
        }

        /// Identifies the `Text` property.
        pub fn text_property() -> StyledProperty<Option<String>> {
            TextBlock::text_property().add_owner_with::<AutoCompleteBox>(
                StyledPropertyMetadata::new(Some(Some(String::new())))
                    .with_default_binding_mode(BindingMode::TwoWay)
                    .with_enable_data_validation(true),
            )
        }

        /// Identifies the `SearchText` property.
        pub fn search_text_property() -> DirectProperty<AutoCompleteBox, Option<String>> {
            FerroProperty::register_direct::<AutoCompleteBox, _>(
                "SearchText",
                |o| o.search_text(),
                None,
                Some(String::new()),
            )
        }

        /// Gets the identifier for the `FilterMode` property.
        pub fn filter_mode_property() -> StyledProperty<AutoCompleteFilterMode> {
            FerroProperty::register_with::<AutoCompleteBox, _>(
                "FilterMode",
                StyledPropertyOptions::new(AutoCompleteFilterMode::StartsWith)
                    .validate(AutoCompleteBox::is_valid_filter_mode),
            )
        }

        /// Identifies the `ItemFilter` property.
        pub fn item_filter_property() -> StyledProperty<Option<AutoCompleteFilterPredicate<Option<BoxedValue>>>> {
            FerroProperty::register::<AutoCompleteBox, _>("ItemFilter", None)
        }

        /// Identifies the `TextFilter` property.
        pub fn text_filter_property() -> StyledProperty<Option<AutoCompleteFilterPredicate<Option<String>>>> {
            FerroProperty::register::<AutoCompleteBox, _>(
                "TextFilter",
                AutoCompleteSearch::get_filter(AutoCompleteFilterMode::StartsWith),
            )
        }

        /// Identifies the `ItemSelector` property.
        pub fn item_selector_property() -> StyledProperty<Option<AutoCompleteSelector<BoxedValue>>> {
            FerroProperty::register::<AutoCompleteBox, _>("ItemSelector", None)
        }

        /// Identifies the `TextSelector` property.
        pub fn text_selector_property() -> StyledProperty<Option<AutoCompleteSelector<Option<String>>>> {
            FerroProperty::register::<AutoCompleteBox, _>("TextSelector", None)
        }

        /// Identifies the `ItemsSource` property.
        pub fn items_source_property() -> StyledProperty<Option<ItemsSource>> {
            FerroProperty::register::<AutoCompleteBox, _>("ItemsSource", None)
        }

        /// Identifies the `AsyncPopulator` property.
        pub fn async_populator_property() -> StyledProperty<Option<AutoCompleteAsyncPopulator>> {
            FerroProperty::register::<AutoCompleteBox, _>("AsyncPopulator", None)
        }

        /// Defines the `MaxLength` property.
        pub fn max_length_property() -> StyledProperty<i32> {
            TextBox::max_length_property().add_owner::<AutoCompleteBox>()
        }

        /// Defines the `InnerLeftContent` property.
        pub fn inner_left_content_property() -> StyledProperty<Option<BoxedValue>> {
            TextBox::inner_left_content_property().add_owner::<AutoCompleteBox>()
        }

        /// Defines the `InnerRightContent` property.
        pub fn inner_right_content_property() -> StyledProperty<Option<BoxedValue>> {
            TextBox::inner_right_content_property().add_owner::<AutoCompleteBox>()
        }

        /// Defines the `ValueMemberBinding` property.
        // XAML-SEAM: [InheritDataTypeFromItems(nameof(ItemsSource))] on the property (compiled bindings in markup).
        pub fn value_member_binding_property() -> StyledProperty<Option<AssignedBinding>> {
            FerroProperty::register_with::<AutoCompleteBox, _>(
                "ValueMemberBinding",
                StyledPropertyOptions::new(None).assign_binding(true),
            )
        }
    }
}

impl AutoCompleteBox {
    /// Defines the `Watermark` property.
    #[deprecated(note = "Use placeholder_text_property instead.")]
    pub fn watermark_property() -> &'static StyledProperty<Option<String>> {
        Self::placeholder_text_property()
    }

    /// Defines the `WatermarkForeground` property.
    #[deprecated(note = "Use placeholder_foreground_property instead.")]
    pub fn watermark_foreground_property() -> &'static StyledProperty<Option<Rc<dyn IBrush>>> {
        Self::placeholder_foreground_property()
    }

    ferro_routed_event!(
        /// Defines the `SelectionChanged` event.
        pub fn selection_changed_event() -> RoutedEvent<SelectionChangedEventArgs> {
            RoutedEvent::register::<AutoCompleteBox, _>("SelectionChanged", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `TextChanged` event.
        pub fn text_changed_event() -> RoutedEvent<TextChangedEventArgs> {
            RoutedEvent::register::<AutoCompleteBox, _>("TextChanged", RoutingStrategies::BUBBLE)
        }
    );

    fn is_valid_minimum_prefix_length(value: &i32) -> bool {
        *value >= -1
    }

    fn is_valid_minimum_populate_delay(value: &TimeSpan) -> bool {
        value.total_milliseconds() >= 0.0
    }

    fn is_valid_max_drop_down_height(value: &f64) -> bool {
        *value >= 0.0
    }

    fn is_valid_filter_mode(mode: &AutoCompleteFilterMode) -> bool {
        match mode {
            AutoCompleteFilterMode::None
            | AutoCompleteFilterMode::StartsWith
            | AutoCompleteFilterMode::StartsWithCaseSensitive
            | AutoCompleteFilterMode::StartsWithOrdinal
            | AutoCompleteFilterMode::StartsWithOrdinalCaseSensitive
            | AutoCompleteFilterMode::Contains
            | AutoCompleteFilterMode::ContainsCaseSensitive
            | AutoCompleteFilterMode::ContainsOrdinal
            | AutoCompleteFilterMode::ContainsOrdinalCaseSensitive
            | AutoCompleteFilterMode::Equals
            | AutoCompleteFilterMode::EqualsCaseSensitive
            | AutoCompleteFilterMode::EqualsOrdinal
            | AutoCompleteFilterMode::EqualsOrdinalCaseSensitive
            | AutoCompleteFilterMode::Custom => true,
        }
    }

    /// Handle the change of the `IsEnabled` property.
    fn on_control_is_enabled_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let is_enabled = e.get_new_value::<bool>();
        if !is_enabled {
            self.set_current_value(Self::is_drop_down_open_property(), false);
        }
    }

    /// `MinimumPopulateDelay` property changed handler. Any current
    /// dispatcher timer will be stopped. The timer will not be restarted
    /// until the next text update call by the user.
    fn on_minimum_populate_delay_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let new_value = e.get_new_value::<TimeSpan>();

        // Stop any existing timer.
        let existing = self.delay_timer.borrow().as_ref().map(|(timer, _)| timer.clone());
        if let Some(timer) = existing {
            timer.stop();

            if new_value == TimeSpan::ZERO {
                if let Some((_, tick)) = self.delay_timer.take() {
                    tick.dispose();
                }
            }
        }

        if new_value > TimeSpan::ZERO {
            // Create or clear a dispatcher timer instance.
            let timer = self.delay_timer.borrow().as_ref().map(|(timer, _)| timer.clone());
            let timer = timer.unwrap_or_else(|| {
                let timer = DispatcherTimer::new();
                let weak = self.to_ref().downgrade();
                let tick = timer.tick(move |_| {
                    if let Some(this) = weak.upgrade() {
                        this.populate_drop_down();
                    }
                });
                *self.delay_timer.borrow_mut() = Some((timer.clone(), tick));
                timer
            });

            // Set the new tick interval: the delay is positive here, so it is a duration.
            timer.set_interval(new_value.to_duration().unwrap_or_default());
        }
    }

    /// `IsDropDownOpen` property changed handler.
    fn on_is_drop_down_open_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        // Ignore the change if requested.
        if self.ignore_property_change.get() {
            self.ignore_property_change.set(false);
            return;
        }

        let (old_value, new_value) = e.get_old_and_new_value::<bool>();

        if new_value {
            self.text_updated(self.text(), true);
        } else {
            self.closing_drop_down(old_value);
        }

        self.update_pseudo_classes();
    }

    fn on_selected_item_property_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        if self.ignore_property_change.get() {
            self.ignore_property_change.set(false);
            return;
        }

        let (old_value, new_value) = e.get_old_and_new_value::<Option<BoxedValue>>();

        // Update the text display.
        if self.skip_selected_item_text_update.get() {
            self.skip_selected_item_text_update.set(false);
        } else {
            self.on_selected_item_changed(&new_value);
        }

        // Fire the SelectionChanged event.
        let mut removed = Vec::new();
        if old_value.is_some() {
            removed.push(old_value);
        }

        let mut added = Vec::new();
        if new_value.is_some() {
            added.push(new_value);
        }

        self.on_selection_changed(&SelectionChangedEventArgs::new(
            Some(Self::selection_changed_event()),
            removed,
            added,
        ));
    }

    /// `Text` property changed handler.
    fn on_text_property_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        if self.ignore_text_property_change.get() > 0 {
            self.ignore_text_property_change.set(self.ignore_text_property_change.get() - 1);
            return;
        }

        self.text_updated(e.get_new_value::<Option<String>>(), false);
    }

    fn on_search_text_property_changed(&self, _e: &FerroPropertyChangedEventArgs<'_>) {
        if self.ignore_property_change.get() {
            self.ignore_property_change.set(false);
            return;
        }

        // Ensure the property is only written when expected.
        if !self.allow_write.get() {
            // The reference resets the old value here before it reports the
            // error; the property has no setter, so there is nothing that
            // could have written it and nothing to write the old value
            // with.
            panic!("Cannot set read-only property SearchText.");
        }
    }

    /// `FilterMode` property changed handler.
    fn on_filter_mode_property_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let mode = e.get_new_value::<AutoCompleteFilterMode>();

        // Sets the filter predicate for the new value.
        self.set_current_value(Self::text_filter_property(), AutoCompleteSearch::get_filter(mode));
    }

    /// `ItemFilter` property changed handler.
    fn on_item_filter_property_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let value = e.get_new_value::<Option<AutoCompleteFilterPredicate<Option<BoxedValue>>>>();

        // If null, revert to the "None" predicate.
        if value.is_none() {
            self.set_current_value(Self::filter_mode_property(), AutoCompleteFilterMode::None);
        } else {
            self.set_current_value(Self::filter_mode_property(), AutoCompleteFilterMode::Custom);
            self.set_current_value(Self::text_filter_property(), None);
        }
    }

    /// `ItemsSource` property changed handler.
    fn on_items_source_property_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        self.on_items_source_changed(e.get_new_value::<Option<ItemsSource>>());
    }

    fn on_item_template_property_changed(&self, _e: &FerroPropertyChangedEventArgs<'_>) {
        if !self.setting_item_template_from_value_member_binding.get() {
            self.item_template_is_from_value_member_binding.set(false);
        }
    }

    fn on_value_member_binding_changed(&self, value: Option<AssignedBinding>) {
        if self.item_template_is_from_value_member_binding.get() {
            let template: Rc<dyn IDataTemplate> = FuncDataTemplate::new(
                |data| data.is_some(),
                move |_, _| {
                    let control = ContentControl::new();
                    if let Some(value) = &value {
                        control.bind_binding(ContentControl::content_property().as_property(), &**value);
                    }
                    Some(control.upcast())
                },
                false,
            );

            self.setting_item_template_from_value_member_binding.set(true);
            self.set_current_value(Self::item_template_property(), Some(template));
            self.setting_item_template_from_value_member_binding.set(false);
        }
    }

    fn static_constructor() {
        InputElement::focusable_property().override_default_value::<AutoCompleteBox>(true);
        InputElement::is_tab_stop_property().override_default_value::<AutoCompleteBox>(false);

        Self::minimum_populate_delay_property()
            .changed()
            .add_class_handler::<AutoCompleteBox>(|x, e| x.on_minimum_populate_delay_changed(e));
        Self::is_drop_down_open_property()
            .changed()
            .add_class_handler::<AutoCompleteBox>(|x, e| x.on_is_drop_down_open_changed(e));
        Self::selected_item_property()
            .changed()
            .add_class_handler::<AutoCompleteBox>(|x, e| x.on_selected_item_property_changed(e));
        Self::text_property().changed().add_class_handler::<AutoCompleteBox>(|x, e| x.on_text_property_changed(e));
        Self::search_text_property()
            .changed()
            .add_class_handler::<AutoCompleteBox>(|x, e| x.on_search_text_property_changed(e));
        Self::filter_mode_property()
            .changed()
            .add_class_handler::<AutoCompleteBox>(|x, e| x.on_filter_mode_property_changed(e));
        Self::item_filter_property()
            .changed()
            .add_class_handler::<AutoCompleteBox>(|x, e| x.on_item_filter_property_changed(e));
        Self::items_source_property()
            .changed()
            .add_class_handler::<AutoCompleteBox>(|x, e| x.on_items_source_property_changed(e));
        Self::item_template_property()
            .changed()
            .add_class_handler::<AutoCompleteBox>(|x, e| x.on_item_template_property_changed(e));
        InputElement::is_enabled_property()
            .changed()
            .add_class_handler::<AutoCompleteBox>(|x, e| x.on_control_is_enabled_changed(e));
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        // The view object is always present: this is the first call of the
        // view clearing helper of the reference.
        let view = Rc::new(FerroList::new());
        let view_source = ItemsSource::from(view.clone());

        Self {
            base: TemplatedControl::construct(),
            items: RefCell::new(None),
            view,
            view_source,
            ignore_text_property_change: Cell::new(0),
            ignore_text_box_text_change: Cell::new(0),
            ignore_property_change: Cell::new(false),
            ignore_text_selection_change: Cell::new(false),
            skip_selected_item_text_update: Cell::new(false),
            text_selection_start: Cell::new(0),
            user_called_populate: Cell::new(false),
            popup_has_opened: Cell::new(false),
            delay_timer: RefCell::new(None),
            allow_write: Cell::new(false),
            cancel_requested: Cell::new(false),
            filter_in_action: Cell::new(false),
            text_box: RefCell::new(None),
            text_box_subscriptions: RefCell::new(None),
            adapter: RefCell::new(None),
            adapter_subscriptions: RefCell::new(Vec::new()),
            value_member_binding_evaluator: RefCell::new(None),
            collection_change_subscription: RefCell::new(None),
            population_cancellation_token_source: RefCell::new(None),
            item_template_is_from_value_member_binding: Cell::new(true),
            setting_item_template_from_value_member_binding: Cell::new(false),
            search_text: RefCell::new(Some(String::new())),
            drop_down_popup: RefCell::new(None),
            populating: HandlerList::new(),
            populated: HandlerList::new(),
            drop_down_opening: HandlerList::new(),
            drop_down_opened: HandlerList::new(),
            drop_down_closing: HandlerList::new(),
            drop_down_closed: HandlerList::new(),
        }
    }

    /// Initializes a new instance of the [`AutoCompleteBox`] class.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    // --- properties ---------------------------------------------------------

    /// Gets or sets the caret index.
    pub fn caret_index(&self) -> i32 {
        self.get_value(Self::caret_index_property())
    }

    pub fn set_caret_index(&self, value: i32) {
        self.set_value(Self::caret_index_property(), value)
    }

    /// Gets or sets the minimum number of characters required to be entered
    /// in the text box before the [`AutoCompleteBox`] displays possible
    /// matches. The default is 1.
    ///
    /// If you set `MinimumPrefixLength` to -1, the control will not provide
    /// possible matches. There is no maximum value, but setting it to a
    /// value that is too large will prevent the control from providing
    /// possible matches as well.
    pub fn minimum_prefix_length(&self) -> i32 {
        self.get_value(Self::minimum_prefix_length_property())
    }

    pub fn set_minimum_prefix_length(&self, value: i32) {
        self.set_value(Self::minimum_prefix_length_property(), value)
    }

    /// Gets or sets a value indicating whether the first possible match
    /// found during the filtering process will be displayed automatically
    /// in the text box. The default is `false`.
    pub fn is_text_completion_enabled(&self) -> bool {
        self.get_value(Self::is_text_completion_enabled_property())
    }

    pub fn set_is_text_completion_enabled(&self, value: bool) {
        self.set_value(Self::is_text_completion_enabled_property(), value)
    }

    /// Gets or sets the data template that is used to display the items in
    /// the drop-down portion of the [`AutoCompleteBox`] control.
    pub fn item_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::item_template_property())
    }

    pub fn set_item_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::item_template_property(), value)
    }

    /// Gets or sets the minimum delay after text is typed in the text box
    /// before the control populates the list of possible matches in the
    /// drop-down. The default is zero.
    pub fn minimum_populate_delay(&self) -> TimeSpan {
        self.get_value(Self::minimum_populate_delay_property())
    }

    pub fn set_minimum_populate_delay(&self, value: TimeSpan) {
        self.set_value(Self::minimum_populate_delay_property(), value)
    }

    /// Gets or sets the maximum height of the drop-down portion of the
    /// [`AutoCompleteBox`] control. The default is infinity.
    pub fn max_drop_down_height(&self) -> f64 {
        self.get_value(Self::max_drop_down_height_property())
    }

    pub fn set_max_drop_down_height(&self, value: f64) {
        self.set_value(Self::max_drop_down_height_property(), value)
    }

    /// Gets or sets a value indicating whether the drop-down portion of the
    /// control is open.
    pub fn is_drop_down_open(&self) -> bool {
        self.get_value(Self::is_drop_down_open_property())
    }

    pub fn set_is_drop_down_open(&self, value: bool) {
        self.set_value(Self::is_drop_down_open_property(), value)
    }

    /// Gets or sets a value that determines whether the control clears its
    /// selection after it loses focus.
    pub fn clear_selection_on_lost_focus(&self) -> bool {
        self.get_value(Self::clear_selection_on_lost_focus_property())
    }

    pub fn set_clear_selection_on_lost_focus(&self, value: bool) {
        self.set_value(Self::clear_selection_on_lost_focus_property(), value)
    }

    /// Gets or sets the binding that is used to get the values for display
    /// in the text box portion of the [`AutoCompleteBox`] control, and to
    /// filter items for display in the drop-down.
    pub fn value_member_binding(&self) -> Option<AssignedBinding> {
        self.get_value(Self::value_member_binding_property())
    }

    pub fn set_value_member_binding(&self, value: Option<AssignedBinding>) {
        self.set_value(Self::value_member_binding_property(), value)
    }

    /// Gets or sets the selected item in the drop-down. The default is
    /// `None`.
    ///
    /// If the `IsTextCompletionEnabled` property is `true` and text typed
    /// by the user matches an item in the `ItemsSource` collection, which
    /// is then displayed in the text box, the `SelectedItem` property will
    /// be a null reference.
    pub fn selected_item(&self) -> Option<BoxedValue> {
        self.get_value(Self::selected_item_property())
    }

    pub fn set_selected_item(&self, value: Option<BoxedValue>) {
        self.set_value(Self::selected_item_property(), value)
    }

    /// Gets or sets the text in the text box portion of the
    /// [`AutoCompleteBox`] control.
    pub fn text(&self) -> Option<String> {
        self.get_value(Self::text_property())
    }

    pub fn set_text(&self, value: Option<&str>) {
        self.set_value(Self::text_property(), value.map(str::to_owned))
    }

    /// Gets the text that is used to filter items in the `ItemsSource` item
    /// collection.
    ///
    /// The search text value is typically the same as the `Text` property,
    /// but is set after the text changed event occurs and before the
    /// `Populating` event.
    pub fn search_text(&self) -> Option<String> {
        self.search_text.borrow().clone()
    }

    fn set_search_text(&self, value: Option<String>) {
        self.allow_write.set(true);
        let _reset = Reset(&self.allow_write);
        self.set_and_raise(Self::search_text_property(), &self.search_text, value);
    }

    /// Gets or sets how the text in the text box is used to filter items
    /// specified by the `ItemsSource` property for display in the
    /// drop-down. The default is [`AutoCompleteFilterMode::StartsWith`].
    ///
    /// Use the `FilterMode` property to specify how possible matches are
    /// filtered. For example, possible matches can be filtered in a
    /// predefined or custom way. `FilterMode` is automatically set to
    /// `Custom` if you set the `ItemFilter` property.
    pub fn filter_mode(&self) -> AutoCompleteFilterMode {
        self.get_value(Self::filter_mode_property())
    }

    pub fn set_filter_mode(&self, value: AutoCompleteFilterMode) {
        self.set_value(Self::filter_mode_property(), value)
    }

    /// Gets or sets the placeholder or descriptive text that is displayed
    /// even if the `Text` property is not yet set.
    pub fn placeholder_text(&self) -> Option<String> {
        self.get_value(Self::placeholder_text_property())
    }

    pub fn set_placeholder_text(&self, value: Option<&str>) {
        self.set_value(Self::placeholder_text_property(), value.map(str::to_owned))
    }

    /// Gets or sets the placeholder or descriptive text that is displayed
    /// even if the `Text` property is not yet set.
    #[deprecated(note = "Use placeholder_text instead.")]
    pub fn watermark(&self) -> Option<String> {
        self.placeholder_text()
    }

    #[deprecated(note = "Use set_placeholder_text instead.")]
    pub fn set_watermark(&self, value: Option<&str>) {
        self.set_placeholder_text(value)
    }

    /// Gets or sets the brush used for the foreground color of the
    /// placeholder text.
    pub fn placeholder_foreground(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::placeholder_foreground_property())
    }

    pub fn set_placeholder_foreground(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::placeholder_foreground_property(), value)
    }

    /// Gets or sets the brush used for the foreground color of the
    /// placeholder text.
    #[deprecated(note = "Use placeholder_foreground instead.")]
    pub fn watermark_foreground(&self) -> Option<Rc<dyn IBrush>> {
        self.placeholder_foreground()
    }

    #[deprecated(note = "Use set_placeholder_foreground instead.")]
    pub fn set_watermark_foreground(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_placeholder_foreground(value)
    }

    /// Gets or sets the custom method that uses user-entered text to filter
    /// the items specified by the `ItemsSource` property for display in the
    /// drop-down. The default is `None`.
    ///
    /// The filter mode is automatically set to `Custom` if you set the
    /// `ItemFilter` property.
    pub fn item_filter(&self) -> Option<AutoCompleteFilterPredicate<Option<BoxedValue>>> {
        self.get_value(Self::item_filter_property())
    }

    pub fn set_item_filter(&self, value: Option<AutoCompleteFilterPredicate<Option<BoxedValue>>>) {
        self.set_value(Self::item_filter_property(), value)
    }

    /// Gets or sets the custom method that uses the user-entered text to
    /// filter items specified by the `ItemsSource` property in a text-based
    /// way for display in the drop-down.
    ///
    /// The search mode is automatically set to `Custom` if you set the
    /// `TextFilter` property.
    pub fn text_filter(&self) -> Option<AutoCompleteFilterPredicate<Option<String>>> {
        self.get_value(Self::text_filter_property())
    }

    pub fn set_text_filter(&self, value: Option<AutoCompleteFilterPredicate<Option<String>>>) {
        self.set_value(Self::text_filter_property(), value)
    }

    /// Gets or sets the custom method that combines the user-entered text
    /// and one of the items specified by the `ItemsSource`.
    pub fn item_selector(&self) -> Option<AutoCompleteSelector<BoxedValue>> {
        self.get_value(Self::item_selector_property())
    }

    pub fn set_item_selector(&self, value: Option<AutoCompleteSelector<BoxedValue>>) {
        self.set_value(Self::item_selector_property(), value)
    }

    /// Gets or sets the custom method that combines the user-entered text
    /// and one of the items specified by the `ItemsSource` in a text-based
    /// way.
    pub fn text_selector(&self) -> Option<AutoCompleteSelector<Option<String>>> {
        self.get_value(Self::text_selector_property())
    }

    pub fn set_text_selector(&self, value: Option<AutoCompleteSelector<Option<String>>>) {
        self.set_value(Self::text_selector_property(), value)
    }

    /// Gets or sets the asynchronous populator of the drop-down.
    pub fn async_populator(&self) -> Option<AutoCompleteAsyncPopulator> {
        self.get_value(Self::async_populator_property())
    }

    pub fn set_async_populator(&self, value: Option<AutoCompleteAsyncPopulator>) {
        self.set_value(Self::async_populator_property(), value)
    }

    /// Gets or sets a collection that is used to generate the items for the
    /// drop-down portion of the [`AutoCompleteBox`] control.
    pub fn items_source(&self) -> Option<ItemsSource> {
        self.get_value(Self::items_source_property())
    }

    pub fn set_items_source(&self, value: Option<ItemsSource>) {
        self.set_value(Self::items_source_property(), value)
    }

    /// Gets or sets the maximum number of characters that the
    /// [`AutoCompleteBox`] can accept. This constraint only applies for
    /// manually entered (user-inputted) text.
    pub fn max_length(&self) -> i32 {
        self.get_value(Self::max_length_property())
    }

    pub fn set_max_length(&self, value: i32) {
        self.set_value(Self::max_length_property(), value)
    }

    /// Gets or sets custom content that is positioned on the left side of
    /// the text layout box.
    pub fn inner_left_content(&self) -> Option<BoxedValue> {
        self.get_value(Self::inner_left_content_property())
    }

    pub fn set_inner_left_content(&self, value: Option<BoxedValue>) {
        self.set_value(Self::inner_left_content_property(), value)
    }

    /// Gets or sets custom content that is positioned on the right side of
    /// the text layout box.
    pub fn inner_right_content(&self) -> Option<BoxedValue> {
        self.get_value(Self::inner_right_content_property())
    }

    pub fn set_inner_right_content(&self, value: Option<BoxedValue>) {
        self.set_value(Self::inner_right_content_property(), value)
    }

    // --- template parts -----------------------------------------------------

    /// The drop down popup control.
    fn drop_down_popup(&self) -> Option<Ref<Popup>> {
        self.drop_down_popup.borrow().as_ref().map(|(popup, _)| popup.clone())
    }

    /// The text box template part.
    fn text_box(&self) -> Option<Ref<TextBox>> {
        self.text_box.borrow().clone()
    }

    fn set_text_box(&self, value: Option<Ref<TextBox>>) {
        if let Some(subscriptions) = self.text_box_subscriptions.take() {
            subscriptions.dispose();
        }
        *self.text_box.borrow_mut() = value.clone();

        // Attach handlers.
        if let Some(text_box) = value {
            let weak = self.to_ref().downgrade();
            // The value at the time of subscribing is skipped.
            let skipped = Cell::new(false);
            let object: &FerroObject = &text_box;
            let subscription =
                FerroObjectExtensions::get_observable(object, TextBox::text_property()).subscribe_fn(move |_| {
                    if !skipped.replace(true) {
                        return;
                    }
                    if let Some(this) = weak.upgrade() {
                        this.on_text_box_text_changed();
                    }
                });
            *self.text_box_subscriptions.borrow_mut() = Some(subscription);

            if let Some(text) = self.text() {
                self.update_text_value(Some(text), None);
            }
        }
    }

    fn text_box_selection_start(&self) -> i32 {
        match self.text_box() {
            Some(text_box) => text_box.selection_start().min(text_box.selection_end()),
            None => 0,
        }
    }

    fn text_box_selection_length(&self) -> i32 {
        match self.text_box() {
            Some(text_box) => (text_box.selection_end() - text_box.selection_start()).abs(),
            None => 0,
        }
    }

    /// Gets the selection adapter used to populate the drop-down with a
    /// list of selectable items.
    ///
    /// You can use this property when you create an automation peer to use
    /// with the control or deriving from [`AutoCompleteBox`] to create a
    /// custom control.
    pub fn selection_adapter(&self) -> Option<Rc<dyn ISelectionAdapter>> {
        self.adapter.borrow().clone()
    }

    /// Sets the selection adapter used to populate the drop-down with a
    /// list of selectable items.
    pub fn set_selection_adapter(&self, value: Option<Rc<dyn ISelectionAdapter>>) {
        let old = self.adapter.borrow().clone();
        if let Some(old) = old {
            for subscription in self.adapter_subscriptions.take() {
                subscription.dispose();
            }
            old.set_items_source(None);
        }

        *self.adapter.borrow_mut() = value.clone();

        if let Some(adapter) = value {
            let weak = self.to_ref().downgrade();
            let selection_changed = adapter.selection_changed(Rc::new({
                let weak = weak.clone();
                move |_| {
                    if let Some(this) = weak.upgrade() {
                        this.on_adapter_selection_changed();
                    }
                }
            }));
            let commit = adapter.commit(Rc::new({
                let weak = weak.clone();
                move |_| {
                    if let Some(this) = weak.upgrade() {
                        this.on_adapter_selection_complete();
                    }
                }
            }));
            let cancel = adapter.cancel(Rc::new({
                let weak = weak.clone();
                move |_| {
                    if let Some(this) = weak.upgrade() {
                        this.on_adapter_selection_canceled();
                    }
                }
            }));
            let cancel_complete = adapter.cancel(Rc::new(move |_| {
                if let Some(this) = weak.upgrade() {
                    this.on_adapter_selection_complete();
                }
            }));
            *self.adapter_subscriptions.borrow_mut() = vec![selection_changed, commit, cancel, cancel_complete];
            adapter.set_items_source(Some(self.view_source.clone()));
        }
    }

    /// Determines whether the text box or drop-down portion of the
    /// [`AutoCompleteBox`] control has focus.
    #[deprecated(note = "Use is_keyboard_focus_within instead.")]
    pub fn has_focus(&self) -> bool {
        self.is_keyboard_focus_within()
    }

    // --- events -------------------------------------------------------------

    /// The handle removing a handler from one of the plain events.
    fn subscription<F: ?Sized + 'static>(
        &self,
        list: fn(&Self) -> &HandlerList<F>,
        handler: Rc<F>,
    ) -> Rc<dyn IDisposable> {
        let token = list(self).add(handler);
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                list(&this).remove(token);
            }
        })
    }

    /// Occurs asynchronously when the text in the text box portion of the
    /// [`AutoCompleteBox`] changes.
    pub fn text_changed(
        &self,
        handler: impl Fn(&Interactive, &TextChangedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::text_changed_event(), handler)
    }

    /// Occurs when the [`AutoCompleteBox`] is populating the drop-down with
    /// possible matches based on the `Text` property.
    ///
    /// If the event is cancelled, by setting the cancel flag of the args,
    /// the control will not automatically populate the selection adapter
    /// contained in the drop-down. In this case, if you want possible
    /// matches to appear, you must provide the logic for populating the
    /// selection adapter.
    pub fn populating(&self, handler: impl Fn(&PopulatingEventArgs) + 'static) -> Rc<dyn IDisposable> {
        self.subscription::<dyn Fn(&PopulatingEventArgs)>(|this| &this.populating, Rc::new(handler))
    }

    /// Occurs when the [`AutoCompleteBox`] has populated the drop-down with
    /// possible matches based on the `Text` property.
    pub fn populated(&self, handler: impl Fn(&PopulatedEventArgs) + 'static) -> Rc<dyn IDisposable> {
        self.subscription::<dyn Fn(&PopulatedEventArgs)>(|this| &this.populated, Rc::new(handler))
    }

    /// Occurs when the value of the `IsDropDownOpen` property is changing
    /// from false to true.
    pub fn drop_down_opening(&self, handler: impl Fn(&CancelEventArgs) + 'static) -> Rc<dyn IDisposable> {
        self.subscription::<dyn Fn(&CancelEventArgs)>(|this| &this.drop_down_opening, Rc::new(handler))
    }

    /// Occurs when the value of the `IsDropDownOpen` property has changed
    /// from false to true and the drop-down is open.
    pub fn drop_down_opened(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        self.subscription::<dyn Fn()>(|this| &this.drop_down_opened, Rc::new(handler))
    }

    /// Occurs when the `IsDropDownOpen` property is changing from true to
    /// false.
    pub fn drop_down_closing(&self, handler: impl Fn(&CancelEventArgs) + 'static) -> Rc<dyn IDisposable> {
        self.subscription::<dyn Fn(&CancelEventArgs)>(|this| &this.drop_down_closing, Rc::new(handler))
    }

    /// Occurs when the `IsDropDownOpen` property was changed from true to
    /// false and the drop-down is open.
    pub fn drop_down_closed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        self.subscription::<dyn Fn()>(|this| &this.drop_down_closed, Rc::new(handler))
    }

    /// Occurs when the selected item in the drop-down portion of the
    /// [`AutoCompleteBox`] has changed.
    pub fn selection_changed(
        &self,
        handler: impl Fn(&Interactive, &SelectionChangedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::selection_changed_event(), handler)
    }

    // --- drop-down ----------------------------------------------------------

    /// Begin closing the drop-down.
    fn closing_drop_down(&self, old_value: bool) {
        let args = CancelEventArgs::new();
        self.on_drop_down_closing(&args);

        if args.cancel() {
            self.ignore_property_change.set(true);
            self.set_current_value(Self::is_drop_down_open_property(), old_value);
        } else {
            self.close_drop_down();
        }

        self.update_pseudo_classes();
    }

    /// Begin opening the drop down by firing cancelable events, opening the
    /// drop-down or reverting, depending on the event argument values.
    fn opening_drop_down(&self, old_value: bool) {
        let args = CancelEventArgs::new();

        // Opening.
        self.on_drop_down_opening(&args);

        if args.cancel() {
            self.ignore_property_change.set(true);
            self.set_current_value(Self::is_drop_down_open_property(), old_value);
        } else {
            self.open_drop_down();
        }

        self.update_pseudo_classes();
    }

    /// Connects to the closed event of the drop down popup.
    fn drop_down_popup_closed(&self) {
        // Force the drop down property to be false.
        if self.is_drop_down_open() {
            self.set_current_value(Self::is_drop_down_open_property(), false);
        }

        // Fire the DropDownClosed event.
        if self.popup_has_opened.get() {
            self.on_drop_down_closed();
        }
    }

    /// Handles the timer tick when using a populate delay.
    fn populate_drop_down(&self) {
        let timer = self.delay_timer.borrow().as_ref().map(|(timer, _)| timer.clone());
        if let Some(timer) = timer {
            timer.stop();
        }

        // Update the prefix/search text.
        self.set_search_text(self.text());

        if self.try_populate_async(self.search_text()) {
            return;
        }

        // The Populated event enables advanced, custom filtering. The
        // client needs to directly update the ItemsSource collection or
        // call the Populate method on the control to continue the display
        // process if Cancel is set to true.
        let populating = PopulatingEventArgs::new(self.search_text());
        self.on_populating(&populating);
        if !populating.cancel() {
            self.populate_complete();
        }
    }

    fn try_populate_async(&self, search_text: Option<String>) -> bool {
        if let Some(source) = self.population_cancellation_token_source.take() {
            source.cancel();
        }

        let Some(populator) = self.async_populator() else {
            return false;
        };

        let source = CancellationTokenSource::new();
        let token = source.token();
        *self.population_cancellation_token_source.borrow_mut() = Some(source);

        // Started the way an asynchronous method is called: it runs up to
        // its first pending await before this returns.
        let context = FerroSynchronizationContext::current().unwrap_or_else(|| {
            FerroSynchronizationContext::with_dispatcher(&Dispatcher::current_dispatcher(), DispatcherPriority::NORMAL)
        });
        drop(context.to_task_scheduler().start_local(Self::populate_async(self.to_ref(), populator, search_text, token)));

        true
    }

    async fn populate_async(
        this: Ref<Self>,
        populator: AutoCompleteAsyncPopulator,
        search_text: Option<String>,
        cancellation_token: CancellationToken,
    ) {
        // A cancelled population is dropped silently.
        if let Ok(result_list) = populator.invoke(search_text, cancellation_token.clone()).await {
            if !cancellation_token.is_cancellation_requested() {
                let target = this.clone();
                let token = cancellation_token.clone();
                let operation = Dispatcher::ui_thread().invoke_async_local(move || {
                    if !token.is_cancellation_requested() {
                        target.set_current_value(
                            Self::items_source_property(),
                            Some(ItemsSource::from_items(result_list)),
                        );
                        target.populate_complete();
                    }
                });
                // An aborted operation ends the population like a cancelled
                // one.
                let _ = operation.await;
            }
        }

        this.population_cancellation_token_source.take();
    }

    /// Private method that directly opens the popup, checks the expander
    /// button, and then fires the Opened event.
    fn open_drop_down(&self) {
        if let Some(popup) = self.drop_down_popup() {
            popup.set_is_open(true);
        }
        self.popup_has_opened.set(true);
        self.on_drop_down_opened();
    }

    /// Private method that directly closes the popup, flips the Checked
    /// value, and then fires the Closed event.
    fn close_drop_down(&self) {
        if self.popup_has_opened.get() {
            if let Some(adapter) = self.selection_adapter() {
                adapter.set_selected_item(None);
            }
            if let Some(popup) = self.drop_down_popup() {
                popup.set_is_open(false);
            }
            self.on_drop_down_closed();
        }
    }

    // --- text ---------------------------------------------------------------

    /// Handle the text changed notification that is directly attached to
    /// the text box part. This ensures that only user initiated actions
    /// will result in an [`AutoCompleteBox`] suggestion and operation.
    fn on_text_box_text_changed(&self) {
        if self.ignore_text_box_text_change.get() > 0 {
            self.ignore_text_box_text_change.set(self.ignore_text_box_text_change.get() - 1);
            return;
        }

        // Posted to allow the text box selection to update before
        // processing.
        let this = self.to_ref();
        Dispatcher::ui_thread().post_local(
            move || {
                // Call the central updated text method as a user-initiated
                // action.
                if let Some(text_box) = this.text_box() {
                    this.text_updated(text_box.text(), true);
                }
            },
            DispatcherPriority::DEFAULT,
        );
    }

    /// Updates both the text box value and underlying text property value
    /// if and when they change. Automatically fires the text changed events
    /// when there is a change.
    ///
    /// `user_initiated` indicates whether the action was user initiated. In
    /// a user initiated mode, the underlying text property is updated. In a
    /// non-user interaction, the text box value is updated. When it is
    /// `None`, all values are updated.
    fn update_text_value(&self, value: Option<String>, user_initiated: Option<bool>) {
        let mut call_text_changed = false;

        // Update the Text property.
        if user_initiated.unwrap_or(true) && self.text() != value {
            self.ignore_text_property_change.set(self.ignore_text_property_change.get() + 1);
            self.set_current_value(Self::text_property(), value.clone());
            call_text_changed = true;
        }

        // Update the Text property of the text box.
        if user_initiated != Some(true) {
            if let Some(text_box) = self.text_box() {
                if text_box.text() != value {
                    self.ignore_text_box_text_change.set(self.ignore_text_box_text_change.get() + 1);
                    text_box.set_text(Some(value.as_deref().unwrap_or("")));

                    // Text property value was set, fire event.
                    if !call_text_changed {
                        let text = self.text();
                        if text == value || text.is_none() {
                            call_text_changed = true;
                        }
                    }
                }
            }
        }

        if call_text_changed {
            self.on_text_changed(&TextChangedEventArgs::with_event(Self::text_changed_event()));
        }
    }

    /// Handle the update of the text for the control from any source,
    /// including the text box part and the `Text` property.
    ///
    /// `user_initiated` indicates whether the update is a user-initiated
    /// action. This should be `true` when the method is called from a text
    /// box event handler.
    fn text_updated(&self, new_text: Option<String>, user_initiated: bool) {
        let new_text = new_text.unwrap_or_default();

        // The text changed event of the text box was not firing immediately
        // and was causing an immediate update, even with wrapping. If there
        // is a selection currently, no update should happen.
        if self.is_text_completion_enabled() {
            if let Some(text_box) = self.text_box() {
                if self.text_box_selection_length() > 0
                    && self.text_box_selection_start() != utf16_length(text_box.text().as_deref())
                {
                    return;
                }
            }
        }

        // Evaluate the conditions needed for completion.
        // 1. Minimum prefix length
        // 2. If a delay timer is in use, use it
        let minimum_prefix_length = self.minimum_prefix_length();
        let minimum_length_reached =
            utf16_length(Some(&new_text)) >= minimum_prefix_length && minimum_prefix_length >= 0;

        self.user_called_populate.set(minimum_length_reached && user_initiated);

        // Update the interface and values only as necessary.
        self.update_text_value(Some(new_text), Some(user_initiated));

        if minimum_length_reached {
            self.ignore_text_selection_change.set(true);

            let timer = self.delay_timer.borrow().as_ref().map(|(timer, _)| timer.clone());
            match timer {
                Some(timer) => timer.start(),
                None => self.populate_drop_down(),
            }
        } else {
            self.set_search_text(Some(String::new()));
            if self.selected_item().is_some() {
                self.skip_selected_item_text_update.set(true);
            }

            self.set_current_value(Self::selected_item_property(), None);

            if self.is_drop_down_open() {
                self.set_current_value(Self::is_drop_down_open_property(), false);
            }
        }
    }

    // --- view ---------------------------------------------------------------

    /// A simple helper method to clear the view and ensure that a view
    /// object is always present and not null.
    fn clear_view(&self) {
        self.view.clear();
    }

    /// Walks through the items enumeration. Performance is not going to be
    /// perfect with the current implementation.
    fn refresh_view(&self) {
        // If we have a running filter, trigger a request first.
        if self.filter_in_action.get() {
            self.cancel_requested.set(true);
        }

        // Indicate that filtering is ongoing.
        self.filter_in_action.set(true);

        // Indicate that filtering is not ongoing anymore, however this
        // method is left.
        let _filter_in_action = Reset(&self.filter_in_action);
        let _cancel_requested = Reset(&self.cancel_requested);

        let items = self.items.borrow().clone();
        let Some(items) = items else {
            self.clear_view();
            return;
        };

        // Cache the current text value.
        let text = self.text().unwrap_or_default();

        // Cache the properties and determine if any filtering mode is on.
        let text_filter = self.text_filter();
        let item_filter = self.item_filter();
        let object_filtering = self.filter_mode() == AutoCompleteFilterMode::Custom && text_filter.is_none();
        let mut new_view_items = Vec::new();

        // If the mode is object filtering and the item filter is null, we
        // report an error.
        if object_filtering && item_filter.is_none() {
            panic!("ItemFilter property can not be null when FilterMode has value AutoCompleteFilterMode.Custom");
        }

        for item in items.iter() {
            // Exit the filter when requested if cancellation is requested.
            if self.cancel_requested.get() {
                return;
            }

            let in_results = if let Some(text_filter) = &text_filter {
                text_filter.invoke(Some(&text), &self.format_value(item))
            } else if let (true, Some(item_filter)) = (object_filtering, &item_filter) {
                item_filter.invoke(Some(&text), item)
            } else {
                true
            };

            if in_results {
                new_view_items.push(item.clone());
            }
        }

        self.view.clear();
        self.view.add_range(new_view_items);
    }

    /// Handle any change to the `ItemsSource` property, update the
    /// underlying observable collection view, and set the selection
    /// adapter's items source to the view if appropriate.
    fn on_items_source_changed(&self, new_value: Option<ItemsSource>) {
        // Remove handler for the collection changed event of the old value
        // (if present).
        if let Some((source, token)) = self.collection_change_subscription.take() {
            source.list().remove_collection_changed(token);
        }

        // Add handler for the collection changed event of the new value (if
        // possible). The collection refers to this control weakly.
        if let Some(source) = &new_value {
            let weak = self.to_ref().downgrade();
            let handler: Rc<ItemsChangedHandler> = Rc::new(move |e: &ItemsChangedEventArgs<'_>| {
                if let Some(this) = weak.upgrade() {
                    this.items_collection_changed(e);
                }
            });
            if let Some(token) = source.list().add_collection_changed(handler) {
                *self.collection_change_subscription.borrow_mut() = Some((source.clone(), token));
            }
        }

        // Store a local cached copy of the data.
        *self.items.borrow_mut() = new_value.map(|source| Rc::new(source.to_vec()));

        // Clear and set the view on the selection adapter.
        self.clear_view();
        self.set_view_on_adapter();
        if self.is_drop_down_open() {
            self.refresh_view();
        }
    }

    /// Hands the view to the selection adapter unless it has it.
    fn set_view_on_adapter(&self) {
        if let Some(adapter) = self.selection_adapter() {
            if adapter.items_source().as_ref() != Some(&self.view_source) {
                adapter.set_items_source(Some(self.view_source.clone()));
            }
        }
    }

    /// Method that handles the collection changed event for the
    /// `ItemsSource` property.
    fn items_collection_changed(&self, e: &ItemsChangedEventArgs<'_>) {
        // Update the cache.
        {
            let mut cache = self.items.borrow_mut();
            let items = cache.as_mut().map(Rc::make_mut);
            if let Some(items) = items {
                if e.action == NotifyCollectionChangedAction::Remove {
                    for _ in 0..e.old_items.len() {
                        items.remove(e.old_starting_index as usize);
                    }
                }
                if e.action == NotifyCollectionChangedAction::Add && items.len() as i32 >= e.new_starting_index {
                    for index in 0..e.new_items.len() {
                        items.insert(e.new_starting_index as usize + index, e.new_items.get(index));
                    }
                }
                if e.action == NotifyCollectionChangedAction::Replace {
                    for index in 0..e.new_items.len() {
                        items[e.new_starting_index as usize] = e.new_items.get(index);
                    }
                }
            }
        }

        // Update the view.
        if e.action == NotifyCollectionChangedAction::Remove || e.action == NotifyCollectionChangedAction::Replace {
            for index in 0..e.old_items.len() {
                self.view.remove(&e.old_items.get(index));
            }
        }

        if e.action == NotifyCollectionChangedAction::Reset {
            // Significant changes to the underlying data.
            self.clear_view();
            if let Some(source) = self.items_source() {
                *self.items.borrow_mut() = Some(Rc::new(source.to_vec()));
            }
        }

        // Refresh the observable collection used in the selection adapter.
        self.refresh_view();
    }

    /// Notifies the [`AutoCompleteBox`] that the `ItemsSource` property has
    /// been set and the data can be filtered to provide possible matches in
    /// the drop-down.
    ///
    /// Call this method when you are providing custom population of the
    /// drop-down portion of the control, to signal the control that you are
    /// done with the population process. Typically, you use it when the
    /// population process is a long-running process and you want to cancel
    /// built-in filtering of the `ItemsSource` items. In this case, you can
    /// handle the `Populating` event and set the cancel flag of its args.
    /// When the long-running process has completed you call
    /// `populate_complete` to indicate the drop-down is populated.
    pub fn populate_complete(&self) {
        // Apply the search filter.
        self.refresh_view();

        // Fire the Populated event containing the read-only view data.
        let populated = PopulatedEventArgs::new(self.view_source.clone());
        self.on_populated(&populated);

        self.set_view_on_adapter();

        let is_drop_down_open = self.user_called_populate.get() && self.view.count() > 0;
        if is_drop_down_open != self.is_drop_down_open() {
            self.ignore_property_change.set(true);
            self.set_current_value(Self::is_drop_down_open_property(), is_drop_down_open);
        }
        if self.is_drop_down_open() {
            self.opening_drop_down(false);
        } else {
            self.closing_drop_down(true);
        }

        self.update_text_completion(self.user_called_populate.get());
    }

    /// Performs text completion, if enabled, and a lookup on the underlying
    /// item values for an exact match. Will update the selected item value.
    ///
    /// `user_initiated` indicates whether the operation was user initiated.
    /// Text completion will not be performed when not directly initiated by
    /// the user.
    fn update_text_completion(&self, user_initiated: bool) {
        // By default this method will clear the selected value.
        let mut new_selected_item: Option<BoxedValue> = None;
        let text = self.text();

        // Text search is StartsWith explicit and only when enabled, in line
        // with WPF's ComboBox lookup. When in use it will associate a Value
        // with the Text if it is found in ItemsSource. This is only valid
        // when there is data and the user initiated the action.
        if self.view.count() > 0 {
            let text_box = if self.is_text_completion_enabled() && user_initiated { self.text_box() } else { None };
            if let Some(text_box) = text_box {
                let current_length = utf16_length(text_box.text().as_deref());
                let selection_start = self.text_box_selection_start();
                if text.as_deref().is_some_and(|text| selection_start == utf16_length(Some(text)))
                    && selection_start > self.text_selection_start.get()
                {
                    // When the FilterMode property is set to either
                    // StartsWith or StartsWithCaseSensitive, the first item
                    // in the view is used. This will improve performance on
                    // the lookup. It assumes that the FilterMode the user
                    // has selected is an acceptable case sensitive matching
                    // function for their scenario.
                    let filter_mode = self.filter_mode();
                    let top = if filter_mode == AutoCompleteFilterMode::StartsWith
                        || filter_mode == AutoCompleteFilterMode::StartsWithCaseSensitive
                    {
                        self.view.get(0)
                    } else {
                        self.try_get_match(
                            text.as_deref(),
                            AutoCompleteSearch::get_filter(AutoCompleteFilterMode::StartsWith),
                        )
                    };

                    // If the search was successful, update SelectedItem.
                    if top.is_some() {
                        let top_string = self.format_value(&top);
                        new_selected_item = top;

                        // Only replace partially when the two words being
                        // the same.
                        let text = self.text();
                        let top_length = utf16_length(top_string.as_deref());
                        let min_length = top_length.min(utf16_length(text.as_deref()));
                        let text_start = text.as_deref().map(|text| utf16_prefix(text, min_length));
                        let top_start = top_string.as_deref().map(|top| utf16_prefix(top, min_length));
                        if AutoCompleteSearch::equals(text_start.as_deref(), &top_start) {
                            // Update the text.
                            self.update_text_value(top_string, None);

                            // Select the text past the user's caret.
                            text_box.set_selection_start(current_length);
                            text_box.set_selection_end(top_length);
                        }
                    }
                }
            } else {
                // Perform an exact string lookup for the text. This is a
                // design change from the original Toolkit release when the
                // IsTextCompletionEnabled property behaved just like the
                // WPF ComboBox's IsTextSearchEnabled property.
                //
                // This change provides the behavior that most people expect
                // to find: a lookup for the value is always performed.
                new_selected_item = self.try_get_match(
                    text.as_deref(),
                    AutoCompleteSearch::get_filter(AutoCompleteFilterMode::EqualsCaseSensitive),
                );
            }
        }

        // Update the selected item property.
        if !crate::reference_equals(&self.selected_item(), &new_selected_item) {
            self.skip_selected_item_text_update.set(true);
        }
        self.set_current_value(Self::selected_item_property(), new_selected_item);

        // Restore updates for the text selection.
        if self.ignore_text_selection_change.get() {
            self.ignore_text_selection_change.set(false);
            if self.text_box().is_some() {
                self.text_selection_start.set(self.text_box_selection_start());
            }
        }
    }

    /// Attempts to look through the view and locate the specific exact text
    /// match.
    ///
    /// `predicate` is the predicate to use for the partial or exact match.
    /// Returns the object or `None`.
    fn try_get_match(
        &self,
        search_text: Option<&str>,
        predicate: Option<AutoCompleteFilterPredicate<Option<String>>>,
    ) -> Option<BoxedValue> {
        let predicate = predicate?;

        let view = self.view.snapshot();
        for item in view.iter() {
            if predicate.invoke(search_text, &self.format_value(item)) {
                return item.clone();
            }
        }

        None
    }

    fn update_pseudo_classes(&self) {
        self.pseudo_classes().set(PC_DROPDOWN_OPEN, self.is_drop_down_open());
    }

    fn clear_text_box_selection(&self) {
        if let Some(text_box) = self.text_box() {
            let length = utf16_length(text_box.text().as_deref());
            text_box.set_selection_start(length);
            text_box.set_selection_end(length);
        }
    }

    /// Called when the selected item is changed, updates the text value
    /// that is displayed in the text box part.
    fn on_selected_item_changed(&self, new_item: &Option<BoxedValue>) {
        let text = match new_item {
            None => self.search_text(),
            Some(item) => {
                if let Some(text_selector) = self.text_selector() {
                    Some(text_selector.invoke(self.search_text().as_deref(), &self.format_value(new_item)))
                } else if let Some(item_selector) = self.item_selector() {
                    Some(item_selector.invoke(self.search_text().as_deref(), item))
                } else {
                    self.format_value(new_item)
                }
            }
        };

        // Update the Text property and the text box values.
        self.update_text_value(text, None);

        // Move the caret to the end of the text box.
        self.clear_text_box_selection();
    }

    /// Handles the selection changed event of the selection adapter.
    fn on_adapter_selection_changed(&self) {
        if let Some(adapter) = self.selection_adapter() {
            self.set_current_value(Self::selected_item_property(), adapter.selected_item());
        }
    }

    /// Handles the commit event on the selection adapter.
    fn on_adapter_selection_complete(&self) {
        self.set_current_value(Self::is_drop_down_open_property(), false);

        // Text should not be selected.
        self.clear_text_box_selection();

        if let Some(text_box) = self.text_box() {
            text_box.focus();
        }
    }

    /// Handles the cancel event on the selection adapter.
    fn on_adapter_selection_canceled(&self) {
        self.update_text_value(self.search_text(), None);

        // Completion will update the selected value.
        self.update_text_completion(false);
    }
}

/// A predefined set of filter functions for the known, built-in
/// [`AutoCompleteFilterMode`] enumeration values.
struct AutoCompleteSearch;

impl AutoCompleteSearch {
    /// Index function that retrieves the filter for the provided
    /// [`AutoCompleteFilterMode`]. Returns the string-based comparison
    /// function.
    ///
    /// The filter of a mode is one object per thread, so that two requests
    /// for a mode give equal filters.
    fn get_filter(filter_mode: AutoCompleteFilterMode) -> Option<AutoCompleteFilterPredicate<Option<String>>> {
        type Filter = fn(Option<&str>, &Option<String>) -> bool;

        thread_local! {
            static FILTERS: [AutoCompleteFilterPredicate<Option<String>>; 12] = {
                let filters: [Filter; 12] = [
                    AutoCompleteSearch::starts_with,
                    AutoCompleteSearch::starts_with_case_sensitive,
                    AutoCompleteSearch::starts_with_ordinal,
                    AutoCompleteSearch::starts_with_ordinal_case_sensitive,
                    AutoCompleteSearch::contains,
                    AutoCompleteSearch::contains_case_sensitive,
                    AutoCompleteSearch::contains_ordinal,
                    AutoCompleteSearch::contains_ordinal_case_sensitive,
                    AutoCompleteSearch::equals,
                    AutoCompleteSearch::equals_case_sensitive,
                    AutoCompleteSearch::equals_ordinal,
                    AutoCompleteSearch::equals_ordinal_case_sensitive,
                ];
                filters.map(AutoCompleteFilterPredicate::new)
            };
        }

        let index = match filter_mode {
            AutoCompleteFilterMode::StartsWith => 0,
            AutoCompleteFilterMode::StartsWithCaseSensitive => 1,
            AutoCompleteFilterMode::StartsWithOrdinal => 2,
            AutoCompleteFilterMode::StartsWithOrdinalCaseSensitive => 3,
            AutoCompleteFilterMode::Contains => 4,
            AutoCompleteFilterMode::ContainsCaseSensitive => 5,
            AutoCompleteFilterMode::ContainsOrdinal => 6,
            AutoCompleteFilterMode::ContainsOrdinalCaseSensitive => 7,
            AutoCompleteFilterMode::Equals => 8,
            AutoCompleteFilterMode::EqualsCaseSensitive => 9,
            AutoCompleteFilterMode::EqualsOrdinal => 10,
            AutoCompleteFilterMode::EqualsOrdinalCaseSensitive => 11,
            AutoCompleteFilterMode::None | AutoCompleteFilterMode::Custom => return None,
        };

        Some(FILTERS.with(|filters| filters[index].clone()))
    }

    /// An implementation of the contains member of a string that takes in
    /// a string comparison. The comparison itself is the one of the shared
    /// string comparison of the base library, which asks the current
    /// culture for the culture-sensitive modes.
    fn contains_with(s: &Option<String>, value: Option<&str>, comparison: StringComparison) -> bool {
        match (s, value) {
            (Some(s), Some(value)) => comparison.contains(s, value),
            _ => false,
        }
    }

    fn starts_with_with(text: Option<&str>, value: &Option<String>, comparison: StringComparison) -> bool {
        match (value, text) {
            (Some(value), Some(text)) => comparison.starts_with(value, text),
            _ => false,
        }
    }

    /// Check if the string value begins with the text.
    fn starts_with(text: Option<&str>, value: &Option<String>) -> bool {
        Self::starts_with_with(text, value, StringComparison::CurrentCultureIgnoreCase)
    }

    /// Check if the string value begins with the text.
    fn starts_with_case_sensitive(text: Option<&str>, value: &Option<String>) -> bool {
        Self::starts_with_with(text, value, StringComparison::CurrentCulture)
    }

    /// Check if the string value begins with the text.
    fn starts_with_ordinal(text: Option<&str>, value: &Option<String>) -> bool {
        Self::starts_with_with(text, value, StringComparison::OrdinalIgnoreCase)
    }

    /// Check if the string value begins with the text.
    fn starts_with_ordinal_case_sensitive(text: Option<&str>, value: &Option<String>) -> bool {
        Self::starts_with_with(text, value, StringComparison::Ordinal)
    }

    /// Check if the prefix is contained in the string value. The current
    /// culture's case insensitive string comparison operator is used.
    fn contains(text: Option<&str>, value: &Option<String>) -> bool {
        Self::contains_with(value, text, StringComparison::CurrentCultureIgnoreCase)
    }

    /// Check if the prefix is contained in the string value.
    fn contains_case_sensitive(text: Option<&str>, value: &Option<String>) -> bool {
        Self::contains_with(value, text, StringComparison::CurrentCulture)
    }

    /// Check if the prefix is contained in the string value.
    fn contains_ordinal(text: Option<&str>, value: &Option<String>) -> bool {
        Self::contains_with(value, text, StringComparison::OrdinalIgnoreCase)
    }

    /// Check if the prefix is contained in the string value.
    fn contains_ordinal_case_sensitive(text: Option<&str>, value: &Option<String>) -> bool {
        Self::contains_with(value, text, StringComparison::Ordinal)
    }

    /// Check if the string values are equal.
    fn equals(text: Option<&str>, value: &Option<String>) -> bool {
        StringComparison::CurrentCultureIgnoreCase.equals(value.as_deref(), text)
    }

    /// Check if the string values are equal.
    fn equals_case_sensitive(text: Option<&str>, value: &Option<String>) -> bool {
        StringComparison::CurrentCulture.equals(value.as_deref(), text)
    }

    /// Check if the string values are equal.
    fn equals_ordinal(text: Option<&str>, value: &Option<String>) -> bool {
        StringComparison::OrdinalIgnoreCase.equals(value.as_deref(), text)
    }

    /// Check if the string values are equal.
    fn equals_ordinal_case_sensitive(text: Option<&str>, value: &Option<String>) -> bool {
        StringComparison::Ordinal.equals(value.as_deref(), text)
    }
}
