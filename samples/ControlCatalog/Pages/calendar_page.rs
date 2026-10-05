//! Port of `Pages/CalendarPage.xaml.cs`: the class of the document
//! `Pages/CalendarPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::utilities::DateTime;
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::{Calendar, CalendarDateRange, ContentPage};

#[repr(C)]
pub struct CalendarPage {
    base: ContentPage,
}

content_page_class!(CalendarPage);
ferro_class_info!(CalendarPage { new: CalendarPage::new });
xaml_class!(CalendarPage, "/Pages/CalendarPage.xaml");

impl CalendarPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        let today = DateTime::today();
        this.display_dates_calendar().set_display_date_start(Some(today.add_days(-25.0)));
        this.display_dates_calendar().set_display_date_end(Some(today.add_days(25.0)));

        this.blackout_dates_calendar().blackout_dates().add_dates_in_past();
        this.blackout_dates_calendar().blackout_dates().add(CalendarDateRange::new(today.add_days(6.0)));
        this
    }

    fn display_dates_calendar(&self) -> Ref<Calendar> {
        self.get_control::<Calendar>("DisplayDatesCalendar")
    }

    fn blackout_dates_calendar(&self) -> Ref<Calendar> {
        self.get_control::<Calendar>("BlackoutDatesCalendar")
    }
}
