//! WinUI's `AutoSuggestBox` (`dxaml/xcp/dxaml/lib/AutoSuggestBox_Partial.cpp` of
//! microsoft-ui-xaml, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c)
//! Microsoft Corporation, see NOTICE.md).
//!
//! A separate control from the upstream `AutoCompleteBox` (docs/porting/winui-controls.md,
//! section 3): the application supplies the suggestions, filtered or not, when the text changes;
//! the box filters nothing.

use crate::automation::peers::{AutoSuggestBoxAutomationPeer, AutomationPeer};
use crate::primitives::{
    HeaderedItemsControl, HeaderedItemsControlImpl, Popup, SelectingItemsControl, TemplateAppliedEventArgs,
    TemplatedControlImpl, TemplatedControlImplExt,
};
use crate::utils::BindingEvaluator;
use crate::{
    AssignedBinding, Border, Button, Control, ControlImpl, IconElement, ItemsChangedEventArgs, ItemsControlImpl,
    ItemsControlImplExt, ListBox, ListBoxItem, TextBox,
};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::ReflectionBinding;
use ferroui_base::input::{FocusChangedEventArgs, InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, MouseButton};
use ferroui_base::interactivity::{InteractiveImpl, RoutingStrategies};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::metadata::{into_markup_value, MarkupDelegate};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_markup_enum, ferro_markup_type, ferro_property, instantiate, BoxedValue,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl,
    StyledProperty, Visual, VisualImpl, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// The text-changed events are raised this long after the last change of the text
/// (`s_textChangedEventTimerDuration`, 150 ms).
const TEXT_CHANGED_EVENT_TIMER_DURATION: Duration = Duration::from_millis(150);

/// Why the text of an [`AutoSuggestBox`] changed (WinUI's `AutoSuggestionBoxTextChangeReason`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum AutoSuggestionBoxTextChangeReason {
    /// The user typed.
    UserInput = 0,
    /// The application set `Text`.
    #[default]
    ProgrammaticChange = 1,
    /// The user chose a suggestion.
    SuggestionChosen = 2,
}

ferro_markup_enum!(AutoSuggestionBoxTextChangeReason { UserInput, ProgrammaticChange, SuggestionChosen }, { namespace: "FerroUI.Controls" });

/// The arguments of the `TextChanged` event of an [`AutoSuggestBox`] (WinUI's
/// `AutoSuggestBoxTextChangedEventArgs`): the reason, and whether the text is still the one of the
/// change. The arguments do not keep the box alive.
#[derive(Clone)]
pub struct AutoSuggestBoxTextChangedEventArgs {
    reason: Rc<Cell<AutoSuggestionBoxTextChangeReason>>,
    owner: WeakRef<AutoSuggestBox>,
    generation: u64,
}

impl PartialEq for AutoSuggestBoxTextChangedEventArgs {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.reason, &other.reason)
    }
}

impl AutoSuggestBoxTextChangedEventArgs {
    /// Why the text changed.
    pub fn reason(&self) -> AutoSuggestionBoxTextChangeReason {
        self.reason.get()
    }

    /// Sets why the text changed.
    pub fn set_reason(&self, value: AutoSuggestionBoxTextChangeReason) {
        self.reason.set(value);
    }

    /// Whether the text of the box has not changed since the change of these arguments (WinUI's
    /// `CheckCurrent`): `false` as soon as a newer change happens, before its event is raised.
    pub fn check_current(&self) -> bool {
        self.owner.upgrade().is_some_and(|owner| owner.generation.get() == self.generation)
    }
}

/// The arguments of the `SuggestionChosen` event of an [`AutoSuggestBox`] (WinUI's
/// `AutoSuggestBoxSuggestionChosenEventArgs`).
#[derive(Clone)]
pub struct AutoSuggestBoxSuggestionChosenEventArgs {
    /// The suggestion chosen, an item of the box.
    pub selected_item: BoxedValue,
}

impl PartialEq for AutoSuggestBoxSuggestionChosenEventArgs {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.selected_item, &other.selected_item)
    }
}

impl AutoSuggestBoxSuggestionChosenEventArgs {
    /// The suggestion chosen.
    pub fn selected_item(&self) -> BoxedValue {
        self.selected_item.clone()
    }
}

