//! WinUI's `PagerControl` (`controls/dev/PagerControl/PagerControl.cpp`, `.h`, `.idl` and
//! `PagerControlSelectedIndexChangedEventArgs.h` of microsoft-ui-xaml, commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).

use crate::automation::peers::{AutomationPeer, ControlAutomationPeer, PagerControlAutomationPeer};
use crate::automation::AutomationProperties;
use crate::primitives::{PagerControlTemplateSettings, SelectingItemsControl, TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt};
use crate::{Button, ComboBox, Control, ControlImpl, ItemsControl, NumberBox, Symbol, SymbolIcon};
use ferroui_base::controls::ResourceKey;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::input::{FocusManager, ICommand, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, NavigationDirection};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::metadata::{into_markup_value, MarkupDelegate};
use ferroui_base::reactive::{Disposable, Subscription};
use ferroui_base::styling::ControlTheme;
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_markup_enum, ferro_markup_type, ferro_property, instantiate, BoxedValue,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Point, Ref,
    StyledElementImpl, StyledProperty, Thickness, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The page number from which the `Auto` display mode shows a number box instead of a combo box
/// (`c_AutoDisplayModeNumberOfPagesThreshold`).
const AUTO_DISPLAY_MODE_NUMBER_OF_PAGES_THRESHOLD: i32 = 10;
/// The number of page numbers the combo box lists while the number of pages is not known
/// (`c_infiniteModeComboBoxItemsIncrement`).
const INFINITE_MODE_COMBO_BOX_ITEMS_INCREMENT: i32 = 100;
/// The key of the style of the buttons of the number panel (`c_numberPanelButtonStyleName`).
const NUMBER_PANEL_BUTTON_STYLE_NAME: &str = "PagerControlNumberPanelButtonStyle";

// The English strings of WinUI's resources (`Strings/en-us/Resources.resw`).
const FIRST_PAGE_BUTTON_TEXT: &str = "First page";
const PREVIOUS_PAGE_BUTTON_TEXT: &str = "Previous page";
const NEXT_PAGE_BUTTON_TEXT: &str = "Next page";
const LAST_PAGE_BUTTON_TEXT: &str = "Last page";
const PAGE_TEXT: &str = "Page";
const PREFIX_TEXT: &str = "Page";
const SUFFIX_TEXT: &str = "of";

/// How a [`PagerControl`] shows the pages (WinUI's `PagerControlDisplayMode`, with its values).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum PagerControlDisplayMode {
    /// A combo box below ten pages, a number box from ten pages or while the number of pages is
    /// not known (the default).
    #[default]
    Auto = 0,
    /// A combo box of the page numbers.
    ComboBox = 1,
    /// A number box to type the page number.
    NumberBox = 2,
    /// A button for each page, with ellipses for the pages left out.
    ButtonPanel = 3,
}

/// When a navigation button of a [`PagerControl`] is shown (WinUI's
/// `PagerControlButtonVisibility`, with its values).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum PagerControlButtonVisibility {
    /// Always shown (the default).
    #[default]
    Visible = 0,
    /// Shown, but transparent and disabled on the first page (the first and previous buttons)
    /// or the last page (the next and last buttons).
    HiddenOnEdge = 1,
    /// Never shown.
    Hidden = 2,
}

ferro_markup_enum!(PagerControlDisplayMode { Auto, ComboBox, NumberBox, ButtonPanel }, { namespace: "FerroUI.Controls" });
ferro_markup_enum!(PagerControlButtonVisibility { Visible, HiddenOnEdge, Hidden }, { namespace: "FerroUI.Controls" });

/// The arguments of the `SelectedIndexChanged` event of a [`PagerControl`] (WinUI's
/// `PagerControlSelectedIndexChangedEventArgs`): the index of the page before the change and the
/// index of the selected page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PagerControlSelectedIndexChangedEventArgs {
    /// The index of the page before the change, -1 before the first change.
    pub previous_page_index: i32,
    /// The index of the selected page.
    pub new_page_index: i32,
}

impl PagerControlSelectedIndexChangedEventArgs {
    /// The index of the page before the change, -1 before the first change.
    pub fn previous_page_index(&self) -> i32 {
        self.previous_page_index
    }

    /// The index of the selected page.
    pub fn new_page_index(&self) -> i32 {
        self.new_page_index
    }
}

ferro_markup_type!(class PagerControlSelectedIndexChangedEventArgs {
    namespace: "FerroUI.Controls",
    handles: [PagerControlSelectedIndexChangedEventArgs],
    properties: [
        NewPageIndex: i32 { get: PagerControlSelectedIndexChangedEventArgs::new_page_index },
        PreviousPageIndex: i32 { get: PagerControlSelectedIndexChangedEventArgs::previous_page_index },
    ],
});

/// A visual state of the template (WinUI's `VisualStateManager::GoToState` with the names of
/// `PagerControl.cpp`): the pseudo-classes of its group, and the one it sets (none for the
/// state that sets nothing).
struct State {
    group: &'static [&'static str],
    pseudo_class: Option<&'static str>,
}

const FIRST_VISIBILITY: &[&str] = &[":firstcollapsed", ":firsthidden"];
const FIRST_ENABLED: &[&str] = &[":firstdisabled"];
const PREVIOUS_VISIBILITY: &[&str] = &[":previouscollapsed", ":previoushidden"];
const PREVIOUS_ENABLED: &[&str] = &[":previousdisabled"];
const NEXT_VISIBILITY: &[&str] = &[":nextcollapsed"];
// WinUI's NextPageButtonHidden is a state of the group of NextPageButtonIsEnabledStates, and
// LastPageButtonHidden of the second group of the last button: hidden and disabled exclude each
// other there.
const NEXT_ENABLED: &[&str] = &[":nextdisabled", ":nexthidden"];
const LAST_VISIBILITY: &[&str] = &[":lastcollapsed"];
const LAST_ENABLED: &[&str] = &[":lastdisabled", ":lasthidden"];
const DISPLAY_MODE: &[&str] = &[":numberbox", ":combobox", ":numberpanel"];
const ITEMS_MODE: &[&str] = &[":infinite"];

