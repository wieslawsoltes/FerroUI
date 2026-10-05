//! Port of `Pages/NavigationPage/PulseHomeView.xaml.cs`: the class of the document
//! `Pages/NavigationPage/PulseHomeView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::input::PointerPressedEventArgs;
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::UserControl;
use std::cell::RefCell;
use std::rc::Rc;

#[repr(C)]
pub struct PulseHomeView {
    base: UserControl,
    workout_detail_requested: RefCell<Option<Rc<dyn Fn()>>>,
}

user_control_class!(PulseHomeView);
ferro_class_info!(PulseHomeView {
    new: PulseHomeView::new,
    markup: {
        methods: [
            fn OnRecCard1Pressed(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<PulseHomeView>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<PointerPressedEventArgs>() {
                        this.on_rec_card1_pressed(&sender, e)
                    }
                },
            fn OnRecCard2Pressed(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<PulseHomeView>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<PointerPressedEventArgs>() {
                        this.on_rec_card2_pressed(&sender, e)
                    }
                },
            fn OnRecCard3Pressed(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<PulseHomeView>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<PointerPressedEventArgs>() {
                        this.on_rec_card3_pressed(&sender, e)
                    }
                },
            fn OnPlayButtonClicked(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<PulseHomeView>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_play_button_clicked(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(PulseHomeView, "/Pages/NavigationPage/PulseHomeView.xaml");

impl PulseHomeView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), workout_detail_requested: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    pub fn workout_detail_requested(&self) -> Option<Rc<dyn Fn()>> {
        self.workout_detail_requested.borrow().clone()
    }

    pub fn set_workout_detail_requested(&self, value: Option<Rc<dyn Fn()>>) {
        *self.workout_detail_requested.borrow_mut() = value;
    }

    fn on_rec_card1_pressed(&self, _sender: &Option<BoxedValue>, _e: &PointerPressedEventArgs) {
        if let Some(workout_detail_requested) = self.workout_detail_requested() {
            workout_detail_requested();
        }
    }

    fn on_rec_card2_pressed(&self, _sender: &Option<BoxedValue>, _e: &PointerPressedEventArgs) {
        if let Some(workout_detail_requested) = self.workout_detail_requested() {
            workout_detail_requested();
        }
    }

    fn on_rec_card3_pressed(&self, _sender: &Option<BoxedValue>, _e: &PointerPressedEventArgs) {
        if let Some(workout_detail_requested) = self.workout_detail_requested() {
            workout_detail_requested();
        }
    }

    fn on_play_button_clicked(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        if let Some(workout_detail_requested) = self.workout_detail_requested() {
            workout_detail_requested();
        }
    }
}
