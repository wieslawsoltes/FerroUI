//! Port of `Pages/NavigationPage/CurvedHeaderHomeScrollView.xaml.cs`: the class of the document
//! `Pages/NavigationPage/CurvedHeaderHomeScrollView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::UserControl;
use std::cell::RefCell;
use std::rc::Rc;

#[repr(C)]
pub struct CurvedHeaderHomeScrollView {
    base: UserControl,
    navigate_requested: RefCell<Option<Rc<dyn Fn()>>>,
}

user_control_class!(CurvedHeaderHomeScrollView);
ferro_class_info!(CurvedHeaderHomeScrollView {
    new: CurvedHeaderHomeScrollView::new,
    markup: {
        methods: [
            fn OnShopNowClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CurvedHeaderHomeScrollView>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_shop_now_click(&sender, e.as_routed_event_args())
                },
            fn OnProductClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CurvedHeaderHomeScrollView>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_product_click(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(CurvedHeaderHomeScrollView, "/Pages/NavigationPage/CurvedHeaderHomeScrollView.xaml");

impl CurvedHeaderHomeScrollView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), navigate_requested: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    pub fn navigate_requested(&self) -> Option<Rc<dyn Fn()>> {
        self.navigate_requested.borrow().clone()
    }

    pub fn set_navigate_requested(&self, value: Option<Rc<dyn Fn()>>) {
        *self.navigate_requested.borrow_mut() = value;
    }

    fn on_shop_now_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        if let Some(navigate_requested) = self.navigate_requested() {
            navigate_requested();
        }
    }

    fn on_product_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        if let Some(navigate_requested) = self.navigate_requested() {
            navigate_requested();
        }
    }
}
