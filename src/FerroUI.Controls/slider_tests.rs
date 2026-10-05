use crate::test_support::test_scope;
use crate::Slider;
use ferroui_base::layout::Orientation;

#[test]
fn default_orientation_should_be_horizontal() {
    let _scope = test_scope();
    let slider = Slider::new();
    assert_eq!(Orientation::Horizontal, slider.orientation());
}

#[test]
fn should_set_horizontal_class() {
    let _scope = test_scope();
    let slider = Slider::new();
    slider.set_orientation(Orientation::Horizontal);

    assert!(slider.classes().contains(":horizontal"));
}

#[test]
fn should_set_vertical_class() {
    let _scope = test_scope();
    let slider = Slider::new();
    slider.set_orientation(Orientation::Vertical);

    assert!(slider.classes().contains(":vertical"));
}
