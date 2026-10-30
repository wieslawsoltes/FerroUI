//! Interactive clock, numeric input, format and landscape examples.
use crate::markup::xaml_class;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, instantiate,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{Button, ContentControlImpl, Control, ControlImpl, TextBlock, UserControl};
use ferroui_themes_material::{MaterialTimeDisplayMode, MaterialTimePicker};
use std::rc::Rc;

#[repr(C)]
pub struct TimePickersPage {
    base: UserControl,
}
ferro_class!(TimePickersPage: UserControl);
ferro_impl_classes!(TimePickersPage: FerroObjectImpl, StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl, TemplatedControlImpl, ContentControlImpl);
ferro_class_info!(TimePickersPage {
    new: TimePickersPage::new
});
xaml_class!(TimePickersPage, "/Pages/TimePickersPage.xaml");
impl TimePickersPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        let weak = this.downgrade();
        this.get_control::<Button>("ToggleMode").click(move |_, _| {
            if let Some(this) = weak.upgrade() {
                let input = this
                    .get_control::<MaterialTimePicker>("ClassicTime")
                    .display_mode()
                    == MaterialTimeDisplayMode::Dial;
                for name in ["ClassicTime", "ExpressiveTime"] {
                    this.get_control::<MaterialTimePicker>(name)
                        .set_display_mode(if input {
                            MaterialTimeDisplayMode::Input
                        } else {
                            MaterialTimeDisplayMode::Dial
                        });
                }
                this.get_control::<Button>("ToggleMode")
                    .set_content(Some(Rc::new(
                        if input { "Use clock" } else { "Use keyboard" }.to_owned(),
                    )));
            }
        });
        let weak = this.downgrade();
        this.get_control::<Button>("ToggleFormat")
            .click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    let format = !this
                        .get_control::<MaterialTimePicker>("ClassicTime")
                        .is_24_hour();
                    for name in ["ClassicTime", "ExpressiveTime"] {
                        this.get_control::<MaterialTimePicker>(name)
                            .set_is_24_hour(format);
                    }
                    this.get_control::<Button>("ToggleFormat")
                        .set_content(Some(Rc::new(
                            if format {
                                "Use 12-hour format"
                            } else {
                                "Use 24-hour format"
                            }
                            .to_owned(),
                        )));
                }
            });
        for name in [
            "ClassicTime",
            "ExpressiveTime",
            "InputTime",
            "LandscapeTime",
        ] {
            let weak = this.downgrade();
            this.get_control::<MaterialTimePicker>(name).add_handler(
                MaterialTimePicker::time_changed_event(),
                move |_, e| {
                    if let Some(this) = weak.upgrade() {
                        let minutes = e.new_time().total_minutes() as i32;
                        this.get_control::<TextBlock>("TimeStatus")
                            .set_text(Some(&format!(
                                "Selected time: {:02}:{:02}",
                                minutes / 60,
                                minutes % 60
                            )));
                    }
                },
            );
        }
        this
    }
}
pub fn create() -> Ref<Control> {
    TimePickersPage::new().upcast()
}