/// The arguments of the `QuerySubmitted` event of an [`AutoSuggestBox`] (WinUI's
/// `AutoSuggestBoxQuerySubmittedEventArgs`).
#[derive(Clone, PartialEq)]
pub struct AutoSuggestBoxQuerySubmittedEventArgs {
    /// The text of the box.
    pub query_text: String,
    /// The suggestion chosen with the keyboard or the pointer, if any.
    pub chosen_suggestion: Option<BoxedValue>,
}

impl AutoSuggestBoxQuerySubmittedEventArgs {
    /// The text of the box.
    pub fn query_text(&self) -> String {
        self.query_text.clone()
    }

    /// The suggestion chosen, if any.
    pub fn chosen_suggestion(&self) -> Option<BoxedValue> {
        self.chosen_suggestion.clone()
    }
}

ferro_markup_type!(class AutoSuggestBoxTextChangedEventArgs {
    namespace: "FerroUI.Controls",
    handles: [AutoSuggestBoxTextChangedEventArgs],
    properties: [
        Reason: AutoSuggestionBoxTextChangeReason {
            get: AutoSuggestBoxTextChangedEventArgs::reason,
            set: AutoSuggestBoxTextChangedEventArgs::set_reason,
        },
    ],
    methods: [fn CheckCurrent() -> bool => AutoSuggestBoxTextChangedEventArgs::check_current],
});

ferro_markup_type!(class AutoSuggestBoxSuggestionChosenEventArgs {
    namespace: "FerroUI.Controls",
    handles: [AutoSuggestBoxSuggestionChosenEventArgs],
    properties: [SelectedItem: BoxedValue { get: AutoSuggestBoxSuggestionChosenEventArgs::selected_item }],
});

ferro_markup_type!(class AutoSuggestBoxQuerySubmittedEventArgs {
    namespace: "FerroUI.Controls",
    handles: [AutoSuggestBoxQuerySubmittedEventArgs],
    properties: [
        QueryText: String { get: AutoSuggestBoxQuerySubmittedEventArgs::query_text },
        ChosenSuggestion: Option<BoxedValue> { get: AutoSuggestBoxQuerySubmittedEventArgs::chosen_suggestion },
    ],
});

#[derive(Clone, Default)]
struct Parts {
    text_box: Option<Ref<TextBox>>,
    suggestions_list: Option<Ref<ListBox>>,
    suggestions_popup: Option<Ref<Popup>>,
    suggestions_container: Option<Ref<Border>>,
    query_button: Option<Ref<Button>>,
}

/// Sets a flag back to its value before a scope.
struct Restore<'a>(&'a Cell<bool>, bool);

impl Drop for Restore<'_> {
    fn drop(&mut self) {
        self.0.set(self.1);
    }
}

fn set_for_scope(flag: &Cell<bool>) -> Restore<'_> {
    Restore(flag, flag.replace(true))
}

/// A text box with a list of suggestions the application supplies (WinUI's `AutoSuggestBox`).
///
/// The application listens to `TextChanged`, raised 150 ms after the last change of the text with
/// its reason, and sets the items (or `ItemsSource`) to the suggestions; the box shows them in a
/// popup while it has the focus. Choosing a suggestion with the arrow keys or the pointer raises
/// `SuggestionChosen` (and puts its text, by `TextMemberPath`, in the box when `UpdateTextOnSelect`
/// is set); Enter, a click of a suggestion and the query button raise `QuerySubmitted`.
///
/// The template is the control theme of the WinUI theme (`Controls/AutoSuggestBox.xaml`, from
/// WinUI's `DefaultAutoSuggestBoxStyle`), with WinUI's parts: the text box `TextBox`, the popup
/// `SuggestionsPopup` with `SuggestionsContainer` and the list `SuggestionsList`, and the button
/// `QueryButton`. The Fluent and Simple themes have no template for it
/// (docs/porting/winui-controls.md, section 5.1).
///
/// Missing (docs/porting/winui-controls.md, Input): `AutoMaximizeSuggestionArea`,
/// `LightDismissOverlayMode`, `TextBoxStyle`, `HeaderPlacement`, the input validation of WinUI and
/// the placement of the list above the box when there is no room below.
#[repr(C)]
pub struct AutoSuggestBox {
    base: HeaderedItemsControl,
    parts: RefCell<Parts>,
    template_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    text_changed_event_timer: RefCell<Option<Rc<DispatcherTimer>>>,
    text_changed_event_args: RefCell<Option<AutoSuggestBoxTextChangedEventArgs>>,
    generation: Cell<u64>,
    updating_text: Cell<bool>,
    ignore_selection_changes: Cell<bool>,
    has_focus: Cell<bool>,
    user_typed_text: RefCell<String>,
    text_changed: HandlerList<dyn Fn(&AutoSuggestBoxTextChangedEventArgs)>,
    suggestion_chosen: HandlerList<dyn Fn(&AutoSuggestBoxSuggestionChosenEventArgs)>,
    query_submitted: HandlerList<dyn Fn(&AutoSuggestBoxQuerySubmittedEventArgs)>,
}

