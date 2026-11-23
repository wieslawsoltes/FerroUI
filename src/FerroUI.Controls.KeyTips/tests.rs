//! Access keys through the real key route of a window.

use crate::*;
use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::Ref;
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::primitives::{AdornerLayer, ToggleButton, VisualLayerManager};
use ferroui_controls::testing::{MockWindowingPlatform, TestServices, UnitTestApplication};
use ferroui_controls::{Button, CheckBox, Control, StackPanel, TextBox, Window};
use std::cell::Cell;
use std::rc::Rc;

struct Scene {
    window: Ref<Window>,
    panel: Ref<StackPanel>,
}

fn scene() -> Scene {
    let panel = StackPanel::new();
    let layers = VisualLayerManager::new();
    layers.set_child(Some(panel.clone().upcast::<Control>()));
    let window = Window::with_impl(MockWindowingPlatform::create_window_mock());
    window.set_content(Some(Control::boxed(layers)));
    window.show();
    window.update_layout();
    Scene { window, panel }
}

impl Scene {
    fn add(&self, control: Ref<Control>) {
        self.panel.children().add(control);
        self.window.update_layout();
    }

    fn button(&self, key: &str) -> (Ref<Button>, Rc<Cell<u32>>) {
        let button = Button::new();
        AccessKeyManager::set_access_key(&button, Some(key));
        self.add(button.clone().upcast::<Control>());
        let clicks = Rc::new(Cell::new(0));
        let count = clicks.clone();
        let _ = button.add_handler(&Button::click_event(), move |_, _: &RoutedEventArgs| {
            count.set(count.get() + 1);
        });
        (button, clicks)
    }

    fn key(
        &self,
        event: fn() -> &'static ferroui_base::interactivity::RoutedEvent<KeyEventArgs>,
        key: Key,
        symbol: Option<&str>,
        modifiers: KeyModifiers,
    ) -> bool {
        let mut args = KeyEventArgs::new();
        args.set_routed_event(Some(event()));
        args.key = key;
        args.key_symbol = symbol.map(str::to_owned);
        args.key_modifiers = modifiers;
        self.panel.raise_event(&args);
        args.handled()
    }

    fn down(&self, key: Key, symbol: Option<&str>) -> bool {
        self.key(
            InputElement::key_down_event,
            key,
            symbol,
            KeyModifiers::NONE,
        )
    }

    fn alt(&self) {
        self.key(
            InputElement::key_down_event,
            Key::LeftAlt,
            None,
            KeyModifiers::ALT,
        );
        self.key(
            InputElement::key_up_event,
            Key::LeftAlt,
            None,
            KeyModifiers::NONE,
        );
    }

    fn letter(&self, text: &str) -> bool {
        self.down(Key::A, Some(text))
    }
}

fn start() -> impl Drop {
    UnitTestApplication::start(TestServices::focusable_window())
}

#[test]
fn alt_shows_the_tips_and_a_key_invokes_its_control() {
    let _app = start();
    let scene = scene();
    let (_file, file) = scene.button("F");
    let (_edit, edit) = scene.button("E");
    assert!(!AccessKeyManager::is_display_mode_enabled());
    scene.alt();
    assert!(AccessKeyManager::is_display_mode_enabled());
    assert_eq!(AccessKeyManager::visible_key_tips(), ["E", "F"]);
    assert!(scene.letter("f"), "lower case finds the key");
    assert_eq!((file.get(), edit.get()), (1, 0));
    assert!(!AccessKeyManager::is_display_mode_enabled());
}

#[test]
fn alt_again_hides_the_tips() {
    let _app = start();
    let scene = scene();
    let (_button, clicks) = scene.button("F");
    scene.alt();
    scene.alt();
    assert!(!AccessKeyManager::is_display_mode_enabled());
    assert_eq!(clicks.get(), 0);
}

