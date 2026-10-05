//! Port of the code-behind of the document
//! `Pages/NavigationPage/FerroFlixSearchView.xaml`: the class of the document.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{Button, NavigationPage, UserControl};
use std::cell::RefCell;
use std::rc::Rc;

#[repr(C)]
pub struct FerroFlixSearchView {
    base: UserControl,
    close_requested: RefCell<Option<Rc<dyn Fn()>>>,
    movie_selected: RefCell<Option<Rc<dyn Fn(&str)>>>,
}

user_control_class!(FerroFlixSearchView);
ferro_class_info!(FerroFlixSearchView {
    new: FerroFlixSearchView::new,
    markup: {
        methods: [
            fn OnCloseClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<FerroFlixSearchView>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_close_click(&sender, e.as_routed_event_args())
                },
            fn OnMovieClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<FerroFlixSearchView>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_movie_click(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(FerroFlixSearchView, "/Pages/NavigationPage/FerroFlixSearchView.xaml");

impl FerroFlixSearchView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), close_requested: RefCell::new(None), movie_selected: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    pub fn close_requested(&self) -> Option<Rc<dyn Fn()>> {
        self.close_requested.borrow().clone()
    }

    pub fn set_close_requested(&self, value: Option<Rc<dyn Fn()>>) {
        *self.close_requested.borrow_mut() = value;
    }

    pub fn movie_selected(&self) -> Option<Rc<dyn Fn(&str)>> {
        self.movie_selected.borrow().clone()
    }

    pub fn set_movie_selected(&self, value: Option<Rc<dyn Fn(&str)>>) {
        *self.movie_selected.borrow_mut() = value;
    }

    fn on_close_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        if let Some(close_requested) = self.close_requested() {
            close_requested();
        } else {
            let nav = self.find_ancestor_of_type::<NavigationPage>(false);
            if let Some(nav) = nav {
                drop(nav.pop_modal_async());
            }
        }
    }

    fn on_movie_click(&self, sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let mut title = String::from("Neon Horizon");
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
