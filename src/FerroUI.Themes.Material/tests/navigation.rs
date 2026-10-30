//! Behavioral checks for Material navigation templates: the selected page remains wired to the
//! real TabControl, responsive rail placement retains its content host, and dropdown fields keep
//! their selection/editing contract after replacing the Windows visual structure.

use crate::{register_types, MaterialTheme};
use ferroui_base::animation::{IClock, IGlobalClock, PlayState, TimeSpan};
use ferroui_base::controls::ResourceKey;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::styling::ControlTheme;
use ferroui_base::{ObjectType, Ref, Visual};
use ferroui_controls::page::{ContentPage, TabPlacement, TabbedPage};
use ferroui_controls::testing::{
    MockWindowingPlatform, TestServices, UnitTestApplication, UnitTestApplicationScope,
};
use ferroui_controls::{
    Application, Border, ComboBox, ComboBoxItem, Control, Dock, TabControl, TabItem, Window,
};
use std::cell::Cell;
use std::rc::Rc;

struct FrozenClock {
    subject: LightweightSubject<TimeSpan>,
    state: Cell<PlayState>,
}

impl IObservable<TimeSpan> for FrozenClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
        self.subject.subscribe(observer)
    }
}
impl IClock for FrozenClock {
    fn play_state(&self) -> PlayState {
        self.state.get()
    }
    fn set_play_state(&self, state: PlayState) {
        self.state.set(state);
    }
}
impl IGlobalClock for FrozenClock {}

fn app() -> UnitTestApplicationScope {
    register_types();
    UnitTestApplication::start(
        TestServices::styled_window()
            .with_global_clock(Rc::new(FrozenClock {
                subject: LightweightSubject::new(),
                state: Cell::new(PlayState::Run),
            }))
            .with_theme(|| MaterialTheme::new().as_style()),
    )
}

fn named<T: ObjectType>(root: &Visual, name: &str) -> Ref<T> {
    root.get_visual_descendants()
        .filter(|node| node.name().as_deref() == Some(name))
        .find_map(|node| node.cast::<T>())
        .unwrap_or_else(|| panic!("required Material template part {name} is missing"))
}

fn window(content: impl Into<ferroui_base::BoxedValue>) -> Ref<Window> {
    let window = Window::with_impl(MockWindowingPlatform::create_window_mock_with_size(
        360.0, 640.0,
    ));
    window.set_content(Some(content.into()));
    window.show();
    window.update_layout();
    window
}

#[test]
fn bottom_navigation_shares_width_and_updates_selected_page() {
    let _app = app();
    let pages = TabbedPage::new();
    for header in ["Home", "Library", "Settings"] {
        let page = ContentPage::new();
        page.set_header(Some(Rc::new(header.to_owned())));
        pages.pages().expect("pages collection").add(page.upcast());
    }
    pages.set_selected_index(0);
    let window = window(Control::boxed(&pages));
    assert_eq!(pages.tab_placement(), TabPlacement::Bottom);
    let tabs: Ref<TabControl> = named(&pages, "PART_TabControl");
    assert_eq!(tabs.tab_strip_placement(), Dock::Bottom);
    let bar: Ref<Border> = named(&tabs, "TabContainerGrid");
    assert_eq!(
        bar.border_thickness(),
        ferroui_base::Thickness::uniform(0.0)
    );
    let realized: Vec<Ref<TabItem>> = tabs
        .get_visual_descendants()
        .filter_map(|node| node.cast())
        .collect();
    assert_eq!(realized.len(), 3);
    for item in &realized {
        let custom: Ref<ferroui_controls::presenters::ContentPresenter> =
            named(item, "CustomIndicator");
        assert!(
            !custom.is_visible(),
            "no fallback data-context text should render when IndicatorTemplate is absent"
        );
        assert!(
            (item.bounds().width - 120.0).abs() <= 1.0,
            "navigation items share the 360dp viewport"
        );
        assert!(
            (item.bounds().height - 64.0).abs() <= 1.0,
            "Expressive navigation bar has a 64dp target"
        );
    }
    tabs.set_selected_index(2);
    window.update_layout();
    assert_eq!(
        pages.selected_index(),
        2,
        "template selection must reach the logical page controller"
    );
    pages.set_selected_index(1);
    assert_eq!(
        tabs.selected_index(),
        1,
        "programmatic page navigation must reach the visual bar"
    );
    realized[1].set_is_enabled(false);
    window.update_layout();
    let root: Ref<Border> = named(&realized[1], "LayoutRoot");
    let icon: Ref<ferroui_controls::presenters::ContentPresenter> = named(&realized[1], "Icon");
    let label: Ref<ferroui_controls::presenters::ContentPresenter> =
        named(&realized[1], "PART_ContentPresenter");
    let indicator: Ref<Border> = named(&realized[1], "SelectionIndicator");
    assert_eq!(
        root.opacity(),
        1.0,
        "disabled navigation must not fade its selected pill"
    );
    assert_eq!(indicator.background().unwrap().opacity(), 1.0);
    assert_eq!(icon.foreground().unwrap().opacity(), 0.38);
    assert_eq!(label.foreground().unwrap().opacity(), 0.38);
    window.close();
}

