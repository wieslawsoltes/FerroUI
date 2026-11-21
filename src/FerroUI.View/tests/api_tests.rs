//! The rest of the API (phase 3): styles, the resources and the theme
//! variant, transitions and animations, popups and context menus.

use crate::generated::builders::*;
use crate::reactive::Owner;
use crate::{
    animation, ms, rerender, style, transition, transitions, use_resource, use_theme_variant, IntoElement, StyleBuilder,
};
use ferroui_base::layout::ILayoutManager;
use ferroui_base::animation::{IClock, PlayState, SignedDuration};
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::platform::IPlatformRenderInterface;
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{BoxedValue, FerroLocator, LocatorExtensions, Ref, Visual};
use ferroui_controls::primitives::Popup;
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Button, Control, TextBlock, Window};
use std::cell::Cell;
use std::rc::Rc;

struct App {
    _app: UnitTestApplicationScope,
    _text: TextTestScope,
}

fn app() -> App {
    let text = TextTestScope::new();
    let render_interface =
        FerroLocator::current().get_service::<dyn IPlatformRenderInterface>().expect("the text services have a render interface");
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

fn text_of(control: &Ref<Control>) -> Option<String> {
    control.get_visual_descendants().find_map(|visual| visual.cast::<TextBlock>()).and_then(|text| text.text())
}

#[test]
fn styles_of_an_element_apply_to_the_controls_below_it() {
    let _app = app();
    let accent: StyleBuilder = style("TextBlock.accent")
        .set(TextBlock::font_size_property(), 30.0)
        .nested(style("^.strong").set(TextBlock::font_size_property(), 40.0))
        .nested(style("^:not(.strong).dim").set(TextBlock::font_size_property(), 10.0));
    let owner = Owner::new_root();
    let panel = owner.with(|| {
        stack_panel()
            .styles([accent])
            .child(text_block().name("a").class("accent").text("accent"))
            .child(text_block().name("b").class("accent strong").text("strong"))
            .child(text_block().name("c").text("plain"))
            .into_element()
            .mount()
    });
    let window = window_with(&panel);
    let find = |name: &str| {
        panel.get_visual_descendants().filter_map(|visual| visual.cast::<TextBlock>()).find(|text| text.name().as_deref() == Some(name)).unwrap()
    };
    assert_eq!(find("a").font_size(), 30.0);
    assert_eq!(find("b").font_size(), 40.0, "a nested style applies with its parent's selector");
    assert_ne!(find("c").font_size(), 30.0);
    window.close();
    owner.dispose();
}

#[derive(Default)]
struct ViewClock {
    subject: LightweightSubject<SignedDuration>,
    paused: Cell<bool>,
}

impl IObservable<SignedDuration> for ViewClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<SignedDuration>>) -> Rc<dyn IDisposable> {
        self.subject.subscribe(observer)
    }
}

impl IClock for ViewClock {
    fn play_state(&self) -> PlayState {
        if self.paused.get() { PlayState::Pause } else { PlayState::Run }
    }
    fn set_play_state(&self, value: PlayState) {
        self.paused.set(value != PlayState::Run);
    }
}