#[test]
fn alt_and_a_key_together_invoke_without_showing_the_tips_after() {
    let _app = start();
    let scene = scene();
    let (_button, clicks) = scene.button("F");
    scene.key(
        InputElement::key_down_event,
        Key::LeftAlt,
        None,
        KeyModifiers::ALT,
    );
    assert!(scene.key(
        InputElement::key_down_event,
        Key::F,
        Some("f"),
        KeyModifiers::ALT
    ));
    scene.key(
        InputElement::key_up_event,
        Key::LeftAlt,
        None,
        KeyModifiers::NONE,
    );
    assert_eq!(clicks.get(), 1);
    assert!(!AccessKeyManager::is_display_mode_enabled());
}

#[test]
fn a_window_without_keys_keeps_its_alt() {
    let _app = start();
    let with_keys = scene();
    let _keys = with_keys.button("F");
    let plain = scene();
    assert!(!plain.key(
        InputElement::key_down_event,
        Key::LeftAlt,
        None,
        KeyModifiers::ALT
    ));
    assert!(!plain.key(
        InputElement::key_up_event,
        Key::LeftAlt,
        None,
        KeyModifiers::NONE
    ));
    assert!(!AccessKeyManager::is_display_mode_enabled());
}

#[test]
fn nested_scopes_show_their_own_keys_and_escape_goes_back_out() {
    let _app = start();
    let scene = scene();
    let (_outer, outer) = scene.button("O");
    let group = StackPanel::new();
    AccessKeyManager::set_access_key(&group, Some("S"));
    AccessKeyManager::set_is_access_key_scope(&group, true);
    let inner = Button::new();
    AccessKeyManager::set_access_key(&inner, Some("A"));
    let inner_group = StackPanel::new();
    AccessKeyManager::set_access_key(&inner_group, Some("N"));
    AccessKeyManager::set_is_access_key_scope(&inner_group, true);
    let deepest = Button::new();
    AccessKeyManager::set_access_key(&deepest, Some("D"));
    inner_group
        .children()
        .add(deepest.clone().upcast::<Control>());
    group.children().add(inner.clone().upcast::<Control>());
    group
        .children()
        .add(inner_group.clone().upcast::<Control>());
    scene.add(group.clone().upcast::<Control>());

    scene.alt();
    assert_eq!(AccessKeyManager::visible_key_tips(), ["O", "S"]);
    assert!(scene.letter("s"));
    assert!(
        AccessKeyManager::is_display_mode_enabled(),
        "a scope stays in the display mode"
    );
    assert_eq!(AccessKeyManager::visible_key_tips(), ["A", "N"]);
    assert!(scene.letter("n"));
    assert_eq!(AccessKeyManager::visible_key_tips(), ["D"]);
    assert!(scene.down(Key::Escape, None));
    assert_eq!(AccessKeyManager::visible_key_tips(), ["A", "N"]);
    assert!(scene.down(Key::Escape, None));
    assert_eq!(AccessKeyManager::visible_key_tips(), ["O", "S"]);
    assert!(scene.down(Key::Escape, None));
    assert!(!AccessKeyManager::is_display_mode_enabled());
    assert_eq!(outer.get(), 0);
}

#[test]
fn a_scope_owner_places_an_element_in_another_scope() {
    let _app = start();
    let scene = scene();
    let owner = Button::new();
    AccessKeyManager::set_access_key(&owner, Some("P"));
    AccessKeyManager::set_is_access_key_scope(&owner, true);
    scene.add(owner.clone().upcast::<Control>());
    let (owned, _) = scene.button("Q");
    AccessKeyManager::set_access_key_scope_owner(&owned, Some(owner.clone().upcast::<Control>()));
    scene.alt();
    assert_eq!(AccessKeyManager::visible_key_tips(), ["P"]);
    assert!(scene.letter("p"));
    assert_eq!(AccessKeyManager::visible_key_tips(), ["Q"]);
}

