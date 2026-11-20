//! Compact native inspector controls shared by column and template authoring.
use crate::icons;
use ferroui_base::Ref;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::{Button, ComboBox, Control, StackPanel, TextBlock, ToolTip};
use std::rc::Rc;

pub(super) fn picker<'a>(
    label: &str,
    values: impl IntoIterator<Item = &'a str>,
    index: usize,
) -> Ref<ComboBox> {
    let control = ComboBox::new();
    control.classes().add("designer-field");
    control.set_horizontal_alignment(HorizontalAlignment::Stretch);
    for value in values {
        control.items().add(Some(Rc::new(value.to_owned())));
    }
    control.set_selected_index(index as i32);
    AutomationProperties::set_name(&control, Some(label));
    control
}

pub(super) fn text(value: &str) -> Ref<TextBlock> {
    let label = TextBlock::new();
    label.set_text(Some(value));
    label.set_vertical_alignment(VerticalAlignment::Center);
    label
}

pub(super) const SOURCE: &str =
    "F1 M1 2H15V14H1Z M2.5 4V12.5H13.5V4Z M5 7H11V8.4H5Z M5 10H9V11.4H5Z";
pub(super) const COPY: &str = "F1 M5 1H15V11H13.5V2.5H5Z M1 5H11V15H1Z M2.5 6.5V13.5H9.5V6.5Z";
pub(super) const RESET: &str =
    "F1 M1 2L2.2 3.2A6 6 0 1 1 4.2 13.8L5 12.4A4.4 4.4 0 1 0 3.4 4.4L5 6H1Z";

pub(super) fn icon_button(icon: &str, label: &str, action: impl Fn() + 'static) -> Ref<Button> {
    let button = Button::new();
    button.classes().add("nav");
    button.set_min_width(26.0);
    button.set_min_height(24.0);
    button.set_content(Some(Control::boxed(icons::vector_icon(icon, 13.0))));
    AutomationProperties::set_name(&button, Some(label));
    ToolTip::set_tip(&button, Some(Rc::new(label.to_owned())));
    button.click(move |_, _| action());
    button
}

pub(super) fn add_button(panel: &StackPanel, icon: &str, label: &str, action: impl Fn() + 'static) {
    panel
        .children()
        .add(icon_button(icon, label, action).upcast::<Control>());
}
