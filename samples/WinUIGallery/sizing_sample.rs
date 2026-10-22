//! The original compact-sizing navigation and state copying, with local resources.
//! Installing a compact dictionary on one page never changes application density.

use ferroui_base::controls::ResourceDictionary;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref,
    StyledElementImpl, VisualImpl,
};
use ferroui_controls::navigation::NavigationError;
use ferroui_controls::primitives::{SelectingItemsControl, TemplatedControlImpl};
use ferroui_controls::{
    ContentControlImpl, Control, ControlImpl, DatePicker, Frame, FramePage, FramePageImpl,
    RadioButtons, StackPanel, TextBlock, TextBox,
};
use ferroui_markup_xaml::XamlLoadException;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::rc::Rc;

macro_rules! sizing_page {
    ($name:ident, $compact:expr) => {
        #[repr(C)]
        pub(crate) struct $name { base: FramePage }
        ferro_class!($name: FramePage);
        ferro_class_info!($name {});
        ferro_impl_classes!($name: FerroObjectImpl, StyledElementImpl, VisualImpl, LayoutableImpl,
            InteractiveImpl, InputElementImpl, ControlImpl, TemplatedControlImpl, ContentControlImpl, FramePageImpl);
        impl $name {
            fn create() -> Result<Ref<FramePage>, NavigationError> {
                let form = FerroRuntimeXamlLoader::parse_as::<Ref<StackPanel>>(include_str!("Pages/SizingForm.xaml"), None)
                    .map_err(|error| NavigationError::FactoryFailed(error.to_string()))?;
                let page = instantiate(Self { base: FramePage::construct() });
                if $compact {
                    // Reuse the actual native dictionary; its values remain scoped to
                    // this page and inherited by this form's themed controls.
                    let resources = FerroRuntimeXamlLoader::parse_as::<Ref<ResourceDictionary>>(
                        include_str!("../../src/FerroUI.Themes.WinUI/DensityStyles/Compact.xaml"), None,
                    ).map_err(|error| NavigationError::FactoryFailed(error.to_string()))?;
                    page.set_resources(resources);
                    form.set_spacing(8.0);
                    form.get_control::<TextBlock>("SizingHeader").set_text(Some("Compact Size"));
                }
                page.set_content(Some(Control::boxed(form)));
                Ok(page.upcast())
            }
        }
    };
}

sizing_page!(StandardSizingPage, false);
sizing_page!(CompactSizingPage, true);

pub(crate) fn form(frame: &Frame) -> Option<Ref<Control>> {
    frame
        .current_page()?
        .content()
        .as_ref()
        .and_then(Control::from_boxed)
}

fn switch(frame: &Frame, compact: bool) -> Result<(), NavigationError> {
    let previous = form(frame);
    let target = if compact {
        CompactSizingPage::TYPE
    } else {
        StandardSizingPage::TYPE
    };
    if frame.current_source_page_type() == Some(target) {
        return Ok(());
    }
    if frame.navigate(target, None)? {
        if let (Some(previous), Some(next)) = (previous, form(frame)) {
            for name in ["FirstName", "LastName", "Password", "ConfirmPassword"] {
                next.get_control::<TextBox>(name)
                    .set_text(previous.get_control::<TextBox>(name).text().as_deref());
            }
            next.get_control::<DatePicker>("ChosenDate")
                .set_selected_date(
                    previous
                        .get_control::<DatePicker>("ChosenDate")
                        .selected_date(),
                );
        }
    }
    Ok(())
}

pub(crate) fn wire(root: &Ref<Control>) -> Result<(), XamlLoadException> {
    let Some(frame) = root
        .find_control::<Control>("SizingFrame")
        .and_then(|control| control.cast::<Frame>())
    else {
        return Ok(());
    };
    let convert_error =
        |error| XamlLoadException::with_inner("Could not initialize the sizing example.", error);
    frame
        .register_page_factory(
            StandardSizingPage::TYPE,
            Rc::new(StandardSizingPage::create),
        )
        .map_err(convert_error)?;
    frame
        .register_page_factory(CompactSizingPage::TYPE, Rc::new(CompactSizingPage::create))
        .map_err(convert_error)?;
    switch(&frame, false).map_err(convert_error)?;
    let options = root.get_control::<RadioButtons>("SizingMode");
    let weak_options = options.downgrade();
    let weak_frame = frame.downgrade();
    let weak_error = root.get_control::<TextBlock>("SizingError").downgrade();
    options.property_changed(move |change| {
        if change.property() != SelectingItemsControl::selected_index_property().as_property() {
            return;
        }
        if let (Some(frame), Some(options), Some(output)) = (
            weak_frame.upgrade(),
            weak_options.upgrade(),
            weak_error.upgrade(),
        ) {
            match options.selected_index() {
                0 | 1 => match switch(&frame, options.selected_index() == 1) {
                    Ok(()) => {
                        output.set_text(None);
                        output.set_is_visible(false);
                    }
                    Err(error) => {
                        output.set_text(Some(&error.to_string()));
                        output.set_is_visible(true);
                    }
                },
                _ => {}
            }
        }
    });
    Ok(())
}
