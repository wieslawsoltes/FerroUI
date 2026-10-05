//! Port of the tests of the reference `PopupTests` that host a popup in the
//! template of an items control:
//! `ItemsControl_With_Popup_In_Template_Should_Set_TemplatedParent` and
//! `Should_Not_Overwrite_TemplatedParent_Of_Item_In_ItemsControl_With_Popup_On_Second_Open`.
//!
//! As in `popup_tests.rs`, every test body takes the `UsePopupHost` flag of
//! the reference and is run by both variants of the suite (`PopupTests`:
//! the platform creates native popups; `PopupTestsWithPopupRoot`: the
//! platform creates none, so popups are shown in the overlay layer).

use super::{Popup, TemplatedControlImpl};
use crate::platform::{IPopupImpl, ITopLevelImpl, IWindowImpl};
use crate::presenters::ItemsPresenter;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::testing::{MockWindowingPlatform, TestServices, UnitTestApplication, UnitTestApplicationScope};
use crate::{Border, Control, ControlImpl, ItemsControl, ItemsControlImpl, Window};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::*;
use std::rc::Rc;

fn create_services(use_popup_host: bool) -> UnitTestApplicationScope {
    UnitTestApplication::start(
        TestServices::styled_window().with_windowing_platform(create_mock_windowing_platform(use_popup_host)),
    )
}

fn create_window_impl(use_popup_host: bool) -> Rc<dyn IWindowImpl> {
    let mock = MockWindowingPlatform::create_window_mock();

    let weak = Rc::downgrade(&mock);
    mock.setup_create_popup(move |_| {
        if use_popup_host {
            return None;
        }
        let parent: Rc<dyn ITopLevelImpl> = weak.upgrade()?;
        Some(create_popup_mock(parent))
    });

    mock
}

fn create_mock_windowing_platform(use_popup_host: bool) -> Rc<MockWindowingPlatform> {
    MockWindowingPlatform::with_window_impl(move || create_window_impl(use_popup_host))
}

fn create_popup_mock(parent: Rc<dyn ITopLevelImpl>) -> Rc<dyn IPopupImpl> {
    let mock = MockWindowingPlatform::create_popup_mock(parent);

    let weak = Rc::downgrade(&mock);
    mock.setup_create_popup(move |_| {
        let parent: Rc<dyn ITopLevelImpl> = weak.upgrade()?;
        Some(create_popup_mock(parent))
    });

    mock
}

fn prepared_window(content: Option<Ref<Control>>) -> Ref<Window> {
    let w = Window::new();
    w.set_content(content.map(Control::boxed));
    w.show();
    w.apply_styling();
    w.apply_template();
    w
}

fn host_control(popup: &Popup) -> Ref<Control> {
    popup.host().expect("the popup is open").as_control()
}

fn popup_items_control_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(|control, scope| {
        let popup = Popup::new();
        popup.set_name(Some("popup".to_string()));
        popup.set_placement_target(control.clone());
        popup.set_child(ItemsPresenter::new());
        popup.register_in_name_scope(&**scope).upcast()
    })
}

#[repr(C)]
struct PopupItemsControl {
    base: ItemsControl,
}

ferro_class!(PopupItemsControl: ItemsControl);
ferro_impl_classes!(
    PopupItemsControl: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ItemsControlImpl
);

impl PopupItemsControl {
    fn new() -> Ref<Self> {
        instantiate(Self { base: ItemsControl::construct() })
    }
}

/// The popup named "popup" in the template of `target`.
fn template_popup(target: &PopupItemsControl) -> Ref<Popup> {
    target
        .get_template_descendants()
        .into_iter()
        .find(|x| x.name().as_deref() == Some("popup"))
        .and_then(|x| x.cast::<Popup>())
        .unwrap()
}

fn items_control_with_popup_in_template_should_set_templated_parent(use_popup_host: bool) {
    // Test uses OverlayPopupHost default template
    let _app = create_services(use_popup_host);
    let item = Border::new();
    let target = PopupItemsControl::new();
    target.items().add(Some(Control::boxed(&item)));
    target.set_template(Some(popup_items_control_template()));
    let root = prepared_window(Some(target.clone().upcast()));
    root.show();

    target.apply_template();

    let popup = template_popup(&target);
    popup.open();

    let popup_root = host_control(&popup);
    popup_root.measure(Size::INFINITY);
    popup_root.arrange(Rect::from_size(popup_root.desired_size()));

    let children: Vec<Ref<Visual>> = popup_root.get_visual_descendants().collect();
    let types: Vec<&str> = children.iter().map(|x| x.get_type().name()).collect();

    if use_popup_host {
        assert_eq!(
            vec![
                "LayoutTransformControl",
                "VisualLayerManager",
                "ContentPresenter",
                "ItemsPresenter",
                "StackPanel",
                "Border",
            ],
            types
        );
    } else {
        assert_eq!(
            vec![
                "LayoutTransformControl",
                "Panel",
                "Border",
                "VisualLayerManager",
                "ContentPresenter",
                "ItemsPresenter",
                "StackPanel",
                "Border",
            ],
            types
        );
    }

    let templated_parents: Vec<Option<Ref<FerroObject>>> =
        children.iter().filter_map(|x| x.cast::<Control>()).map(|x| x.templated_parent()).collect();

    let popup_root: Option<Ref<FerroObject>> = Some(popup_root.upcast());
    let target: Option<Ref<FerroObject>> = Some(target.upcast());

    if use_popup_host {
        assert_eq!(
            vec![popup_root.clone(), popup_root.clone(), popup_root, target.clone(), target, None],
            templated_parents
        );
    } else {
        assert_eq!(
            vec![
                popup_root.clone(),
                popup_root.clone(),
                popup_root.clone(),
                popup_root.clone(),
                popup_root,
                target.clone(),
                target,
                None,
            ],
            templated_parents
        );
    }
}

fn should_not_overwrite_templated_parent_of_item_in_items_control_with_popup_on_second_open(use_popup_host: bool) {
    // Test uses OverlayPopupHost default template
    let _app = create_services(use_popup_host);
    let item = Border::new();
    let target = PopupItemsControl::new();
    target.items().add(Some(Control::boxed(&item)));
    target.set_template(Some(popup_items_control_template()));
    let root = prepared_window(Some(target.clone().upcast()));
    root.show();

    target.apply_template();

    let popup = template_popup(&target);
    popup.open();

    let popup_root = host_control(&popup);
    popup_root.measure(Size::INFINITY);
    popup_root.arrange(Rect::from_size(popup_root.desired_size()));

    assert!(item.templated_parent().is_none());

    popup.close();
    popup.open();

    assert!(item.templated_parent().is_none());
}

macro_rules! popup_tests {
    ($($name:ident,)*) => {
        /// The reference `PopupTests`: the platform creates native popups.
        mod popup_tests {
            $(
                #[test]
                fn $name() {
                    super::$name(false);
                }
            )*
        }

        /// The reference `PopupTestsWithPopupRoot`: the platform creates no
        /// popups, so they are shown in the overlay layer.
        mod popup_tests_with_popup_root {
            $(
                #[test]
                fn $name() {
                    super::$name(true);
                }
            )*
        }
    };
}

popup_tests! {
    items_control_with_popup_in_template_should_set_templated_parent,
    should_not_overwrite_templated_parent_of_item_in_items_control_with_popup_on_second_open,
}