ferro_class!(AutoSuggestBox: HeaderedItemsControl);
ferroui_base::ferro_class_info!(AutoSuggestBox {
    new: AutoSuggestBox::new,
    markup: {
        namespace: "FerroUI.Controls",
        content: Items,
        events: [
            TextChanged(Option<BoxedValue>, AutoSuggestBoxTextChangedEventArgs) => |this: &Ref<AutoSuggestBox>, handler: MarkupDelegate| {
                let sender = this.downgrade();
                this.text_changed(move |args: &AutoSuggestBoxTextChangedEventArgs| {
                    handler.invoke(&[into_markup_value(sender.upgrade()), into_markup_value(args.clone())]);
                })
            },
            SuggestionChosen(Option<BoxedValue>, AutoSuggestBoxSuggestionChosenEventArgs) => |this: &Ref<AutoSuggestBox>, handler: MarkupDelegate| {
                let sender = this.downgrade();
                this.suggestion_chosen(move |args: &AutoSuggestBoxSuggestionChosenEventArgs| {
                    handler.invoke(&[into_markup_value(sender.upgrade()), into_markup_value(args.clone())]);
                })
            },
            QuerySubmitted(Option<BoxedValue>, AutoSuggestBoxQuerySubmittedEventArgs) => |this: &Ref<AutoSuggestBox>, handler: MarkupDelegate| {
                let sender = this.downgrade();
                this.query_submitted(move |args: &AutoSuggestBoxQuerySubmittedEventArgs| {
                    handler.invoke(&[into_markup_value(sender.upgrade()), into_markup_value(args.clone())]);
                })
            },
        ],
        attributes: [
            TemplatePart("TextBox", type(Ref<TextBox>)),
            TemplatePart("SuggestionsPopup", type(Ref<Popup>)),
            TemplatePart("SuggestionsContainer", type(Ref<Border>)),
            TemplatePart("SuggestionsList", type(Ref<ListBox>)),
            TemplatePart("QueryButton", type(Ref<Button>)),
        ],
    },
});
ferro_impl_classes!(AutoSuggestBox: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl, HeaderedItemsControlImpl);

impl FerroObjectImpl for AutoSuggestBox {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        // The keys of the text box, before the text box handles them.
        let weak = this.to_ref().downgrade();
        this.add_handler_with(
            InputElement::key_down_event(),
            move |_, args| {
                if let Some(this) = weak.upgrade() {
                    this.on_text_box_key_down(args);
                }
            },
            RoutingStrategies::TUNNEL,
            false,
        );
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let property = change.property();
        if property == Self::text_property().as_property() {
            if !this.updating_text.get() {
                this.update_text_box_text(AutoSuggestionBoxTextChangeReason::ProgrammaticChange);
            }
        } else if property == Self::is_suggestion_list_open_property().as_property() {
            if !this.is_suggestion_list_open() {
                this.clear_selection();
            }
            this.update_suggestions_popup();
        } else if property == Self::query_icon_property().as_property() {
            this.set_text_box_query_button_icon();
        } else if property == Self::max_suggestion_list_height_property().as_property()
            || property == Visual::bounds_property().as_property()
        {
            this.update_suggestion_list_size();
        }
    }
}

impl ControlImpl for AutoSuggestBox {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        AutoSuggestBoxAutomationPeer::new(this).upcast()
    }
}

