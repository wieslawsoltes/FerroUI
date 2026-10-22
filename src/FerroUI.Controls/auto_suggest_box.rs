//! Native suggestion editor with the WinUI event and selection contract.
//!
//! Derived from the pinned AutoSuggestBox_Partial.cpp. The application supplies
//! suggestions; this control deliberately performs no implicit text filtering.

use crate::automation::peers::{
    AutoSuggestBoxAutomationPeer, AutomationPeer,
};
use crate::automation::AutomationProperties;
use crate::presenters::ContentPresenter;
use crate::primitives::{
    HeaderedItemsControl, HeaderedItemsControlImpl, Popup, TemplateAppliedEventArgs,
    TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
};
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use crate::utils::BindingEvaluator;
use crate::{
    AssignedBinding, Button, Control, ControlImpl, Grid, IconElement, ItemsChangedEventArgs,
    ItemsControl, ItemsControlImpl, ItemsControlImplExt, ListBox, ListBoxItem, PlacementMode,
    StackPanel, TextBox,
};
use ferroui_base::data::{core::ValueTypes, ReflectionBinding};
use ferroui_base::input::{
    FocusChangedEventArgs, InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs,
    MouseButton,
};
use ferroui_base::interactivity::{InteractiveImpl, RoutingStrategies};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::metadata::{into_markup_value, MarkupDelegate, MarkupType, MarkupTyped};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_markup_enum, ferro_markup_type,
    ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty, StyledPropertyOptions,
    Visual, VisualImpl, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum AutoSuggestionBoxTextChangeReason {
    UserInput = 0,
    #[default]
    ProgrammaticChange = 1,
    SuggestionChosen = 2,
}
ferro_markup_enum!(AutoSuggestionBoxTextChangeReason { UserInput, ProgrammaticChange, SuggestionChosen }, { namespace: "FerroUI.Controls" });

/// A retained event does not retain the editor. CheckCurrent becomes false as
/// soon as a newer text change occurs, even before its deferred event is raised.
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
    pub fn reason(&self) -> AutoSuggestionBoxTextChangeReason {
        self.reason.get()
    }
    pub fn set_reason(&self, value: AutoSuggestionBoxTextChangeReason) {
        self.reason.set(value);
    }
    pub fn check_current(&self) -> bool {
        self.owner
            .upgrade()
            .is_some_and(|owner| owner.generation.get() == self.generation)
    }
}

#[derive(Clone)]
pub struct AutoSuggestBoxSuggestionChosenEventArgs {
    pub selected_item: BoxedValue,
}
impl PartialEq for AutoSuggestBoxSuggestionChosenEventArgs {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.selected_item, &other.selected_item)
    }
}
#[derive(Clone, PartialEq)]
pub struct AutoSuggestBoxQuerySubmittedEventArgs {
    pub query_text: String,
    pub chosen_suggestion: Option<BoxedValue>,
}

ferro_markup_type!(class AutoSuggestBoxTextChangedEventArgs {
    namespace: "FerroUI.Controls", handles: [AutoSuggestBoxTextChangedEventArgs],
    properties: [Reason: AutoSuggestionBoxTextChangeReason { get: AutoSuggestBoxTextChangedEventArgs::reason, set: AutoSuggestBoxTextChangedEventArgs::set_reason }],
    methods: [fn CheckCurrent() -> bool => AutoSuggestBoxTextChangedEventArgs::check_current],
});
ferro_markup_type!(class AutoSuggestBoxSuggestionChosenEventArgs {
    namespace: "FerroUI.Controls", handles: [AutoSuggestBoxSuggestionChosenEventArgs],
    properties: [SelectedItem: BoxedValue { get: |args: &AutoSuggestBoxSuggestionChosenEventArgs| args.selected_item.clone() }],
});
ferro_markup_type!(class AutoSuggestBoxQuerySubmittedEventArgs {
    namespace: "FerroUI.Controls", handles: [AutoSuggestBoxQuerySubmittedEventArgs],
    properties: [
        QueryText: String { get: |args: &AutoSuggestBoxQuerySubmittedEventArgs| args.query_text.clone() },
        ChosenSuggestion: Option<BoxedValue> { get: |args: &AutoSuggestBoxQuerySubmittedEventArgs| args.chosen_suggestion.clone() },
    ],
});