#[test]
fn rail_theme_keeps_navigation_and_96dp_container_after_placement_changes() {
    let _app = app();
    let tabs = TabControl::new();
    let resource = Application::current()
        .unwrap()
        .try_get_resource(&ResourceKey::from("MaterialNavigationRail"), None)
        .flatten();
    let theme = from_markup_value::<Ref<ControlTheme>>(&resource).expect("Material rail theme");
    tabs.set_theme(theme);
    for label in ["First", "Second"] {
        let tab = TabItem::new();
        tab.set_header(Some(Rc::new(label.to_string())));
        tabs.items().add(Some(Control::boxed(tab)));
    }
    tabs.set_selected_index(0);
    let window = window(Control::boxed(&tabs));
    let strip: Ref<Border> = named(&tabs, "TabContainerGrid");
    assert_eq!(tabs.tab_strip_placement(), Dock::Left);
    assert!((strip.bounds().width - 96.0).abs() <= 1.0);
    tabs.set_selected_index(1);
    window.update_layout();
    assert_eq!(tabs.selected_index(), 1);
    tabs.set_tab_strip_placement(Dock::Right);
    window.update_layout();
    assert!(
        strip.bounds().x > 0.0,
        "rail follows right placement without a mirrored content transform"
    );
    assert_eq!(tabs.selected_index(), 1);
    window.close();
}

#[test]
fn exposed_dropdown_retains_selection_and_editable_text_box_parts() {
    let _app = app();
    let field = ComboBox::new();
    field.set_width(280.0);
    for label in ["First", "Second"] {
        let item = ComboBoxItem::new();
        item.set_content(Some(Rc::new(label.to_owned())));
        field.items().add(Some(Control::boxed(item)));
    }
    field.set_selected_index(1);
    let window = window(Control::boxed(&field));
    assert_eq!(field.selected_index(), 1);
    assert_eq!(field.min_height(), 56.0);
    field.set_is_editable(true);
    window.update_layout();
    let editor: Ref<ferroui_controls::TextBox> = named(&field, "PART_EditableTextBox");
    assert!(editor.is_visible());
    field.set_text(Some("search".to_owned()));
    window.update_layout();
    assert_eq!(editor.text().as_deref(), Some("search"));
    window.close();
}

#[test]
fn tall_bar_and_narrow_rail_keep_their_published_sizes() {
    let _app = app();
    for (key, expected_width, expected_height) in [
        ("MaterialNavigationBarTall", 180.0, 80.0),
        ("MaterialNavigationRailNarrow", 80.0, 64.0),
    ] {
        let tabs = TabControl::new();
        let resource = Application::current()
            .unwrap()
            .try_get_resource(&ResourceKey::from(key), None)
            .flatten();
        tabs.set_theme(
            from_markup_value::<Ref<ControlTheme>>(&resource).expect("sized navigation theme"),
        );
        for label in ["First", "Second"] {
            let item = TabItem::new();
            item.set_header(Some(Rc::new(label.to_string())));
            tabs.items().add(Some(Control::boxed(item)));
        }
        let window = window(Control::boxed(&tabs));
        let realized: Vec<Ref<TabItem>> = tabs
            .get_visual_descendants()
            .filter_map(|node| node.cast())
            .collect();
        assert_eq!(realized.len(), 2);
        for item in &realized {
            assert!(
                (item.bounds().width - expected_width).abs() <= 1.0,
                "{key} width: {:?}",
                item.bounds()
            );
            assert!(
                (item.bounds().height - expected_height).abs() <= 1.0,
                "{key} height: {:?}",
                item.bounds()
            );
            let icon: Ref<ferroui_controls::presenters::ContentPresenter> = named(item, "Icon");
            assert_eq!(icon.bounds().size(), ferroui_base::Size::new(24.0, 24.0));
            assert!(
                (icon.transform_to_visual(item).unwrap().m11 - 1.0).abs() < 0.001,
                "selection indicator animation must not scale the icon"
            );
        }
        if key == "MaterialNavigationRailNarrow" {
            assert!(
                (realized[1].bounds().y - realized[0].bounds().bottom() - 4.0).abs() <= 1.0,
                "narrow rail keeps the native 4dp spacing"
            );
        }
        window.close();
    }
}

