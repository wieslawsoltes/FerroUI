//! Port of `Pages/TabbedPage/TabbedPagePlacementPage.xaml.cs`: the class of the document
//! `Pages/TabbedPage/TabbedPagePlacementPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{RadioButton, TabPlacement, TabbedPage, UserControl};
use std::cell::Cell;
use std::rc::Rc;

#[repr(C)]
pub struct TabbedPagePlacementPage {
    base: UserControl,
    /// Whether `InitializeComponent()` has returned: the fields of the named elements are
    /// null until then, which the handlers that run while the document loads test for.
    component_initialized: Cell<bool>,
}

user_control_class!(TabbedPagePlacementPage);
ferro_class_info!(TabbedPagePlacementPage {
    new: TabbedPagePlacementPage::new,
    markup: {
        methods: [
            fn OnPlacementChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPagePlacementPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_placement_changed(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(TabbedPagePlacementPage, "/Pages/TabbedPage/TabbedPagePlacementPage.xaml");

impl TabbedPagePlacementPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), component_initialized: Cell::new(false) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.component_initialized.set(true);
        this
    }

    /// `radio?.IsChecked == true` of a named radio button (its field is null until
    /// `InitializeComponent()` has returned).
    fn is_checked(&self, name: &str) -> bool {
        self.get_control::<RadioButton>(name).is_checked() == Some(true)
    }

    fn on_placement_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        if !self.component_initialized.get() {
            return;
        }
        let demo_tabs = self.get_control::<TabbedPage>("DemoTabs");
        if self.is_checked("TopRadio") {
            demo_tabs.set_tab_placement(TabPlacement::Top);
        } else if self.is_checked("BottomRadio") {
            demo_tabs.set_tab_placement(TabPlacement::Bottom);
        } else if self.is_checked("LeftRadio") {
            demo_tabs.set_tab_placement(TabPlacement::Left);
        } else if self.is_checked("RightRadio") {
            demo_tabs.set_tab_placement(TabPlacement::Right);
        }
    }
}
