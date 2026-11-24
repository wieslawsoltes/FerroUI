//! The spin buttons of a `ButtonSpinner` (upstream bd835340fb): each spins in the direction of
//! its part, whatever element the click is raised on.

use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use crate::testing::{TestServices, UnitTestApplication};
use crate::{Button, ButtonSpinner, Control, SpinDirection, StackPanel};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::Ref;
use std::cell::RefCell;
use std::rc::Rc;

fn spinner_with_buttons() -> (Ref<ButtonSpinner>, Ref<Button>, Ref<Button>) {
    let spinner = ButtonSpinner::new();
    spinner.set_template(Some(FuncControlTemplate::for_type::<ButtonSpinner>(|_control, scope| {
        let panel = StackPanel::new();
        for name in ["PART_IncreaseButton", "PART_DecreaseButton"] {
            let button = Button::new();
            button.set_name(Some(name.to_string()));
            panel.children().add(button.register_in_name_scope(&**scope).upcast::<Control>());
        }
        panel.upcast()
    })));
    spinner.apply_template();
    let part = |name: &str| {
        spinner
            .get_template_descendants()
            .into_iter()
            .filter_map(|descendant| descendant.cast::<Button>())
            .find(|button| button.name().as_deref() == Some(name))
            .expect("the template has the part")
    };
    let (increase, decrease) = (part("PART_IncreaseButton"), part("PART_DecreaseButton"));
    (spinner, increase, decrease)
}

#[test]
fn each_spin_button_spins_in_the_direction_of_its_part() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (spinner, increase, decrease) = spinner_with_buttons();
    let spins = Rc::new(RefCell::new(Vec::new()));
    let recorded = spins.clone();
    spinner.spin(move |_, e| recorded.borrow_mut().push(e.direction()));

    increase.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    decrease.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    increase.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(*spins.borrow(), [SpinDirection::Increase, SpinDirection::Decrease, SpinDirection::Increase]);

    // A spinner that does not allow spinning ignores the clicks.
    spinner.set_allow_spin(false);
    decrease.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(spins.borrow().len(), 3);
}
