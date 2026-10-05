//! Port of `Pages/MenuPage.xaml.cs`: the class of the document
//! `Pages/MenuPage.xaml`.

use crate::markup::xaml_class;
use crate::view_models::MenuPageViewModel;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl,
    StyledElementImplExt, VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ContentPage, ControlImpl, PageImpl};
use std::cell::RefCell;
use std::rc::Rc;

#[repr(C)]
pub struct MenuPage {
    base: ContentPage,
    model: RefCell<Option<Rc<MenuPageViewModel>>>,
}

ferro_class!(MenuPage: ContentPage);
ferro_impl_classes!(
    MenuPage: FerroObjectImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    PageImpl
);
ferro_class_info!(MenuPage { new: MenuPage::new });
xaml_class!(MenuPage, "/Pages/MenuPage.xaml");

impl StyledElementImpl for MenuPage {
    fn on_data_context_changed(this: &Self) {
        // Taken out first: the view of a model is set without a borrow of the field held.
        let previous = this.model.borrow_mut().take();
        if let Some(model) = previous {
            model.set_view(None);
        }
        let model = from_markup_value::<Rc<MenuPageViewModel>>(&this.data_context());
        *this.model.borrow_mut() = model.clone();
        if let Some(model) = model {
            model.set_view(Some(this.to_ref().upcast()));
        }

        Self::parent_on_data_context_changed(this);
    }
}

impl MenuPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct(), model: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_data_context(Some(MenuPageViewModel::new() as BoxedValue));
        this
    }
}
