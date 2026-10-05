//! Port of `Pages/TextBox/TextBoxValidationPage.xaml.cs`: the class of the
//! document `Pages/TextBox/TextBoxValidationPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::models::Person;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct TextBoxValidationPage {
    base: UserControl,
}

user_control_class!(TextBoxValidationPage);
ferro_class_info!(TextBoxValidationPage { new: TextBoxValidationPage::new });
xaml_class!(TextBoxValidationPage, "/Pages/TextBox/TextBoxValidationPage.xaml");

impl TextBoxValidationPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        let person = Person::new();
        person.set_first_name(String::from("John"));
        person.set_last_name(String::from("Doe"));
        this.set_data_context(Some(person as BoxedValue));
        this
    }
}
