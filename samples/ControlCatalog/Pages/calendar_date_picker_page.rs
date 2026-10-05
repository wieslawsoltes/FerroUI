//! Port of `Pages/CalendarDatePickerPage.xaml.cs`: the class of the
//! document `Pages/CalendarDatePickerPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::utilities::DateTime;
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::{CalendarDatePicker, ContentPage};

#[repr(C)]
pub struct CalendarDatePickerPage {
    base: ContentPage,
}

content_page_class!(CalendarDatePickerPage);
ferro_class_info!(CalendarDatePickerPage { new: CalendarDatePickerPage::new });
xaml_class!(CalendarDatePickerPage, "/Pages/CalendarDatePickerPage.xaml");

impl CalendarDatePickerPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        this.date_picker("DatePicker1").set_selected_date(Some(DateTime::today()));
        this.date_picker("DatePicker2").set_selected_date(Some(DateTime::today().add_days(10.0)));
        this.date_picker("DatePicker3").set_selected_date(Some(DateTime::today().add_days(20.0)));
        this.date_picker("DatePicker5").set_selected_date(Some(DateTime::today()));

        // The handler of an event of the picker itself holds it weakly.
        let date_picker4 = this.date_picker("DatePicker4");
        let weak = date_picker4.downgrade();
        date_picker4.template_applied(move |_s, _e| {
            if let Some(blackout_dates) = weak.upgrade().and_then(|date_picker4| date_picker4.blackout_dates()) {
                blackout_dates.add_dates_in_past();
            }
        });
        this
    }

    /// The named elements `DatePicker1` to `DatePicker5`.
    fn date_picker(&self, name: &str) -> Ref<CalendarDatePicker> {
        self.get_control::<CalendarDatePicker>(name)
    }
}