const FIRST_PAGE_BUTTON_VISIBLE: State = State { group: FIRST_VISIBILITY, pseudo_class: None };
const FIRST_PAGE_BUTTON_COLLAPSED: State = State { group: FIRST_VISIBILITY, pseudo_class: Some(":firstcollapsed") };
const FIRST_PAGE_BUTTON_HIDDEN: State = State { group: FIRST_VISIBILITY, pseudo_class: Some(":firsthidden") };
const FIRST_PAGE_BUTTON_ENABLED: State = State { group: FIRST_ENABLED, pseudo_class: None };
const FIRST_PAGE_BUTTON_DISABLED: State = State { group: FIRST_ENABLED, pseudo_class: Some(":firstdisabled") };
const PREVIOUS_PAGE_BUTTON_VISIBLE: State = State { group: PREVIOUS_VISIBILITY, pseudo_class: None };
const PREVIOUS_PAGE_BUTTON_COLLAPSED: State = State { group: PREVIOUS_VISIBILITY, pseudo_class: Some(":previouscollapsed") };
const PREVIOUS_PAGE_BUTTON_HIDDEN: State = State { group: PREVIOUS_VISIBILITY, pseudo_class: Some(":previoushidden") };
const PREVIOUS_PAGE_BUTTON_ENABLED: State = State { group: PREVIOUS_ENABLED, pseudo_class: None };
const PREVIOUS_PAGE_BUTTON_DISABLED: State = State { group: PREVIOUS_ENABLED, pseudo_class: Some(":previousdisabled") };
const NEXT_PAGE_BUTTON_VISIBLE: State = State { group: NEXT_VISIBILITY, pseudo_class: None };
const NEXT_PAGE_BUTTON_COLLAPSED: State = State { group: NEXT_VISIBILITY, pseudo_class: Some(":nextcollapsed") };
const NEXT_PAGE_BUTTON_HIDDEN: State = State { group: NEXT_ENABLED, pseudo_class: Some(":nexthidden") };
const NEXT_PAGE_BUTTON_ENABLED: State = State { group: NEXT_ENABLED, pseudo_class: None };
const NEXT_PAGE_BUTTON_DISABLED: State = State { group: NEXT_ENABLED, pseudo_class: Some(":nextdisabled") };
const LAST_PAGE_BUTTON_VISIBLE: State = State { group: LAST_VISIBILITY, pseudo_class: None };
const LAST_PAGE_BUTTON_COLLAPSED: State = State { group: LAST_VISIBILITY, pseudo_class: Some(":lastcollapsed") };
const LAST_PAGE_BUTTON_HIDDEN: State = State { group: LAST_ENABLED, pseudo_class: Some(":lasthidden") };
const LAST_PAGE_BUTTON_ENABLED: State = State { group: LAST_ENABLED, pseudo_class: None };
const LAST_PAGE_BUTTON_DISABLED: State = State { group: LAST_ENABLED, pseudo_class: Some(":lastdisabled") };
const NUMBER_BOX_VISIBLE: State = State { group: DISPLAY_MODE, pseudo_class: Some(":numberbox") };
const COMBO_BOX_VISIBLE: State = State { group: DISPLAY_MODE, pseudo_class: Some(":combobox") };
const NUMBER_PANEL_VISIBLE: State = State { group: DISPLAY_MODE, pseudo_class: Some(":numberpanel") };
const FINITE_ITEMS: State = State { group: ITEMS_MODE, pseudo_class: None };
const INFINITE_ITEMS: State = State { group: ITEMS_MODE, pseudo_class: Some(":infinite") };

#[derive(Clone, Default)]
struct Parts {
    combo_box: Option<Ref<ComboBox>>,
    number_box: Option<Ref<NumberBox>>,
    number_panel: Option<Ref<ItemsControl>>,
    selected_page_indicator: Option<Ref<Control>>,
}

/// A control to move through the pages of a paged content (WinUI's `PagerControl`): buttons to
/// the first, previous, next and last page around a combo box of the page numbers, a number box
/// to type one, or a button for each page (`DisplayMode`). `NumberOfPages` is -1 while the number
/// of pages is not known; `SelectedPageIndex` is the index of the page, from 0, kept in the range
/// of the pages, and its changes raise `SelectedIndexChanged`.
///
/// The template is the control theme of the WinUI theme (`Controls/PagerControl.xaml`, from
/// WinUI's `PagerControl.xaml`), with WinUI's parts `FirstPageButton`, `PreviousPageButton`,
/// `NextPageButton`, `LastPageButton`, `ComboBoxDisplay`, `NumberBoxDisplay`,
/// `NumberPanelItemsRepeater` and `NumberPanelCurrentPageIndicator`. The control sets WinUI's
/// visual states as pseudo-classes: `:numberbox`, `:combobox` and `:numberpanel` (the display
/// mode), `:infinite` (InfiniteItems), and for each button `:<button>collapsed`,
/// `:<button>hidden` and `:<button>disabled` (`:firstcollapsed`, `:nexthidden`, ...). The
/// Fluent and Simple themes have no template for it (docs/porting/winui-controls.md, section 5.1).
///
/// Differences (docs/porting/winui-controls.md, G2): the number panel is an items control whose
/// panel is a horizontal stack panel, where WinUI has an `ItemsRepeater` with a `StackLayout`
/// (`ferroui-controls` cannot depend on the repeater crate); the indicator of the current page
/// moves without WinUI's `RepositionThemeTransition`.
#[repr(C)]
pub struct PagerControl {
    base: TemplatedControl,
    parts: RefCell<Parts>,
    template_subscriptions: RefCell<Vec<Subscription>>,
    template_applied: Cell<bool>,
    last_selected_page_index: Cell<i32>,
    last_number_of_pages_count: Cell<i32>,
    selected_index_changed: HandlerList<dyn Fn(&PagerControl, &PagerControlSelectedIndexChangedEventArgs)>,
}

ferro_class!(PagerControl: TemplatedControl);
ferroui_base::ferro_class_info!(PagerControl {
    new: PagerControl::new,
    markup: {
        namespace: "FerroUI.Controls",
        events: [
            SelectedIndexChanged(Option<BoxedValue>, PagerControlSelectedIndexChangedEventArgs) => |this: &Ref<PagerControl>, handler: MarkupDelegate| {
                this.selected_index_changed(move |sender: &PagerControl, args: &PagerControlSelectedIndexChangedEventArgs| {
                    handler.invoke(&[into_markup_value(Some(sender.to_ref())), into_markup_value(args.clone())]);
                })
            },
        ],
        attributes: [
            TemplatePart("FirstPageButton", type(Ref<Button>)),
            TemplatePart("PreviousPageButton", type(Ref<Button>)),
            TemplatePart("NextPageButton", type(Ref<Button>)),
            TemplatePart("LastPageButton", type(Ref<Button>)),
            TemplatePart("ComboBoxDisplay", type(Ref<ComboBox>)),
            TemplatePart("NumberBoxDisplay", type(Ref<NumberBox>)),
            TemplatePart("NumberPanelItemsRepeater", type(Ref<ItemsControl>)),
            TemplatePart("NumberPanelCurrentPageIndicator", type(Ref<Control>)),
            PseudoClasses(
                ":numberbox", ":combobox", ":numberpanel", ":infinite",
                ":firstcollapsed", ":firsthidden", ":firstdisabled",
                ":previouscollapsed", ":previoushidden", ":previousdisabled",
                ":nextcollapsed", ":nexthidden", ":nextdisabled",
                ":lastcollapsed", ":lasthidden", ":lastdisabled"
            ),
        ],
    },
});
ferro_impl_classes!(PagerControl: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl);

