//! The class of the document `Pages/TextCalibrationPage.xaml`: plain TextBlocks, one line per size
//! and weight, for the calibration of text against native WinUI 3 (docs/porting/text-rendering.md,
//! section 10).

use crate::markup::xaml_class;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ContentControlImpl, Control, ControlImpl, UserControl};

#[repr(C)]
pub struct TextCalibrationPage {
    base: UserControl,
}

ferro_class!(TextCalibrationPage: UserControl);
ferro_impl_classes!(
    TextCalibrationPage: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(TextCalibrationPage { new: TextCalibrationPage::new });
xaml_class!(TextCalibrationPage, "/Pages/TextCalibrationPage.xaml");

impl TextCalibrationPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// The page as a control (the entry of the page table).
pub fn create() -> Ref<Control> {
    TextCalibrationPage::new().upcast()
}
