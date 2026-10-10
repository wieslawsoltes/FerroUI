//! Port of `Pages/ComboBoxPage.xaml.cs`: the class of the document `Pages/ComboBoxPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{ComboBox, UserControl};
use std::rc::Rc;

#[repr(C)]
pub struct ComboBoxPage {
    base: UserControl,
}

user_control_class!(ComboBoxPage);
ferro_class_info!(ComboBoxPage {
    new: ComboBoxPage::new,
    markup: {
        methods: [
            fn ComboBoxSelectionClear_Click(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ComboBoxPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.combo_box_selection_clear_click(&sender, e.as_routed_event_args())
                },
            fn ComboBoxSelectFirst_Click(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ComboBoxPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.combo_box_select_first_click(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(ComboBoxPage, "/Pages/ComboBoxPage.xaml");

impl ComboBoxPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    fn basic_combo_box(&self) -> Ref<ComboBox> {
        self.get_control::<ComboBox>("BasicComboBox")
    }

    fn combo_box_selection_clear_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.basic_combo_box().set_selected_index(-1);
    }

    fn combo_box_select_first_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.basic_combo_box().set_selected_index(0);
    }
}
