//! Port of the code-behind of the document
//! `Pages/NavigationPage/FerroFlixHomeView.xaml`: the class of the document.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{Button, UserControl};
use std::cell::RefCell;
use std::rc::Rc;

#[repr(C)]
pub struct FerroFlixHomeView {
    base: UserControl,
    movie_selected: RefCell<Option<Rc<dyn Fn(&str)>>>,
    search_requested: RefCell<Option<Rc<dyn Fn()>>>,
}

user_control_class!(FerroFlixHomeView);
ferro_class_info!(FerroFlixHomeView {
    new: FerroFlixHomeView::new,
    markup: {
        methods: [
            fn OnMovieClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<FerroFlixHomeView>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_movie_click(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(FerroFlixHomeView, "/Pages/NavigationPage/FerroFlixHomeView.xaml");

impl FerroFlixHomeView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), movie_selected: RefCell::new(None), search_requested: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    pub fn movie_selected(&self) -> Option<Rc<dyn Fn(&str)>> {
        self.movie_selected.borrow().clone()
    }

    pub fn set_movie_selected(&self, value: Option<Rc<dyn Fn(&str)>>) {
        *self.movie_selected.borrow_mut() = value;
    }

    pub fn search_requested(&self) -> Option<Rc<dyn Fn()>> {
        self.search_requested.borrow().clone()
    }

    pub fn set_search_requested(&self, value: Option<Rc<dyn Fn()>>) {
        *self.search_requested.borrow_mut() = value;
    }

    fn on_movie_click(&self, sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let mut title = String::from("Cyber Dune");
        let tag = sender
            .as_ref()
            .and_then(|sender| ValueTypes::as_object(&**sender))
            .and_then(|sender| sender.cast::<Button>())
            .and_then(|btn| btn.tag())
            .and_then(|tag| tag.downcast_ref::<String>().cloned());
        if let Some(tag) = tag {
            title = tag;
        }
        if let Some(movie_selected) = self.movie_selected() {
            movie_selected(&title);
        }
    }
}
