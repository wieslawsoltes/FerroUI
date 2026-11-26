//! `Calendar.IsGroupLabelVisible` (`Controls/CalendarDayButton.xaml`; DEVIATIONS.md, CDB-7): the
//! first day of each month of the month view swaps to the template of the `:grouplabel`
//! pseudo-class, whose `PART_GroupLabel` shows the abbreviated name of the month in the
//! foreground of the day. Without labels no day button has that part.

use super::support::*;
use ferroui_base::utilities::DateTime;
use ferroui_base::Ref;
use ferroui_controls::primitives::CalendarDayButton;
use ferroui_controls::{Calendar, Control, TextBlock, Window};

/// The text blocks `PART_GroupLabel` of the day buttons of `calendar`, with their text.
fn group_labels(calendar: &Calendar) -> Vec<(Ref<TextBlock>, String)> {
    calendar
        .get_visual_descendants()
        .filter_map(|visual| visual.cast::<CalendarDayButton>())
        .filter_map(|button| {
            button
                .get_template_descendants()
                .into_iter()
                .find(|element| element.name().as_deref() == Some("PART_GroupLabel"))
                .and_then(|element| element.cast::<TextBlock>())
        })
        .map(|label| {
            let text = label.text().unwrap_or_default();
            (label, text)
        })
        .collect()
}

#[test]
fn the_first_day_of_each_month_shows_its_group_label() {
    let _app = start_themed_application();
    let calendar = Calendar::new();
    calendar.set_display_date(DateTime::new(2024, 2, 15));
    let window = Window::new();
    window.set_content(Some(Control::boxed(&calendar)));
    window.show();
    window.update_layout();
    assert!(group_labels(&calendar).is_empty(), "no labels by default");

    calendar.set_is_group_label_visible(true);
    window.update_layout();
    // 1 February and 1 March 2024 are in the month view whichever day starts the week.
    let labels = group_labels(&calendar);
    assert_eq!(2, labels.len(), "{:?}", labels.iter().map(|(_, text)| text).collect::<Vec<_>>());
    assert!(labels.iter().all(|(_, text)| !text.is_empty()));
    assert_ne!(labels[0].1, labels[1].1);
    assert!(labels.iter().all(|(label, _)| label.foreground().is_some()));

    calendar.set_is_group_label_visible(false);
    window.update_layout();
    assert!(group_labels(&calendar).is_empty(), "no labels once they are turned off");
}