#[derive(Clone, Default)]
struct Parts {
    editor: Option<Ref<TextBox>>,
    list: Option<Ref<ListBox>>,
    popup: Option<Ref<Popup>>,
    query: Option<Ref<Button>>,
    header: Option<Ref<ContentPresenter>>,
    description: Option<Ref<ContentPresenter>>,
}

#[repr(C)]
pub struct AutoSuggestBox {
    base: HeaderedItemsControl,
    parts: RefCell<Parts>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    timer: RefCell<Option<Rc<DispatcherTimer>>>,
    pending: RefCell<Option<AutoSuggestBoxTextChangedEventArgs>>,
    generation: Cell<u64>,
    syncing_text: Cell<bool>,
    syncing_selection: Cell<bool>,
    has_focus: Cell<bool>,
    typed_text: RefCell<String>,
    text_changed: HandlerList<dyn Fn(&AutoSuggestBoxTextChangedEventArgs)>,
    suggestion_chosen: HandlerList<dyn Fn(&AutoSuggestBoxSuggestionChosenEventArgs)>,
    query_submitted: HandlerList<dyn Fn(&AutoSuggestBoxQuerySubmittedEventArgs)>,
}

ferro_class!(AutoSuggestBox: HeaderedItemsControl);
ferro_class_info!(AutoSuggestBox {
    new: AutoSuggestBox::new,
    markup: { namespace: "FerroUI.Controls", content: Items, events: [
        TextChanged(Option<BoxedValue>, AutoSuggestBoxTextChangedEventArgs) => |this: &Ref<AutoSuggestBox>, handler: MarkupDelegate| {
            let weak = this.downgrade();
            this.text_changed(move |args| { if let Some(owner) = weak.upgrade() { handler.invoke(&[into_markup_value(Some(owner)), into_markup_value(args.clone())]); } })
        },
        SuggestionChosen(Option<BoxedValue>, AutoSuggestBoxSuggestionChosenEventArgs) => |this: &Ref<AutoSuggestBox>, handler: MarkupDelegate| {
            let weak = this.downgrade();
            this.suggestion_chosen(move |args| { if let Some(owner) = weak.upgrade() { handler.invoke(&[into_markup_value(Some(owner)), into_markup_value(args.clone())]); } })
        },
        QuerySubmitted(Option<BoxedValue>, AutoSuggestBoxQuerySubmittedEventArgs) => |this: &Ref<AutoSuggestBox>, handler: MarkupDelegate| {
            let weak = this.downgrade();
            this.query_submitted(move |args| { if let Some(owner) = weak.upgrade() { handler.invoke(&[into_markup_value(Some(owner)), into_markup_value(args.clone())]); } })
        },
    ] },
});
ferro_impl_classes!(AutoSuggestBox: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl, HeaderedItemsControlImpl);

struct Reset<'a>(&'a Cell<bool>, bool);
impl Drop for Reset<'_> {
    fn drop(&mut self) {
        self.0.set(self.1);
    }
}

