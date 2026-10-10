//! Port of `Pages/DesktopPage.xaml.cs`: the class of the document `Pages/DesktopPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::App;
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{Application, TextBlock, TrayIcon, UserControl};
use std::cell::Cell;
use std::rc::Rc;

#[repr(C)]
pub struct DesktopPage {
    base: UserControl,
    dock_menu_item_count: Cell<i32>,
}

user_control_class!(DesktopPage);
ferro_class_info!(DesktopPage {
    new: DesktopPage::new,
    markup: {
        methods: [
            fn ToggleTrayIconVisible_Click(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DesktopPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.toggle_tray_icon_visible_click(&sender, e.as_routed_event_args())
                },
            fn AddDockMenuItem_Click(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DesktopPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.add_dock_menu_item_click(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(DesktopPage, "/Pages/DesktopPage.xaml");

impl DesktopPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), dock_menu_item_count: Cell::new(0) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    fn dock_menu_item_count_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("DockMenuItemCount")
    }

    /// # Panics
    /// Panics if there is no application or the application has no tray icon (a null reference
    /// in the managed original).
    fn toggle_tray_icon_visible_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let application = Application::current().expect("the current application");
        let icons = TrayIcon::get_icons(&application).expect("the tray icons of the application");
        let icon = icons.to_vec().into_iter().next().expect("the first tray icon of the application");
        icon.set_is_visible(!icon.is_visible());
    }

    /// # Panics
    /// Panics if the current application is not the application of the sample (an invalid cast
    /// in the managed original).
    fn add_dock_menu_item_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let app = Application::current()
            .and_then(|application| application.cast::<App>())
            .expect("the current application is the application of the sample");
        self.dock_menu_item_count.set(self.dock_menu_item_count.get() + 1);
        app.add_dock_menu_item(&format!("Dynamic Item {}", self.dock_menu_item_count.get()));
        self.dock_menu_item_count_text().set_text(Some(&app.get_dock_menu_item_count().to_string()));
    }
}
