//! Port of `Pages/LabelsPage.xaml.cs`: the class of the document
//! `Pages/LabelsPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use crate::models::Person;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::ContentPage;
use std::cell::RefCell;
use std::rc::Rc;

#[repr(C)]
pub struct LabelsPage {
    base: ContentPage,
    person: RefCell<Option<Rc<Person>>>,
}

content_page_class!(LabelsPage);
ferro_class_info!(LabelsPage {
    new: LabelsPage::new,
    markup: {
        methods: [
            fn DoSave() => |this: &Ref<LabelsPage>| this.do_save(),
            fn DoCancel() => |this: &Ref<LabelsPage>| this.do_cancel(),
        ],
    },
});
xaml_class!(LabelsPage, "/Pages/LabelsPage.xaml");

impl LabelsPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct(), person: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.create_default_person();
        this.initialize_component();
        this
    }

    fn create_default_person(&self) {
        let person = Person::new();
        person.set_first_name(String::from("John"));
        person.set_last_name(String::from("Doe"));
        person.set_is_banned(true);
        *self.person.borrow_mut() = Some(person.clone());
        self.set_data_context(Some(person as BoxedValue));
    }

    pub fn do_save(&self) {}

    pub fn do_cancel(&self) {
        self.create_default_person();
    }
}
