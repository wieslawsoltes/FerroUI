//! Control themes, templates and dictionaries in Rust: the same controls and values as the same
//! theme in XAML.

use crate::generated::builders::*;
use crate::reactive::{signal, Owner};
use crate::{
    any, control_template_with, control_theme, nesting, resource_dictionary, rgb, style, IntoElement, PseudoClassSelectors,
    TypedSelector,
};
use ferroui_base::layout::ILayoutManager;
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::media::{Color, IBrush};
use ferroui_base::platform::IPlatformRenderInterface;
use ferroui_base::styling::{ControlTheme, ThemeVariant};
use ferroui_base::{BoxedValue, FerroLocator, LocatorExtensions, Ref, Thickness};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Border, Button, ContentControl, Control, Decorator, TextBlock, Window};
use std::cell::Cell;
use std::rc::Rc;

struct App {
    _app: UnitTestApplicationScope,
    _text: TextTestScope,
}

fn app() -> App {
    let text = TextTestScope::new();
    let render_interface = FerroLocator::current()
        .get_service::<dyn IPlatformRenderInterface>()
        .expect("the text services have a render interface");
    let app = UnitTestApplication::start(TestServices::styled_window().with_render_interface(render_interface));
    App { _app: app, _text: text }
}

fn window_with(content: &Ref<Control>) -> Ref<Window> {
    let window = Window::new();
    window.set_width(300.0);
    window.set_height(200.0);
    window.set_content(Some(Control::boxed(content)));
    window.show();
    window.layout_manager().execute_layout_pass();
    window
}

fn rust_theme() -> Ref<ControlTheme> {
    control_theme::<Button>()
        .set(TemplatedControl::background_property(), rgb(0x102030))
        .set(TemplatedControl::padding_property(), Thickness::uniform(5.0))
        .template(|| {
            border()
                .name("PART_Border")
                .template_bind(Border::background_property(), TemplatedControl::background_property())
                .template_bind(Decorator::padding_property(), TemplatedControl::padding_property())
                .child(
                    content_presenter()
                        .name("PART_ContentPresenter")
                        .template_bind(ContentPresenter::content_property(), ContentControl::content_property()),
                )
                .into_element()
        })
        .nested(
            style(nesting().pointer_over().template().of::<Border>().name("PART_Border"))
                .set(Border::background_property(), rgb(0x405060)),
        )
        .build()
}

const XAML_THEME: &str = r##"<ControlTheme xmlns="https://github.com/ferroui" TargetType="Button">
  <Setter Property="Background" Value="#102030" />
  <Setter Property="Padding" Value="5" />
  <Setter Property="Template">
    <ControlTemplate>
      <Border Name="PART_Border" Background="{TemplateBinding Background}" Padding="{TemplateBinding Padding}">
        <ContentPresenter Name="PART_ContentPresenter" Content="{TemplateBinding Content}" />
      </Border>
    </ControlTemplate>
  </Setter>
  <Style Selector="^:pointerover /template/ Border#PART_Border">
    <Setter Property="Background" Value="#405060" />
  </Style>
</ControlTheme>"##;

fn xaml_theme() -> Ref<ControlTheme> {
    ferroui_markup_xaml::register_types();
    crate::register_types();
    ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::register();
    let loaded = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(XAML_THEME, None)
        .unwrap_or_else(|error| panic!("{error}"));
    ferroui_base::metadata::from_markup_value::<Ref<ControlTheme>>(&Some(loaded)).expect("a control theme")
}

fn color_of(brush: Option<Rc<dyn IBrush>>) -> Option<Color> {
    brush?.as_solid_color_brush().map(|solid| solid.color())
}

/// What a themed button shows: its visual tree, and the values of its template, plain and hovered.
fn shape(theme: Ref<ControlTheme>) -> (Vec<(String, Option<String>)>, Option<Color>, Thickness, Option<Color>, Option<String>) {
    let owner = Owner::new_root();
    let root = owner.with(|| button().theme(theme).content("Hello").into_element().mount());
    let window = window_with(&root);
    let target = root.cast::<Button>().unwrap();
    let tree = target
        .get_visual_descendants()
        .filter_map(|visual| visual.cast::<Control>())
        .map(|control| (control.get_type().name().to_string(), control.name()))
        .collect();
    let part = target
        .get_visual_descendants()
        .find_map(|visual| visual.cast::<Border>())
        .expect("the border of the template");
    let plain = color_of(part.background());
    let padding = part.padding();
    target.pseudo_classes().set(":pointerover", true);
    let hovered = color_of(part.background());
    let text = target
        .get_visual_descendants()
        .find_map(|visual| visual.cast::<TextBlock>())
        .and_then(|text| text.text());
    window.close();
    owner.dispose();
    (tree, plain, padding, hovered, text)
}

