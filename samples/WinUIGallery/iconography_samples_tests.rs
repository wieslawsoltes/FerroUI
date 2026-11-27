//! Tests of the Iconography page (iconography_samples.rs).

use crate::iconography_samples::{filter, found_text, icons, IconData};
use crate::{examples, tests::start};
use ferroui_base::Ref;
use ferroui_controls::{AutoSuggestBox, Border, Control, TextBlock};
use ferroui_controls_items_view::ItemsView;
use std::rc::Rc;

fn page() -> Ref<Control> {
    let sample = examples::find("Iconography").expect("the Iconography page");
    examples::create(sample).expect("the page loads")
}

fn text(root: &Ref<Control>, name: &str) -> String {
    root.get_control::<TextBlock>(name).text().unwrap_or_default()
}

fn selected(root: &Ref<Control>) -> Rc<IconData> {
    let context = root
        .get_control::<Border>("SidePanel")
        .data_context()
        .expect("the selected icon");
    context.downcast_ref::<Rc<IconData>>().expect("an IconData").clone()
}

fn shown(root: &Ref<Control>) -> usize {
    root.get_control::<ItemsView>("IconsItemsView")
        .items_source()
        .map_or(0, |items| items.count())
}

#[test]
fn icons_data_keeps_the_entries_of_the_source() {
    let icons = icons();
    assert_eq!(icons.len(), 1574);
    let first = &icons[0];
    assert_eq!((first.code(), first.name()), ("E700", "GlobalNavButton"));
    assert_eq!(first.tags(), ["menu", "hamburger", "line", "three", "symbol-icon"]);
    assert_eq!(first.character(), "\u{E700}");
    assert_eq!(first.code_glyph(), "\\uE700");
    assert_eq!(first.text_glyph(), "&#xE700;");
    assert_eq!(first.symbol_name(), None, "GlobalNavButton is not a member of Symbol");
    let save = icons.iter().find(|icon| icon.name() == "Save").expect("the Save icon");
    assert_eq!(save.symbol_name().as_deref(), Some("Save"));
    assert_eq!(
        icons.iter().filter(|icon| icon.record.is_segoe_fluent_only).count(),
        168
    );
}

#[test]
fn filter_keeps_the_icons_with_every_word_in_their_code_name_or_tags() {
    let icons = icons();
    let names = |search: &str| {
        filter(&icons, search)
            .iter()
            .map(|icon| icon.name())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        filter(&icons, "").len(),
        icons.len(),
        "an empty word matches every icon"
    );
    assert_eq!(names("e700"), ["GlobalNavButton"], "the code, ignoring case");
    assert_eq!(names("menu hamburger"), ["GlobalNavButton", "ButtonMenu"], "every word");
    assert_eq!(names("WIFI").len(), 46);
    assert!(names("zzzz").is_empty());
    assert_eq!(found_text(0), "No icons found.");
    assert_eq!(found_text(1), "1 icon found.");
    assert_eq!(found_text(46), "46 icons found.");
}

#[test]
fn iconography_shows_the_icons_and_the_first_icon_in_the_side_panel() {
    let _application = start();
    let root = page();
    assert_eq!(shown(&root), 1574);
    let view = root.get_control::<ItemsView>("IconsItemsView");
    assert!(view.is_selected(0), "IconsItemsView.Select(0)");
    assert!(root.get_control::<Border>("SidePanel").is_visible());
    assert_eq!(selected(&root).name(), "GlobalNavButton");
    assert_eq!(text(&root, "XAMLCodePresenterFont"), "<FontIcon Glyph=\"&#xE700;\" />");
    assert_eq!(
        text(&root, "CSharpCodePresenterFont"),
        "FontIcon icon = new FontIcon();\nicon.Glyph = \"\\uE700\";"
    );
    // GlobalNavButton is not a member of Symbol: the SymbolIcon rows are empty.
    assert_eq!(text(&root, "XAMLCodePresenterSymbol"), "");
    assert!(!root.get_control::<TextBlock>("SymbolXamlLabel").is_visible());
    assert!(!root.get_control::<TextBlock>("SymbolCSharpLabel").is_visible());
    let tags = root.get_control::<ItemsView>("TagsItemsView");
    assert!(tags.is_visible() && !root.get_control::<TextBlock>("NoTagsTextBlock").is_visible());
    assert_eq!(tags.items_source().map(|items| items.count()), Some(5));
}

/// The page in a window with the WinUI theme: the search box raises `TextChanged` from its text
/// box part, after its 150 ms timer ([`tick`]).
#[cfg(feature = "theme")]
fn window_page() -> (Ref<ferroui_controls::Window>, Ref<Control>) {
    use ferroui_controls::testing::MockWindowingPlatform;
    let root = page();
    let window =
        ferroui_controls::Window::with_impl(MockWindowingPlatform::create_window_mock_with_size(1400.0, 900.0));
    window.set_content(Some(Control::boxed(&root)));
    window.show();
    window.update_layout();
    (window, root)
}