impl ItemsControlImpl for AutoSuggestBox {
    fn on_items_view_collection_changed(this: &Self, args: &ItemsChangedEventArgs<'_>) {
        Self::parent_on_items_view_collection_changed(this, args);
        this.update_suggestions_source();
        // The list processes its own notification of the change first.
        let weak = this.to_ref().downgrade();
        Dispatcher::ui_thread().post_local(
            move || {
                if let Some(this) = weak.upgrade() {
                    this.update_suggestion_list_visibility();
                }
            },
            DispatcherPriority::NORMAL,
        );
    }
}

impl InputElementImpl for AutoSuggestBox {
    fn on_got_focus(this: &Self, args: &FocusChangedEventArgs) {
        Self::parent_on_got_focus(this, args);
        this.has_focus.set(true);
        this.update_suggestion_list_visibility();
    }

    fn on_lost_focus(this: &Self, args: &FocusChangedEventArgs) {
        Self::parent_on_lost_focus(this, args);
        // The focus stays within the box when it moves to the list of the popup.
        let list = this.parts.borrow().suggestions_list.clone();
        let within = args.new_focused_element.as_ref().is_some_and(|next| {
            this.is_visual_ancestor_of(next)
                || list.as_ref().is_some_and(|list| list.is_visual_ancestor_of(next) || list.to_ref().upcast::<InputElement>() == *next)
        });
        if !within {
            this.has_focus.set(false);
            this.set_current_value(Self::is_suggestion_list_open_property(), false);
        }
    }
}

impl TemplatedControlImpl for AutoSuggestBox {
    fn on_apply_template(this: &Self, args: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, args);
        for subscription in this.template_subscriptions.take() {
            subscription.dispose();
        }
        if let Some(popup) = this.parts.take().suggestions_popup {
            popup.set_is_open(false);
        }
        let scope = args.name_scope();
        let parts = Parts {
            text_box: scope.find_as("TextBox"),
            suggestions_list: scope.find_as("SuggestionsList"),
            suggestions_popup: scope.find_as("SuggestionsPopup"),
            suggestions_container: scope.find_as("SuggestionsContainer"),
            query_button: scope.find_as("QueryButton"),
        };
        *this.parts.borrow_mut() = parts.clone();
        let mut subscriptions = Vec::new();
        if let Some(text_box) = &parts.text_box {
            let weak = this.to_ref().downgrade();
            subscriptions.push(text_box.property_changed(move |change| {
                if change.property() == TextBox::text_property().as_property() {
                    if let Some(this) = weak.upgrade() {
                        this.on_text_box_text_changed();
                    }
                }
            }));
        }
        if let Some(list) = &parts.suggestions_list {
            let weak = this.to_ref().downgrade();
            let token = list.selection_changed(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.on_suggestion_selection_changed();
                }
            });
            let weak_list = list.downgrade();
            subscriptions.push(Disposable::create(move || {
                if let Some(list) = weak_list.upgrade() {
                    list.remove_handler(SelectingItemsControl::selection_changed_event(), token);
                }
            }));
            // OnListViewItemClick: a release of the left button over a suggestion submits it.
            let weak = this.to_ref().downgrade();
            let weak_list = list.downgrade();
            let token = list.add_handler_with(
                InputElement::pointer_released_event(),
                move |_, args| {
                    if args.initial_press_mouse_button() != MouseButton::Left {
                        return;
                    }
                    let (Some(this), Some(list), Some(source)) =
                        (weak.upgrade(), weak_list.upgrade(), args.source().and_then(|source| source.cast::<Visual>()))
                    else {
                        return;
                    };
                    // Not a release over the scroll bar or the empty area of the popup.
                    if !std::iter::once(source.clone()).chain(source.get_visual_ancestors()).any(|v| v.cast::<ListBoxItem>().is_some()) {
                        return;
                    }
                    let selected = list.selected_item();
                    if selected.is_some() {
                        let weak = this.downgrade();
                        Dispatcher::ui_thread().post_local(
                            move || {
                                if let Some(this) = weak.upgrade() {
                                    this.submit_query(selected);
                                }
                            },
                            DispatcherPriority::NORMAL,
                        );
                    }
                },
                RoutingStrategies::BUBBLE,
                true,
            );
            let weak_list = list.downgrade();
            subscriptions.push(Disposable::create(move || {
                if let Some(list) = weak_list.upgrade() {
                    list.remove_handler(InputElement::pointer_released_event(), token);
                }
            }));
        }
        if let Some(query_button) = &parts.query_button {
            // OnTextBoxQueryButtonClick
            let weak = this.to_ref().downgrade();
            let token = query_button.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.programmatic_submit_query();
                }
            });
            let weak_button = query_button.downgrade();
            subscriptions.push(Disposable::create(move || {
                if let Some(button) = weak_button.upgrade() {
                    button.remove_handler(Button::click_event(), token);
                }
            }));
        }
        if let Some(popup) = &parts.suggestions_popup {
            let weak = this.to_ref().downgrade();
            subscriptions.push(popup.closed(move || {
                if let Some(this) = weak.upgrade() {
                    this.set_current_value(Self::is_suggestion_list_open_property(), false);
                }
            }));
        }
        *this.template_subscriptions.borrow_mut() = subscriptions;
        this.set_text_box_query_button_icon();
        this.update_suggestions_source();
        this.update_suggestion_list_size();
        this.update_text_box_text(AutoSuggestionBoxTextChangeReason::ProgrammaticChange);
        this.update_suggestions_popup();
    }
}

