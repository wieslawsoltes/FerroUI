//! Port of `Pages/ButtonSpinnerPage.xaml.cs`: the class of the document
//! `Pages/ButtonSpinnerPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::interactivity::IRoutedEventArgs;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{ButtonSpinner, ContentPage, Control, SpinDirection, SpinEventArgs, TextBlock};
use std::rc::Rc;

const MOUNTAINS: [&str; 10] = [
    "Everest",
    "K2 (Mount Godwin Austen)",
    "Kangchenjunga",
    "Lhotse",
    "Makalu",
    "Cho Oyu",
    "Dhaulagiri",
    "Manaslu",
    "Nanga Parbat",
    "Annapurna",
];

#[repr(C)]
pub struct ButtonSpinnerPage {
    base: ContentPage,
    mountains: [&'static str; 10],
}

content_page_class!(ButtonSpinnerPage);
ferro_class_info!(ButtonSpinnerPage {
    new: ButtonSpinnerPage::new,
    markup: {
        methods: [
            fn OnSpin(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ButtonSpinnerPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    let e = e.downcast_ref::<SpinEventArgs>().expect("the arguments of a spin event");
                    this.on_spin(&sender, e)
                },
        ],
    },
});
xaml_class!(ButtonSpinnerPage, "/Pages/ButtonSpinnerPage.xaml");

impl ButtonSpinnerPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct(), mountains: MOUNTAINS }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    /// # Panics
    /// Panics if the sender is not a `ButtonSpinner` (an invalid cast in the
    /// managed original).
    pub fn on_spin(&self, sender: &Option<BoxedValue>, e: &SpinEventArgs) {
        let spinner = sender
            .as_ref()
            .and_then(|sender| ValueTypes::as_object(&**sender))
            .and_then(|sender| sender.cast::<ButtonSpinner>())
            .expect("the sender of a spin event handled by the page is a ButtonSpinner");

        let content = spinner.content().and_then(|content| Control::from_boxed(&content));
        if let Some(txt_box) = content.and_then(|content| content.cast::<TextBlock>()) {
            let text = txt_box.text();
            let mut value = match text.as_deref().and_then(|text| self.mountains.iter().position(|m| *m == text)) {
                Some(index) => index as i32,
                None => -1,
            };
            if e.direction() == SpinDirection::Increase {
                value += 1;
            } else {
                value -= 1;
            }

            if value < 0 {
                value = self.mountains.len() as i32 - 1;
            } else if value >= self.mountains.len() as i32 {
                value = 0;
            }

            txt_box.set_text(Some(self.mountains[value as usize]));
        }
    }
}
