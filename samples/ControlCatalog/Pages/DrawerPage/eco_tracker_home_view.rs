//! Port of `Pages/DrawerPage/EcoTrackerHomeView.xaml.cs`: the class of the document
//! `Pages/DrawerPage/EcoTrackerHomeView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::UserControl;
use std::cell::RefCell;
use std::rc::Rc;

#[repr(C)]
pub struct EcoTrackerHomeView {
    base: UserControl,
    tree_detail_requested: RefCell<Option<Rc<dyn Fn()>>>,
}

user_control_class!(EcoTrackerHomeView);
ferro_class_info!(EcoTrackerHomeView {
    new: EcoTrackerHomeView::new,
    markup: {
        methods: [
            fn OnHeroClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<EcoTrackerHomeView>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_hero_click(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(EcoTrackerHomeView, "/Pages/DrawerPage/EcoTrackerHomeView.xaml");

impl EcoTrackerHomeView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), tree_detail_requested: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    pub fn tree_detail_requested(&self) -> Option<Rc<dyn Fn()>> {
        self.tree_detail_requested.borrow().clone()
    }

    pub fn set_tree_detail_requested(&self, value: Option<Rc<dyn Fn()>>) {
        *self.tree_detail_requested.borrow_mut() = value;
    }

    fn on_hero_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        if let Some(tree_detail_requested) = self.tree_detail_requested() {
            tree_detail_requested();
        }
    }
}
