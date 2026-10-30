//! What the tests of the WinUI input controls share (`number_box_tests` and the others of
//! docs/porting/winui-controls.md, Input): the parts of the templates of the WinUI theme that the
//! controls rely on, and the services of tests with a window and keyboard focus.
//!
//! The controls have no template of their own (the WinUI theme has them, `Controls/NumberBox.xaml`
//! and the others of `src/FerroUI.Themes.WinUI`), so a test gives a control a small template with
//! the parts of that template the control looks up.

use crate::documents::TextElement;
use crate::presenters::{ContentPresenter, ItemsPresenter};
use crate::primitives::{HeaderedSelectingItemsControl, Popup, RadioButtonsPanel, TemplatedControl};
use crate::templates::{FuncControlTemplate, FuncTemplate, FuncTemplateNameScopeExtensions, ITemplateOf};
use crate::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use crate::{Border, Button, Control, Grid, ItemsControl, ListBox, Panel, RepeatButton, StackPanel, TextBlock, TextBox};
use ferroui_base::data::TemplateBinding;
use ferroui_base::layout::Orientation;
use ferroui_base::media::Brushes;
use ferroui_base::{ObjectType, Ref, StyledElement};
use std::rc::Rc;

/// An application with a styled window and real keyboard focus.
pub(crate) fn start_with_focus() -> UnitTestApplicationScope {
    let mut services = TestServices::styled_window();
    let focus = TestServices::real_focus();
    services.keyboard_device = focus.keyboard_device;
    services.keyboard_navigation = focus.keyboard_navigation;
    services.input_manager = focus.input_manager;
    UnitTestApplication::start(services)
}

/// The element named `name` of the template of `control`.
pub(crate) fn part<T: ObjectType>(control: &TemplatedControl, name: &str) -> Ref<T> {
    control
        .get_template_descendants()
        .into_iter()
        .find(|element| element.name().as_deref() == Some(name))
        .and_then(|element| element.cast::<T>())
        .unwrap_or_else(|| panic!("the template part {name}"))
}

fn named<T: ObjectType>(element: Ref<T>, name: &str) -> Ref<T>
where
    T: ferroui_base::Upcast<StyledElement>,
{
    let styled: &StyledElement = (*element).upcast();
    styled.set_name(Some(name.to_string()));
    element
}

/// The parts of `Controls/NumberBox.xaml`: `HeaderContentPresenter`, the text box `InputBox`, the
/// spin buttons `UpSpinButton` and `DownSpinButton`, and the popup `UpDownPopup` with
/// `PopupUpSpinButton` and `PopupDownSpinButton`.
pub(crate) fn number_box_template() -> Rc<FuncControlTemplate> {
    FuncControlTemplate::new(|_, scope| {
        let root = StackPanel::new();
        let header = named(ContentPresenter::new(), "HeaderContentPresenter").register_in_name_scope(&**scope);
        header.set_is_visible(false);
        root.children().add(&header);
        let row = StackPanel::new();
        row.set_orientation(Orientation::Horizontal);
        let text_box = named(TextBox::new(), "InputBox").register_in_name_scope(&**scope);
        row.children().add(&text_box);
        for name in ["UpSpinButton", "DownSpinButton"] {
            let button = named(RepeatButton::new(), name).register_in_name_scope(&**scope);
            button.set_focusable(false);
            row.children().add(&button);
        }
        let buttons = StackPanel::new();
        for name in ["PopupUpSpinButton", "PopupDownSpinButton"] {
            let button = named(RepeatButton::new(), name).register_in_name_scope(&**scope);
            button.set_focusable(false);
            buttons.children().add(&button);
        }
        let popup = named(Popup::new(), "UpDownPopup").register_in_name_scope(&**scope);
        popup.set_child(buttons);
        popup.set_placement_target(text_box);
        row.children().add(&popup);
        root.children().add(&row);
        root.upcast::<Control>()
    })
}