#[test]
fn a_key_of_two_letters_narrows_the_tips() {
    let _app = start();
    let scene = scene();
    let (_first, first) = scene.button("AB");
    let (_second, second) = scene.button("AC");
    let (_other, _) = scene.button("X");
    scene.alt();
    assert!(scene.letter("a"));
    assert_eq!(AccessKeyManager::visible_key_tips(), ["AB", "AC"]);
    assert!(scene.letter("c"));
    assert_eq!((first.get(), second.get()), (0, 1));
}

#[test]
fn a_key_that_matches_nothing_starts_over() {
    let _app = start();
    let scene = scene();
    let (_button, clicks) = scene.button("AB");
    scene.alt();
    assert!(scene.letter("a"));
    assert!(scene.letter("z"), "still consumed");
    assert_eq!(AccessKeyManager::visible_key_tips(), ["AB"]);
    assert!(AccessKeyManager::is_display_mode_enabled());
    assert!(scene.letter("a"));
    assert!(scene.letter("b"));
    assert_eq!(clicks.get(), 1);
}

#[test]
fn disabled_and_hidden_controls_have_no_tip() {
    let _app = start();
    let scene = scene();
    let (disabled, _) = scene.button("D");
    disabled.set_is_enabled(false);
    let (hidden, _) = scene.button("H");
    hidden.set_is_visible(false);
    let (_shown, _) = scene.button("S");
    scene.alt();
    assert_eq!(AccessKeyManager::visible_key_tips(), ["S"]);
}

#[test]
fn a_hidden_placement_still_has_a_working_key() {
    let _app = start();
    let scene = scene();
    let (button, clicks) = scene.button("F");
    AccessKeyManager::set_key_tip_placement_mode(&button, KeyTipPlacementMode::Hidden);
    scene.alt();
    assert!(scene.letter("f"));
    assert_eq!(clicks.get(), 1);
}

#[test]
fn a_handler_of_the_invoked_event_replaces_the_default_action() {
    let _app = start();
    let scene = scene();
    let (button, clicks) = scene.button("F");
    let seen = Rc::new(Cell::new(0));
    let count = seen.clone();
    let _ = button.add_handler(
        AccessKeyManager::access_key_invoked_event(),
        move |_, e: &AccessKeyInvokedEventArgs| {
            count.set(count.get() + 1);
            e.set_handled(true);
        },
    );
    scene.alt();
    assert!(scene.letter("f"));
    assert_eq!((seen.get(), clicks.get()), (1, 0));
}

#[test]
fn a_toggle_toggles_and_a_text_box_takes_the_focus() {
    let _app = start();
    let scene = scene();
    let check = CheckBox::new();
    AccessKeyManager::set_access_key(&check, Some("C"));
    scene.add(check.clone().upcast::<Control>());
    let text = TextBox::new();
    AccessKeyManager::set_access_key(&text, Some("T"));
    scene.add(text.clone().upcast::<Control>());
    scene.alt();
    assert!(scene.letter("c"));
    assert_eq!(check.upcast_ref::<ToggleButton>().is_checked(), Some(true));
    scene.alt();
    assert!(scene.letter("t"));
    assert!(text.is_focused());
}

#[test]
fn another_key_ends_the_display_mode_and_goes_on_to_its_target() {
    let _app = start();
    let scene = scene();
    let (_button, clicks) = scene.button("F");
    scene.alt();
    assert!(!scene.down(Key::F5, None), "the key is not consumed");
    assert!(!AccessKeyManager::is_display_mode_enabled());
    assert_eq!(clicks.get(), 0);
}

#[test]
fn a_control_can_stay_in_the_display_mode_after_its_key() {
    let _app = start();
    let scene = scene();
    let (button, clicks) = scene.button("F");
    AccessKeyManager::set_exit_display_mode_on_access_key_invoked(&button, false);
    scene.alt();
    assert!(scene.letter("f"));
    assert!(scene.letter("f"));
    assert_eq!(clicks.get(), 2);
    assert!(AccessKeyManager::is_display_mode_enabled());
    AccessKeyManager::exit_display_mode();
}

