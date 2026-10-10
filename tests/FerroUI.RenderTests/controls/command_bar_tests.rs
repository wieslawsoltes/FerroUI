//! Port of upstream's `Controls/CommandBarTests.cs`.
//!
//! `CommandBar_Default_PrimaryCommands` passes `gpuAllowedError: 0.03` upstream, which concerns upstream's
//! Mesa GL outputs; the port does not render those, so the argument is dropped.

use crate::test_base::{test_font_family, CompareOptions, TestBase};
use ferroui_base::media::{Brushes, Stretch, StreamGeometry};
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::shapes::Path;
use ferroui_controls::{
    CommandBar, CommandBarButton, CommandBarDefaultLabelPosition, CommandBarOverflowButtonVisibility,
    CommandBarSeparator, CommandBarToggleButton, Control, Decorator, TextBlock,
};
use ferroui_themes_simple::SimpleTheme;

const NEW_ICON: &str = "M19,13H13V19H11V13H5V11H11V5H13V11H19V13Z";
const SAVE_ICON: &str = "M15,9H5V5H15M12,19A3,3 0 0,1 9,16A3,3 0 0,1 12,13A3,3 0 0,1 15,16A3,3 0 0,1 12,19M17,3H5C3.89,3 3,3.9 3,5V19A2,2 0 0,0 5,21H19A2,2 0 0,0 21,19V7L17,3Z";
const BOLD_ICON: &str = "M15.6,10.79C17.04,10.07 18,8.64 18,7C18,4.79 16.21,3 14,3H7V21H14.73C16.78,21 18.5,19.37 18.5,17.32C18.5,15.82 17.72,14.53 16.5,13.77C16.2,13.59 15.9,13.44 15.6,13.32V10.79M10,6.5H13C13.83,6.5 14.5,7.17 14.5,8C14.5,8.83 13.83,9.5 13,9.5H10V6.5M13.5,17.5H10V14H13.5C14.33,14 15,14.67 15,15.5C15,16.33 14.33,17.5 13.5,17.5Z";

fn base() -> TestBase {
    TestBase::new(r"Controls\CommandBar")
}

fn font_style() -> Ref<Style> {
    Style::with_setters(
        Selectors::of_type::<TextBlock>(),
        [Setter::new(TextBlock::font_family_property(), test_font_family())],
    )
}

fn icon(data: &str) -> Option<BoxedValue> {
    let path = Path::new();
    path.set_data(StreamGeometry::parse(data).expect("the path data is valid"));
    path.set_fill(Some(Brushes::black()));
    path.set_width(20.0);
    path.set_height(20.0);
    path.set_stretch(Stretch::Uniform);
    Some(Control::boxed(path))
}

#[test]
fn command_bar_default_primary_commands() {
    let t = base();
    let target = Decorator::new();
    target.set_width(500.0);
    target.set_height(60.0);
    let bar = CommandBar::new();
    bar.set_background(Some(Brushes::light_gray()));
    let new = CommandBarButton::new();
    new.set_label(Some("New"));
    new.set_icon(icon(NEW_ICON));
    bar.primary_commands().add(new.as_command_bar_element());
    let save = CommandBarButton::new();
    save.set_label(Some("Save"));
    save.set_icon(icon(SAVE_ICON));
    bar.primary_commands().add(save.as_command_bar_element());
    bar.primary_commands().add(CommandBarSeparator::new().as_command_bar_element());
    let bold = CommandBarToggleButton::new();
    bold.set_label(Some("Bold"));
    bold.set_icon(icon(BOLD_ICON));
    bar.primary_commands().add(bold.as_command_bar_element());
    target.set_child(bar);

    target.styles().add(SimpleTheme::new().as_style());
    target.styles().add(font_style());
    t.render_to_file(&target, "CommandBar_Default_PrimaryCommands");
    t.compare_images_with(
        "CommandBar_Default_PrimaryCommands",
        CompareOptions { skip_immediate: true, ..Default::default() },
    );
}

#[test]
fn command_bar_compact_label_collapsed() {
    let t = base();
    let target = Decorator::new();
    target.set_width(300.0);
    target.set_height(60.0);
    let bar = CommandBar::new();
    bar.set_background(Some(Brushes::light_gray()));
    bar.set_default_label_position(CommandBarDefaultLabelPosition::Collapsed);
    bar.set_overflow_button_visibility(CommandBarOverflowButtonVisibility::Collapsed);
    let new = CommandBarButton::new();
    new.set_icon(icon(NEW_ICON));
    bar.primary_commands().add(new.as_command_bar_element());
    let save = CommandBarButton::new();
    save.set_icon(icon(SAVE_ICON));
    bar.primary_commands().add(save.as_command_bar_element());
    bar.primary_commands().add(CommandBarSeparator::new().as_command_bar_element());
    let bold = CommandBarToggleButton::new();
    bold.set_is_checked(Some(true));
    bold.set_icon(icon(BOLD_ICON));
    bar.primary_commands().add(bold.as_command_bar_element());
    target.set_child(bar);

    target.styles().add(SimpleTheme::new().as_style());
    target.styles().add(font_style());
    t.render_to_file(&target, "CommandBar_Compact_LabelCollapsed");
    t.compare_images_with(
        "CommandBar_Compact_LabelCollapsed",
        CompareOptions { skip_immediate: true, ..Default::default() },
    );
}
