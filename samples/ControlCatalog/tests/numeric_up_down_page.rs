//! Tests of the numeric up-down page.
//!
//! Not ports: the upstream sample has no tests.

use super::support::*;
use crate::pages::{NumbersPageViewModel, NumericUpDownPage};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::{CultureInfo, Decimal, TestCultureDataProvider};
use ferroui_base::Ref;
use ferroui_controls::{ComboBox, Control, NumericUpDown, Window};
use std::rc::Rc;

fn show(page: &Ref<NumericUpDownPage>) -> Ref<Window> {
    let window = Window::new();
    window.set_width(1100.0);
    window.set_height(800.0);
    window.set_content(Some(Control::boxed(page.clone())));
    window.show();
    run_jobs();
    window
}

fn run_jobs() {
    Dispatcher::ui_thread().run_jobs(None);
}

fn view_model(page: &Ref<NumericUpDownPage>) -> Rc<NumbersPageViewModel> {
    from_markup_value::<Rc<NumbersPageViewModel>>(&page.data_context()).expect("the view model of the page")
}

fn decimal(text: &str) -> Decimal {
    Decimal::parse(text).expect("a decimal")
}

#[test]
fn the_values_of_the_view_model_are_shown_in_the_selected_format() {
    let _app = start_catalog_application();
    let page = NumericUpDownPage::new();
    let window = show(&page);
    let view_model = view_model(&page);
    let up_down = page.get_control::<NumericUpDown>("upDown");

    // Without culture data the runtime has no specific culture: the list of the cultures is
    // empty and the numbers have the conventions of the current culture, the invariant one.
    assert_eq!(0, page.get_control::<ComboBox>("CultureSelector").item_count());
    assert!(up_down.number_format().is_some_and(|format| format == CultureInfo::current_culture().number_format()));

    // The first format, the currency.
    view_model.set_decimal_value(decimal("1.5"));
    run_jobs();
    assert_eq!(Some(decimal("1.5")), up_down.value());
    assert_eq!(Some("\u{00A4}1.50"), up_down.text().as_deref());

    let formats = view_model.formats().items().to_vec();
    view_model.set_selected_format(Some(formats[1].clone()));
    run_jobs();
    assert_eq!("F2", up_down.format_string());
    assert_eq!(Some("1.50"), up_down.text().as_deref());
    view_model.set_selected_format(Some(formats[5].clone()));
    run_jobs();
    assert_eq!(Some("1.50 \u{00B0}"), up_down.text().as_deref());

    // The value of the control goes back to the view model.
    up_down.set_numeric_value(Some(decimal("3")));
    run_jobs();
    assert_eq!(decimal("3"), view_model.decimal_value());

    window.close();
}

#[test]
fn the_selected_culture_gives_the_number_format() {
    let _app = start_catalog_application();
    // The cultures of the runtime: `en`, `en-GB` and `en-US`.
    TestCultureDataProvider::register();
    let page = NumericUpDownPage::new();
    let window = show(&page);
    let view_model = view_model(&page);
    let up_down = page.get_control::<NumericUpDown>("upDown");
    let selector = page.get_control::<ComboBox>("CultureSelector");

    // The specific cultures the page names.
    let cultures: Vec<String> =
        view_model.cultures().items().to_vec().iter().map(|culture| culture.name().to_string()).collect();
    assert_eq!(vec!["en-GB", "en-US"], cultures);
    assert_eq!(2, selector.item_count());

    view_model.set_decimal_value(decimal("1.5"));
    run_jobs();
    assert_eq!(Some("\u{00A4}1.50"), up_down.text().as_deref());

    selector.set_selected_index(1);
    run_jobs();
    let en_us = CultureInfo::get_culture_info("en-US").number_format();
    assert!(up_down.number_format().is_some_and(|format| format == en_us));
    assert_eq!(Some("$1.50"), up_down.text().as_deref());

    selector.set_selected_index(0);
    run_jobs();
    assert_eq!(Some("\u{00A3}1.50"), up_down.text().as_deref());

    // No culture: the current one.
    selector.set_selected_index(-1);
    run_jobs();
    assert_eq!(Some("\u{00A4}1.50"), up_down.text().as_deref());
    window.close();
}

#[test]
#[ignore = "gap C315: a floating-point value is not converted for a decimal property by a binding"]
fn gap_c315_double_bound_to_the_decimal_value() {
    let _app = start_catalog_application();
    let page = NumericUpDownPage::new();
    let window = show(&page);
    let view_model = view_model(&page);
    // `Value="{Binding DoubleValue}"`: upstream's binding converts the `double` of the view
    // model to the `decimal?` of the control.
    let double_up_down = page.get_control::<NumericUpDown>("DoubleUpDown");
    view_model.set_double_value(2.5);
    run_jobs();
    assert_eq!(Some(decimal("2.5")), double_up_down.value());
    window.close();
}