#[test]
fn rsx_visual_states_apply_native_directional_motion_and_restore_the_base_value() {
    use crate::{set_visual_state, visual_state, visual_state_group};
    use ferroui_base::layout::Layoutable;
    use ferroui_controls::{Border, StackPanel};
    let _app = app();
    let owner = Owner::new_root();
    let panel = owner.with(|| crate::rsx! {
        StackPanel {
            visual_states: [visual_state_group("Review")
                .state(visual_state("Idle").set_on("Card", Layoutable::width_property(), 120.0))
                .state(visual_state("Ready").set_on("Card", Layoutable::width_property(), 260.0)
                    .set(Visual::opacity_property(), 0.5))
                .transition(None, "Ready", ms(350), "LinearEasing")
                .transition(Some("Idle"), "Ready", ms(640), "CubicEaseIn")
                .transition(Some("Ready"), "Idle", ms(370), "LinearEasing")],
            Border { name: "Card", width: 100.0, height: 40.0 }
        }
    }.mount());
    let clock = Rc::new(ViewClock::default());
    panel.set_clock(Some(clock.clone()));
    let window = window_with(&panel);
    let card = panel.get_visual_descendants().find_map(|child| child.cast::<Border>()).unwrap();
    let pulse = |seconds| clock.subject.on_next(SignedDuration::from_seconds(seconds));
    assert_eq!(card.width(), 100.0);
    assert!(set_visual_state(&panel, "Review", Some("Ready"), true));
    pulse(0.0);
    pulse(0.175);
    assert!((card.width() - 180.0).abs() < 0.001, "default width was {}", card.width());
    assert!((panel.opacity() - 0.75).abs() < 0.001);
    pulse(0.35);
    assert_eq!(card.width(), 260.0);
    assert!(set_visual_state(&panel, "Review", Some("Idle"), true));
    pulse(1.0);
    pulse(1.185);
    assert!((card.width() - 190.0).abs() < 0.001, "reverse width was {}", card.width());
    pulse(1.37);
    assert_eq!(card.width(), 120.0);
    assert!(set_visual_state(&panel, "Review", Some("Ready"), true));
    pulse(2.0);
    pulse(2.32);
    assert!((card.width() - 137.5).abs() < 0.001, "forward width was {}", card.width());
    assert!(!set_visual_state(&panel, "Review", Some("Unknown"), true));
    assert!(set_visual_state(&panel, "Review", None, false));
    window.layout_manager().execute_layout_pass();
    pulse(2.321);
    assert_eq!(card.width(), 100.0);
    assert!(set_visual_state(&panel, "Review", Some("Ready"), false));
    pulse(3.0);
    pulse(3.001);
    assert_eq!(card.width(), 260.0);
    assert!(set_visual_state(&panel, "Review", None, false));
    assert_eq!(card.width(), 100.0);

    let widths = Rc::new(LightweightSubject::<f64>::new());
    let property = Layoutable::width_property();
    let binding = card.bind(property, widths.clone(), ferroui_base::data::BindingPriority::LocalValue);
    widths.on_next(115.0);
    assert!(set_visual_state(&panel, "Review", Some("Ready"), true));
    pulse(4.0);
    pulse(4.35);
    assert_eq!(card.width(), 260.0);
    widths.on_next(135.0);
    assert_eq!(card.width(), 260.0);
    assert!(set_visual_state(&panel, "Review", None, false));
    assert_eq!(card.width(), 135.0);
    widths.on_next(145.0);
    assert_eq!(card.width(), 145.0);
    binding.dispose();
    window.close();
    owner.dispose();
}

#[test]
#[should_panic(expected = "does not parse")]
fn a_selector_that_does_not_parse_says_so() {
    let _ = style("Button..");
}

#[test]
fn a_component_follows_the_theme_variant_and_a_resource() {
    let _app = app();
    let owner = Owner::new_root();
    let panel = owner.with(|| {
        stack_panel()
            .child(rerender(|| {
                let variant = match use_theme_variant() {
                    Some(variant) if variant == ThemeVariant::dark() => "dark",
                    Some(_) => "other",
                    None => "none",
                };
                let greeting = use_resource("Greeting")
                    .and_then(|value| value.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| String::from("no greeting"));
                text_block().text(format!("{variant}: {greeting}"))
            }))
            .into_element()
            .mount()
    });
    let window = window_with(&panel);
    window.resources().set("Greeting", Some(Rc::new(String::from("hello")) as BoxedValue));
    window.set_value(ThemeVariant::requested_theme_variant_property(), Some(ThemeVariant::dark()));
    assert_eq!(text_of(&panel).as_deref(), Some("dark: hello"));
    window.resources().set("Greeting", Some(Rc::new(String::from("hi")) as BoxedValue));
    assert_eq!(text_of(&panel).as_deref(), Some("dark: hi"), "the component follows a changed resource");
    window.close();
    owner.dispose();
}