#[test]
fn navigation_back_icon_follows_platform_rtl_mirroring_once() {
    let _app = app();
    let button = ferroui_controls::Button::new();
    let resource = Application::current()
        .unwrap()
        .try_get_resource(&ResourceKey::from("NavigationBackButtonNormalStyle"), None)
        .flatten();
    button.set_theme(
        from_markup_value::<Ref<ControlTheme>>(&resource).expect("navigation icon button"),
    );
    button.set_flow_direction(ferroui_base::media::FlowDirection::RightToLeft);
    let window = window(Control::boxed(&button));
    let arrow: Ref<ferroui_controls::shapes::Path> = named(&button, "Content");
    let transform = arrow.transform_to_visual(&window).expect("rooted arrow");
    assert!(
        transform.m11 < 0.0,
        "RTL layout must mirror the back arrow once, without a second theme flip"
    );
    button.set_flow_direction(ferroui_base::media::FlowDirection::LeftToRight);
    window.update_layout();
    assert!(arrow.transform_to_visual(&window).unwrap().m11 > 0.0);
    window.close();
}

#[test]
fn disabled_chip_and_segment_keep_independent_content_fill_and_outline_roles() {
    let _app = app();
    for key in [
        "MaterialFilterChip",
        "MaterialInputChip",
        "MaterialSegmentedButton",
    ] {
        let chip = ferroui_controls::primitives::ToggleButton::new();
        let resource = Application::current()
            .unwrap()
            .try_get_resource(&ResourceKey::from(key), None)
            .flatten();
        chip.set_theme(from_markup_value::<Ref<ControlTheme>>(&resource).unwrap());
        chip.set_content(Some(Rc::new("Choice".to_owned())));
        chip.set_is_checked(Some(true));
        chip.set_is_enabled(false);
        chip.set_flow_direction(ferroui_base::media::FlowDirection::RightToLeft);
        let window = window(Control::boxed(&chip));
        let surface: Ref<Border> = named(&chip, "ChipContainer");
        let check: Ref<ferroui_controls::Viewbox> = named(&chip, "CheckIcon");
        let glyph: Ref<ferroui_controls::shapes::Path> = check
            .get_visual_descendants()
            .find_map(|node| node.cast())
            .expect("selected check icon");
        assert!(
            glyph.transform_to_visual(&window).unwrap().m11 > 0.0,
            "non-directional {key} check must not mirror in RTL"
        );
        assert_eq!(
            surface.opacity(),
            1.0,
            "{key} container must not fade its contents twice"
        );
        assert_eq!(
            chip.foreground().unwrap().opacity(),
            0.38,
            "{key} disabled content"
        );
        if key == "MaterialSegmentedButton" {
            assert_eq!(
                surface.background().unwrap().opacity(),
                1.0,
                "disabled selected segment keeps its fill"
            );
            assert_eq!(surface.border_brush().unwrap().opacity(), 0.12);
        } else {
            assert_eq!(
                surface.background().unwrap().opacity(),
                0.12,
                "disabled selected chip fill"
            );
            assert_eq!(
                surface.border_thickness(),
                ferroui_base::Thickness::uniform(0.0)
            );
        }
        chip.set_is_checked(Some(false));
        window.update_layout();
        assert_eq!(
            surface.border_brush().unwrap().opacity(),
            0.12,
            "{key} disabled outline"
        );
        assert_eq!(chip.foreground().unwrap().opacity(), 0.38);
        window.close();
    }
}
