//! The selection rules, the invoke rules, the visual states and the theme of `ItemsView`.

use crate::{
    register_types, ItemContainer, ItemContainerInteractionTrigger as Trigger, ItemsView, ItemsViewSelectionMode as Mode,
    ASSEMBLY,
};
use ferroui_base::animation::{IClock, IGlobalClock, PlayState, SignedDuration};
use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::styling::{IStyle, Styles};
use ferroui_base::Ref;
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::visual_state::VisualStateManager;
use ferroui_controls::{Application, Control, ItemsSource, Panel, TextBlock, Window};
use ferroui_markup_xaml::{RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct MockGlobalClock {
    subject: LightweightSubject<SignedDuration>,
    play_state: Cell<PlayState>,
}

impl IObservable<SignedDuration> for MockGlobalClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<SignedDuration>>) -> Rc<dyn IDisposable> {
        self.subject.subscribe(observer)
    }
}

impl IClock for MockGlobalClock {
    fn play_state(&self) -> PlayState {
        self.play_state.get()
    }

    fn set_play_state(&self, value: PlayState) {
        self.play_state.set(value)
    }
}

impl IGlobalClock for MockGlobalClock {}

/// A themed application with the styles of this crate, included as an application includes them.
fn start() -> UnitTestApplicationScope {
    register_types();
    let clock = Rc::new(MockGlobalClock { subject: LightweightSubject::new(), play_state: Cell::new(PlayState::Run) });
    let services = TestServices::styled_window()
        .with_global_clock(clock)
        .with_theme(|| ferroui_themes_winui::WinUITheme::new().as_style());
    let scope = UnitTestApplication::start(services);
    FerroRuntimeXamlLoader::register();
    let xaml = "<Styles xmlns=\"https://github.com/ferroui\">\
                  <StyleInclude Source=\"ferres://FerroUI.Controls.ItemsView/Themes/WinUI/WinUI.xaml\" />\
                </Styles>";
    let mut configuration = RuntimeXamlLoaderConfiguration::new();
    configuration.local_assembly = Some(&ASSEMBLY);
    let loaded = FerroRuntimeXamlLoader::load_document(RuntimeXamlLoaderDocument::new(xaml), Some(configuration))
        .unwrap_or_else(|error| panic!("WinUI.xaml failed to load: {}", error.message()));
    let styles = from_markup_value::<Ref<Styles>>(&Some(loaded)).expect("a Styles");
    let style: Rc<dyn IStyle> = styles.into();
    Application::current().expect("the application").styles().add(style);
    scope
}

const VIEW: &str = "<ItemsView xmlns='https://github.com/ferroui' Width='200' Height='300'>\
    <ItemsView.ItemTemplate>\
      <DataTemplate><ItemContainer Height='30'><TextBlock Text='{Binding}' /></ItemContainer></DataTemplate>\
    </ItemsView.ItemTemplate>\
  </ItemsView>";

fn view(mode: Mode) -> (Ref<Window>, Ref<ItemsView>) {
    let view = FerroRuntimeXamlLoader::parse_as::<Ref<ItemsView>>(VIEW, Some(&ASSEMBLY)).expect("the view loads");
    view.set_selection_mode(mode);
    view.set_items_source(Some(ItemsSource::from_strs(["zero", "one", "two", "three", "four"])));
    let window = Window::new();
    window.set_content(Some(Control::boxed(&view)));
    window.show();
    window.update_layout();
    (window, view)
}

fn container(view: &ItemsView, index: i32) -> Ref<ItemContainer> {
    view.items_repeater()
        .expect("the template is applied")
        .try_get_element(index)
        .and_then(|element| element.cast::<ItemContainer>())
        .unwrap_or_else(|| panic!("item {index} is realized"))
}

fn click(view: &ItemsView, index: i32, modifiers: KeyModifiers) {
    view.on_container_interaction(&container(view, index), Trigger::PointerReleased, modifiers);
}

fn selected(view: &ItemsView) -> Vec<i32> {
    (0..5).filter(|index| view.is_selected(*index)).collect()
}

fn state(item: &ItemContainer, group: &str) -> String {
    let root = item.get_visual_children().first().and_then(|root| root.clone().cast::<Control>()).expect("the template root");
    let groups = VisualStateManager::get_visual_state_groups(&root);
    let groups = groups.to_vec();
    let group = groups.iter().find(|candidate| candidate.name() == group).unwrap_or_else(|| panic!("the group {group}"));
    group.current_state().map(|state| state.name()).unwrap_or_default()
}