#[test]
fn transitions_and_animations_are_built_for_the_animation_system() {
    let owner = Owner::new_root();
    let control = owner.with(|| {
        border()
            .transitions(transitions([
                transition(Visual::opacity_property(), ms(200)),
                transition(ferroui_controls::Border::background_property(), ms(300)),
            ]))
            .into_element()
            .mount()
    });
    let transitions = control.transitions().expect("transitions");
    assert_eq!(transitions.count(), 2);

    let built = animation(ms(600))
        .key_frame(0.0, |frame| frame.set(Visual::opacity_property(), 0.0))
        .key_frame(1.0, |frame| frame.set(Visual::opacity_property(), 1.0).key_spline(0.25, 0.0, 0.75, 1.0))
        .build();
    assert_eq!(built.children().count(), 2);
    assert_eq!(built.duration(), ferroui_base::animation::SignedDuration::from(ms(600)));
    let animated_style = style("Border").animation(animation(ms(600))
        .key_frame(0.0, |frame| frame.set(Visual::opacity_property(), 0.0)))
        .build();
    assert_eq!(animated_style.animations().count(), 1);
    owner.dispose();
}

#[test]
fn a_popup_holds_its_child_element_and_a_context_menu_is_an_element() {
    let owner = Owner::new_root();
    let panel = owner.with(|| {
        stack_panel()
            .child(popup().child(text_block().text("in the popup")))
            .child(button().content("menu").context_menu(context_menu().child(menu_item().header("Copy")).child(menu_item().header("Paste"))))
            .into_element()
            .mount()
    });
    let children: Vec<Ref<Control>> = panel.cast::<ferroui_controls::StackPanel>().unwrap().children().to_vec().into_iter().collect();
    let popup = children[0].cast::<Popup>().expect("a popup");
    let child = popup.child().expect("the child of the popup");
    assert_eq!(child.cast::<TextBlock>().and_then(|text| text.text()).as_deref(), Some("in the popup"));
    let button = children[1].cast::<Button>().expect("a button");
    let menu = button.context_menu().expect("the context menu");
    assert_eq!(menu.items().count(), 2);
    owner.dispose();
}

#[cfg(feature = "treedatagrid")]
#[test]
fn the_builders_of_other_crates_make_their_controls() {
    let owner = Owner::new_root();
    let control = owner.with(|| crate::generated::treedatagrid::tree_data_grid().into_element().mount());
    assert!(control.cast::<ferroui_controls_treedatagrid::TreeDataGrid>().is_some());
    owner.dispose();
}

#[test]
fn rsx_table_columns_mount_native_bindings_and_values() {
    use ferroui_base::data::ReflectionBinding;
    use ferroui_base::layout::HorizontalAlignment;
    use ferroui_controls::GridLength;

    let _app = app();
    let owner = Owner::new_root();
    let table = owner.with(|| crate::rsx! {
        TableView {
            columns: crate::table_columns([
                crate::table_column().header("Account").width(GridLength::from_pixels(156.0))
                    .binding_path("Name").horizontal_content_alignment(HorizontalAlignment::Right),
                crate::table_column().header("Status").is_visible(false).can_user_resize(false),
            ])
        }
    }.mount()).cast::<TableView>().unwrap();
    let columns = table.columns();
    let account = columns.get(0);
    assert_eq!(account.header().unwrap().downcast_ref::<String>().unwrap(), "Account");
    assert_eq!(account.width(), GridLength::from_pixels(156.0));
    assert_eq!(account.horizontal_content_alignment(), HorizontalAlignment::Right);
    assert!(account.binding().unwrap().as_any().is_some_and(|binding| binding.is::<ReflectionBinding>()));
    let status = columns.get(1);
    assert!(!status.is_visible());
    assert_eq!(status.can_user_resize(), Some(false));
    owner.dispose();
}