impl FerroObjectImpl for PagerControl {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if !change.is_effective_value_change() || !this.template_applied.get() {
            return;
        }
        let property = change.property();
        if property == Self::first_button_visibility_property().as_property() {
            this.on_button_visibility_changed(
                this.first_button_visibility(),
                FIRST_PAGE_BUTTON_VISIBLE,
                FIRST_PAGE_BUTTON_COLLAPSED,
                FIRST_PAGE_BUTTON_HIDDEN,
                0,
            );
        } else if property == Self::previous_button_visibility_property().as_property() {
            this.on_button_visibility_changed(
                this.previous_button_visibility(),
                PREVIOUS_PAGE_BUTTON_VISIBLE,
                PREVIOUS_PAGE_BUTTON_COLLAPSED,
                PREVIOUS_PAGE_BUTTON_HIDDEN,
                0,
            );
        } else if property == Self::next_button_visibility_property().as_property() {
            this.on_button_visibility_changed(
                this.next_button_visibility(),
                NEXT_PAGE_BUTTON_VISIBLE,
                NEXT_PAGE_BUTTON_COLLAPSED,
                NEXT_PAGE_BUTTON_HIDDEN,
                this.number_of_pages() - 1,
            );
        } else if property == Self::last_button_visibility_property().as_property() {
            this.on_button_visibility_changed(
                this.last_button_visibility(),
                LAST_PAGE_BUTTON_VISIBLE,
                LAST_PAGE_BUTTON_COLLAPSED,
                LAST_PAGE_BUTTON_HIDDEN,
                this.number_of_pages() - 1,
            );
        } else if property == Self::display_mode_property().as_property() {
            this.on_display_mode_changed();
            // "That method only updates what it currently needs to update. So when we switch to
            // ComboBox from NumberPanel, the NumberPanel element list might be out of date."
            this.update_template_setting_element_lists();
        } else if property == Self::number_of_pages_property().as_property() {
            this.on_number_of_pages_changed(change.get_old_value::<i32>().unwrap_or(0));
        } else if property == Self::selected_page_index_property().as_property() {
            this.on_selected_page_index_change(change.get_old_value::<i32>().unwrap_or(0));
        } else if property == Self::button_panel_always_show_first_last_page_index_property().as_property() {
            this.update_number_panel(this.number_of_pages());
        }
    }
}

impl ControlImpl for PagerControl {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        PagerControlAutomationPeer::new(this).upcast()
    }
}

impl TemplatedControlImpl for PagerControl {
    fn on_apply_template(this: &Self, args: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, args);
        drop(this.template_subscriptions.take());
        if this.prefix_text().is_empty() {
            this.set_prefix_text(PREFIX_TEXT);
        }
        if this.suffix_text().is_empty() {
            this.set_suffix_text(SUFFIX_TEXT);
        }
        let scope = args.name_scope();
        let mut subscriptions = Vec::new();
        for (name, text, kind) in [
            ("FirstPageButton", FIRST_PAGE_BUTTON_TEXT, NavigationButton::First),
            ("PreviousPageButton", PREVIOUS_PAGE_BUTTON_TEXT, NavigationButton::Previous),
            ("NextPageButton", NEXT_PAGE_BUTTON_TEXT, NavigationButton::Next),
            ("LastPageButton", LAST_PAGE_BUTTON_TEXT, NavigationButton::Last),
        ] {
            let Some(button) = scope.find_as::<Button>(name) else { continue };
            AutomationProperties::set_name(&button, Some(text));
            let weak = this.to_ref().downgrade();
            let token = button.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.navigation_button_clicked(kind);
                }
            });
            let button = button.downgrade();
            subscriptions.push(Subscription::new(Disposable::create(move || {
                if let Some(button) = button.upgrade() {
                    button.remove_handler(Button::click_event(), token);
                }
            })));
        }

        let combo_box = scope.find_as::<ComboBox>("ComboBoxDisplay");
        // The parts are known before the combo box and the number box are set up: setting their
        // values raises their changes, which read them.
        *this.parts.borrow_mut() = Parts {
            combo_box: None,
            number_box: None,
            number_panel: scope.find_as::<ItemsControl>("NumberPanelItemsRepeater"),
            selected_page_indicator: scope.find_as::<Control>("NumberPanelCurrentPageIndicator"),
        };
        if let Some(combo_box) = &combo_box {
            this.parts.borrow_mut().combo_box = Some(combo_box.clone());
            this.fill_combo_box_collection_to_size(this.number_of_pages());
            combo_box.set_selected_index(this.selected_page_index() - 1);
            AutomationProperties::set_name(combo_box, Some(PAGE_TEXT));
            let weak = this.to_ref().downgrade();
            let token = combo_box.selection_changed(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.combo_box_selection_changed();
                }
            });
            let combo_box = combo_box.downgrade();
            subscriptions.push(Subscription::new(Disposable::create(move || {
                if let Some(combo_box) = combo_box.upgrade() {
                    combo_box.remove_handler(SelectingItemsControl::selection_changed_event(), token);
                }
            })));
        }
        if let Some(number_box) = scope.find_as::<NumberBox>("NumberBoxDisplay") {
            this.parts.borrow_mut().number_box = Some(number_box.clone());
            number_box.set_numeric_value(this.selected_page_index() as f64 + 1.0);
            AutomationProperties::set_name(&number_box, Some(PAGE_TEXT));
            let weak = this.to_ref().downgrade();
            subscriptions.push(number_box.value_changed(move |args| {
                if let Some(this) = weak.upgrade() {
                    // NumberBoxValueChanged: an empty box (NaN) is page 0, as the cast of C++.
                    let value = args.new_value();
                    let page = if value.is_finite() { value as i32 } else { 0 };
                    this.set_selected_page_index(page - 1);
                }
            }));
        }
        *this.template_subscriptions.borrow_mut() = subscriptions;
        this.template_applied.set(true);

        // "This is for the initial loading of the control"
        this.on_display_mode_changed();
        this.update_on_edge_button_visual_states();
        this.on_number_of_pages_changed(0);

        // Update button visibilities
        this.on_button_visibility_changed(
            this.first_button_visibility(),
            FIRST_PAGE_BUTTON_VISIBLE,
            FIRST_PAGE_BUTTON_COLLAPSED,
            FIRST_PAGE_BUTTON_HIDDEN,
            0,
        );
        this.on_button_visibility_changed(
            this.previous_button_visibility(),
            PREVIOUS_PAGE_BUTTON_VISIBLE,
            PREVIOUS_PAGE_BUTTON_COLLAPSED,
            PREVIOUS_PAGE_BUTTON_HIDDEN,
            0,
        );
        this.on_button_visibility_changed(
            this.next_button_visibility(),
            NEXT_PAGE_BUTTON_VISIBLE,
            NEXT_PAGE_BUTTON_COLLAPSED,
            NEXT_PAGE_BUTTON_HIDDEN,
            this.number_of_pages() - 1,
        );
        this.on_button_visibility_changed(
            this.last_button_visibility(),
            LAST_PAGE_BUTTON_VISIBLE,
            LAST_PAGE_BUTTON_COLLAPSED,
            LAST_PAGE_BUTTON_HIDDEN,
            this.number_of_pages() - 1,
        );

        this.on_selected_page_index_change(-1);
    }
}

