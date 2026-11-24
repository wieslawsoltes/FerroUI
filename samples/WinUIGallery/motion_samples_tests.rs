//! The motion pages: the storyboards of their examples run on the real controls.

use crate::motion_samples::easing_function;
use crate::{examples, tests::start};
use ferroui_base::animation::ClockState;
use ferroui_base::controls::ResourceKey;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::media::TranslateTransform;
use ferroui_base::platform::AssetLoader;
use ferroui_base::utilities::UriRef;
use ferroui_base::media::Colors;
use ferroui_base::numerics::Vector3;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{Ref, Vector3D};
use ferroui_controls::animation::{DoubleAnimation, EasingMode, ElasticEase, ExponentialEase, Storyboard};
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::{Button, ComboBox, Control, Grid, ListView, NumberBox, RadioButton};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::ThemeVariantScope;
use ferroui_controls_motion::{ImplicitTransitions, ParallaxView};

fn page() -> Ref<Control> {
    examples::create(examples::find("EasingFunction").unwrap()).unwrap()
}

fn storyboard(root: &Ref<Control>, index: u32) -> Ref<Storyboard> {
    let grid = root.get_control::<Grid>(&format!("Grid{index}"));
    let key = ResourceKey::from(format!("Storyboard{index}").as_str());
    let value = grid.resources().try_get_resource(&key, None).flatten().unwrap();
    ValueTypes::as_object(&*value).unwrap().cast::<Storyboard>().unwrap()
}

fn transform(root: &Ref<Control>, index: u32) -> Ref<TranslateTransform> {
    let rectangle = root.get_control::<Rectangle>(&format!("Rectangle{index}"));
    rectangle.render_transform().unwrap().as_object().unwrap().to_ref().cast::<TranslateTransform>().unwrap()
}

fn click(root: &Ref<Control>, index: u32) {
    let button = root.get_control::<Button>(&format!("Button{index}"));
    button.raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
}

#[test]
fn each_easing_example_runs_its_storyboard_to_the_other_end_and_back() {
    let _application = start();
    let root = page();
    for index in 1..=4 {
        let board = storyboard(&root, index);
        click(&root, index);
        assert_eq!(board.get_current_state(), ClockState::Active, "example {index}");
        board.skip_to_fill();
        assert_eq!(transform(&root, index).x(), 200.0, "example {index}");
    }
}

#[test]
fn the_exponents_follow_their_number_boxes() {
    let _application = start();
    let root = page();
    let accelerate = root.get_control::<NumberBox>("AccelerateEasingExponent");
    assert_eq!(accelerate.value(), 4.5);
    accelerate.set_value(NumberBox::value_property(), 2.0);
    let board = storyboard(&root, 2);
    let animation = board.children().get(0).cast::<DoubleAnimation>().unwrap();
    let ease = animation.easing_function().unwrap().cast::<ExponentialEase>().unwrap();
    assert_eq!(ease.exponent(), 2.0);
    click(&root, 2);
    board.skip_to_fill();
    assert_eq!(transform(&root, 2).x(), 200.0);
}

#[test]
fn the_fourth_example_takes_the_function_and_the_mode_of_its_options() {
    let _application = start();
    let root = page();
    for index in 0..11 {
        assert!(easing_function(index).is_some(), "{index}");
    }
    assert!(easing_function(11).is_none());
    root.get_control::<ComboBox>("EasingComboBox").set_selected_index(4);
    root.get_control::<RadioButton>("EaseInRadio").set_is_checked(Some(true));
    click(&root, 4);
    let board = storyboard(&root, 4);
    let animation = board.children().get(0);
    let animation = animation.cast::<DoubleAnimation>().unwrap();
    let ease = animation.easing_function().unwrap();
    assert!(ease.clone().cast::<ElasticEase>().is_some());
    assert_eq!(ease.easing_mode(), EasingMode::EaseIn);
}