/// The parts of `Controls/AutoSuggestBox.xaml`: the text box `TextBox`, the button `QueryButton`,
/// and the popup `SuggestionsPopup` with `SuggestionsContainer` and the list `SuggestionsList`.
pub(crate) fn auto_suggest_box_template() -> Rc<FuncControlTemplate> {
    FuncControlTemplate::new(|_, scope| {
        let root = StackPanel::new();
        let text_box = named(TextBox::new(), "TextBox").register_in_name_scope(&**scope);
        root.children().add(&text_box);
        let query = named(Button::new(), "QueryButton").register_in_name_scope(&**scope);
        query.set_focusable(false);
        root.children().add(&query);
        let list = named(ListBox::new(), "SuggestionsList").register_in_name_scope(&**scope);
        list.set_focusable(false);
        let container = named(Border::new(), "SuggestionsContainer").register_in_name_scope(&**scope);
        container.set_child(Some(list.upcast::<Control>()));
        let popup = named(Popup::new(), "SuggestionsPopup").register_in_name_scope(&**scope);
        popup.set_child(container);
        popup.set_placement_target(text_box);
        popup.set_is_light_dismiss_enabled(true);
        root.children().add(&popup);
        root.upcast::<Control>()
    })
}

/// The parts of `Controls/RadioButtons.xaml`: the header presenter `PART_HeaderPresenter` and the
/// items presenter `PART_ItemsPresenter`, which shows the items panel of the group.
pub(crate) fn radio_buttons_template() -> Rc<FuncControlTemplate> {
    FuncControlTemplate::new(|_, scope| {
        let header = named(ContentPresenter::new(), "PART_HeaderPresenter").register_in_name_scope(&**scope);
        header.bind_binding(
            ContentPresenter::content_property().as_property(),
            &*TemplateBinding::new(HeaderedSelectingItemsControl::header_property().as_property()),
        );
        header.bind_binding(
            ContentPresenter::content_template_property().as_property(),
            &*TemplateBinding::new(HeaderedSelectingItemsControl::header_template_property().as_property()),
        );
        let presenter = named(ItemsPresenter::new(), "PART_ItemsPresenter").register_in_name_scope(&**scope);
        let items_panel = ItemsControl::items_panel_property().as_property();
        presenter.bind_binding(items_panel, &*TemplateBinding::new(items_panel));
        let root = StackPanel::new();
        root.children().add(&header);
        root.children().add(&presenter);
        root.upcast::<Control>()
    })
}

/// The items panel the WinUI theme gives a group: a radio buttons panel with the spacing of
/// `RadioButtonsColumnSpacing` (7) and `RadioButtonsRowSpacing` (8).
pub(crate) fn radio_buttons_panel() -> Rc<dyn ITemplateOf<Option<Ref<Panel>>>> {
    FuncTemplate::new(|| {
        let panel = RadioButtonsPanel::new();
        panel.set_column_spacing(7.0);
        panel.set_row_spacing(8.0);
        Some(panel.upcast::<Panel>())
    })
}

/// The parts of `Controls/RatingControl.xaml`: the stack panels `RatingBackgroundStackPanel` and
/// `RatingForegroundStackPanel`, in the font size `RatingControlFontSizeForRendering` (32) and
/// the spacing `RatingControlItemSpacing` (8), and a text block `Caption`.
pub(crate) fn rating_control_template() -> Rc<FuncControlTemplate> {
    FuncControlTemplate::new(|_, scope| {
        let stars = Grid::new();
        for name in ["RatingBackgroundStackPanel", "RatingForegroundStackPanel"] {
            let panel = named(StackPanel::new(), name).register_in_name_scope(&**scope);
            panel.set_orientation(Orientation::Horizontal);
            panel.set_spacing(8.0);
            TextElement::set_font_size_on(&panel, 32.0);
            if name == "RatingBackgroundStackPanel" {
                panel.set_background(Some(Brushes::transparent()));
            } else {
                panel.set_is_hit_test_visible(false);
            }
            stars.children().add(&panel);
        }
        let caption = named(TextBlock::new(), "Caption").register_in_name_scope(&**scope);
        let root = StackPanel::new();
        root.set_orientation(Orientation::Horizontal);
        root.children().add(&stars);
        root.children().add(&caption);
        root.upcast::<Control>()
    })
}
