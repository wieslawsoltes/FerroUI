//! Port of `Pages/NavigationPage/PulseLoginView.xaml.cs`: the class of the document
//! `Pages/NavigationPage/PulseLoginView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::UserControl;
use std::cell::RefCell;
use std::rc::Rc;

#[repr(C)]
pub struct PulseLoginView {
    base: UserControl,
    login_requested: RefCell<Option<Rc<dyn Fn()>>>,
}

user_control_class!(PulseLoginView);
ferro_class_info!(PulseLoginView {
    new: PulseLoginView::new,
    markup: {
        methods: [
            fn OnLoginClicked(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<PulseLoginView>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_login_clicked(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(PulseLoginView, "/Pages/NavigationPage/PulseLoginView.xaml");

impl PulseLoginView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), login_requested: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    pub fn login_requested(&self) -> Option<Rc<dyn Fn()>> {
        self.login_requested.borrow().clone()
    }

    pub fn set_login_requested(&self, value: Option<Rc<dyn Fn()>>) {
        *self.login_requested.borrow_mut() = value;
    }

    fn on_login_clicked(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        if let Some(login_requested) = self.login_requested() {
            login_requested();
        }
    }
}
