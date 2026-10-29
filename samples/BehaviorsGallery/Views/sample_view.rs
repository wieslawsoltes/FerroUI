//! Port of `Views/SampleView.xaml.cs` (the class of `Views/SampleView.xaml`).

use crate::markup::{user_control_class, xaml_class};
use crate::view_models::SampleViewModel;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::UserControl;

/// A view with its own view model, whose commands write when the view is attached to the visual
/// tree and detached from it.
#[repr(C)]
pub struct SampleView {
    base: UserControl,
}

user_control_class!(SampleView);
ferro_class_info!(SampleView {
    new: SampleView::new
});
xaml_class!(SampleView, "/Views/SampleView.xaml");

impl SampleView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `SampleView()`: the view has its own view model.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_data_context(Some(SampleViewModel::new() as BoxedValue));
        this
    }
}