ferroui_base::ferro_properties! { impl AutoSuggestBox {
    ferro_property!(
        /// Defines the `Text` property: the text of the box.
        pub fn text_property() -> StyledProperty<String> {
            FerroProperty::register::<AutoSuggestBox, _>("Text", String::new())
        }
    );

    ferro_property!(
        /// Defines the `TextMemberPath` property: the path of the text of a chosen suggestion; the
        /// text of the item itself when it is empty (the default).
        pub fn text_member_path_property() -> StyledProperty<String> {
            FerroProperty::register::<AutoSuggestBox, _>("TextMemberPath", String::new())
        }
    );

    ferro_property!(
        /// Defines the `PlaceholderText` property: the text shown while the box is empty.
        pub fn placeholder_text_property() -> StyledProperty<String> {
            FerroProperty::register::<AutoSuggestBox, _>("PlaceholderText", String::new())
        }
    );

    ferro_property!(
        /// Defines the `UpdateTextOnSelect` property: whether choosing a suggestion puts its text
        /// in the box. `true` by default.
        pub fn update_text_on_select_property() -> StyledProperty<bool> {
            FerroProperty::register::<AutoSuggestBox, _>("UpdateTextOnSelect", true)
        }
    );

    ferro_property!(
        /// Defines the `IsSuggestionListOpen` property: whether the popup of the suggestions is
        /// open.
        pub fn is_suggestion_list_open_property() -> StyledProperty<bool> {
            FerroProperty::register::<AutoSuggestBox, _>("IsSuggestionListOpen", false)
        }
    );

    ferro_property!(
        /// Defines the `MaxSuggestionListHeight` property: the largest height of the list of the
        /// suggestions; infinite by default, and a negative value is infinite too.
        pub fn max_suggestion_list_height_property() -> StyledProperty<f64> {
            FerroProperty::register::<AutoSuggestBox, _>("MaxSuggestionListHeight", f64::INFINITY)
        }
    );

    ferro_property!(
        /// Defines the `QueryIcon` property: the icon of the query button; no button without one
        /// (the default).
        pub fn query_icon_property() -> StyledProperty<Option<Ref<IconElement>>> {
            FerroProperty::register::<AutoSuggestBox, _>("QueryIcon", None)
        }
    );

    ferro_property!(
        /// Defines the `Description` property: the content below the box.
        pub fn description_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<AutoSuggestBox, _>("Description", None)
        }
    );
} }

