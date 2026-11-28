// Replaces the resource benchmark entry point for this experiment.
use ferroui_base::controls::ResourceKey;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::styling::{ControlTheme, Setter, SetterValue, ThemeVariant};
use ferroui_base::{FerroObject, Ref, Thickness};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::templates::IControlTemplate;
use ferroui_controls::{Button, Control};
use ferroui_markup_xaml::templates::ControlTemplate;
use ferroui_themes_winui::WinUITheme;
use std::{hint::black_box, rc::Rc, time::Instant};

fn resolve(mut theme: Ref<ControlTheme>) -> Rc<ControlTemplate> {
    loop {
        for setter in theme.setters().snapshot().iter() {
            let Some(setter) = setter.as_any().and_then(|s| s.downcast_ref::<Setter>()) else {
                continue;
            };
            if setter.property().map(|p| p.name()) != Some("Template") {
                continue;
            }
            let Some(SetterValue::Value(value)) = setter.value() else {
                continue;
            };
            let handle = value
                .downcast_ref::<Option<Rc<dyn IControlTemplate>>>()
                .cloned()
                .flatten()
                .or_else(|| from_markup_value::<Rc<dyn IControlTemplate>>(&Some(value)));
            if let Some(template) = handle.and_then(|h| ControlTemplate::from_control_template(&h))
            {
                return template;
            }
        }
        theme = theme.based_on().expect("button theme has a template");
    }
}

fn main() {
    let start = Instant::now();
    let theme = WinUITheme::new();
    let registration_ns = start.elapsed().as_nanos();
    let start = Instant::now();
    let value = theme
        .try_get_resource(
            &ResourceKey::Type(Button::TYPE),
            Some(&ThemeVariant::light()),
        )
        .flatten();
    let style = from_markup_value::<Ref<ControlTheme>>(&value).expect("button control theme");
    let template = resolve(style);
    let lookup_ns = start.elapsed().as_nanos();
    let button = Button::new();
    let make = || {
        let built = template
            .build(button.upcast_ref::<TemplatedControl>())
            .unwrap();
        assert_eq!(
            built.result().upcast_ref::<Control>().name().as_deref(),
            Some("PART_ContentPresenter")
        );
        black_box(built);
    };
    let start = Instant::now();
    make();
    let first_ns = start.elapsed().as_nanos();
    let start = Instant::now();
    for _ in 0..128 {
        make();
    }
    let repeated_ns = start.elapsed().as_nanos() / 128;
    let built = template
        .build(button.upcast_ref::<TemplatedControl>())
        .unwrap();
    let presenter = built.result().cast::<ContentPresenter>().unwrap();
    presenter.set_templated_parent(button.clone().upcast::<FerroObject>());
    for padding in [7.0, 13.0] {
        button.set_padding(Thickness::uniform(padding));
        assert_eq!(presenter.padding(), Thickness::uniform(padding));
    }
    for (variant, target) in [
        (ThemeVariant::light(), "ControlFillColorDefaultBrush"),
        (ThemeVariant::dark(), "ControlFillColorDefaultBrush"),
        (
            WinUITheme::high_contrast(),
            "SystemControlBackgroundBaseLowBrush",
        ),
    ] {
        let lookup = |name| {
            let value = theme
                .try_get_resource(&ResourceKey::from(name), Some(&variant))
                .flatten()
                .unwrap();
            ValueTypes::as_object(&*value).unwrap()
        };
        assert!(lookup("ButtonBackground").ptr_eq(&lookup(target)));
    }
    println!("{registration_ns} {lookup_ns} {first_ns} {repeated_ns}");
}
