//! A control inside the template of a control theme for one of its base classes, from compiled
//! markup: the setters of the theme are for the control it is applied to.

use crate::emitter::generated;
use crate::support::app::styled_window_application;
use ferroui_controls::presenters::TextPresenter;
use ferroui_controls::testing::MockWindowingPlatform;
use ferroui_controls::{Control, TextBox, Window};
use ferroui_markup_xaml::xaml_il::runtime::XamlIlRuntimeHelpers;

#[test]
fn compiled_text_box_in_a_base_class_theme_keeps_its_own_template() {
    let _app = styled_window_application();
    let provider = XamlIlRuntimeHelpers::create_root_service_provider_v3(None);
    let root = generated::build_control_theme_base_class_template_xaml(Some(provider)).unwrap();
    let window = Window::with_impl(MockWindowingPlatform::create_window_mock_with_size(640.0, 480.0));
    window.set_content(Some(Control::boxed(&root)));
    window.show();
    window.update_layout();

    let text_box = root
        .get_visual_descendants()
        .find_map(|descendant| descendant.cast::<TextBox>())
        .expect("the text box of the template");
    assert!(text_box.is_tab_stop());
    assert!(text_box.get_visual_descendants().any(|descendant| descendant.cast::<TextPresenter>().is_some()));
}
