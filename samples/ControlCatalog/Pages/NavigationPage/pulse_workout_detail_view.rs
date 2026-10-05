//! Port of `Pages/NavigationPage/PulseWorkoutDetailView.xaml.cs`: the class of the document
//! `Pages/NavigationPage/PulseWorkoutDetailView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::UserControl;
use std::cell::RefCell;
use std::rc::Rc;

#[repr(C)]
pub struct PulseWorkoutDetailView {
    base: UserControl,
    back_requested: RefCell<Option<Rc<dyn Fn()>>>,
}

user_control_class!(PulseWorkoutDetailView);
ferro_class_info!(PulseWorkoutDetailView {
    new: PulseWorkoutDetailView::new,
    markup: {
        methods: [
            fn OnBackClicked(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<PulseWorkoutDetailView>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_back_clicked(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(PulseWorkoutDetailView, "/Pages/NavigationPage/PulseWorkoutDetailView.xaml");

impl PulseWorkoutDetailView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), back_requested: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    pub fn back_requested(&self) -> Option<Rc<dyn Fn()>> {
        self.back_requested.borrow().clone()
    }

    pub fn set_back_requested(&self, value: Option<Rc<dyn Fn()>>) {
        *self.back_requested.borrow_mut() = value;
    }

    fn on_back_clicked(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        if let Some(back_requested) = self.back_requested() {
            back_requested();
        }
    }
}
