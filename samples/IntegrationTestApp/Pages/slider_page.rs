//! Port of `Pages/SliderPage.xaml.cs`: the class of the document `Pages/SliderPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{Slider, UserControl};
use std::rc::Rc;

#[repr(C)]
pub struct SliderPage {
    base: UserControl,
}

user_control_class!(SliderPage);
ferro_class_info!(SliderPage {
    new: SliderPage::new,
    markup: {
        methods: [
            fn ResetSliders_Click(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<SliderPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.reset_sliders_click(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(SliderPage, "/Pages/SliderPage.xaml");

impl SliderPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    fn horizontal_slider(&self) -> Ref<Slider> {
        self.get_control::<Slider>("HorizontalSlider")
    }

    fn reset_sliders_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.horizontal_slider().set_range_value(50.0);
    }
}