fn press(view: &ItemsView, key: Key, modifiers: KeyModifiers) {
    let mut args = KeyEventArgs::new();
    args.key = key;
    args.key_modifiers = modifiers;
    args.set_routed_event(Some(InputElement::key_down_event()));
    view.raise_event(&args);
}

#[test]
fn single_selection_follows_the_click_and_control_deselects() {
    let _app = start();
    let (_window, view) = view(Mode::Single);
    click(&view, 1, KeyModifiers::NONE);
    click(&view, 3, KeyModifiers::NONE);
    assert_eq!(selected(&view), [3]);
    assert!(container(&view, 3).is_selected());
    assert!(!container(&view, 1).is_selected());
    click(&view, 3, KeyModifiers::CONTROL);
    assert!(selected(&view).is_empty());
}

#[test]
fn multiple_selection_toggles_and_shift_extends_from_the_anchor() {
    let _app = start();
    let (_window, view) = view(Mode::Multiple);
    click(&view, 1, KeyModifiers::NONE);
    click(&view, 3, KeyModifiers::SHIFT);
    assert_eq!(selected(&view), [1, 2, 3]);
    click(&view, 2, KeyModifiers::NONE);
    assert_eq!(selected(&view), [1, 3]);
}

#[test]
fn extended_selection_replaces_adds_with_control_and_ranges_with_shift() {
    let _app = start();
    let (_window, view) = view(Mode::Extended);
    click(&view, 0, KeyModifiers::NONE);
    click(&view, 2, KeyModifiers::CONTROL);
    assert_eq!(selected(&view), [0, 2]);
    click(&view, 4, KeyModifiers::NONE);
    assert_eq!(selected(&view), [4]);
    click(&view, 2, KeyModifiers::SHIFT);
    assert_eq!(selected(&view), [2, 3, 4]);
}

#[test]
fn no_selection_mode_keeps_every_item_unselected() {
    let _app = start();
    let (_window, view) = view(Mode::None);
    click(&view, 1, KeyModifiers::NONE);
    view.select(2);
    container(&view, 3).set_is_selected(true);
    assert!(selected(&view).is_empty());
    assert!(!container(&view, 3).is_selected());
}

#[test]
fn item_invoked_follows_the_rules_of_each_mode_and_trigger() {
    let _app = start();
    for (mode, expected) in [
        (Mode::None, [true, false, true, true]),
        (Mode::Single, [false, true, true, false]),
        (Mode::Multiple, [false, true, true, false]),
        (Mode::Extended, [false, true, true, false]),
    ] {
        let (_window, view) = view(mode);
        view.set_is_item_invoked_enabled(true);
        let invoked = Rc::new(RefCell::new(Vec::new()));
        let sink = invoked.clone();
        let _subscription = view.item_invoked(move |args| {
            sink.borrow_mut().push(args.invoked_item.as_ref().and_then(|item| item.downcast_ref::<String>().cloned()));
        });
        for (trigger, raises) in [Trigger::Tap, Trigger::DoubleTap, Trigger::EnterKey, Trigger::SpaceKey].into_iter().zip(expected) {
            invoked.borrow_mut().clear();
            view.on_container_interaction(&container(&view, 1), trigger, KeyModifiers::NONE);
            assert_eq!(invoked.borrow().len(), usize::from(raises), "{mode:?} {trigger:?}");
            if raises {
                assert_eq!(invoked.borrow()[0].as_deref(), Some("one"));
            }
        }
        // A press is never an invoke, and nothing is raised while it is off.
        invoked.borrow_mut().clear();
        view.on_container_interaction(&container(&view, 1), Trigger::PointerReleased, KeyModifiers::NONE);
        view.set_is_item_invoked_enabled(false);
        view.on_container_interaction(&container(&view, 1), Trigger::EnterKey, KeyModifiers::NONE);
        assert!(invoked.borrow().is_empty());
    }
}