impl InputElementImpl for PagerControl {
    // OnRootGridKeyDown: the left and right keys move the focus in their direction, from the
    // element that has it inside the pager (the key down of the root grid of the template bubbles
    // to the control).
    fn on_key_down(this: &Self, args: &KeyEventArgs) {
        Self::parent_on_key_down(this, args);
        let direction = match args.key {
            Key::Left => NavigationDirection::Left,
            Key::Right => NavigationDirection::Right,
            _ => return,
        };
        if let Some(manager) = FocusManager::get_focus_manager(this) {
            manager.try_move_focus(direction, None);
        }
    }
}

#[derive(Clone, Copy)]
enum NavigationButton {
    First,
    Previous,
    Next,
    Last,
}

ferroui_base::ferro_properties! { impl PagerControl {
    ferro_property!(
        /// Defines the `DisplayMode` property: how the pages are shown. `Auto` by default.
        pub fn display_mode_property() -> StyledProperty<PagerControlDisplayMode> {
            FerroProperty::register::<PagerControl, _>("DisplayMode", PagerControlDisplayMode::Auto)
        }
    );

    ferro_property!(
        /// Defines the `NumberOfPages` property: the number of pages, -1 while it is not known. 0 by
        /// default.
        pub fn number_of_pages_property() -> StyledProperty<i32> {
            FerroProperty::register::<PagerControl, _>("NumberOfPages", 0)
        }
    );

    ferro_property!(
        /// Defines the `SelectedPageIndex` property: the index of the selected page, from 0.
        pub fn selected_page_index_property() -> StyledProperty<i32> {
            FerroProperty::register::<PagerControl, _>("SelectedPageIndex", 0)
        }
    );

    ferro_property!(
        /// Defines the `FirstButtonVisibility` property: when the button to the first page is
        /// shown.
        pub fn first_button_visibility_property() -> StyledProperty<PagerControlButtonVisibility> {
            FerroProperty::register::<PagerControl, _>("FirstButtonVisibility", PagerControlButtonVisibility::Visible)
        }
    );

    ferro_property!(
        /// Defines the `PreviousButtonVisibility` property: when the button to the previous page is
        /// shown.
        pub fn previous_button_visibility_property() -> StyledProperty<PagerControlButtonVisibility> {
            FerroProperty::register::<PagerControl, _>("PreviousButtonVisibility", PagerControlButtonVisibility::Visible)
        }
    );

    ferro_property!(
        /// Defines the `NextButtonVisibility` property: when the button to the next page is shown.
        pub fn next_button_visibility_property() -> StyledProperty<PagerControlButtonVisibility> {
            FerroProperty::register::<PagerControl, _>("NextButtonVisibility", PagerControlButtonVisibility::Visible)
        }
    );

    ferro_property!(
        /// Defines the `LastButtonVisibility` property: when the button to the last page is shown.
        pub fn last_button_visibility_property() -> StyledProperty<PagerControlButtonVisibility> {
            FerroProperty::register::<PagerControl, _>("LastButtonVisibility", PagerControlButtonVisibility::Visible)
        }
    );

    ferro_property!(
        /// Defines the `FirstButtonCommand` property: the command the button to the first page
        /// executes after it moves to the page.
        pub fn first_button_command_property() -> StyledProperty<Option<Rc<dyn ICommand>>> {
            FerroProperty::register::<PagerControl, _>("FirstButtonCommand", None)
        }
    );

    ferro_property!(
        /// Defines the `PreviousButtonCommand` property: the command the button to the previous
        /// page executes after it moves to the page.
        pub fn previous_button_command_property() -> StyledProperty<Option<Rc<dyn ICommand>>> {
            FerroProperty::register::<PagerControl, _>("PreviousButtonCommand", None)
        }
    );

    ferro_property!(
        /// Defines the `NextButtonCommand` property: the command the button to the next page
        /// executes after it moves to the page.
        pub fn next_button_command_property() -> StyledProperty<Option<Rc<dyn ICommand>>> {
            FerroProperty::register::<PagerControl, _>("NextButtonCommand", None)
        }
    );

    ferro_property!(
        /// Defines the `LastButtonCommand` property: the command the button to the last page
        /// executes after it moves to the page.
        pub fn last_button_command_property() -> StyledProperty<Option<Rc<dyn ICommand>>> {
            FerroProperty::register::<PagerControl, _>("LastButtonCommand", None)
        }
    );

    ferro_property!(
        /// Defines the `PagerInputCommand` property, which WinUI declares and never executes.
        pub fn pager_input_command_property() -> StyledProperty<Option<Rc<dyn ICommand>>> {
            FerroProperty::register::<PagerControl, _>("PagerInputCommand", None)
        }
    );

    ferro_property!(
        /// Defines the `FirstButtonStyle` property: the control theme of the button to the first
        /// page.
        pub fn first_button_style_property() -> StyledProperty<Option<Ref<ControlTheme>>> {
            FerroProperty::register::<PagerControl, _>("FirstButtonStyle", None)
        }
    );

    ferro_property!(
        /// Defines the `PreviousButtonStyle` property: the control theme of the button to the
        /// previous page.
        pub fn previous_button_style_property() -> StyledProperty<Option<Ref<ControlTheme>>> {
            FerroProperty::register::<PagerControl, _>("PreviousButtonStyle", None)
        }
    );

    ferro_property!(
        /// Defines the `NextButtonStyle` property: the control theme of the button to the next
        /// page.
        pub fn next_button_style_property() -> StyledProperty<Option<Ref<ControlTheme>>> {
            FerroProperty::register::<PagerControl, _>("NextButtonStyle", None)
        }
    );

    ferro_property!(
        /// Defines the `LastButtonStyle` property: the control theme of the button to the last
        /// page.
        pub fn last_button_style_property() -> StyledProperty<Option<Ref<ControlTheme>>> {
            FerroProperty::register::<PagerControl, _>("LastButtonStyle", None)
        }
    );

    ferro_property!(
        /// Defines the `ButtonPanelAlwaysShowFirstLastPageIndex` property: whether the button panel
        /// always shows the buttons of the first and the last page. `true` by default.
        pub fn button_panel_always_show_first_last_page_index_property() -> StyledProperty<bool> {
            FerroProperty::register::<PagerControl, _>("ButtonPanelAlwaysShowFirstLastPageIndex", true)
        }
    );

    ferro_property!(
        /// Defines the `PrefixText` property: the text before the combo box or the number box
        /// ("Page" when the template is applied without one).
        pub fn prefix_text_property() -> StyledProperty<String> {
            FerroProperty::register::<PagerControl, _>("PrefixText", String::new())
        }
    );

    ferro_property!(
        /// Defines the `SuffixText` property: the text after the combo box or the number box, before
        /// the number of pages ("of" when the template is applied without one).
        pub fn suffix_text_property() -> StyledProperty<String> {
            FerroProperty::register::<PagerControl, _>("SuffixText", String::new())
        }
    );

    ferro_property!(
        /// Defines the `TemplateSettings` property: the page numbers and the number panel items the
        /// template binds to.
        pub fn template_settings_property() -> StyledProperty<Option<Ref<PagerControlTemplateSettings>>> {
            FerroProperty::register::<PagerControl, _>("TemplateSettings", None)
        }
    );
} }

