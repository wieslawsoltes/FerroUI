//! Port of `Pages/GesturesPage.xaml.cs`: the class of the document `Pages/GesturesPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{Border, TextBlock, UserControl};
use std::rc::Rc;

#[repr(C)]
pub struct GesturesPage {
    base: UserControl,
}

user_control_class!(GesturesPage);
// The handlers of the gestures take the arguments of a tap in the managed original
// (`TappedEventArgs`); none of them reads its arguments.
ferro_class_info!(GesturesPage {
    new: GesturesPage::new,
    markup: {
        methods: [
            fn GestureBorder_Tapped(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<GesturesPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.gesture_border_tapped(&sender, e.as_routed_event_args())
                },
            fn GestureBorder_DoubleTapped(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<GesturesPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.gesture_border_double_tapped(&sender, e.as_routed_event_args())
                },
            fn GestureBorder_RightTapped(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<GesturesPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.gesture_border_right_tapped(&sender, e.as_routed_event_args())
                },
            fn GestureBorder2_DoubleTapped(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<GesturesPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.gesture_border2_double_tapped(&sender, e.as_routed_event_args())
                },
            fn ResetGestures_Click(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<GesturesPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.reset_gestures_click(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(GesturesPage, "/Pages/GesturesPage.xaml");

impl GesturesPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    fn last_gesture(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("LastGesture")
    }

    fn gesture_border(&self) -> Ref<Border> {
        self.get_control::<Border>("GestureBorder")
    }

    fn gesture_border2(&self) -> Ref<Border> {
        self.get_control::<Border>("GestureBorder2")
    }

    fn gesture_border_tapped(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.last_gesture().set_text(Some("Tapped"));
    }

    fn gesture_border_double_tapped(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.last_gesture().set_text(Some("DoubleTapped"));

        // Testing #8733
        self.gesture_border().set_is_visible(false);
        self.gesture_border2().set_is_visible(true);
    }

    fn gesture_border_right_tapped(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.last_gesture().set_text(Some("RightTapped"));
    }

    fn gesture_border2_double_tapped(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.last_gesture().set_text(Some("DoubleTapped2"));
    }

    fn reset_gestures_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.last_gesture().set_text(Some(""));
        self.gesture_border().set_is_visible(true);
        self.gesture_border2().set_is_visible(false);
    }
}