#[test]
fn both_parallax_examples_follow_their_scrolling_source() {
    let _application = start();
    let root = examples::create(examples::find("ParallaxView").unwrap()).unwrap();
    // The test platform decodes no JPEG: the asset is checked, and the image is drawn by the
    // real backends.
    let image = UriRef::parse_absolute(crate::motion_samples::PARALLAX_IMAGE).unwrap();
    assert!(AssetLoader::exists(&UriRef::from(image), None));
    for (view, source, image) in
        [("ListParallax", "ParallaxList", "ListParallaxImage"), ("ScrollParallax", "ParallaxScroll", "ScrollParallaxImage")]
    {
        let parallax = root.get_control::<ParallaxView>(view);
        assert_eq!(parallax.vertical_shift(), 500.0, "{view}");
        let bound = parallax.source().expect(view);
        assert!(std::ptr::eq(&*bound, &*root.get_control::<Control>(source)), "{view}");
        let child = parallax.child().expect(view);
        assert!(std::ptr::eq(&*child, &*root.get_control::<Control>(image)), "{image}");
    }
    let list = root.get_control::<ListView>("ParallaxList");
    assert_eq!(list.item_count() as usize, crate::catalog_data::CATALOG.len());
}

fn press(root: &Ref<Control>, name: &str) {
    root.get_control::<Button>(name).raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
}

#[test]
fn each_implicit_transition_example_sets_its_facade_through_its_transition() {
    let _application = start();
    let root = examples::create(examples::find("ImplicitTransition").unwrap()).unwrap();

    let opacity = root.get_control::<Rectangle>("OpacityRectangle");
    assert!(opacity.opacity_transition().is_some());
    assert_eq!(opacity.opacity(), 0.5);
    press(&root, "OpacityButton");
    assert_eq!(opacity.opacity(), 1.0);

    let rotation = root.get_control::<Rectangle>("RotationRectangle");
    assert!(rotation.rotation_transition().is_some());
    press(&root, "RotationButton");
    assert_eq!(ImplicitTransitions::get_rotation(&rotation), 45.0);
    assert_eq!(rotation.rotation(), 45.0);

    let scale = root.get_control::<Rectangle>("ScaleRectangle");
    assert!(scale.scale_transition().is_some());
    press(&root, "ScaleButton");
    assert_eq!(scale.scale(), Vector3::new(2.0, 2.0, 2.0));

    let translation = root.get_control::<Rectangle>("TranslationRectangle");
    assert!(translation.translation_transition().is_some());
    press(&root, "TranslationButton");
    assert_eq!(translation.translation(), Vector3D::new(20.0, 20.0, 20.0));
}

#[test]
fn the_background_examples_change_their_brush_and_their_theme() {
    let _application = start();
    let root = examples::create(examples::find("ImplicitTransition").unwrap()).unwrap();
    let presenter = root.get_control::<ContentPresenter>("BrushPresenter");
    assert!(ImplicitTransitions::get_background_transition(&presenter).is_some());
    let color = || presenter.background().and_then(|brush| brush.as_solid_color_brush().map(|solid| solid.color()));
    assert_eq!(color(), Some(Colors::BLUE));
    press(&root, "BrushButton");
    assert_eq!(color(), Some(Colors::YELLOW));
    press(&root, "BrushButton");
    assert_eq!(color(), Some(Colors::BLUE));

    let grid = root.get_control::<Grid>("ThemeExampleGrid");
    assert!(ImplicitTransitions::get_background_transition(&grid).is_some());
    let scope = root.get_control::<ThemeVariantScope>("ThemeExampleScope");
    assert_eq!(scope.requested_theme_variant(), Some(ThemeVariant::dark()));
    press(&root, "ThemeButton");
    assert_eq!(scope.requested_theme_variant(), Some(ThemeVariant::light()));
    press(&root, "ThemeButton");
    assert_eq!(scope.requested_theme_variant(), Some(ThemeVariant::dark()));
}
