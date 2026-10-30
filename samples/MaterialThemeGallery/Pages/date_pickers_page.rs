//! Native Material calendar, range, year selection and typed date entry.
use crate::markup::xaml_class;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::FlowDirection;
use ferroui_base::utilities::{CultureInfo, DateTime, DateTimeFormatInfo, DayOfWeek};
use ferroui_base::{
    FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, instantiate,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{Button, ContentControlImpl, ControlImpl, TextBlock, UserControl};
use ferroui_themes_material::{MaterialDatePicker, MaterialDateSelectionMode};
use std::rc::Rc;

#[repr(C)]
pub struct DatePickersPage {
    base: UserControl,
}
ferro_class!(DatePickersPage: UserControl);
ferro_class_info!(DatePickersPage {
    new: DatePickersPage::new
});
ferro_impl_classes!(DatePickersPage: FerroObjectImpl, StyledElementImpl, VisualImpl,
    LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl, TemplatedControlImpl, ContentControlImpl);
xaml_class!(DatePickersPage, "/Pages/DatePickersPage.xaml");

impl DatePickersPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        let picker = this.get_control::<MaterialDatePicker>("DemoPicker");
        picker.set_culture(Some(gallery_culture(false)));
        let today = DateTime::today();
        picker.set_selected_date(Some(today));
        picker.set_displayed_month(today);
        for (name, action) in [
            ("ToggleRange", 0),
            ("ToggleCulture", 1),
            ("ToggleDirection", 2),
            ("ChooseToday", 3),
            ("ClearSelection", 4),
        ] {
            let weak = this.downgrade();
            this.get_control::<Button>(name).click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    let picker = this.get_control::<MaterialDatePicker>("DemoPicker");
                    match action {
                        0 => {
                            let range =
                                picker.selection_mode() == MaterialDateSelectionMode::Single;
                            picker.set_selection_mode(if range {
                                MaterialDateSelectionMode::Range
                            } else {
                                MaterialDateSelectionMode::Single
                            });
                            this.get_control::<Button>("ToggleRange")
                                .set_content(Some(Rc::new(
                                    if range {
                                        "Choose one date"
                                    } else {
                                        "Choose a date range"
                                    }
                                    .to_owned(),
                                )));
                        }
                        1 => {
                            let british = picker.culture().name() != "en-GB";
                            picker.set_culture(Some(gallery_culture(british)));
                            this.get_control::<Button>("ToggleCulture")
                                .set_content(Some(Rc::new(
                                    if british {
                                        "Use month/day/year"
                                    } else {
                                        "Use day/month/year"
                                    }
                                    .to_owned(),
                                )));
                        }
                        2 => picker.set_flow_direction(
                            if picker.flow_direction() == FlowDirection::LeftToRight {
                                FlowDirection::RightToLeft
                            } else {
                                FlowDirection::LeftToRight
                            },
                        ),
                        3 => {
                            let today = DateTime::today();
                            picker.set_selection(
                                Some(today),
                                if picker.selection_mode() == MaterialDateSelectionMode::Range {
                                    Some(today.add_days(6.0))
                                } else {
                                    None
                                },
                            );
                            picker.set_displayed_month(today);
                        }
                        _ => picker.set_selection(None, None),
                    }
                    this.update_selection();
                }
            });
        }
        let weak = this.downgrade();
        picker.add_handler(
            MaterialDatePicker::selection_changed_event(),
            move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.update_selection();
                }
            },
        );
        let weak = this.downgrade();
        picker.add_handler(MaterialDatePicker::confirmed_event(), move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.get_control::<TextBlock>("Confirmation")
                    .set_text(Some("Dates saved."));
            }
        });
        let weak = this.downgrade();
        picker.add_handler(MaterialDatePicker::dismissed_event(), move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.get_control::<TextBlock>("Confirmation")
                    .set_text(Some("Selection dismissed."));
            }
        });
        this.update_selection();
        this
    }
    fn update_selection(&self) {
        let p = self.get_control::<MaterialDatePicker>("DemoPicker");
        let culture = p.culture();
        let selection = match (p.selected_start_date(), p.selected_end_date()) {
            (Some(start), Some(end)) => format!(
                "{} – {}",
                start.to_string_format("d", &culture),
                end.to_string_format("d", &culture)
            ),
            (Some(start), None) => format!(
                "{}{}",
                start.to_string_format("d", &culture),
                if p.selection_mode() == MaterialDateSelectionMode::Range {
                    " — choose an end date"
                } else {
                    ""
                }
            ),
            _ => "No date selected".into(),
        };
        self.get_control::<TextBlock>("SelectionSummary")
            .set_text(Some(&selection));
        self.get_control::<TextBlock>("Confirmation").set_text(None);
    }
}

// The base library can receive an ICultureDataProvider from the platform. This
// standalone sample supplies its two demonstrated cultures explicitly as well.
fn gallery_culture(british: bool) -> CultureInfo {
    let mut format = DateTimeFormatInfo::new();
    format.set_short_date_pattern(if british { "dd/MM/yyyy" } else { "M/d/yyyy" });
    format.set_long_date_pattern(if british {
        "dddd, d MMMM yyyy"
    } else {
        "dddd, MMMM d, yyyy"
    });
    format.set_year_month_pattern("MMMM yyyy");
    format.set_first_day_of_week(if british {
        DayOfWeek::Monday
    } else {
        DayOfWeek::Sunday
    });
    CultureInfo::get_culture_info(if british { "en-GB" } else { "en-US" })
        .with_date_time_format(format)
}

/// Creates the page for interactive navigation and strict render capture.
pub fn create() -> Ref<ferroui_controls::Control> {
    DatePickersPage::new().upcast()
}

/// Creates the same interactive page with a stable complete range for capture and review.
pub fn create_range() -> Ref<ferroui_controls::Control> {
    let page = DatePickersPage::new();
    let picker = page.get_control::<MaterialDatePicker>("DemoPicker");
    picker.set_selection_mode(MaterialDateSelectionMode::Range);
    picker.set_selection(
        Some(DateTime::new(2026, 10, 22)),
        Some(DateTime::new(2026, 10, 29)),
    );
    picker.set_displayed_month(DateTime::new(2026, 10, 1));
    page.get_control::<Button>("ToggleRange")
        .set_content(Some(Rc::new("Choose one date".to_owned())));
    page.update_selection();
    page.upcast()
}