impl FerroObjectImpl for AutoSuggestBox {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let weak = this.to_ref().downgrade();
        this.add_handler_with(
            InputElement::key_down_event(),
            move |_, args| {
                if let Some(owner) = weak.upgrade() {
                    owner.handle_key(args);
                }
            },
            RoutingStrategies::TUNNEL,
            false,
        );
    }
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let property = change.property();
        if property == Self::text_property().as_property() && !this.syncing_text.get() {
            this.synchronize_text(AutoSuggestionBoxTextChangeReason::ProgrammaticChange);
        } else if property == Self::is_suggestion_list_open_property().as_property() {
            if !this.is_suggestion_list_open() {
                this.clear_selection();
            }
            this.update_parts();
        } else if property == Self::placeholder_text_property().as_property()
            || property == Self::query_icon_property().as_property()
            || property == Self::description_property().as_property()
            || property == Self::max_suggestion_list_height_property().as_property()
            || property == HeaderedItemsControl::header_property().as_property()
            || property == HeaderedItemsControl::header_template_property().as_property()
            || property == ItemsControl::item_template_property().as_property()
            || property == Visual::bounds_property().as_property()
        {
            this.update_parts();
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
        this.update_parts();
        // Let the inner selector process its collection notification first.
        let weak = this.to_ref().downgrade();
        Dispatcher::ui_thread().post_local(
            move || {
                if let Some(owner) = weak.upgrade() {
                    owner.update_suggestion_visibility();
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
        this.update_suggestion_visibility();
    }
    fn on_lost_focus(this: &Self, args: &FocusChangedEventArgs) {
        Self::parent_on_lost_focus(this, args);
        let parts = this.parts.borrow().clone();
        let within = args.new_focused_element.as_ref().is_some_and(|next| {
            this.is_visual_ancestor_of(next)
                || parts.list.as_ref().is_some_and(|list| {
                    list.is_visual_ancestor_of(next)
                        || list.to_ref().upcast::<InputElement>() == *next
                })
        });
        if !within {
            this.has_focus.set(false);
            this.set_current_value(Self::is_suggestion_list_open_property(), false);
        }
    }
}
impl TemplatedControlImpl for AutoSuggestBox {
    fn on_apply_template(this: &Self, args: &TemplateAppliedEventArgs) {
        for subscription in this.subscriptions.take() {
            subscription.dispose();
        }
        if let Some(popup) = this.parts.borrow().popup.clone() {
            popup.set_is_open(false);
        }
        Self::parent_on_apply_template(this, args);
        let scope = args.name_scope();
        let parts = Parts {
            editor: scope.find_as("PART_TextBox"),
            list: scope.find_as("PART_SuggestionsList"),
            popup: scope.find_as("PART_Popup"),
            query: scope.find_as("PART_QueryButton"),
            header: scope.find_as("PART_HeaderPresenter"),
            description: scope.find_as("PART_DescriptionPresenter"),
        };
        *this.parts.borrow_mut() = parts.clone();
        if let Some(editor) = &parts.editor {
            let weak = this.to_ref().downgrade();
            this.subscriptions
                .borrow_mut()
                .push(editor.property_changed(move |change| {
                    if change.property() == TextBox::text_property().as_property() {
                        if let Some(owner) = weak.upgrade() {
                            owner.editor_changed();
                        }
                    }
                }));
        }
        if let Some(list) = &parts.list {
            let weak = this.to_ref().downgrade();
            let token = list.selection_changed(move |_, _| {
                if let Some(owner) = weak.upgrade() {
                    owner.selection_changed();
                }
            });
            let weak_list = list.downgrade();
            this.subscriptions
                .borrow_mut()
                .push(Disposable::create(move || {
                    if let Some(list) = weak_list.upgrade() {
                        list.remove_handler(
                            crate::primitives::SelectingItemsControl::selection_changed_event(),
                            token,
                        );
                    }
                }));
            let weak = this.to_ref().downgrade();
            let weak_list = list.downgrade();
            let token = list.add_handler_with(
                InputElement::pointer_released_event(),
                move |_, args| {
                    if args.initial_press_mouse_button() != MouseButton::Left {
                        return;
                    }
                    let (Some(owner), Some(list), Some(source)) = (
                        weak.upgrade(),
                        weak_list.upgrade(),
                        args.source().and_then(|source| source.cast::<Visual>()),
                    ) else {
                        return;
                    };
                    // Ignore releases in the scrollbar or empty popup area.
                    if !std::iter::once(source.clone())
                        .chain(source.get_visual_ancestors())
                        .any(|v| v.cast::<ListBoxItem>().is_some())
                    {
                        return;
                    }
                    let selected = list.selected_item();
                    if selected.is_some() {
                        let weak = owner.downgrade();
                        Dispatcher::ui_thread().post_local(
                            move || {
                                if let Some(owner) = weak.upgrade() {
                                    owner.submit(selected);
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
            this.subscriptions
                .borrow_mut()
                .push(Disposable::create(move || {
                    if let Some(list) = weak_list.upgrade() {
                        list.remove_handler(InputElement::pointer_released_event(), token);
                    }
                }));
        }
        if let Some(query) = &parts.query {
            let weak = this.to_ref().downgrade();
            let token = query.click(move |_, _| {
                if let Some(owner) = weak.upgrade() {
                    owner.submit_query();
                }
            });
            let weak_query = query.downgrade();
            this.subscriptions
                .borrow_mut()
                .push(Disposable::create(move || {
                    if let Some(query) = weak_query.upgrade() {
                        query.remove_handler(Button::click_event(), token);
                    }
                }));
        }
        if let Some(popup) = &parts.popup {
            let weak = this.to_ref().downgrade();
            this.subscriptions.borrow_mut().push(popup.closed(move || {
                if let Some(owner) = weak.upgrade() {
                    owner.set_current_value(Self::is_suggestion_list_open_property(), false);
                }
            }));
        }
        this.synchronize_text(AutoSuggestionBoxTextChangeReason::ProgrammaticChange);
        this.update_parts();
    }
}

ferro_properties! { impl AutoSuggestBox {
    pub fn text_property() -> StyledProperty<String> { FerroProperty::register::<AutoSuggestBox, _>("Text", String::new()) }
    pub fn text_member_path_property() -> StyledProperty<String> { FerroProperty::register::<AutoSuggestBox, _>("TextMemberPath", String::new()) }
    pub fn placeholder_text_property() -> StyledProperty<String> { FerroProperty::register::<AutoSuggestBox, _>("PlaceholderText", String::new()) }
    pub fn update_text_on_select_property() -> StyledProperty<bool> { FerroProperty::register::<AutoSuggestBox, _>("UpdateTextOnSelect", true) }
    pub fn is_suggestion_list_open_property() -> StyledProperty<bool> { FerroProperty::register::<AutoSuggestBox, _>("IsSuggestionListOpen", false) }
    pub fn max_suggestion_list_height_property() -> StyledProperty<f64> { FerroProperty::register_with::<AutoSuggestBox, _>("MaxSuggestionListHeight", StyledPropertyOptions::new(f64::INFINITY).validate(|value| *value >= 0.0)) }
    pub fn query_icon_property() -> StyledProperty<Option<Ref<IconElement>>> { FerroProperty::register::<AutoSuggestBox, _>("QueryIcon", None) }
    pub fn description_property() -> StyledProperty<Option<BoxedValue>> { FerroProperty::register::<AutoSuggestBox, _>("Description", None) }
} }

impl AutoSuggestBox {
    fn static_constructor() {
        InputElement::focusable_property().override_default_value::<AutoSuggestBox>(false);
        let template = FuncControlTemplate::for_type::<AutoSuggestBox>(|_, scope| {
            let root = StackPanel::new();
            root.set_spacing(4.0);
            let header = ContentPresenter::new();
            header.set_name(Some("PART_HeaderPresenter".into()));
            root.children()
                .add(header.register_in_name_scope(&**scope).upcast::<Control>());
            let row = Grid::new();
            row.set_column_definitions("*,Auto".parse().expect("constant grid columns"));
            let editor = TextBox::new();
            editor.set_name(Some("PART_TextBox".into()));
            row.children().add(
                editor
                    .clone()
                    .register_in_name_scope(&**scope)
                    .upcast::<Control>(),
            );
            let query = Button::new();
            query.set_name(Some("PART_QueryButton".into()));
            Grid::set_column(&query, 1);
            AutomationProperties::set_name(&query, Some("Submit query"));
            row.children()
                .add(query.register_in_name_scope(&**scope).upcast::<Control>());
            let list = ListBox::new();
            list.set_name(Some("PART_SuggestionsList".into()));
            list.set_focusable(false);
            list.clone().register_in_name_scope(&**scope);
            let popup = Popup::new();
            popup.set_name(Some("PART_Popup".into()));
            popup.set_placement(PlacementMode::Bottom);
            popup.set_placement_target(Some(editor.upcast()));
            popup.set_is_light_dismiss_enabled(true);
            popup.set_child(Some(list.upcast()));
            row.children()
                .add(popup.register_in_name_scope(&**scope).upcast::<Control>());
            root.children().add(row.upcast::<Control>());
            let description = ContentPresenter::new();
            description.set_name(Some("PART_DescriptionPresenter".into()));
            root.children().add(
                description
                    .register_in_name_scope(&**scope)
                    .upcast::<Control>(),
            );
            root.upcast()
        });
        TemplatedControl::template_property()
            .override_default_value::<AutoSuggestBox>(Some(template));
    }
    pub fn construct() -> Self {
        Self {
            base: HeaderedItemsControl::construct(),
            parts: RefCell::new(Parts::default()),
            subscriptions: RefCell::new(Vec::new()),
            timer: RefCell::new(None),
            pending: RefCell::new(None),
            generation: Cell::new(0),
            syncing_text: Cell::new(false),
            syncing_selection: Cell::new(false),
            has_focus: Cell::new(false),
            typed_text: RefCell::new(String::new()),
            text_changed: HandlerList::new(),
            suggestion_chosen: HandlerList::new(),
            query_submitted: HandlerList::new(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn text(&self) -> String {
        self.get_value(Self::text_property())
    }
    pub fn set_text(&self, value: &str) {
        self.set_value(Self::text_property(), value.to_owned());
    }
    pub fn text_member_path(&self) -> String {
        self.get_value(Self::text_member_path_property())
    }
    pub fn set_text_member_path(&self, value: &str) {
        self.set_value(Self::text_member_path_property(), value.to_owned());
    }
    pub fn placeholder_text(&self) -> String {
        self.get_value(Self::placeholder_text_property())
    }
    pub fn set_placeholder_text(&self, value: &str) {
        self.set_value(Self::placeholder_text_property(), value.to_owned());
    }
    pub fn update_text_on_select(&self) -> bool {
        self.get_value(Self::update_text_on_select_property())
    }
    pub fn set_update_text_on_select(&self, value: bool) {
        self.set_value(Self::update_text_on_select_property(), value);
    }
    pub fn is_suggestion_list_open(&self) -> bool {
        self.get_value(Self::is_suggestion_list_open_property())
    }
    pub fn set_is_suggestion_list_open(&self, value: bool) {
        self.set_value(Self::is_suggestion_list_open_property(), value);
        if value {
            if let Some(editor) = self.parts.borrow().editor.clone() {
                editor.focus();
            }
        }
    }
    pub fn max_suggestion_list_height(&self) -> f64 {
        self.get_value(Self::max_suggestion_list_height_property())
    }
    pub fn set_max_suggestion_list_height(&self, value: f64) {
        self.set_value(Self::max_suggestion_list_height_property(), value);
    }
    pub fn query_icon(&self) -> Option<Ref<IconElement>> {
        self.get_value(Self::query_icon_property())
    }
    pub fn set_query_icon(&self, value: Option<Ref<IconElement>>) {
        self.set_value(Self::query_icon_property(), value);
    }
    pub fn description(&self) -> Option<BoxedValue> {
        self.get_value(Self::description_property())
    }
    pub fn set_description(&self, value: Option<BoxedValue>) {
        self.set_value(Self::description_property(), value);
    }

    pub fn text_changed(
        &self,
        handler: impl Fn(&AutoSuggestBoxTextChangedEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.text_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.text_changed.remove(token);
            }
        })
    }
    pub fn suggestion_chosen(
        &self,
        handler: impl Fn(&AutoSuggestBoxSuggestionChosenEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.suggestion_chosen.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.suggestion_chosen.remove(token);
            }
        })
    }
    pub fn query_submitted(
        &self,
        handler: impl Fn(&AutoSuggestBoxQuerySubmittedEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.query_submitted.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.query_submitted.remove(token);
            }
        })
    }

    /// Submit the typed query, deliberately ignoring any highlighted suggestion.
    /// The query button and automation invoke provider share this implementation.
    pub fn submit_query(&self) {
        self.clear_selection();
        self.submit(None);
    }

    fn submit(&self, selected: Option<BoxedValue>) {
        let args = AutoSuggestBoxQuerySubmittedEventArgs {
            query_text: self.text(),
            chosen_suggestion: selected,
        };
        for (_, handler) in self.query_submitted.snapshot().iter() {
            handler(&args);
        }
        self.set_current_value(Self::is_suggestion_list_open_property(), false);
    }
    fn synchronize_text(&self, reason: AutoSuggestionBoxTextChangeReason) {
        let Some(editor) = self.parts.borrow().editor.clone() else {
            return;
        };
        let text = self.text();
        if editor.text().as_deref().unwrap_or("") == text {
            return;
        }
        let _guard = Reset(&self.syncing_text, self.syncing_text.replace(true));
        editor.set_current_value(TextBox::text_property(), Some(text));
        self.queue_text_changed(reason);
    }
    fn editor_changed(&self) {
        if self.syncing_text.get() {
            return;
        }
        let Some(editor) = self.parts.borrow().editor.clone() else {
            return;
        };
        let text = editor.text().unwrap_or_default();
        let _guard = Reset(&self.syncing_text, self.syncing_text.replace(true));
        self.set_current_value(Self::text_property(), text.clone());
        *self.typed_text.borrow_mut() = text;
        self.clear_selection();
        self.queue_text_changed(AutoSuggestionBoxTextChangeReason::UserInput);
        self.update_suggestion_visibility();
    }
    fn queue_text_changed(&self, reason: AutoSuggestionBoxTextChangeReason) {
        let generation = self.generation.get().wrapping_add(1);
        self.generation.set(generation);
        *self.pending.borrow_mut() = Some(AutoSuggestBoxTextChangedEventArgs {
            reason: Rc::new(Cell::new(reason)),
            owner: self.to_ref().downgrade(),
            generation,
        });
        let existing_timer = self.timer.borrow().clone();
        let timer = existing_timer.unwrap_or_else(|| {
            let timer = DispatcherTimer::new();
            timer.set_interval(Duration::from_millis(150));
            let weak = self.to_ref().downgrade();
            timer.tick(move |timer| {
                timer.stop();
                if let Some(owner) = weak.upgrade() {
                    let pending = owner.pending.take();
                    if let Some(args) = pending {
                        for (_, handler) in owner.text_changed.snapshot().iter() {
                            handler(&args);
                        }
                    }
                    owner.update_suggestion_visibility();
                }
            });
            *self.timer.borrow_mut() = Some(timer.clone());
            timer
        });
        timer.stop();
        timer.start();
    }
    fn clear_selection(&self) {
        let _guard = Reset(
            &self.syncing_selection,
            self.syncing_selection.replace(true),
        );
        if let Some(list) = self.parts.borrow().list.clone() {
            list.set_selected_index(-1);
        }
    }
    fn selection_changed(&self) {
        if self.syncing_selection.get() {
            return;
        }
        let selected = self
            .parts
            .borrow()
            .list
            .as_ref()
            .and_then(|list| list.selected_item());
        let Some(item) = selected else {
            return;
        };
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
            let _guard = Reset(&self.syncing_text, self.syncing_text.replace(true));
            self.set_current_value(Self::text_property(), text);
            self.synchronize_text(AutoSuggestionBoxTextChangeReason::SuggestionChosen);
            if let Some(editor) = self.parts.borrow().editor.clone() {
                editor.set_caret_index(self.text().encode_utf16().count() as i32);
            }
        }
        let args = AutoSuggestBoxSuggestionChosenEventArgs {
            selected_item: item,
        };
        for (_, handler) in self.suggestion_chosen.snapshot().iter() {
            handler(&args);
        }
    }
    fn update_suggestion_visibility(&self) {
        self.set_current_value(
            Self::is_suggestion_list_open_property(),
            self.has_focus.get() && self.items().count() > 0,
        );
    }
    fn update_parts(&self) {
        let parts = self.parts.borrow().clone();
        if let Some(editor) = parts.editor {
            editor.set_placeholder_text(Some(&self.placeholder_text()));
        }
        if let Some(header) = parts.header {
            header.set_content(self.header());
            header.set_content_template(self.header_template());
            header.set_is_visible(self.header().is_some());
        }
        if let Some(description) = parts.description {
            description.set_content(self.description());
            description.set_is_visible(self.description().is_some());
        }
        if let Some(query) = parts.query {
            query.set_content(self.query_icon().map(Control::boxed));
            query.set_is_visible(self.query_icon().is_some());
        }
        if let Some(list) = parts.list {
            list.set_items_source(Some(self.items().source()));
            list.set_item_template(self.item_template());
            list.set_max_height(self.max_suggestion_list_height());
        }
        if let Some(popup) = parts.popup {
            popup.set_min_width(self.bounds().width);
            popup.set_is_open(self.is_suggestion_list_open());
        }
    }
    fn handle_key(&self, args: &KeyEventArgs) {
        if args.handled() || !self.is_effectively_enabled() {
            return;
        }
        let parts = self.parts.borrow().clone();
        if parts
            .query
            .as_ref()
            .is_some_and(|query| args.source().as_ref() == Some(&query.to_ref().upcast()))
        {
            return;
        }
        let Some(list) = parts.list else {
            return;
        };
        match args.key {
            Key::Up | Key::Down if self.is_suggestion_list_open() => {
                let count = self.items().count() as i32;
                let index = list.selected_index();
                if count > 0 && !(count == 1 && index == 0) {
                    let next = if args.key == Key::Down {
                        if index == count - 1 {
                            -1
                        } else {
                            index + 1
                        }
                    } else if index < 0 {
                        count - 1
                    } else {
                        index - 1
                    };
                    if next < 0 {
                        self.set_text(&self.typed_text.borrow().clone());
                    }
                    list.set_selected_index(next);
                }
                args.set_handled(true);
            }
            Key::Escape | Key::Tab if args.key == Key::Escape || self.is_suggestion_list_open() => {
                self.set_text(&self.typed_text.borrow().clone());
                self.set_current_value(Self::is_suggestion_list_open_property(), false);
                if args.key == Key::Escape {
                    args.set_handled(true);
                }
            }
            Key::Enter => {
                self.submit(list.selected_item());
                args.set_handled(true);
            }
            _ => {}
        }
    }
}
impl Drop for AutoSuggestBox {
    fn drop(&mut self) {
        if let Some(timer) = self.timer.get_mut().take() {
            timer.stop();
        }
        for subscription in self.subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
    }
}

pub(crate) const TYPES: &[&MarkupType] = &[
    <AutoSuggestionBoxTextChangeReason as MarkupTyped>::MARKUP,
    <AutoSuggestBoxTextChangedEventArgs as MarkupTyped>::MARKUP,
    <AutoSuggestBoxSuggestionChosenEventArgs as MarkupTyped>::MARKUP,
    <AutoSuggestBoxQuerySubmittedEventArgs as MarkupTyped>::MARKUP,
];
pub(crate) fn register_markup_types() {
    MarkupType::register_all(TYPES);
}
