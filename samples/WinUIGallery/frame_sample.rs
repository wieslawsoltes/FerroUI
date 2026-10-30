//! The PageTransition page's native Frame navigation and source sample-page layouts.
//! Original transition effects are not substituted with unrelated animations.

use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref,
    StyledElementImpl, VisualImpl,
};
use ferroui_controls::navigation::NavigationError;
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    Button, ContentControlImpl, Control, ControlImpl, Frame, FramePage, FramePageImpl,
    TextBlock,
};
use ferroui_markup_xaml::XamlLoadException;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::rc::Rc;

macro_rules! sample_page {
    ($name:ident, $source:literal) => {
        #[repr(C)]
        pub(crate) struct $name { base: FramePage }
        ferro_class!($name: FramePage);
        ferro_class_info!($name {});
        ferro_impl_classes!($name: FerroObjectImpl, StyledElementImpl, VisualImpl, LayoutableImpl,
            InteractiveImpl, InputElementImpl, ControlImpl, TemplatedControlImpl, ContentControlImpl,
            FramePageImpl);
        impl $name {
            fn create() -> Result<Ref<FramePage>, NavigationError> {
                let content = FerroRuntimeXamlLoader::parse_as::<Ref<Control>>(include_str!($source), None)
                    .map_err(|error| NavigationError::FactoryFailed(error.to_string()))?;
                let page = instantiate(Self { base: FramePage::construct() });
                page.set_content(Some(Rc::new(content)));
                Ok(page.upcast())
            }
        }
    };
}

sample_page!(FirstSamplePage, "Pages/NavigationSample1.xaml");
sample_page!(SecondSamplePage, "Pages/NavigationSample2.xaml");

pub(crate) fn wire(root: &Ref<Control>) -> Result<(), XamlLoadException> {
    let Some(frame) = root
        .find_control::<Control>("ContentFrame")
        .and_then(|control| control.cast::<Frame>())
    else {
        return Ok(());
    };
    let status = root.get_control::<TextBlock>("NavigationStatus");
    frame
        .register_page_factory(FirstSamplePage::TYPE, Rc::new(FirstSamplePage::create))
        .map_err(load_error)?;
    frame
        .register_page_factory(SecondSamplePage::TYPE, Rc::new(SecondSamplePage::create))
        .map_err(load_error)?;
    let weak_frame = frame.downgrade();
    let weak_status = status.downgrade();
    frame.navigated(move |event| {
        if let (Some(frame), Some(status)) = (weak_frame.upgrade(), weak_status.upgrade()) {
            let page = if event.source_page_type() == FirstSamplePage::TYPE {
                "First"
            } else {
                "Second"
            };
            status.set_text(Some(&format!(
                "{page} page · {:?} · {} back entries",
                event.navigation_mode(),
                frame.back_stack_depth()
            )));
        }
    });
    for name in ["NavigationNext", "NavigationBack", "NavigationForward"] {
        let weak_frame = frame.downgrade();
        let weak_status = status.downgrade();
        root.get_control::<Button>(name).click(move |_, _| {
            if let (Some(frame), Some(status)) = (weak_frame.upgrade(), weak_status.upgrade()) {
                let result = match name {
                    "NavigationBack" => frame.go_back(),
                    "NavigationForward" => frame.go_forward(),
                    _ => {
                        // The source sample alternates page types using its BackStackDepth.
                        let next = if frame.back_stack_depth() % 2 == 1 {
                            FirstSamplePage::TYPE
                        } else {
                            SecondSamplePage::TYPE
                        };
                        frame.navigate(next, None).map(|_| ())
                    }
                };
                if let Err(error) = result {
                    status.set_text(Some(&format!("Navigation failed: {error}")));
                }
            }
        });
    }
    frame
        .navigate(FirstSamplePage::TYPE, None)
        .map_err(load_error)?;
    Ok(())
}

fn load_error(error: NavigationError) -> XamlLoadException {
    XamlLoadException::with_inner("Could not initialize the navigation example.", error)
}
