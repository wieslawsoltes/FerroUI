//! The PageTransition page's native Frame navigation and source sample-page layouts, with the code
//! behind of `PageTransitionPage.xaml.cs`. The transition modes are the
//! `NavigationTransitionInfo` of the navigations.

use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref,
    StyledElementImpl, VisualImpl,
};
use ferroui_controls::navigation::{NavigationError, NavigationTransitionInfo};
use ferroui_controls::primitives::{SelectingItemsControl, TemplatedControlImpl};
use ferroui_controls::{
    Button, ContentControlImpl, Control, ControlImpl, Frame, FramePage, FramePageImpl, RadioButtons,
    TextBlock,
};
use ferroui_markup_xaml::XamlLoadError;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::cell::Cell;
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
            /// The page type of the page, created by the page factory the sample registers.
            pub(crate) const PAGE_TYPE: ferroui_controls::PageType = ferroui_controls::PageType::for_factory::<$name>();

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

pub(crate) fn wire(root: &Ref<Control>) -> Result<(), XamlLoadError> {
    let Some(frame) = root
        .find_control::<Control>("ContentFrame")
        .and_then(|control| control.cast::<Frame>())
    else {
        return Ok(());
    };
    let status = root.get_control::<TextBlock>("NavigationStatus");
    frame
        .register_page_factory(FirstSamplePage::PAGE_TYPE, Rc::new(FirstSamplePage::create))
        .map_err(load_error)?;
    frame
        .register_page_factory(SecondSamplePage::PAGE_TYPE, Rc::new(SecondSamplePage::create))
        .map_err(load_error)?;
    let weak_frame = frame.downgrade();
    let weak_status = status.downgrade();
    frame.navigated(move |event| {
        if let (Some(frame), Some(status)) = (weak_frame.upgrade(), weak_status.upgrade()) {
            let page = if event.source_page_type() == FirstSamplePage::PAGE_TYPE {
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
    }).detach();
    let transition = Rc::new(Cell::new(None));
    for name in ["NavigationNext", "NavigationBack"] {
        let transition = transition.clone();
        let weak_frame = frame.downgrade();
        let weak_status = status.downgrade();
        root.get_control::<Button>(name).click(move |_, _| {
            if let (Some(frame), Some(status)) = (weak_frame.upgrade(), weak_status.upgrade()) {
                let result = match name {
                    // BackwardButton1_Click goes back while the back stack has a page.
                    "NavigationBack" if frame.back_stack_depth() == 0 => Ok(()),
                    "NavigationBack" => frame.go_back(),
                    _ => {
                        // The source sample alternates page types using its BackStackDepth.
                        let next = if frame.back_stack_depth() % 2 == 1 {
                            FirstSamplePage::PAGE_TYPE
                        } else {
                            SecondSamplePage::PAGE_TYPE
                        };
                        // ForwardButton1_Click: the kept transition, or the default navigation.
                        match transition.get() {
                            Some(info) => frame.navigate_with_transition(next, None, info).map(|_| ()),
                            None => frame.navigate(next, None).map(|_| ()),
                        }
                    }
                };
                if let Err(error) = result {
                    status.set_text(Some(&format!("Navigation failed: {error}")));
                }
            }
        });
    }
    wire_transition_modes(root, transition);
    frame
        .navigate(FirstSamplePage::PAGE_TYPE, None)
        .map_err(load_error)?;
    Ok(())
}

/// The code of the NavigationTransitionInfo that `TransitionRadioButton_Checked` keeps for the
/// mode of `content` (the source's TransitionParameter substitution; empty for Default).
pub(crate) fn transition_parameter(content: &str) -> String {
    match content {
        "Default" => String::new(),
        "Slide from Right" => {
            ", new SlideNavigationTransitionInfo() { Effect = SlideNavigationTransitionEffect.FromRight }".to_string()
        }
        "Slide from Left" => {
            ", new SlideNavigationTransitionInfo() { Effect = SlideNavigationTransitionEffect.FromLeft }".to_string()
        }
        other => format!(", new {other}NavigationTransitionInfo()"),
    }
}

/// The NavigationTransitionInfo of the mode of `content`; `None` for Default.
pub(crate) fn transition_info(content: &str) -> Option<NavigationTransitionInfo> {
    match content {
        "Entrance" => Some(NavigationTransitionInfo::Entrance),
        "DrillIn" => Some(NavigationTransitionInfo::DrillIn),
        "Suppress" => Some(NavigationTransitionInfo::Suppress),
        "Slide from Right" => Some(NavigationTransitionInfo::SlideFromRight),
        "Slide from Left" => Some(NavigationTransitionInfo::SlideFromLeft),
        _ => None,
    }
}

/// `TransitionRadioButton_Checked`: the checked mode's NavigationTransitionInfo, which the next
/// navigation plays.
fn wire_transition_modes(root: &Ref<Control>, transition: Rc<Cell<Option<NavigationTransitionInfo>>>) {
    let modes = root.get_control::<RadioButtons>("TransitionModes");
    let weak_modes = modes.downgrade();
    let weak_value = root.get_control::<TextBlock>("TransitionValue").downgrade();
    modes.property_changed(move |change| {
        if change.property() != SelectingItemsControl::selected_index_property().as_property() {
            return;
        }
        let (Some(modes), Some(value)) = (weak_modes.upgrade(), weak_value.upgrade()) else {
            return;
        };
        let content = modes
            .selected_item()
            .and_then(|item| Control::from_boxed(&item))
            .and_then(|item| item.cast::<ferroui_controls::ContentControl>())
            .and_then(|item| item.content())
            .and_then(|content| content.downcast_ref::<String>().cloned())
            .unwrap_or_default();
        value.set_text(Some(&transition_parameter(&content)));
        transition.set(transition_info(&content));
    });
}

fn load_error(error: NavigationError) -> XamlLoadError {
    XamlLoadError::with_inner("Could not initialize the navigation example.", error)
}
