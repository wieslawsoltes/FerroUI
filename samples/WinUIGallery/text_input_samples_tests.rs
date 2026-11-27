//! The TextBlock, TextBox, PasswordBox and NumberBox pages: the source's examples with their
//! x:Bind and the handlers of `NumberBoxPage.xaml.cs` (`text_input_samples.rs`). The handlers of
//! the PasswordBox page are tested in `tests.rs`.

use crate::text_input_samples::DollarAmountFormatter;
use crate::{examples, tests::start};
use ferroui_base::media::{Color, Colors, FontStyle, IBrush, TextWrapping};
use ferroui_base::Ref;
use ferroui_controls::{
    Control, INumberBoxNumberFormatter, NumberBox, NumberBoxSpinButtonPlacementMode, RadioButtons,
    RichTextBlock, TextBlock, TextBox, ToggleSwitch,
};
use std::rc::Rc;

fn page(id: &str) -> Ref<Control> {
    examples::create(examples::find(id).unwrap()).unwrap_or_else(|error| panic!("{id}: {error}"))
}

fn color(brush: Option<Rc<dyn IBrush>>) -> Option<Color> {
    brush.and_then(|brush| brush.as_solid_color_brush().map(|brush| brush.color()))
}

#[test]
fn text_block_examples_keep_the_source_style_properties_and_inlines() {
    let _application = start();
    let root = page("TextBlock");
    assert_eq!(
        root.get_control::<TextBlock>("SimpleTextBlock")
            .text()
            .as_deref(),
        Some("I am a TextBlock.")
    );
    // The keyed style of the source is a keyed ControlTheme.
    let styled = root.get_control::<TextBlock>("StyledTextBlock");
    styled.apply_styling();
    assert_eq!(styled.font_family().name(), "Comic Sans MS");
    assert_eq!(styled.font_style(), FontStyle::Italic);
    let customized = root.get_control::<TextBlock>("CustomizedTextBlock");
    assert_eq!(customized.letter_spacing(), 4.8);
    assert_eq!(customized.font_size(), 24.0);
    assert_eq!(customized.text_wrapping(), TextWrapping::WrapWithOverflow);
    assert_eq!(
        color(customized.foreground()),
        Some(Colors::CORNFLOWER_BLUE)
    );
    // A Run, a LineBreak and a Span of runs, bold, italic and underlined.
    let inlines = root
        .get_control::<TextBlock>("InlineTextBlock")
        .inlines()
        .unwrap();
    assert_eq!(inlines.count(), 3);
}

#[test]
fn selectable_text_follows_the_selection_switch() {
    let _application = start();
    let root = page("TextBlock");
    let text = root.get_control::<ferroui_controls::SelectableTextBlock>("SelectableTextBlock");
    // The source's OneWay x:Bind of the switch, off at first.
    assert!(!text.is_text_selection_enabled());
    assert_eq!(color(text.selection_brush()), Some(Colors::DARK_ORANGE));
    let switch = root.get_control::<ToggleSwitch>("SelectionEnabledSwitch");
    switch.set_is_checked(Some(true));
    assert!(text.is_text_selection_enabled());
    switch.set_is_checked(Some(false));
    assert!(!text.is_text_selection_enabled());
}

#[test]
fn text_box_examples_keep_the_source_properties() {
    let _application = start();
    let root = page("TextBox");
    let name = root.get_control::<TextBox>("NameTextBox");
    assert_eq!(name.placeholder_text().as_deref(), Some("Name"));
    let customized = root.get_control::<TextBox>("CustomizedTextBox");
    assert!(customized.is_read_only());
    assert_eq!(customized.letter_spacing(), 4.8);
    assert_eq!(
        customized.text().as_deref(),
        Some("I am super excited to be here!")
    );
    assert_eq!(
        color(customized.foreground()),
        Some(Color::from_rgb(0x51, 0x78, 0xBE))
    );
    let multi_line = root.get_control::<TextBox>("MultiLineTextBox");
    assert!(multi_line.accepts_return());
    assert_eq!(multi_line.text_wrapping(), TextWrapping::Wrap);
    assert_eq!(multi_line.min_width(), 400.0);
    assert_eq!(color(multi_line.selection_brush()), Some(Colors::GREEN));
}

