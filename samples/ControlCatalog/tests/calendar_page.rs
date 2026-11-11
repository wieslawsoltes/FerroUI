use super::support::start_catalog_app;
use crate::pages::CalendarPage;
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::DateTime;
use ferroui_base::{Ref, Vector};
use ferroui_controls::{Calendar, Control, Panel, ScrollViewer, Window};
use ferroui_controls_repeater::ItemsRepeater;

fn show(page: &Ref<CalendarPage>) -> Ref<Window> {
    let window = Window::new();
    window.set_width(1000.0);
    window.set_height(500.0);
    window.set_content(Some(Control::boxed(page)));
    window.show();
    Dispatcher::ui_thread().run_jobs(None);
    window
}

#[test]
fn calendar_examples_virtualize_and_keep_state_when_scrolled_back() {
    let _app = start_catalog_app();
    let page = CalendarPage::new();
    let window = show(&page);
    let repeater = page
        .get_visual_descendants()
        .find_map(|control| control.cast::<ItemsRepeater>())
        .unwrap();
    let scroller = page
        .get_visual_descendants()
        .find_map(|control| control.cast::<ScrollViewer>())
        .unwrap();
    let source = repeater.items_source_view().unwrap();
    assert_eq!(9, source.count());
    let realized = repeater.realized_elements().len();
    assert!(realized > 0 && realized < 9, "initially realized {realized} calendars");
    assert!(repeater.try_get_element(0).is_some());
    assert!(repeater.try_get_element(8).is_none());

    let second = Control::from_boxed(&source.get_at(1).unwrap()).unwrap();
    let first = repeater.try_get_element(0).unwrap();
    assert!((first.bounds().y - second.bounds().y).abs() < 1.0);
    window.set_width(550.0);
    Dispatcher::ui_thread().run_jobs(None);
    assert!(
        second.bounds().y > first.bounds().y,
        "narrow windows wrap examples into one column"
    );
    window.set_width(1000.0);
    Dispatcher::ui_thread().run_jobs(None);
    assert!((first.bounds().y - second.bounds().y).abs() < 1.0);
    let calendar = second
        .cast::<Panel>()
        .unwrap()
        .children()
        .get(1)
        .cast::<Calendar>()
        .unwrap();
    let selected = DateTime::today().add_days(2.0);
    calendar.set_selected_date(Some(selected));

    scroller.set_offset(Vector::new(0.0, 10_000.0));
    Dispatcher::ui_thread().run_jobs(None);
    assert!(scroller.offset().y > 0.0);
    assert!(repeater.try_get_element(8).is_some());
    assert!(repeater.try_get_element(0).is_none());
    assert!(page.get_control::<Calendar>("WeekNumbers").is_week_number_visible());
    let bounded = page.get_control::<Calendar>("DisplayDatesCalendar");
    assert_eq!(Some(DateTime::today().add_days(-25.0)), bounded.display_date_start());
    assert_eq!(Some(DateTime::today().add_days(25.0)), bounded.display_date_end());
    assert!(page
        .get_control::<Calendar>("BlackoutDatesCalendar")
        .blackout_dates()
        .contains_date(DateTime::today().add_days(-1.0)));

    scroller.set_offset(Vector::default());
    Dispatcher::ui_thread().run_jobs(None);
    assert!(repeater
        .try_get_element(1)
        .is_some_and(|control| control.ptr_eq(&second)));
    assert_eq!(Some(selected), calendar.selected_date());
    window.close();
    drop(window);
    let weak = page.downgrade();
    drop(page);
    assert!(
        weak.upgrade().is_none(),
        "retaining the source must not retain its page"
    );
}
