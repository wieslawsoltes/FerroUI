//! What the tests of the WinUI input controls share (`number_box_tests` and the others of
//! docs/porting/winui-controls.md, Input): the parts of the templates of the WinUI theme that the
//! controls rely on, and the services of tests with a window and keyboard focus.
//!
//! The controls have no template of their own (the WinUI theme has them, `Controls/NumberBox.xaml`
//! and the others of `src/FerroUI.Themes.WinUI`), so a test gives a control a small template with
//! the parts of that template the control looks up.

use crate::presenters::ContentPresenter;
use crate::primitives::{Popup, TemplatedControl};
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use crate::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use crate::{Control, RepeatButton, StackPanel, TextBox};
use ferroui_base::layout::Orientation;
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
