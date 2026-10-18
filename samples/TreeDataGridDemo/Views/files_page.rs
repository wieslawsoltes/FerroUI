//! The files page: the tab "Files" of the main window document of the reference demo, with the
//! handler of its path box from the code-behind of the main window (`SelectedPath_KeyDown`), as a
//! view of its own that the main window and the ControlCatalog both show.

use crate::markup::{user_control_class, xaml_class};
use crate::view_models::FilesPageViewModel;
use ferroui_base::input::{Key, KeyEventArgs};
use ferroui_base::interactivity::IRoutedEventArgs;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{TextBox, UserControl};
use std::rc::Rc;

#[repr(C)]
pub struct FilesPage {
    base: UserControl,
}

user_control_class!(FilesPage);
ferro_class_info!(FilesPage {
    new: FilesPage::new,
    markup: {
        methods: [
            fn SelectedPath_KeyDown(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<FilesPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<KeyEventArgs>() {
                        this.selected_path_key_down(&sender, e)
                    }
                },
        ],
    },
});
xaml_class!(FilesPage, "/Views/FilesPage.xaml");

impl FilesPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    fn view_model(&self) -> Option<Rc<FilesPageViewModel>> {
        from_markup_value::<Rc<FilesPageViewModel>>(&self.data_context())
    }

    /// `SelectedPath_KeyDown`: Enter selects the entry of the path typed in the box.
    pub fn selected_path_key_down(&self, sender: &Option<BoxedValue>, e: &KeyEventArgs) {
        if e.key == Key::Enter {
            let Some(vm) = self.view_model() else {
                return;
            };
            let Some(text_box) = from_markup_value::<Ref<TextBox>>(sender) else {
                return;
            };
            vm.set_selected_path(text_box.text());
        }
    }
}
