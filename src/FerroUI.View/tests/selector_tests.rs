//! Typed selectors: each form matches the elements its text form matches.

use crate::generated::builders::*;
use crate::generated::pseudo_classes::PseudoClassSelectors;
use crate::reactive::Owner;
use crate::{any, any_of, is_type, nesting, of, parse_selector, style, IntoElement, TypedSelector};
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::platform::IPlatformRenderInterface;
use ferroui_base::input::InputElement;
use ferroui_base::layout::ILayoutManager;
use ferroui_base::styling::Selector;
use ferroui_base::{FerroLocator, LocatorExtensions, Ref};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Border, Button, Control, StackPanel, TextBlock, Window};

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

fn tree() -> Ref<Control> {
    stack_panel()
        .name("root")
        .class("outer")
        .child(button().name("a").class("accent").content("a"))
        .child(button().name("b").class("accent big").is_enabled(false).content("b"))
        .child(text_block().name("c").class("accent").text("c"))
        .child(border().name("d").child(text_block().name("e").class("inner").text("e")))
        .into_element()
        .mount()
}

/// The names of the controls of the tree that `selector` matches.
fn matched(controls: &[Ref<Control>], selector: &Selector) -> Vec<String> {
    controls
        .iter()
        .filter(|control| selector.match_(control, None, false).is_match())
        .map(|control| control.name().unwrap_or_else(|| control.get_type().name().to_string()))
        .collect()
}

#[test]
fn typed_selectors_match_what_their_text_matches() {
    let _app = app();
    let owner = Owner::new_root();
    let root = owner.with(tree);
    let window = window_with(&root);
    let mut controls = vec![root.clone()];
    controls.extend(root.get_visual_descendants().filter_map(|visual| visual.cast::<Control>()));
    let pressed = controls.iter().find(|control| control.name().as_deref() == Some("a")).unwrap();
    pressed.pseudo_classes().set(":pressed", true);

    let forms: Vec<(&str, Selector)> = vec![
        ("Button", of::<Button>()),
        (":is(Control)", is_type::<Control>()),
        (".accent", any().class("accent")),
        ("#a", any().name("a")),
        (":pressed", any().pressed()),
        ("Button.accent", of::<Button>().class("accent")),
        ("Button.accent.big", of::<Button>().class("accent").class("big")),
        ("Button#a.accent", of::<Button>().name("a").class("accent")),
        ("Button:pressed", of::<Button>().pressed()),
        ("StackPanel > Button", of::<StackPanel>().child().of::<Button>()),
        ("StackPanel TextBlock", of::<StackPanel>().descendant().of::<TextBlock>()),
        ("Border > TextBlock.inner", of::<Border>().child().of::<TextBlock>().class("inner")),
        ("StackPanel > :is(Control)", of::<StackPanel>().child().is_type::<Control>()),
        ("Button /template/ ContentPresenter", of::<Button>().template().of::<ContentPresenter>()),
        ("Button:not(.big)", of::<Button>().not(any().class("big"))),
        (":not(.accent)", any().not(any().class("accent"))),
        (":nth-child(2n+1)", any().nth_child(2, 1)),
        (":nth-last-child(2)", any().nth_last_child(0, 2)),
        ("Button:nth-child(2)", of::<Button>().nth_child(0, 2)),
        ("Button[IsEnabled=False]", of::<Button>().property_equals(InputElement::is_enabled_property(), false)),
        ("Button, TextBlock", any_of([of::<Button>(), of::<TextBlock>()])),
    ];
    for (text, typed) in &forms {
        let parsed = parse_selector(text).unwrap_or_else(|error| panic!("`{text}`: {error}"));
        assert_eq!(matched(&controls, typed), matched(&controls, &parsed), "the elements of `{text}`");
    }
    window.close();
    owner.dispose();
}

#[test]
fn a_typed_selector_writes_the_text_it_stands_for() {
    assert_eq!(of::<Button>().class("accent").pointer_over().to_string(), "Button.accent:pointerover");
    assert_eq!(is_type::<Control>().to_string(), ":is(Control)");
    assert_eq!(any().focus_within().to_string(), ":focus-within");
    assert_eq!(nesting().pressed().to_string(), "^:pressed");
    assert_eq!(of::<StackPanel>().child().of::<TextBlock>().to_string(), "StackPanel > TextBlock");
}

#[test]
fn one_prefix_starts_several_selectors() {
    let prefix = of::<Button>().class("accent");
    let pressed = prefix.clone().pressed();
    let over = prefix.clone().pointer_over();
    assert_eq!(pressed.to_string(), "Button.accent:pressed");
    assert_eq!(over.to_string(), "Button.accent:pointerover");
    assert_eq!(prefix.to_string(), "Button.accent", "the prefix keeps its conditions");
}

#[test]
fn a_nested_typed_style_applies_like_its_text() {
    let _app = app();
    let typed = style(of::<TextBlock>().class("accent"))
        .set(TextBlock::font_size_property(), 30.0)
        .nested(style(nesting().class("strong")).set(TextBlock::font_size_property(), 40.0))
        .nested(style(nesting().not(any().class("strong")).class("dim")).set(TextBlock::font_size_property(), 10.0));
    let owner = Owner::new_root();
    let panel = owner.with(|| {
        stack_panel()
            .styles([typed])
            .child(text_block().name("a").class("accent").text("accent"))
            .child(text_block().name("b").class("accent strong").text("strong"))
            .child(text_block().name("d").class("accent dim").text("dim"))
            .child(text_block().name("c").text("plain"))
            .into_element()
            .mount()
    });
    let window = window_with(&panel);
    let find = |name: &str| {
        panel
            .get_visual_descendants()
            .filter_map(|visual| visual.cast::<TextBlock>())
            .find(|text| text.name().as_deref() == Some(name))
            .unwrap()
    };
    assert_eq!(find("a").font_size(), 30.0);
    assert_eq!(find("b").font_size(), 40.0);
    assert_eq!(find("d").font_size(), 10.0);
    assert_ne!(find("c").font_size(), 30.0);
    window.close();
    owner.dispose();
}

#[test]
fn rsx_takes_a_typed_selector() {
    let _app = app();
    let owner = Owner::new_root();
    let panel = owner.with(|| {
        let element: crate::Element = crate::rsx! {
            StackPanel {
                styles: [style(of::<TextBlock>().class("accent")).set(TextBlock::font_size_property(), 30.0)],
                TextBlock { name: "a", class: "accent", text: "accent" }
            }
        };
        element.mount()
    });
    let window = window_with(&panel);
    let text = panel.get_visual_descendants().find_map(|visual| visual.cast::<TextBlock>()).unwrap();
    assert_eq!(text.font_size(), 30.0);
    window.close();
    owner.dispose();
}
