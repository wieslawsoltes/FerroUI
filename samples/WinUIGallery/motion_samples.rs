//! The code behind of the motion pages of the gallery, in Rust.

use ferroui_base::controls::ResourceKey;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Brushes, Colors, IImage};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{BoxedValue, FerroObject, Ref, Thickness, Vector3D};
use ferroui_controls::animation::{
    BackEase, BounceEase, CircleEase, CubicEase, DoubleAnimation, EasingFunctionBase, EasingMode, ElasticEase,
    ExponentialEase, PowerEase, QuadraticEase, QuarticEase, QuinticEase, SineEase, Storyboard,
};
use ferroui_base::media::TranslateTransform;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::{Popup, SelectingItemsControl};
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::{
    Button, ComboBox, Control, Grid, Image, ItemsSource, ListView, NumberBox, RadioButton, StackPanel, ThemeVariantScope,
};
use ferroui_controls_motion::ImplicitTransitions;
use std::cell::Cell;
use std::rc::Rc;

/// The image both examples of the ParallaxView page move.
pub(crate) const PARALLAX_IMAGE: &str = "ferres://WinUIGallery/Assets/SampleMedia/cliff.jpg";

pub(crate) fn wire(page_id: &str, root: &Ref<Control>) {
    match page_id {
        "EasingFunction" => wire_easing_functions(root),
        "ParallaxView" => wire_parallax_view(root),
        "ImplicitTransition" => wire_implicit_transitions(root),
        "ThemeTransition" => wire_theme_transitions(root),
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

/// `button` runs `apply` with the page and the value of its number box.
fn on_click(root: &Ref<Control>, button: &str, number: &str, apply: fn(&Ref<Control>, f64)) {
    let weak = root.downgrade();
    let number = number.to_owned();
    root.get_control::<Button>(button).click(move |_, _| {
        if let Some(root) = weak.upgrade() {
            let value = root.get_control::<NumberBox>(&number).value();
            if value.is_finite() {
                apply(&root, value);
            }
        }
    });
}

/// The handlers of the ImplicitTransition page: each sets the facade whose transition animates.
fn wire_implicit_transitions(root: &Ref<Control>) {
    on_click(root, "OpacityButton", "OpacityNumberBox", |root, value| {
        root.get_control::<Rectangle>("OpacityRectangle").set_opacity(value);
    });
    on_click(root, "RotationButton", "RotationNumberBox", |root, value| {
        ImplicitTransitions::set_rotation(&root.get_control::<Rectangle>("RotationRectangle"), value);
    });
    on_click(root, "ScaleButton", "ScaleNumberBox", |root, value| {
        ImplicitTransitions::set_scale(&root.get_control::<Rectangle>("ScaleRectangle"), Vector3D::new(value, value, value));
    });
    on_click(root, "TranslationButton", "TranslationNumberBox", |root, value| {
        root.get_control::<Rectangle>("TranslationRectangle").set_translation(Vector3D::new(value, value, value));
    });
    // A new brush each time, as the source does: not a new color on the same brush.
    let weak = root.downgrade();
    root.get_control::<Button>("BrushButton").click(move |_, _| {
        let Some(root) = weak.upgrade() else { return };
        let presenter = root.get_control::<ContentPresenter>("BrushPresenter");
        let is_blue = presenter
            .background()
            .and_then(|brush| brush.as_solid_color_brush().map(|solid| solid.color()))
            .is_some_and(|color| color == Colors::BLUE);
        let next = if is_blue { Colors::YELLOW } else { Colors::BLUE };
        presenter.set_background(Some(Rc::new(ImmutableSolidColorBrush::new(next))));
    });
    let weak = root.downgrade();
    root.get_control::<Button>("ThemeButton").click(move |_, _| {
        let Some(root) = weak.upgrade() else { return };
        let scope = root.get_control::<ThemeVariantScope>("ThemeExampleScope");
        let is_dark = scope.requested_theme_variant().is_some_and(|theme| theme == ThemeVariant::dark());
        scope.set_requested_theme_variant(Some(if is_dark { ThemeVariant::light() } else { ThemeVariant::dark() }));
    });
}

fn list_item(text: String) -> Option<BoxedValue> {
    Some(Rc::new(text) as BoxedValue)
}

/// `button` runs `apply` with the page.
fn on_press(root: &Ref<Control>, button: &str, apply: impl Fn(&Ref<Control>) + 'static) {
    let weak = root.downgrade();
    root.get_control::<Button>(button).click(move |_, _| {
        if let Some(root) = weak.upgrade() {
            apply(&root);
        }
    });
}

/// The handlers of the five examples of the ThemeTransition page.
fn wire_theme_transitions(root: &Ref<Control>) {
    on_press(root, "EntranceButton", |root| {
        let panel = root.get_control::<StackPanel>("EntranceStackPanel");
        panel.children().clear();
        for _ in 0..5 {
            let rectangle = Rectangle::new();
            rectangle.set_width(50.0);
            rectangle.set_height(50.0);
            rectangle.set_margin(Thickness::uniform(5.0));
            rectangle.set_fill(Some(Brushes::light_blue()));
            panel.children().add(rectangle.upcast::<Control>());
        }
    });
    on_press(root, "RepositionButton", |root| {
        let left = root.get_control::<Rectangle>("LeftRectangle");
        left.set_width(if left.width() == 100.0 { 200.0 } else { 100.0 });
    });
    let refresh = |root: &Ref<Control>| {
        let items = root.get_control::<ListView>("ContentList").items();
        items.clear();
        for index in 1..=4 {
            items.add(list_item(format!("Item {index}")));
        }
    };
    refresh(root);
    on_press(root, "ContentButton", refresh);
    let next = Rc::new(Cell::new(1));
    on_press(root, "AddItemButton", move |root| {
        let value = next.replace(next.get() + 1);
        root.get_control::<ListView>("AddDeleteList").items().add(list_item(format!("Item {value}")));
    });
    on_press(root, "RemoveItemButton", |root| {
        let items = root.get_control::<ListView>("AddDeleteList").items();
        if items.count() > 0 {
            items.remove_at(items.count() - 1);
        }
    });
    on_press(root, "OpenPopupButton", |root| root.get_control::<Popup>("ExamplePopup").set_is_open(true));
    on_press(root, "ClosePopupButton", |root| root.get_control::<Popup>("ExamplePopup").set_is_open(false));
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