#[test]
fn a_theme_in_rust_makes_what_the_same_theme_in_xaml_makes() {
    let _app = app();
    let rust = shape(rust_theme());
    let xaml = shape(xaml_theme());
    assert_eq!(rust, xaml);
    assert_eq!(rust.1, Some(rgb(0x102030)), "the template binding");
    assert_eq!(rust.2, Thickness::uniform(5.0));
    assert_eq!(rust.3, Some(rgb(0x405060)), "a style with a condition overrides a value of the template");
    assert_eq!(rust.4.as_deref(), Some("Hello"));
}

#[test]
fn the_named_parts_are_in_the_name_scope_of_the_template() {
    let _app = app();
    let owner = Owner::new_root();
    let found = Rc::new(Cell::new(false));
    let seen = found.clone();
    let root = owner.with(|| {
        button()
            .theme(rust_theme())
            .content("Hello")
            .on(TemplatedControl::template_applied_event(), move |_, e| {
                seen.set(e.name_scope().find("PART_Border").is_some() && e.name_scope().find("PART_ContentPresenter").is_some());
            })
            .into_element()
            .mount()
    });
    let window = window_with(&root);
    assert!(found.get());
    window.close();
    owner.dispose();
}

#[test]
fn a_dictionary_holds_values_variants_and_a_theme_for_a_class() {
    let _app = app();
    let dictionary = resource_dictionary()
        .value("Gap", 8.0_f64)
        .deferred("Later", || Rc::new(String::from("made")) as BoxedValue)
        .theme_variant(ThemeVariant::light(), resource_dictionary().value("Accent", rgb(0x0063B1)))
        .theme_variant(ThemeVariant::dark(), resource_dictionary().value("Accent", rgb(0x60CDFF)))
        .control_theme(rust_theme())
        .build();
    let owner = Owner::new_root();
    let root = owner.with(|| button().content("Hello").into_element().mount());
    let window = Window::new();
    window.set_resources(dictionary);
    window.set_content(Some(Control::boxed(&root)));
    window.show();
    window.layout_manager().execute_layout_pass();

    let found = |key: &str, variant: ThemeVariant| window.try_find_resource(&key.into(), Some(&variant)).flatten();
    let value = |key: &str, variant| found(key, variant).expect("the resource");
    assert_eq!(value("Gap", ThemeVariant::light()).downcast_ref::<f64>(), Some(&8.0));
    assert_eq!(value("Later", ThemeVariant::light()).downcast_ref::<String>().map(String::as_str), Some("made"));
    assert_eq!(value("Accent", ThemeVariant::light()).downcast_ref::<Color>(), Some(&rgb(0x0063B1)));
    assert_eq!(value("Accent", ThemeVariant::dark()).downcast_ref::<Color>(), Some(&rgb(0x60CDFF)));
    let target = root.cast::<Button>().unwrap();
    assert!(
        target.get_visual_descendants().any(|visual| visual.cast::<Control>().is_some_and(|c| c.name().as_deref() == Some("PART_Border"))),
        "the button found its theme under its class"
    );
    window.close();
    owner.dispose();
}

#[test]
fn a_template_that_is_replaced_stops_what_it_started() {
    let _app = app();
    let owner = Owner::new_root();
    let (runs, source) = (Rc::new(Cell::new(0)), owner.with(|| signal(0)));
    let counted = runs.clone();
    let template = control_template_with(move || {
        let counted = counted.clone();
        text_block()
            .text(move || {
                counted.set(counted.get() + 1);
                source.get().to_string()
            })
            .into_element()
    });
    let root = owner.with(|| button().template(Some(template)).into_element().mount());
    let window = window_with(&root);
    let started = runs.get();
    assert!(started >= 1);
    source.set(1);
    assert_eq!(runs.get(), started + 1, "the template follows the signal");

    let target = root.cast::<Button>().unwrap();
    target.set_template(Some(control_template_with(|| border().into_element())));
    window.layout_manager().execute_layout_pass();
    let after = runs.get();
    source.set(2);
    assert_eq!(runs.get(), after, "the template that left stops following the signal");
    window.close();
    owner.dispose();
}

#[test]
fn a_typed_selector_with_no_type_works_in_a_theme() {
    let _app = app();
    let theme = control_theme::<Button>()
        .template(|| border().name("PART_Border").into_element())
        .nested(style(nesting().template().of::<Border>().not(any().class("skip"))).set(Border::background_property(), rgb(0x112233)))
        .build();
    let owner = Owner::new_root();
    let root = owner.with(|| button().theme(theme).into_element().mount());
    let window = window_with(&root);
    let part = root.get_visual_descendants().find_map(|visual| visual.cast::<Border>()).unwrap();
    assert_eq!(color_of(part.background()), Some(rgb(0x112233)));
    window.close();
    owner.dispose();
}
