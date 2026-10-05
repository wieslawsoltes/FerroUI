//! Port of `Controls/SectionControl.xaml.cs`: the class of the document
//! `Controls/SectionControl.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct SectionControl {
    base: UserControl,
}

user_control_class!(SectionControl);
ferro_class_info!(SectionControl { new: SectionControl::new });
xaml_class!(SectionControl, "/Controls/SectionControl.xaml");

impl SectionControl {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
