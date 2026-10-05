//! Port of `Pages/DateTimePickerPage.xaml.cs`: the class of the document
//! `Pages/DateTimePickerPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::{ContentPage, TextBlock};

#[repr(C)]
pub struct DateTimePickerPage {
    base: ContentPage,
}

content_page_class!(DateTimePickerPage);
ferro_class_info!(DateTimePickerPage { new: DateTimePickerPage::new });
xaml_class!(DateTimePickerPage, "/Pages/DateTimePickerPage.xaml");

impl DateTimePickerPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.date_picker_desc().set_text(Some(concat!(
            "Use a DatePicker to let users set a date in your app, ",
            "for example to schedule an appointment. The DatePicker displays three controls for month, day, and year. ",
            "These controls are easy to use with touch or mouse, and they can be styled and configured in several different ways. ",
            "Order of month, day, and year is dynamically set based on user date settings",
        )));

        this.time_picker_desc().set_text(Some(concat!(
            "Use a TimePicker to let users set a time in your app, for example ",
            "to set a reminder. The TimePicker displays four controls for hour, minute, seconds(optional), and AM / PM(if necessary).These controls ",
            "are easy to use with touch or mouse, and they can be styled and configured in several different ways. ",
            "12 - hour or 24 - hour clock and visibility of AM / PM is dynamically set based on user time settings, or can be overridden.",
        )));
        this
    }

    fn date_picker_desc(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("DatePickerDesc")
    }

    fn time_picker_desc(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("TimePickerDesc")
    }
}
