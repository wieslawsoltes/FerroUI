//! Port of `Pages/NavigationPage/LAvenirMenuView.xaml.cs`: the class of the document
//! `Pages/NavigationPage/LAvenirMenuView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::input::PointerPressedEventArgs;
use ferroui_base::interactivity::IRoutedEventArgs;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::UserControl;
use std::cell::RefCell;
use std::rc::Rc;

/// `Action<string, string, string, string>`: name, price, description and
/// image file of a dish.
pub type DishSelected = Rc<dyn Fn(&str, &str, &str, &str)>;

#[repr(C)]
pub struct LAvenirMenuView {
    base: UserControl,
    dish_selected: RefCell<Option<DishSelected>>,
}

user_control_class!(LAvenirMenuView);
ferro_class_info!(LAvenirMenuView {
    new: LAvenirMenuView::new,
    markup: {
        methods: [
            fn OnDish1Pressed(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<LAvenirMenuView>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<PointerPressedEventArgs>() {
                        this.on_dish1_pressed(&sender, e)
                    }
                },
            fn OnDish2Pressed(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<LAvenirMenuView>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<PointerPressedEventArgs>() {
                        this.on_dish2_pressed(&sender, e)
                    }
                },
            fn OnDish3Pressed(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<LAvenirMenuView>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<PointerPressedEventArgs>() {
                        this.on_dish3_pressed(&sender, e)
                    }
                },
            fn OnDish4Pressed(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<LAvenirMenuView>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<PointerPressedEventArgs>() {
                        this.on_dish4_pressed(&sender, e)
                    }
                },
        ],
    },
});
xaml_class!(LAvenirMenuView, "/Pages/NavigationPage/LAvenirMenuView.xaml");

impl LAvenirMenuView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), dish_selected: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    pub fn dish_selected(&self) -> Option<DishSelected> {
        self.dish_selected.borrow().clone()
    }

    pub fn set_dish_selected(&self, value: Option<DishSelected>) {
        *self.dish_selected.borrow_mut() = value;
    }

    fn on_dish1_pressed(&self, _sender: &Option<BoxedValue>, _e: &PointerPressedEventArgs) {
        if let Some(dish_selected) = self.dish_selected() {
            dish_selected("Seared Scallops", "$38", "Fresh scallops with truffle butter and microgreens", "dish1.jpg");
        }
    }

    fn on_dish2_pressed(&self, _sender: &Option<BoxedValue>, _e: &PointerPressedEventArgs) {
        if let Some(dish_selected) = self.dish_selected() {
            dish_selected("Truffle Risotto", "$34", "Creamy arborio rice with black truffle shavings", "dish2.jpg");
        }
    }

    fn on_dish3_pressed(&self, _sender: &Option<BoxedValue>, _e: &PointerPressedEventArgs) {
        if let Some(dish_selected) = self.dish_selected() {
            dish_selected("Wagyu Tartare", "$42", "Hand-cut wagyu beef with quail egg yolk", "dish3.jpg");
        }
    }

    fn on_dish4_pressed(&self, _sender: &Option<BoxedValue>, _e: &PointerPressedEventArgs) {
        if let Some(dish_selected) = self.dish_selected() {
            dish_selected("Lobster Bisque", "$24", "Classic French bisque with cream and cognac", "dish4.jpg");
        }
    }
}
