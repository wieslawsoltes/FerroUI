//! Port of `Pages/AdornerLayerPage.xaml.cs`: the class of the document
//! `Pages/AdornerLayerPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::primitives::AdornerLayer;
use ferroui_controls::{Button, ContentPage, Control};
use std::cell::RefCell;
use std::rc::Rc;

#[repr(C)]
pub struct AdornerLayerPage {
    base: ContentPage,
    adorner: RefCell<Option<Ref<Control>>>,
}

content_page_class!(AdornerLayerPage);
ferro_class_info!(AdornerLayerPage {
    new: AdornerLayerPage::new,
    markup: {
        methods: [
            fn RemoveAdorner_OnClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<AdornerLayerPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.remove_adorner_on_click(&sender, e.as_routed_event_args())
                },
            fn AddAdorner_OnClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<AdornerLayerPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.add_adorner_on_click(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(AdornerLayerPage, "/Pages/AdornerLayerPage.xaml");

impl AdornerLayerPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct(), adorner: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    fn adorner_button(&self) -> Ref<Button> {
        self.get_control::<Button>("AdornerButton")
    }

    fn remove_adorner_on_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let adorner_button = self.adorner_button();
        let adorner = AdornerLayer::get_adorner(&adorner_button);
        if let Some(adorner) = adorner {
            *self.adorner.borrow_mut() = Some(adorner);
        }
        AdornerLayer::set_adorner(&adorner_button, None::<Ref<Control>>);
    }

    fn add_adorner_on_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let adorner = self.adorner.borrow().clone();
        if let Some(adorner) = adorner {
            AdornerLayer::set_adorner(&self.adorner_button(), adorner);
        }
    }
}