#[test]
fn key_tips_can_be_turned_off() {
    let _app = start();
    let scene = scene();
    let (_button, _) = scene.button("F");
    AccessKeyManager::set_are_key_tips_enabled(false);
    assert!(!scene.key(
        InputElement::key_down_event,
        Key::LeftAlt,
        None,
        KeyModifiers::ALT
    ));
    scene.key(
        InputElement::key_up_event,
        Key::LeftAlt,
        None,
        KeyModifiers::NONE,
    );
    assert!(!AccessKeyManager::is_display_mode_enabled());
    AccessKeyManager::set_are_key_tips_enabled(true);
}

#[test]
fn the_key_is_also_the_access_key_of_automation() {
    let _app = start();
    let scene = scene();
    let (button, _) = scene.button("F");
    assert_eq!(
        AutomationProperties::get_access_key(&button).as_deref(),
        Some("F")
    );
    let (named, _) = (Button::new(), ());
    AutomationProperties::set_access_key(&named, "Z");
    AccessKeyManager::set_access_key(&named, Some("Y"));
    assert_eq!(
        AutomationProperties::get_access_key(&named).as_deref(),
        Some("Z"),
        "an explicit value stays"
    );
}

#[test]
fn one_overlay_shows_the_tips_and_goes_away_with_them() {
    let _app = start();
    let scene = scene();
    let (button, _) = scene.button("F");
    let layer = AdornerLayer::get_adorner_layer(&button).expect("a layer");
    let overlays = || {
        layer
            .get_visual_children()
            .iter()
            .filter_map(|child| child.cast::<KeyTipOverlay>())
            .collect::<Vec<_>>()
    };
    assert!(
        overlays().is_empty(),
        "nothing is added before the tips show"
    );
    scene.alt();
    let shown = overlays();
    assert_eq!(shown.len(), 1);
    assert_eq!(shown[0].tip_texts(), ["F"]);
    AccessKeyManager::exit_display_mode();
    assert!(overlays().is_empty());
}

#[test]
fn the_display_change_is_reported() {
    let _app = start();
    let scene = scene();
    let (_button, _) = scene.button("F");
    let changes = Rc::new(Cell::new(0));
    let count = changes.clone();
    let _subscription =
        AccessKeyManager::is_display_mode_enabled_changed(move || count.set(count.get() + 1));
    scene.alt();
    scene.alt();
    assert_eq!(changes.get(), 2);
}

#[test]
fn the_run_time_loader_resolves_the_properties_without_a_prefix() {
    use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
    let _app = start();
    crate::register_types();
    let root = FerroRuntimeXamlLoader::parse_as::<Ref<Control>>(
        r#"<StackPanel xmlns="https://github.com/ferroui"
                       AccessKeyManager.AccessKey="S" AccessKeyManager.IsAccessKeyScope="True">
             <Button AccessKeyManager.AccessKey="F"
                     AccessKeyManager.KeyTipPlacementMode="Right"
                     AccessKeyManager.KeyTipHorizontalOffset="4" />
           </StackPanel>"#,
        None,
    )
    .expect("the document loads without a prefix");
    assert_eq!(
        AccessKeyManager::get_access_key(&root).as_deref(),
        Some("S")
    );
    assert!(AccessKeyManager::get_is_access_key_scope(&root));
    let panel = root.cast::<StackPanel>().unwrap();
    let button = panel.children().get(0);
    assert_eq!(
        AccessKeyManager::get_access_key(&button).as_deref(),
        Some("F")
    );
    assert_eq!(
        AccessKeyManager::get_key_tip_placement_mode(&button),
        KeyTipPlacementMode::Right
    );
    assert_eq!(
        AccessKeyManager::get_key_tip_horizontal_offset(&button),
        4.0
    );
}