/// Fires the timer of `AutoSuggestBox.TextChanged` and runs the jobs it posts.
#[cfg(feature = "theme")]
fn tick() {
    use ferroui_base::threading::Dispatcher;
    for timer in Dispatcher::timers_for_unit_tests() {
        if timer.interval() == std::time::Duration::from_millis(150) {
            Dispatcher::force_fire_timer_for_unit_tests(&timer);
        }
    }
    Dispatcher::ui_thread().run_jobs(None);
}

#[cfg(feature = "theme")]
#[test]
fn iconography_search_filters_the_icons_and_selects_the_first() {
    let _application = crate::tests::start_winui();
    let (_window, root) = window_page();
    let search = root.get_control::<AutoSuggestBox>("IconsAutoSuggestBox");

    search.set_text("E74E");
    tick();
    assert_eq!(shown(&root), 1);
    assert_eq!(text(&root, "IconsFoundText"), "1 icon found.");
    assert_eq!(selected(&root).name(), "Save");
    assert_eq!(text(&root, "XAMLCodePresenterSymbol"), "<SymbolIcon Symbol=\"Save\" />");
    assert_eq!(
        text(&root, "CSharpCodePresenterSymbol"),
        "SymbolIcon icon = new SymbolIcon();\nicon.Symbol = Symbol.Save;"
    );
    assert!(root.get_control::<TextBlock>("SymbolXamlLabel").is_visible());

    search.set_text("wifi");
    tick();
    assert_eq!(shown(&root), 46);
    assert_eq!(text(&root, "IconsFoundText"), "46 icons found.");
    assert_eq!(selected(&root).name(), "Wifi");

    // No icon: the side panel keeps the selected icon.
    search.set_text("zzzz");
    tick();
    assert_eq!(shown(&root), 0);
    assert_eq!(text(&root, "IconsFoundText"), "No icons found.");
    assert_eq!(selected(&root).name(), "Wifi");

    search.set_text("");
    tick();
    assert_eq!(shown(&root), 1574);
}

/// In a window: the grid realizes the icons in view only, with their glyph and name, the tags are
/// in a flow layout, and invoking a tag searches for it (`TagsItemsView_ItemInvoked`).
#[cfg(feature = "theme")]
#[test]
fn iconography_tags_search_for_their_text_when_invoked() {
    use ferroui_base::input::{InputElement, Key, KeyEventArgs};
    use ferroui_controls::FontIcon;
    use ferroui_controls_items_view::ItemContainer;
    use ferroui_controls_repeater::{FlowLayout, ItemsRepeater};

    let _application = crate::tests::start_winui();
    let (_window, root) = window_page();

    let repeater = |view: &Ref<ItemsView>| {
        view.get_visual_descendants()
            .find_map(|visual| visual.cast::<ItemsRepeater>())
            .expect("the view's repeater")
    };
    let icons = root.get_control::<ItemsView>("IconsItemsView");
    let realized = (0..1574)
        .filter(|index| repeater(&icons).try_get_element(*index).is_some())
        .count();
    assert!(
        realized > 0 && realized < 1574,
        "only the icons in view are realized: {realized}"
    );
    let first = repeater(&icons)
        .try_get_element(0)
        .expect("the first icon")
        .cast::<ItemContainer>()
        .unwrap();
    let glyph = first
        .get_visual_descendants()
        .find_map(|visual| visual.cast::<FontIcon>())
        .expect("its glyph");
    assert_eq!(glyph.glyph(), "\u{E700}");

    let tags = root.get_control::<ItemsView>("TagsItemsView");
    assert!(tags.layout().and_then(|layout| layout.cast::<FlowLayout>()).is_some());
    let tag = repeater(&tags)
        .try_get_element(1)
        .expect("the second tag")
        .cast::<ItemContainer>()
        .unwrap();
    assert!(
        tag.focus(),
        "the tag takes the focus: focusable {}, visible {}, enabled {}, template {}",
        tag.focusable(),
        tag.is_effectively_visible(),
        tag.is_effectively_enabled(),
        tag.template().is_some()
    );
    let mut enter = KeyEventArgs::new();
    enter.key = Key::Enter;
    enter.set_routed_event(Some(InputElement::key_down_event()));
    tag.raise_event(&enter);
    tick();
    assert_eq!(
        root.get_control::<AutoSuggestBox>("IconsAutoSuggestBox").text(),
        "hamburger"
    );
    assert_eq!(
        shown(&root),
        filter(&crate::iconography_samples::icons(), "hamburger").len()
    );
}
