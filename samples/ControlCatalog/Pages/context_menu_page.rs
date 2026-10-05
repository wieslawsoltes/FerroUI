//! Port of `Pages/ContextMenuPage.xaml.cs`: the class of the document
//! `Pages/ContextMenuPage.xaml`.

use crate::markup::xaml_class;
use crate::view_models::ContextPageViewModel;
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
use ferroui_base::input::ContextRequestedEventArgs;
use ferroui_base::interactivity::IRoutedEventArgs;
use ferroui_base::utilities::CancelEventArgs;
use ferroui_controls::{Border, CheckBox, TextBlock};
use std::cell::RefCell;
use std::rc::Rc;

#[repr(C)]
pub struct ContextMenuPage {
    base: ContentPage,
    model: RefCell<Option<Rc<ContextPageViewModel>>>,
}

ferro_class!(ContextMenuPage: ContentPage);
ferro_impl_classes!(
    ContextMenuPage: FerroObjectImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    PageImpl
);
ferro_class_info!(ContextMenuPage {
    new: ContextMenuPage::new,
    markup: {
        methods: [
            fn ContextFlyoutPage_Closing(Option<BoxedValue>, CancelEventArgs) =>
                |this: &Ref<ContextMenuPage>, sender: Option<BoxedValue>, e: CancelEventArgs| {
                    this.context_flyout_page_closing(&sender, &e)
                },
            fn ContextFlyoutPage_Opening(Option<BoxedValue>, CancelEventArgs) =>
                |this: &Ref<ContextMenuPage>, sender: Option<BoxedValue>, e: CancelEventArgs| {
                    this.context_flyout_page_opening(&sender, &e)
                },
            fn CustomContextRequested(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ContextMenuPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<ContextRequestedEventArgs>() {
                        this.custom_context_requested(&sender, e)
                    }
                },
        ],
    },
});
xaml_class!(ContextMenuPage, "/Pages/ContextMenuPage.xaml");

impl StyledElementImpl for ContextMenuPage {
    fn on_data_context_changed(this: &Self) {
        // Taken out first: the view of a model is set without a borrow of the field held.
        let previous = this.model.borrow_mut().take();
        if let Some(model) = previous {
            model.set_view(None);
        }
        let model = from_markup_value::<Rc<ContextPageViewModel>>(&this.data_context());
        *this.model.borrow_mut() = model.clone();
        if let Some(model) = model {
            model.set_view(Some(this.to_ref().upcast()));
        }

        Self::parent_on_data_context_changed(this);
    }
}

impl ContextMenuPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct(), model: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_data_context(Some(ContextPageViewModel::new() as BoxedValue));
        this
    }

    fn cancel_close_check_box(&self) -> Option<Ref<CheckBox>> {
        self.find_control::<CheckBox>("CancelCloseCheckBox")
    }

    fn cancel_open_check_box(&self) -> Option<Ref<CheckBox>> {
        self.find_control::<CheckBox>("CancelOpenCheckBox")
    }

    fn context_flyout_page_closing(&self, _sender: &Option<BoxedValue>, e: &CancelEventArgs) {
        e.set_cancel(self.cancel_close_check_box().and_then(|check_box| check_box.is_checked()).unwrap_or(false));
    }

    fn context_flyout_page_opening(&self, _sender: &Option<BoxedValue>, e: &CancelEventArgs) {
        e.set_cancel(self.cancel_open_check_box().and_then(|check_box| check_box.is_checked()).unwrap_or(false));
    }

    pub fn custom_context_requested(&self, sender: &Option<BoxedValue>, e: &ContextRequestedEventArgs) {
        let border = from_markup_value::<Ref<Border>>(sender);
        let text_block = border.as_ref().and_then(|border| border.child()).and_then(|child| child.cast::<TextBlock>());
        if let (Some(border), Some(text_block)) = (border, text_block) {
            let text = match e.try_get_position(Some(&border)) {
                Some(point) => format!(
                    "Context was requested with pointer at: {}, {}",
                    format_n0(point.x),
                    format_n0(point.y)
                ),
                None => "Context was requested without pointer".to_string(),
            };
            text_block.set_text(Some(&text));
            e.set_handled(true);
        }
    }
}

/// `value.ToString("N0")` of the invariant culture: rounded to an integer
/// (halves away from zero), with a comma between the groups of thousands.
fn format_n0(value: f64) -> String {
    let rounded = value.abs().round();
    let digits = format!("{rounded:.0}");
    let mut text = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            text.push(',');
        }
        text.push(digit);
    }
    if value < 0.0 && rounded != 0.0 {
        text.insert(0, '-');
    }
    text
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn a_coordinate_is_shown_rounded_with_grouped_thousands() {
        assert_eq!("0", format_n0(0.4));
        assert_eq!("13", format_n0(12.5));
        assert_eq!("1,235", format_n0(1234.56));
        assert_eq!("-1,234,568", format_n0(-1234567.9));
        assert_eq!("0", format_n0(-0.2));
    }
}