impl AutoSuggestBox {
    fn static_constructor() {
        // The box is not a tab stop of its own (IsTabStop False of its style): the text box is.
        InputElement::focusable_property().override_default_value::<AutoSuggestBox>(false);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: HeaderedItemsControl::construct(),
            parts: RefCell::new(Parts::default()),
            template_subscriptions: RefCell::new(Vec::new()),
            text_changed_event_timer: RefCell::new(None),
            text_changed_event_args: RefCell::new(None),
            generation: Cell::new(0),
            updating_text: Cell::new(false),
            ignore_selection_changes: Cell::new(false),
            has_focus: Cell::new(false),
            user_typed_text: RefCell::new(String::new()),
            text_changed: HandlerList::new(),
            suggestion_chosen: HandlerList::new(),
            query_submitted: HandlerList::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the text of the box.
    pub fn text(&self) -> String {
        self.get_value(Self::text_property())
    }

    /// Sets the text of the box.
    pub fn set_text(&self, value: impl Into<String>) {
        self.set_value(Self::text_property(), value.into());
    }

    /// Gets the path of the text of a chosen suggestion.
    pub fn text_member_path(&self) -> String {
        self.get_value(Self::text_member_path_property())
    }

    /// Sets the path of the text of a chosen suggestion.
    pub fn set_text_member_path(&self, value: impl Into<String>) {
        self.set_value(Self::text_member_path_property(), value.into());
    }

    /// Gets the text shown while the box is empty.
    pub fn placeholder_text(&self) -> String {
        self.get_value(Self::placeholder_text_property())
    }

    /// Sets the text shown while the box is empty.
    pub fn set_placeholder_text(&self, value: impl Into<String>) {
        self.set_value(Self::placeholder_text_property(), value.into());
    }

    /// Gets whether choosing a suggestion puts its text in the box.
    pub fn update_text_on_select(&self) -> bool {
        self.get_value(Self::update_text_on_select_property())
    }

    /// Sets whether choosing a suggestion puts its text in the box.
    pub fn set_update_text_on_select(&self, value: bool) {
        self.set_value(Self::update_text_on_select_property(), value);
    }

    /// Gets whether the popup of the suggestions is open.
    pub fn is_suggestion_list_open(&self) -> bool {
        self.get_value(Self::is_suggestion_list_open_property())
    }

    /// Opens or closes the popup of the suggestions; opening it focuses the text box.
    pub fn set_is_suggestion_list_open(&self, value: bool) {
        self.set_value(Self::is_suggestion_list_open_property(), value);
        if value {
            let text_box = self.parts.borrow().text_box.clone();
            if let Some(text_box) = text_box {
                text_box.focus();
            }
        }
    }

    /// Gets the largest height of the list of the suggestions.
    pub fn max_suggestion_list_height(&self) -> f64 {
        self.get_value(Self::max_suggestion_list_height_property())
    }

    /// Sets the largest height of the list of the suggestions.
    pub fn set_max_suggestion_list_height(&self, value: f64) {
        self.set_value(Self::max_suggestion_list_height_property(), value);
    }

    /// Gets the icon of the query button.
    pub fn query_icon(&self) -> Option<Ref<IconElement>> {
        self.get_value(Self::query_icon_property())
    }

    /// Sets the icon of the query button.
    pub fn set_query_icon(&self, value: Option<Ref<IconElement>>) {
        self.set_value(Self::query_icon_property(), value);
    }

    /// Gets the content below the box.
    pub fn description(&self) -> Option<BoxedValue> {
        self.get_value(Self::description_property())
    }

    /// Sets the content below the box.
    pub fn set_description(&self, value: Option<BoxedValue>) {
        self.set_value(Self::description_property(), value);
    }

    /// Adds a handler of the `TextChanged` event, raised 150 ms after the last change of the text.
    /// Disposing the result removes the handler.
    pub fn text_changed(&self, handler: impl Fn(&AutoSuggestBoxTextChangedEventArgs) + 'static) -> Rc<dyn IDisposable> {
        let token = self.text_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.text_changed.remove(token);
            }
        })
    }

    /// Adds a handler of the `SuggestionChosen` event. Disposing the result removes the handler.
    pub fn suggestion_chosen(
        &self,
        handler: impl Fn(&AutoSuggestBoxSuggestionChosenEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.suggestion_chosen.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.suggestion_chosen.remove(token);
            }
        })
    }

    /// Adds a handler of the `QuerySubmitted` event. Disposing the result removes the handler.
    pub fn query_submitted(
        &self,
        handler: impl Fn(&AutoSuggestBoxQuerySubmittedEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.query_submitted.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.query_submitted.remove(token);
            }
        })
    }

    /// Submits the text alone, without the suggestion that is selected (WinUI's
    /// `ProgrammaticSubmitQuery`, of the query button and of the automation peer).
    pub(crate) fn programmatic_submit_query(&self) {
        self.clear_selection();
        self.submit_query(None);
    }

    /// Raises `QuerySubmitted` with the text and `chosen_suggestion`, and closes the list (WinUI's
    /// `SubmitQuery`).
    fn submit_query(&self, chosen_suggestion: Option<BoxedValue>) {
        let args = AutoSuggestBoxQuerySubmittedEventArgs { query_text: self.text(), chosen_suggestion };
        for (_, handler) in self.query_submitted.snapshot().iter() {
            handler(&args);
        }
        self.set_current_value(Self::is_suggestion_list_open_property(), false);
    }

    /// Puts `Text` in the text box, and raises `TextChanged` with `reason` after the delay
    /// (WinUI's `UpdateText`).
    fn update_text_box_text(&self, reason: AutoSuggestionBoxTextChangeReason) {
        let Some(text_box) = self.parts.borrow().text_box.clone() else { return };
        let text = self.text();
        if text_box.text().as_deref().unwrap_or("") == text {
            return;
        }
        let _updating = set_for_scope(&self.updating_text);
        text_box.set_current_value(TextBox::text_property(), Some(text));
        self.queue_text_changed_event(reason);
    }

    /// The text box changed: the user typed (WinUI's `OnTextBoxTextChanged`).
    fn on_text_box_text_changed(&self) {
        if self.updating_text.get() {
            return;
        }
        let Some(text_box) = self.parts.borrow().text_box.clone() else { return };
        let text = text_box.text().unwrap_or_default();
        let _updating = set_for_scope(&self.updating_text);
        self.set_current_value(Self::text_property(), text.clone());
        *self.user_typed_text.borrow_mut() = text;
        self.clear_selection();
        self.queue_text_changed_event(AutoSuggestionBoxTextChangeReason::UserInput);
        self.update_suggestion_list_visibility();
    }

    /// Replaces the pending arguments of `TextChanged` and restarts the timer: the event is raised
    /// once for the changes within 150 ms, with the reason of the last one.
    fn queue_text_changed_event(&self, reason: AutoSuggestionBoxTextChangeReason) {
        let generation = self.generation.get().wrapping_add(1);
        self.generation.set(generation);
        *self.text_changed_event_args.borrow_mut() = Some(AutoSuggestBoxTextChangedEventArgs {
            reason: Rc::new(Cell::new(reason)),
            owner: self.to_ref().downgrade(),
            generation,
        });
        let existing = self.text_changed_event_timer.borrow().clone();
        let timer = existing.unwrap_or_else(|| {
            let timer = DispatcherTimer::new();
            timer.set_interval(TEXT_CHANGED_EVENT_TIMER_DURATION);
            let weak = self.to_ref().downgrade();
            // The subscription lives as long as the timer, which the box owns.
            let _ = timer.tick(move |timer| {
                timer.stop();
                if let Some(this) = weak.upgrade() {
                    this.on_text_changed_event_timer_tick();
                }
            });
            *self.text_changed_event_timer.borrow_mut() = Some(timer.clone());
            timer
        });
        timer.stop();
        timer.start();
    }

    fn on_text_changed_event_timer_tick(&self) {
        let args = self.text_changed_event_args.take();
        if let Some(args) = args {
            for (_, handler) in self.text_changed.snapshot().iter() {
                handler(&args);
            }
        }
        self.update_suggestion_list_visibility();
    }

    fn clear_selection(&self) {
        let _ignore = set_for_scope(&self.ignore_selection_changes);
        let list = self.parts.borrow().suggestions_list.clone();
        if let Some(list) = list {
            list.set_selected_index(-1);
        }
    }

    /// A suggestion is selected (WinUI's `OnSuggestionSelectionChanged`): its text goes to the box
    /// when `UpdateTextOnSelect` is set, then `SuggestionChosen` is raised.
    fn on_suggestion_selection_changed(&self) {
        if self.ignore_selection_changes.get() {
            return;
        }
        let selected = self.parts.borrow().suggestions_list.as_ref().and_then(|list| list.selected_item());
        let Some(item) = selected else { return };
        if self.update_text_on_select() {
            let path = self.text_member_path();
            let text = if path.is_empty() {
                ValueTypes::try_to_string(&*item)
            } else {
                let evaluator = BindingEvaluator::new();
                evaluator.update_binding(&AssignedBinding::new(ReflectionBinding::new(&path)));
                let value = evaluator.evaluate_string(&Some(item.clone()));
                evaluator.dispose();
                value
            }
            .unwrap_or_default();
            {
                let _updating = set_for_scope(&self.updating_text);
                self.set_current_value(Self::text_property(), text);
            }
            self.update_text_box_text(AutoSuggestionBoxTextChangeReason::SuggestionChosen);
            let text_box = self.parts.borrow().text_box.clone();
            if let Some(text_box) = text_box {
                text_box.set_caret_index(self.text().encode_utf16().count() as i32);
            }
        }
        let args = AutoSuggestBoxSuggestionChosenEventArgs { selected_item: item };
        for (_, handler) in self.suggestion_chosen.snapshot().iter() {
            handler(&args);
        }
    }

    /// Opens the list while the box has the focus and suggestions.
    fn update_suggestion_list_visibility(&self) {
        self.set_current_value(Self::is_suggestion_list_open_property(), self.has_focus.get() && self.items().count() > 0);
    }

    fn update_suggestions_popup(&self) {
        let popup = self.parts.borrow().suggestions_popup.clone();
        if let Some(popup) = popup {
            popup.set_is_open(self.is_suggestion_list_open());
        }
    }

    /// The list shows the items of the box.
    fn update_suggestions_source(&self) {
        let list = self.parts.borrow().suggestions_list.clone();
        if let Some(list) = list {
            list.set_items_source(Some(self.items().source()));
        }
    }

    /// The suggestions are as wide as the box and at most `MaxSuggestionListHeight` high (WinUI's
    /// `UpdateSuggestionListSize`).
    fn update_suggestion_list_size(&self) {
        let container = self.parts.borrow().suggestions_container.clone();
        if let Some(container) = container {
            let max_height = self.max_suggestion_list_height();
            // A negative height is the available height in WinUI; the port does not measure it.
            container.set_max_height(if max_height < 0.0 { f64::INFINITY } else { max_height });
            container.set_width(self.bounds().width);
        }
    }

    /// Puts the query icon in the query button, and hides the button without one (WinUI's
    /// `SetTextBoxQueryButtonIcon`).
    fn set_text_box_query_button_icon(&self) {
        let Some(button) = self.parts.borrow().query_button.clone() else { return };
        match self.query_icon() {
            Some(icon) => {
                // WinUI sets the FontSize of a SymbolIcon to zero here, so that its glyph takes
                // AutoSuggestBoxIconFontSize from the button; the control theme sets that size on
                // the text part of the icon instead.
                button.set_is_visible(true);
                button.set_content(Some(Control::boxed(icon)));
            }
            None => {
                button.set_is_visible(false);
                button.set_content(None);
            }
        }
    }

    /// The keys of the list while it is open, and Enter (WinUI's `OnKeyDown` of the text box).
    fn on_text_box_key_down(&self, args: &KeyEventArgs) {
        if args.handled() || !self.is_effectively_enabled() {
            return;
        }
        let parts = self.parts.borrow().clone();
        // Keys of the query button are the button's.
        if parts.query_button.as_ref().is_some_and(|query| args.source().as_ref() == Some(&query.to_ref().upcast())) {
            return;
        }
        let Some(list) = parts.suggestions_list else { return };
        match args.key {
            Key::Up | Key::Down if self.is_suggestion_list_open() => {
                // The selection moves through the suggestions and back to the text the user typed.
                let count = self.items().count() as i32;
                let index = list.selected_index();
                if count > 0 && !(count == 1 && index == 0) {
                    let next = if args.key == Key::Down {
                        if index == count - 1 { -1 } else { index + 1 }
                    } else if index < 0 {
                        count - 1
                    } else {
                        index - 1
                    };
                    if next < 0 {
                        let typed = self.user_typed_text.borrow().clone();
                        self.set_text(typed);
                    }
                    list.set_selected_index(next);
                }
                args.set_handled(true);
            }
            Key::Escape | Key::Tab if args.key == Key::Escape || self.is_suggestion_list_open() => {
                let typed = self.user_typed_text.borrow().clone();
                self.set_text(typed);
                self.set_current_value(Self::is_suggestion_list_open_property(), false);
                if args.key == Key::Escape {
                    args.set_handled(true);
                }
            }
            Key::Enter => {
                self.submit_query(list.selected_item());
                args.set_handled(true);
            }
            _ => {}
        }
    }
}

impl Drop for AutoSuggestBox {
    fn drop(&mut self) {
        if let Some(timer) = self.text_changed_event_timer.get_mut().take() {
            timer.stop();
        }
        for subscription in self.template_subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
    }
}