#[test]
fn the_container_goes_through_the_visual_states_of_its_theme() {
    let _app = start();
    let (_window, view) = view(Mode::Single);
    let item = container(&view, 1);
    assert_eq!(state(&item, "CombinedStates"), "UnselectedNormal");
    assert_eq!(state(&item, "MultiSelectStates"), "Single");
    assert_eq!(state(&item, "DisabledStates"), "Enabled");

    item.set_pointer_state(true, false);
    assert_eq!(state(&item, "CombinedStates"), "UnselectedPointerOver");
    item.set_pointer_state(true, true);
    assert_eq!(state(&item, "CombinedStates"), "UnselectedPressed");
    item.set_pointer_state(false, false);
    view.select(1);
    assert_eq!(state(&item, "CombinedStates"), "SelectedNormal");
    item.set_pointer_state(true, false);
    assert_eq!(state(&item, "CombinedStates"), "SelectedPointerOver");
    item.set_pointer_state(true, true);
    assert_eq!(state(&item, "CombinedStates"), "SelectedPressed");
    item.set_pointer_state(false, false);

    item.set_is_enabled(false);
    assert_eq!(state(&item, "DisabledStates"), "Disabled");
    assert_eq!(state(&item, "CombinedStates"), "SelectedNormal");
    item.set_is_enabled(true);
    assert_eq!(state(&item, "DisabledStates"), "Enabled");

    view.set_selection_mode(Mode::Multiple);
    assert_eq!(state(&item, "MultiSelectStates"), "Multiple");
}

#[test]
fn the_template_holds_the_child_above_its_background_and_below_the_selection() {
    let _app = start();
    let (_window, view) = view(Mode::Single);
    let item = container(&view, 0);
    let root = item.get_visual_children().first().and_then(|root| root.clone().cast::<Panel>()).expect("a panel");
    let names: Vec<Option<String>> = root.children().to_vec().iter().map(|child| child.name()).collect();
    let position = |name: &str| names.iter().position(|known| known.as_deref() == Some(name)).expect(name);
    let child = root.children().to_vec().iter().position(|child| child.is::<TextBlock>()).expect("the child");
    assert!(position("PART_ContainerBackground") < child);
    assert!(child < position("PART_SelectionVisual"));
    assert!(child < position("PART_SelectionCheckbox"));
}

/// The key of this crate selects the same dictionary as the high contrast variant of the theme.
#[test]
fn the_high_contrast_resources_answer_to_the_variant_of_the_winui_theme() {
    use ferroui_base::controls::ResourceKey;
    use ferroui_base::media::IBrush;
    use ferroui_base::styling::ThemeVariant;
    use ferroui_themes_winui::WinUITheme;

    let _app = start();
    let (_window, view) = view(Mode::Single);
    assert_eq!(crate::markup_types::high_contrast_key(), WinUITheme::high_contrast());
    let color = |key: &str, variant: ThemeVariant| {
        let value = view.try_find_resource(&ResourceKey::from(key), Some(&variant)).unwrap_or_else(|| panic!("{key}"));
        let brush = from_markup_value::<Rc<dyn IBrush>>(&value).unwrap_or_else(|| panic!("{key} is a brush"));
        brush.as_solid_color_brush().map(|solid| solid.color()).unwrap_or_else(|| panic!("{key} is a solid brush"))
    };
    let key = "ItemContainerPointerOverBorderBrush";
    let high_contrast = WinUITheme::high_contrast();
    assert_eq!(color(key, high_contrast.clone()), color("SystemColorHighlightColorBrush", high_contrast.clone()));
    assert_eq!(color(key, ThemeVariant::light()), color("SubtleFillColorTransparentBrush", ThemeVariant::light()));
    assert_ne!(color(key, ThemeVariant::light()), color(key, high_contrast));
}

#[test]
fn the_keyboard_moves_the_current_item_and_selects_all() {
    let _app = start();
    let (window, view) = view(Mode::Single);
    click(&view, 1, KeyModifiers::NONE);
    press(&view, Key::Down, KeyModifiers::NONE);
    window.update_layout();
    assert_eq!(view.current_item_index(), 2);
    assert_eq!(selected(&view), [2]);
    press(&view, Key::Home, KeyModifiers::NONE);
    assert_eq!(view.current_item_index(), 0);
    assert_eq!(selected(&view), [0]);

    view.set_selection_mode(Mode::Extended);
    press(&view, Key::A, KeyModifiers::CONTROL);
    assert_eq!(selected(&view), [0, 1, 2, 3, 4]);
    view.invert_selection();
    assert!(selected(&view).is_empty());
}