#[test]
fn password_box_examples_keep_the_source_characters() {
    let _application = start();
    let root = page("PasswordBox");
    assert_eq!(
        root.get_control::<TextBox>("SimplePassword")
            .password_char(),
        '●'
    );
    let header = root.get_control::<TextBox>("HeaderPassword");
    assert_eq!(header.password_char(), '#');
    assert_eq!(
        header.placeholder_text().as_deref(),
        Some("Enter your password")
    );
}

#[test]
fn number_box_expression_keeps_nan_and_evaluates() {
    let _application = start();
    let root = page("NumberBox");
    let expression = root.get_control::<NumberBox>("Expression");
    assert!(expression.value().is_nan());
    // Setting the text commits it, as WinUI's UpdateValueToText.
    expression.set_text("1 + 2^2");
    assert_eq!(expression.value(), 5.0);
    assert_eq!(expression.text(), "5");
    expression.set_text("");
    assert!(expression.value().is_nan());
}

#[test]
fn spin_button_placement_group_places_the_spin_buttons() {
    let _application = start();
    let root = page("NumberBox");
    let number_box = root.get_control::<NumberBox>("NumberBoxSpinButtonPlacementExample");
    // As in the source, the markup placement holds until the selection changes.
    assert_eq!(
        number_box.spin_button_placement_mode(),
        NumberBoxSpinButtonPlacementMode::Compact
    );
    assert_eq!(
        (number_box.small_change(), number_box.large_change()),
        (10.0, 100.0)
    );
    let group = root.get_control::<RadioButtons>("SpinButtonPlacementGroup");
    assert_eq!(group.selected_index(), 0);
    group.set_selected_index(1);
    assert_eq!(
        number_box.spin_button_placement_mode(),
        NumberBoxSpinButtonPlacementMode::Compact
    );
    group.set_selected_index(0);
    assert_eq!(
        number_box.spin_button_placement_mode(),
        NumberBoxSpinButtonPlacementMode::Inline
    );
}

#[test]
fn formatted_number_box_rounds_to_the_nearest_quarter() {
    let _application = start();
    let root = page("NumberBox");
    let dollars = root.get_control::<NumberBox>("FormattedNumberBox");
    assert!(dollars.value().is_nan());
    // Setting the text commits it: the value keeps what was typed, the text is rounded to 0.25
    // with two fraction digits.
    dollars.set_text("1.1");
    assert_eq!(dollars.value(), 1.1);
    assert_eq!(dollars.text(), "1.00");
    dollars.set_text("2.125");
    assert_eq!(dollars.value(), 2.125);
    assert_eq!(dollars.text(), "2.25");
    // Text that is not a decimal number is replaced with the text of the value.
    dollars.set_text("1e3");
    assert_eq!(dollars.value(), 2.125);
    assert_eq!(dollars.text(), "2.25");
}

#[test]
fn dollar_amount_formatter_rounds_half_up_and_reads_decimals() {
    let formatter = DollarAmountFormatter;
    assert_eq!(formatter.format_double(0.0), "0.00");
    assert_eq!(formatter.format_double(0.6), "0.50");
    assert_eq!(formatter.format_double(0.625), "0.75");
    assert_eq!(formatter.format_double(-0.625), "-0.50");
    assert_eq!(formatter.format_double(-0.1), "0.00");
    assert_eq!(formatter.format_double(12.374), "12.25");
    assert_eq!(formatter.parse_double("3.5"), Some(3.5));
    assert_eq!(formatter.parse_double("-.5"), Some(-0.5));
    assert_eq!(formatter.parse_double("1e3"), None);
    assert_eq!(formatter.parse_double("NaN"), None);
    assert_eq!(formatter.parse_double("1.2.3"), None);
    assert_eq!(formatter.parse_double("-"), None);
}