impl PagerControl {
    fn static_constructor() {
        // The lists of the template settings are declared typed lists: the bindings of the
        // template deliver them to the items sources of the combo box and the number panel.
        crate::markup_types::lists::PipsList::register();
        crate::markup_types::lists::ControlList::register();
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            parts: RefCell::new(Parts::default()),
            template_subscriptions: RefCell::new(Vec::new()),
            template_applied: Cell::new(false),
            last_selected_page_index: Cell::new(-1),
            last_number_of_pages_count: Cell::new(0),
            selected_index_changed: HandlerList::new(),
        }
    }

    /// Creates a pager without pages.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.set_value(Self::template_settings_property(), Some(PagerControlTemplateSettings::new()));
        this
    }

    /// Gets how the pages are shown.
    pub fn display_mode(&self) -> PagerControlDisplayMode {
        self.get_value(Self::display_mode_property())
    }

    /// Sets how the pages are shown.
    pub fn set_display_mode(&self, value: PagerControlDisplayMode) {
        self.set_value(Self::display_mode_property(), value);
    }

    /// Gets the number of pages, -1 while it is not known.
    pub fn number_of_pages(&self) -> i32 {
        self.get_value(Self::number_of_pages_property())
    }

    /// Sets the number of pages, -1 while it is not known.
    pub fn set_number_of_pages(&self, value: i32) {
        self.set_value(Self::number_of_pages_property(), value);
    }

    /// Gets the index of the selected page.
    pub fn selected_page_index(&self) -> i32 {
        self.get_value(Self::selected_page_index_property())
    }

    /// Sets the index of the selected page; raises `SelectedIndexChanged`.
    pub fn set_selected_page_index(&self, value: i32) {
        self.set_value(Self::selected_page_index_property(), value);
    }

    /// Gets when the button to the first page is shown.
    pub fn first_button_visibility(&self) -> PagerControlButtonVisibility {
        self.get_value(Self::first_button_visibility_property())
    }

    /// Sets when the button to the first page is shown.
    pub fn set_first_button_visibility(&self, value: PagerControlButtonVisibility) {
        self.set_value(Self::first_button_visibility_property(), value);
    }

    /// Gets when the button to the previous page is shown.
    pub fn previous_button_visibility(&self) -> PagerControlButtonVisibility {
        self.get_value(Self::previous_button_visibility_property())
    }

    /// Sets when the button to the previous page is shown.
    pub fn set_previous_button_visibility(&self, value: PagerControlButtonVisibility) {
        self.set_value(Self::previous_button_visibility_property(), value);
    }

    /// Gets when the button to the next page is shown.
    pub fn next_button_visibility(&self) -> PagerControlButtonVisibility {
        self.get_value(Self::next_button_visibility_property())
    }

    /// Sets when the button to the next page is shown.
    pub fn set_next_button_visibility(&self, value: PagerControlButtonVisibility) {
        self.set_value(Self::next_button_visibility_property(), value);
    }

    /// Gets when the button to the last page is shown.
    pub fn last_button_visibility(&self) -> PagerControlButtonVisibility {
        self.get_value(Self::last_button_visibility_property())
    }

    /// Sets when the button to the last page is shown.
    pub fn set_last_button_visibility(&self, value: PagerControlButtonVisibility) {
        self.set_value(Self::last_button_visibility_property(), value);
    }

    /// Gets the command of the button to the first page.
    pub fn first_button_command(&self) -> Option<Rc<dyn ICommand>> {
        self.get_value(Self::first_button_command_property())
    }

    /// Sets the command of the button to the first page.
    pub fn set_first_button_command(&self, value: Option<Rc<dyn ICommand>>) {
        self.set_value(Self::first_button_command_property(), value);
    }

    /// Gets the command of the button to the previous page.
    pub fn previous_button_command(&self) -> Option<Rc<dyn ICommand>> {
        self.get_value(Self::previous_button_command_property())
    }

    /// Sets the command of the button to the previous page.
    pub fn set_previous_button_command(&self, value: Option<Rc<dyn ICommand>>) {
        self.set_value(Self::previous_button_command_property(), value);
    }

    /// Gets the command of the button to the next page.
    pub fn next_button_command(&self) -> Option<Rc<dyn ICommand>> {
        self.get_value(Self::next_button_command_property())
    }

    /// Sets the command of the button to the next page.
    pub fn set_next_button_command(&self, value: Option<Rc<dyn ICommand>>) {
        self.set_value(Self::next_button_command_property(), value);
    }

    /// Gets the command of the button to the last page.
    pub fn last_button_command(&self) -> Option<Rc<dyn ICommand>> {
        self.get_value(Self::last_button_command_property())
    }

    /// Sets the command of the button to the last page.
    pub fn set_last_button_command(&self, value: Option<Rc<dyn ICommand>>) {
        self.set_value(Self::last_button_command_property(), value);
    }

    /// Gets the command WinUI declares as `PagerInputCommand` (never executed).
    pub fn pager_input_command(&self) -> Option<Rc<dyn ICommand>> {
        self.get_value(Self::pager_input_command_property())
    }

    /// Sets the command WinUI declares as `PagerInputCommand` (never executed).
    pub fn set_pager_input_command(&self, value: Option<Rc<dyn ICommand>>) {
        self.set_value(Self::pager_input_command_property(), value);
    }

    /// Gets the control theme of the button to the first page.
    pub fn first_button_style(&self) -> Option<Ref<ControlTheme>> {
        self.get_value(Self::first_button_style_property())
    }

    /// Sets the control theme of the button to the first page.
    pub fn set_first_button_style(&self, value: Option<Ref<ControlTheme>>) {
        self.set_value(Self::first_button_style_property(), value);
    }

    /// Gets the control theme of the button to the previous page.
    pub fn previous_button_style(&self) -> Option<Ref<ControlTheme>> {
        self.get_value(Self::previous_button_style_property())
    }

    /// Sets the control theme of the button to the previous page.
    pub fn set_previous_button_style(&self, value: Option<Ref<ControlTheme>>) {
        self.set_value(Self::previous_button_style_property(), value);
    }

    /// Gets the control theme of the button to the next page.
    pub fn next_button_style(&self) -> Option<Ref<ControlTheme>> {
        self.get_value(Self::next_button_style_property())
    }

    /// Sets the control theme of the button to the next page.
    pub fn set_next_button_style(&self, value: Option<Ref<ControlTheme>>) {
        self.set_value(Self::next_button_style_property(), value);
    }

    /// Gets the control theme of the button to the last page.
    pub fn last_button_style(&self) -> Option<Ref<ControlTheme>> {
        self.get_value(Self::last_button_style_property())
    }

    /// Sets the control theme of the button to the last page.
    pub fn set_last_button_style(&self, value: Option<Ref<ControlTheme>>) {
        self.set_value(Self::last_button_style_property(), value);
    }

    /// Gets whether the button panel always shows the buttons of the first and the last page.
    pub fn button_panel_always_show_first_last_page_index(&self) -> bool {
        self.get_value(Self::button_panel_always_show_first_last_page_index_property())
    }

    /// Sets whether the button panel always shows the buttons of the first and the last page.
    pub fn set_button_panel_always_show_first_last_page_index(&self, value: bool) {
        self.set_value(Self::button_panel_always_show_first_last_page_index_property(), value);
    }

    /// Gets the text before the combo box or the number box.
    pub fn prefix_text(&self) -> String {
        self.get_value(Self::prefix_text_property())
    }

    /// Sets the text before the combo box or the number box.
    pub fn set_prefix_text(&self, value: impl Into<String>) {
        self.set_value(Self::prefix_text_property(), value.into());
    }

    /// Gets the text after the combo box or the number box.
    pub fn suffix_text(&self) -> String {
        self.get_value(Self::suffix_text_property())
    }

    /// Sets the text after the combo box or the number box.
    pub fn set_suffix_text(&self, value: impl Into<String>) {
        self.set_value(Self::suffix_text_property(), value.into());
    }

    /// Gets the page numbers and the number panel items the template binds to.
    pub fn template_settings(&self) -> Ref<PagerControlTemplateSettings> {
        self.get_value(Self::template_settings_property()).expect("The pager creates its template settings.")
    }

    /// Adds a handler of the `SelectedIndexChanged` event, raised for every change of
    /// `SelectedPageIndex` and when the template is applied.
    ///
    /// Dropping the returned guard removes the handler; [`Subscription::detach`] keeps it
    /// for the life of the control.
    pub fn selected_index_changed(
        &self,
        handler: impl Fn(&PagerControl, &PagerControlSelectedIndexChangedEventArgs) + 'static,
    ) -> Subscription {
        let token = self.selected_index_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Subscription::new(Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.selected_index_changed.remove(token);
            }
        }))
    }

    fn go_to_state(&self, state: State) {
        let pseudo_classes = self.pseudo_classes();
        for name in state.group {
            pseudo_classes.set(name, state.pseudo_class == Some(*name));
        }
    }

    fn on_display_mode_changed(&self) {
        match self.display_mode() {
            PagerControlDisplayMode::ButtonPanel => self.go_to_state(NUMBER_PANEL_VISIBLE),
            PagerControlDisplayMode::ComboBox => self.go_to_state(COMBO_BOX_VISIBLE),
            PagerControlDisplayMode::NumberBox => self.go_to_state(NUMBER_BOX_VISIBLE),
            PagerControlDisplayMode::Auto => self.update_display_mode_auto_state(),
        }
    }

    fn update_display_mode_auto_state(&self) {
        let number_of_pages = self.number_of_pages();
        if number_of_pages > -1 && number_of_pages < AUTO_DISPLAY_MODE_NUMBER_OF_PAGES_THRESHOLD {
            self.go_to_state(COMBO_BOX_VISIBLE);
        } else {
            self.go_to_state(NUMBER_BOX_VISIBLE);
        }
    }

    fn on_number_of_pages_changed(&self, old_value: i32) {
        self.last_number_of_pages_count.set(old_value);
        let number_of_pages = self.number_of_pages();
        if number_of_pages < self.selected_page_index() && number_of_pages > -1 {
            self.set_selected_page_index(number_of_pages - 1);
        }
        self.update_template_setting_element_lists();
        if self.display_mode() == PagerControlDisplayMode::Auto {
            self.update_display_mode_auto_state();
        }
        let number_box = self.parts.borrow().number_box.clone();
        if number_of_pages > -1 {
            self.go_to_state(FINITE_ITEMS);
            if let Some(number_box) = number_box {
                number_box.set_maximum(number_of_pages as f64);
            }
        } else {
            self.go_to_state(INFINITE_ITEMS);
            if let Some(number_box) = number_box {
                number_box.set_maximum(f64::INFINITY);
            }
        }
        self.update_on_edge_button_visual_states();
    }

    fn on_selected_page_index_change(&self, old_value: i32) {
        // "Ensure that SelectedPageIndex will end up in the valid range of values. Special case
        // is NumberOfPages being 0, in that case, don't verify upperbound restrictions."
        let number_of_pages = self.number_of_pages();
        if self.selected_page_index() > number_of_pages - 1 && number_of_pages > 0 {
            self.set_selected_page_index(number_of_pages - 1);
        } else if self.selected_page_index() < 0 {
            self.set_selected_page_index(0);
        }
        self.last_selected_page_index.set(old_value);

        let (combo_box, number_box) = {
            let parts = self.parts.borrow();
            (parts.combo_box.clone(), parts.number_box.clone())
        };
        if let Some(combo_box) = combo_box {
            if (self.selected_page_index() as usize) < self.template_settings().pages().count() {
                combo_box.set_selected_index(self.selected_page_index());
            }
        }
        if let Some(number_box) = number_box {
            number_box.set_numeric_value(self.selected_page_index() as f64 + 1.0);
        }

        self.update_on_edge_button_visual_states();
        self.update_template_setting_element_lists();

        if self.display_mode() == PagerControlDisplayMode::ButtonPanel {
            // "NumberPanel needs to update based on multiple parameters. SelectedPageIndex is one
            // of them, so let's do that now!"
            self.update_number_panel(self.number_of_pages());
        }

        // Notify UIA that the selection has changed.
        if let Some(peer) = ControlAutomationPeer::from_element(self).and_then(|peer| peer.cast::<PagerControlAutomationPeer>()) {
            peer.raise_selection_changed();
        }
        self.raise_selected_index_changed();
    }

    fn raise_selected_index_changed(&self) {
        let args = PagerControlSelectedIndexChangedEventArgs {
            previous_page_index: self.last_selected_page_index.get(),
            new_page_index: self.selected_page_index(),
        };
        for (_, handler) in self.selected_index_changed.snapshot().iter() {
            handler(self, &args);
        }
    }

    fn on_button_visibility_changed(
        &self,
        visibility: PagerControlButtonVisibility,
        visible_state: State,
        collapsed_state: State,
        hidden_state: State,
        hidden_on_edge_page_criteria: i32,
    ) {
        match visibility {
            PagerControlButtonVisibility::Visible => self.go_to_state(visible_state),
            PagerControlButtonVisibility::Hidden => self.go_to_state(collapsed_state),
            PagerControlButtonVisibility::HiddenOnEdge => {
                if self.selected_page_index() != hidden_on_edge_page_criteria {
                    self.go_to_state(visible_state);
                } else {
                    self.go_to_state(hidden_state);
                }
            }
        }
    }

    fn update_template_setting_element_lists(&self) {
        let number_of_pages = self.number_of_pages();
        match self.display_mode() {
            PagerControlDisplayMode::ComboBox | PagerControlDisplayMode::Auto => {
                if number_of_pages > -1 {
                    self.fill_combo_box_collection_to_size(number_of_pages);
                } else if (self.template_settings().pages().count() as i32) < INFINITE_MODE_COMBO_BOX_ITEMS_INCREMENT {
                    self.fill_combo_box_collection_to_size(INFINITE_MODE_COMBO_BOX_ITEMS_INCREMENT);
                }
            }
            PagerControlDisplayMode::ButtonPanel => self.update_number_panel(number_of_pages),
            PagerControlDisplayMode::NumberBox => {}
        }
    }

    fn fill_combo_box_collection_to_size(&self, number_of_pages: i32) {
        let pages = self.template_settings().pages();
        let current = pages.count() as i32;
        if current <= number_of_pages {
            // "We are increasing the number of pages, so add the missing numbers."
            for page in current..number_of_pages {
                pages.add(page + 1);
            }
        } else {
            // "We are decreasing the number of pages, so remove numbers starting at the end."
            for _ in number_of_pages.max(0)..current {
                pages.remove_at(pages.count() - 1);
            }
        }
    }

    fn update_on_edge_button_visual_states(&self) {
        let selected_page_index = self.selected_page_index();
        let number_of_pages = self.number_of_pages();

        // Handle disabled/enabled status of buttons
        if selected_page_index == 0 {
            self.go_to_state(FIRST_PAGE_BUTTON_DISABLED);
            self.go_to_state(PREVIOUS_PAGE_BUTTON_DISABLED);
            self.go_to_state(NEXT_PAGE_BUTTON_ENABLED);
            self.go_to_state(LAST_PAGE_BUTTON_ENABLED);
        } else if selected_page_index >= number_of_pages - 1 {
            self.go_to_state(FIRST_PAGE_BUTTON_ENABLED);
            self.go_to_state(PREVIOUS_PAGE_BUTTON_ENABLED);
            if number_of_pages > -1 {
                self.go_to_state(NEXT_PAGE_BUTTON_DISABLED);
            } else {
                self.go_to_state(NEXT_PAGE_BUTTON_ENABLED);
            }
            self.go_to_state(LAST_PAGE_BUTTON_DISABLED);
        } else {
            self.go_to_state(FIRST_PAGE_BUTTON_ENABLED);
            self.go_to_state(PREVIOUS_PAGE_BUTTON_ENABLED);
            self.go_to_state(NEXT_PAGE_BUTTON_ENABLED);
            self.go_to_state(LAST_PAGE_BUTTON_ENABLED);
        }

        // Handle HiddenOnEdge states
        if self.first_button_visibility() == PagerControlButtonVisibility::HiddenOnEdge {
            self.go_to_state(if selected_page_index != 0 { FIRST_PAGE_BUTTON_VISIBLE } else { FIRST_PAGE_BUTTON_HIDDEN });
        }
        if self.previous_button_visibility() == PagerControlButtonVisibility::HiddenOnEdge {
            self.go_to_state(if selected_page_index != 0 { PREVIOUS_PAGE_BUTTON_VISIBLE } else { PREVIOUS_PAGE_BUTTON_HIDDEN });
        }
        if self.next_button_visibility() == PagerControlButtonVisibility::HiddenOnEdge {
            self.go_to_state(if selected_page_index != number_of_pages - 1 {
                NEXT_PAGE_BUTTON_VISIBLE
            } else {
                NEXT_PAGE_BUTTON_HIDDEN
            });
        }
        if self.last_button_visibility() == PagerControlButtonVisibility::HiddenOnEdge {
            self.go_to_state(if selected_page_index != number_of_pages - 1 {
                LAST_PAGE_BUTTON_VISIBLE
            } else {
                LAST_PAGE_BUTTON_HIDDEN
            });
        }
    }

    // --- the number panel ---

    fn update_number_panel(&self, number_of_pages: i32) {
        if number_of_pages < 0 {
            self.update_number_of_panel_collection_infinite_items();
        } else if number_of_pages < 8 {
            self.update_number_panel_collection_all_items(number_of_pages);
        } else {
            let selected_index = self.selected_page_index();
            // "Choose correct template based on SelectedPageIndex (x) and NumberOfPages (n)"
            if selected_index < 4 {
                // 1 2 3 4 5 ... n
                self.update_number_panel_collection_start_with_ellipsis(number_of_pages, selected_index);
            } else if selected_index >= number_of_pages - 4 {
                // 1 ... n-4 n-3 n-2 n-1 n
                self.update_number_panel_collection_end_with_ellipsis(number_of_pages, selected_index);
            } else {
                // 1 ... x-2 x-1 x x+1 x+2 ... n
                self.update_number_panel_collection_center_with_ellipsis(number_of_pages, selected_index);
            }
        }
    }

    fn update_number_of_panel_collection_infinite_items(&self) {
        let selected_index = self.selected_page_index();
        self.template_settings().number_panel_items().clear();
        if selected_index < 3 {
            for page in 1..=5 {
                self.append_button_to_number_panel_list(page, 0);
            }
            self.move_identifier_to_element(selected_index);
        } else {
            self.append_button_to_number_panel_list(1, 0);
            self.append_ellipsis_icon_to_number_panel_list();
            self.append_button_to_number_panel_list(selected_index, 0);
            self.append_button_to_number_panel_list(selected_index + 1, 0);
            self.append_button_to_number_panel_list(selected_index + 2, 0);
            self.move_identifier_to_element(3);
        }
    }

    fn update_number_panel_collection_all_items(&self, number_of_pages: i32) {
        // "Check that the NumberOfPages did not change, so we can skip recreating collection"
        if self.last_number_of_pages_count.get() != number_of_pages {
            self.template_settings().number_panel_items().clear();
            for index in 0..number_of_pages.min(7) {
                self.append_button_to_number_panel_list(index + 1, number_of_pages);
            }
        }
        self.move_identifier_to_element(self.selected_page_index());
    }

    fn update_number_panel_collection_start_with_ellipsis(&self, number_of_pages: i32, selected_index: i32) {
        if self.last_number_of_pages_count.get() != number_of_pages {
            self.template_settings().number_panel_items().clear();
            for page in 1..=5 {
                self.append_button_to_number_panel_list(page, number_of_pages);
            }
            if self.button_panel_always_show_first_last_page_index() {
                self.append_ellipsis_icon_to_number_panel_list();
                self.append_button_to_number_panel_list(number_of_pages, number_of_pages);
            }
        }
        // "SelectedIndex is definitely the correct index here as we are counting from start"
        self.move_identifier_to_element(selected_index);
    }

    fn update_number_panel_collection_end_with_ellipsis(&self, number_of_pages: i32, selected_index: i32) {
        let show_first_last_page_index = self.button_panel_always_show_first_last_page_index();
        if self.last_number_of_pages_count.get() != number_of_pages {
            self.template_settings().number_panel_items().clear();
            if show_first_last_page_index {
                self.append_button_to_number_panel_list(1, number_of_pages);
                self.append_ellipsis_icon_to_number_panel_list();
            }
            for page in (number_of_pages - 4)..=number_of_pages {
                self.append_button_to_number_panel_list(page, number_of_pages);
            }
        }
        // "We can only be either the last, the second from last or third from last"
        if show_first_last_page_index {
            self.move_identifier_to_element(selected_index - number_of_pages + 7);
        } else {
            self.move_identifier_to_element(selected_index - number_of_pages + 5);
        }
    }

    fn update_number_panel_collection_center_with_ellipsis(&self, number_of_pages: i32, selected_index: i32) {
        let show_first_last_page_index = self.button_panel_always_show_first_last_page_index();
        if self.last_number_of_pages_count.get() != number_of_pages {
            self.template_settings().number_panel_items().clear();
            if show_first_last_page_index {
                self.append_button_to_number_panel_list(1, number_of_pages);
                self.append_ellipsis_icon_to_number_panel_list();
            }
            self.append_button_to_number_panel_list(selected_index, number_of_pages);
            self.append_button_to_number_panel_list(selected_index + 1, number_of_pages);
            self.append_button_to_number_panel_list(selected_index + 2, number_of_pages);
            if show_first_last_page_index {
                self.append_ellipsis_icon_to_number_panel_list();
                self.append_button_to_number_panel_list(number_of_pages, number_of_pages);
            }
        }
        // "selectedIndex + 1 is our representation for SelectedIndex."
        self.move_identifier_to_element(if show_first_last_page_index { 3 } else { 1 });
    }

    /// Moves the indicator of the current page under the element at `index` of the number panel
    /// (WinUI's `MoveIdentifierToElement`): its left margin is the offset of the element in the
    /// panel, its width the width of the element.
    fn move_identifier_to_element(&self, index: i32) {
        let (indicator, panel) = {
            let parts = self.parts.borrow();
            (parts.selected_page_indicator.clone(), parts.number_panel.clone())
        };
        let (Some(indicator), Some(panel)) = (indicator, panel) else { return };
        panel.update_layout();
        let Some(element) = panel.container_from_index(index) else { return };
        let Some(offset) = element.translate_point(Point::new(0.0, 0.0), &panel) else { return };
        indicator.set_margin(Thickness::new(offset.x, 0.0, 0.0, 0.0));
        indicator.set_width(element.bounds().width);
    }

    fn append_button_to_number_panel_list(&self, page_number: i32, number_of_pages: i32) {
        let button = Button::new();
        button.set_content(Some(Rc::new(page_number) as BoxedValue));
        let weak = self.to_ref().downgrade();
        // The handler lives as long as the button, which the list of the template settings owns.
        let _ = button.click(move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.set_selected_page_index(page_number - 1);
            }
        });
        // Set the default style of buttons
        let key = ResourceKey::from(NUMBER_PANEL_BUTTON_STYLE_NAME);
        let theme = self
            .try_find_resource(&key, self.actual_theme_variant().as_ref())
            .flatten()
            .and_then(|value| ValueTypes::try_cast(&value, ValueType::of::<Option<Ref<ControlTheme>>>()))
            .and_then(|value| value.downcast_ref::<Option<Ref<ControlTheme>>>().cloned().flatten());
        if let Some(theme) = theme {
            button.set_theme(theme);
        }
        AutomationProperties::set_name(&button, Some(&format!("{PAGE_TEXT} {page_number}")));
        AutomationProperties::set_position_in_set(&button, page_number);
        AutomationProperties::set_size_of_set(&button, number_of_pages);
        self.template_settings().number_panel_items().add(button.upcast());
    }

    fn append_ellipsis_icon_to_number_panel_list(&self) {
        let ellipsis_icon = SymbolIcon::new();
        ellipsis_icon.set_symbol(Symbol::More);
        self.template_settings().number_panel_items().add(ellipsis_icon.upcast());
    }

    fn combo_box_selection_changed(&self) {
        let combo_box = self.parts.borrow().combo_box.clone();
        if let Some(combo_box) = combo_box {
            self.set_selected_page_index(combo_box.selected_index());
        }
    }

    fn navigation_button_clicked(&self, button: NavigationButton) {
        let (page, command) = match button {
            NavigationButton::First => (0, self.first_button_command()),
            // "In this method, SelectedPageIndex is always greater than 1."
            NavigationButton::Previous => (self.selected_page_index() - 1, self.previous_button_command()),
            // "In this method, SelectedPageIndex is always less than maximum."
            NavigationButton::Next => (self.selected_page_index() + 1, self.next_button_command()),
            NavigationButton::Last => (self.number_of_pages() - 1, self.last_button_command()),
        };
        self.set_selected_page_index(page);
        if let Some(command) = command {
            command.execute(None);
        }
    }
}
