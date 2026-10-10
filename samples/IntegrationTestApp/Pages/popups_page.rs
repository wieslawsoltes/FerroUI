//! Port of `Pages/PopupsPage.xaml.cs`: the class of the document `Pages/PopupsPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::ShowWindowTest;
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref, Visual};
use ferroui_controls::primitives::Popup;
use ferroui_controls::{TopLevel, UserControl, Window};
use std::rc::Rc;

#[repr(C)]
pub struct PopupsPage {
    base: UserControl,
}

user_control_class!(PopupsPage);
ferro_class_info!(PopupsPage {
    new: PopupsPage::new,
    markup: {
        methods: [
            fn ButtonLightDismiss_OnClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<PopupsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.button_light_dismiss_on_click(&sender, e.as_routed_event_args())
                },
            fn ButtonPopupStaysOpen_OnClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<PopupsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.button_popup_stays_open_on_click(&sender, e.as_routed_event_args())
                },
            fn StaysOpenPopupCloseButton_OnClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<PopupsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.stays_open_popup_close_button_on_click(&sender, e.as_routed_event_args())
                },
            fn ButtonTopMostPopupStaysOpen(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<PopupsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.button_top_most_popup_stays_open(&sender, e.as_routed_event_args())
                },
            fn TopMostPopupCloseButton_OnClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<PopupsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.top_most_popup_close_button_on_click(&sender, e.as_routed_event_args())
                },
            fn OpenRegularNewWindow_Click(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<PopupsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.open_regular_new_window_click(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(PopupsPage, "/Pages/PopupsPage.xaml");

impl PopupsPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    fn light_dismiss_popup(&self) -> Ref<Popup> {
        self.get_control::<Popup>("LightDismissPopup")
    }

    fn stays_open_popup(&self) -> Ref<Popup> {
        self.get_control::<Popup>("StaysOpenPopup")
    }

    fn top_most_popup(&self) -> Ref<Popup> {
        self.get_control::<Popup>("TopMostPopup")
    }

    fn button_light_dismiss_on_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.light_dismiss_popup().open();
    }

    fn button_popup_stays_open_on_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.stays_open_popup().open();
    }

    fn stays_open_popup_close_button_on_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.stays_open_popup().close();
    }

    fn button_top_most_popup_stays_open(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.top_most_popup().open();
    }

    fn top_most_popup_close_button_on_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.top_most_popup().close();
    }

    /// # Panics
    /// Panics if the page is not in a window (a null reference or an invalid cast in the
    /// managed original).
    fn open_regular_new_window_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let new_window = ShowWindowTest::new();
        let visual: &Visual = self;
        let owner = TopLevel::get_top_level(Some(visual))
            .and_then(|top_level| top_level.cast::<Window>())
            .expect("the page is in a window");
        new_window.show_with_owner(&owner);
    }
}
