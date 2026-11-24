//! The code behind of the motion pages of the gallery, in Rust.

use ferroui_base::controls::ResourceKey;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::media::IImage;
use ferroui_base::{BoxedValue, FerroObject, Ref};
use ferroui_controls::animation::{
    BackEase, BounceEase, CircleEase, CubicEase, DoubleAnimation, EasingFunctionBase, EasingMode, ElasticEase,
    ExponentialEase, PowerEase, QuadraticEase, QuarticEase, QuinticEase, SineEase, Storyboard,
};
use ferroui_base::media::TranslateTransform;
use ferroui_controls::primitives::SelectingItemsControl;
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::{Button, ComboBox, Control, Grid, Image, ItemsSource, ListView, NumberBox, RadioButton};
use std::rc::Rc;

/// The image both examples of the ParallaxView page move.
pub(crate) const PARALLAX_IMAGE: &str = "ferres://WinUIGallery/Assets/SampleMedia/cliff.jpg";

pub(crate) fn wire(page_id: &str, root: &Ref<Control>) {
    match page_id {
        "EasingFunction" => wire_easing_functions(root),
        "ParallaxView" => wire_parallax_view(root),
        _ => {}
    }
}

/// The image of both examples, and the items of the list: the source's `Items`, every control of
/// the catalog by title.
fn wire_parallax_view(root: &Ref<Control>) {
    match crate::assets::load_bitmap(PARALLAX_IMAGE) {
        Ok(bitmap) => {
            let image: Rc<dyn IImage> = Rc::new(bitmap);
            for name in ["ListParallaxImage", "ScrollParallaxImage"] {
                root.get_control::<Image>(name).set_source(Some(image.clone()));
            }
        }
        Err(error) => {
            if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::VISUAL) {
                logger.log(None, &format!("The image of the ParallaxView page was not loaded: {error}"));
            }
        }
    }
    let mut titles: Vec<&str> = crate::catalog_data::CATALOG.iter().map(|entry| entry.title).collect();
    titles.sort_unstable();
    let items = titles.into_iter().map(|title| Some(Rc::new(title.to_owned()) as BoxedValue));
    root.get_control::<ListView>("ParallaxList").set_items_source(Some(ItemsSource::from_items(items)));
}

/// The storyboard of an example, with its animation aimed at the transform of the rectangle.
fn example(root: &Ref<Control>, index: u32) -> Option<(Ref<Storyboard>, Ref<DoubleAnimation>, Ref<TranslateTransform>)> {
    let grid = root.find_control::<Control>(&format!("Grid{index}"))?.cast::<Grid>()?;
    let key = ResourceKey::from(format!("Storyboard{index}").as_str());
    let value = grid.resources().try_get_resource(&key, None).flatten()?;
    let storyboard = ValueTypes::as_object(&*value)?.cast::<Storyboard>()?;
    let animation = storyboard.children().get(0).cast::<DoubleAnimation>()?;
    let rectangle = root.find_control::<Control>(&format!("Rectangle{index}"))?.cast::<Rectangle>()?;
    let transform = rectangle.render_transform()?.as_object()?.to_ref().cast::<TranslateTransform>()?;
    Storyboard::set_target(&animation, transform.upcast_ref::<FerroObject>());
    Some((storyboard, animation, transform))
}

/// `From` is where the rectangle is and `To` the other end, as in `Button1_Click` of the source.
fn run(storyboard: &Storyboard, animation: &DoubleAnimation, transform: &TranslateTransform) {
    let x = transform.x();
    animation.set_from(Some(x));
    animation.set_to(Some(if x > 0.0 { 0.0 } else { 200.0 }));
    storyboard.begin();
}

/// The easing function of an entry of the combo box of the fourth example.
pub(crate) fn easing_function(index: usize) -> Option<Ref<EasingFunctionBase>> {
    Some(match index {
        0 => BackEase::new().upcast(),
        1 => BounceEase::new().upcast(),
        2 => CircleEase::new().upcast(),
        3 => CubicEase::new().upcast(),
        4 => ElasticEase::new().upcast(),
        5 => ExponentialEase::new().upcast(),
        6 => PowerEase::new().upcast(),
        7 => QuadraticEase::new().upcast(),
        8 => QuarticEase::new().upcast(),
        9 => QuinticEase::new().upcast(),
        10 => SineEase::new().upcast(),
        _ => return None,
    })
}

fn is_checked(root: &Ref<Control>, name: &str) -> bool {
    root.find_control::<Control>(name)
        .and_then(|control| control.cast::<RadioButton>())
        .is_some_and(|radio| radio.is_checked() == Some(true))
}

/// `GetEaseValue` of the source.
fn ease_value(root: &Ref<Control>) -> EasingMode {
    if is_checked(root, "EaseOutRadio") {
        EasingMode::EaseOut
    } else if is_checked(root, "EaseInRadio") {
        EasingMode::EaseIn
    } else {
        EasingMode::EaseInOut
    }
}

/// The exponent of an example follows its number box, as the `x:Bind` of the source.
fn bind_exponent(root: &Ref<Control>, name: &str, animation: &Ref<DoubleAnimation>) {
    let Some(number_box) = root.find_control::<Control>(name).and_then(|control| control.cast::<NumberBox>()) else {
        return;
    };
    let weak = animation.downgrade();
    number_box.value_changed(move |args| {
        let ease = weak.upgrade().and_then(|animation| animation.easing_function()).and_then(|ease| ease.cast::<ExponentialEase>());
        if let Some(ease) = ease {
            if args.new_value().is_finite() {
                ease.set_exponent(args.new_value());
            }
        }
    })
    .detach();
}

fn wire_easing_functions(root: &Ref<Control>) {
    for index in 1..=4 {
        let Some((storyboard, animation, transform)) = example(root, index) else { continue };
        match index {
            2 => bind_exponent(root, "AccelerateEasingExponent", &animation),
            3 => bind_exponent(root, "DecelerateEasingExponent", &animation),
            _ => {}
        }
        let Some(button) = root.find_control::<Control>(&format!("Button{index}")).and_then(|control| control.cast::<Button>()) else {
            continue;
        };
        let weak_root = root.downgrade();
        button.click(move |_, _| {
            if index == 4 {
                // `Button4_Click`: the function and the mode of the options.
                let Some(root) = weak_root.upgrade() else { return };
                let selected = root
                    .find_control::<Control>("EasingComboBox")
                    .and_then(|control| control.cast::<ComboBox>())
                    .map_or(-1, |combo| combo.upcast_ref::<SelectingItemsControl>().selected_index());
                let Some(ease) = usize::try_from(selected).ok().and_then(easing_function) else { return };
                ease.set_easing_mode(ease_value(&root));
                animation.set_easing_function(Some(ease));
            }
            run(&storyboard, &animation, &transform);
        });
    }
}
