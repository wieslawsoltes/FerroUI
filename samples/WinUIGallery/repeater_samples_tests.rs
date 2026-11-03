//! The ItemsRepeater and LayoutPanel pages on the native repeater: their sources, layouts,
//! templates and handlers. No WinUI visual-parity claim. The repeaters are measured directly:
//! outside a window a repeater has no scroller, and realizes the items of an unbounded window.

use crate::repeater_samples::{
    self, ActivityFeedLayout, FeaturedTileLayout, ParityTemplateSelector, Recipe,
    StringOrIntTemplateSelector, VariedImageSizeLayout,
};
use crate::{examples, tests::start};
use ferroui_base::input::{InputElement, Key, KeyEventArgs};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::layout::Orientation;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{BoxedValue, Rect, Ref, Size};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::{
    Border, Button, ComboBox, Control, RadioButtons, ScrollViewer, Slider, StackPanel, TextBlock,
    TextBox,
};
use ferroui_controls_repeater::{
    FlowLayout, FlowLayoutLineAlignment, IDataTemplateSelector, ItemsRepeater, LayoutPanel,
    StackLayout, UniformGridLayout,
};
use std::rc::Rc;

fn page(id: &str) -> Ref<Control> {
    examples::create(examples::find(id).unwrap()).unwrap()
}

fn realize(repeater: &ItemsRepeater, width: f64, height: f64) {
    repeater.measure(Size::new(width, height));
    repeater.arrange(Rect::new(0.0, 0.0, width, height));
}

fn tag(element: &Control) -> Option<String> {
    element.tag().and_then(|tag| tag.downcast_ref::<String>().cloned())
}

fn click(button: &Button) {
    button.raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
}

fn key_down(element: &InputElement, key: Key) -> bool {
    let mut args = KeyEventArgs::new();
    args.key = key;
    args.set_routed_event(Some(InputElement::key_down_event()));
    element.raise_event(&args);
    args.handled()
}

fn element(repeater: &ItemsRepeater, index: i32) -> Ref<Control> {
    repeater
        .try_get_element(index)
        .unwrap_or_else(|| panic!("element {index} is realized"))
}

#[test]
fn items_repeater_page_loads_every_example_with_native_sources_and_layouts() {
    let _application = start();
    let root = page("ItemsRepeater");
    for name in [
        "ItemsRepeaterExample1",
        "ItemsRepeaterExample2",
        "ItemsRepeaterExample3",
        "ItemsRepeaterExample4",
        "ItemsRepeaterExample5",
        "ItemsRepeaterExample6",
        "ItemsRepeaterNestedExample",
    ] {
        assert!(root.find_control::<StackPanel>(name).is_some(), "{name}");
    }
    let count = |name: &str| {
        root.get_control::<ItemsRepeater>(name)
            .items_source()
            .map_or(0, |source| source.count())
    };
    for (name, expected) in [
        ("repeater", 3),
        ("repeater2", 500),
        ("FlowRepeater", 500),
        ("MixedTypeRepeater", 9),
        ("outerRepeater", 4),
        ("animatedScrollRepeater", 20),
        ("VariedImageSizeRepeater", 1000),
    ] {
        assert_eq!(count(name), expected, "{name}");
    }
    let layout = |name: &str| root.get_control::<ItemsRepeater>(name).layout().unwrap();
    let feed = layout("repeater2").cast::<ActivityFeedLayout>().unwrap();
    assert_eq!((feed.row_spacing(), feed.column_spacing()), (12.0, 12.0));
    assert_eq!(feed.min_item_size(), Some(Size::new(80.0, 108.0)));
    let flow = layout("FlowRepeater").cast::<FlowLayout>().unwrap();
    assert_eq!((flow.min_item_spacing(), flow.line_spacing()), (8.0, 8.0));
    assert_eq!(flow.line_alignment(), FlowLayoutLineAlignment::Start);
    let varied = layout("VariedImageSizeRepeater").cast::<VariedImageSizeLayout>().unwrap();
    assert_eq!(varied.width(), 200.0);
    assert!(layout("MixedTypeRepeater").cast::<UniformGridLayout>().is_some());
}

#[test]
fn basic_repeater_adds_and_removes_bars_and_switches_layout_and_template() {
    let _application = start();
    let root = page("ItemsRepeater");
    let repeater = root.get_control::<ItemsRepeater>("repeater");
    let add = root.get_control::<Button>("AddBtn");
    let delete = root.get_control::<Button>("DeleteBtn");
    let count = || repeater.items_source().unwrap().count();
    assert_eq!(repeater.max_width(), 437.0);
    let vertical = repeater.layout().unwrap().cast::<StackLayout>().unwrap();
    assert_eq!((vertical.orientation(), vertical.spacing()), (Orientation::Vertical, 8.0));

    // HorizontalBarTemplate: the bar's MaxLength and Length through {Binding}.
    realize(&repeater, 500.0, 500.0);
    let first = element(&repeater, 0).cast::<Border>().unwrap();
    assert_eq!(first.width(), 425.0);
    assert_eq!(first.child().unwrap().width(), 300.0);

    click(&add);
    assert_eq!(count(), 4);
    assert!(delete.is_enabled());
    for remaining in [3, 2, 1, 0] {
        click(&delete);
        assert_eq!(count(), remaining);
    }
    assert!(!delete.is_enabled());
    click(&delete);
    assert_eq!(count(), 0);
    click(&add);
    assert_eq!(count(), 1);
    assert!(delete.is_enabled());

    let layouts = root.get_control::<RadioButtons>("BarLayouts");
    layouts.set_selected_index(1);
    let horizontal = repeater.layout().unwrap().cast::<StackLayout>().unwrap();
    assert_eq!(horizontal.orientation(), Orientation::Horizontal);
    assert_eq!(repeater.max_width(), 6000.0);
    // VerticalBarTemplate: MaxHeight is the integer quarter of 425.
    realize(&repeater, 500.0, 500.0);
    assert_eq!(element(&repeater, 0).height(), 106.0);
    layouts.set_selected_index(2);
    assert!(repeater.layout().unwrap().cast::<UniformGridLayout>().is_some());
    assert_eq!(repeater.max_width(), 540.0);
    layouts.set_selected_index(0);
    assert_eq!(
        repeater.layout().unwrap().cast::<StackLayout>().unwrap().orientation(),
        Orientation::Vertical
    );
    assert_eq!(repeater.max_width(), 437.0);
}

#[test]
fn activity_feed_layout_alternates_narrow_and_wide_tiles_with_parity_templates() {
    let _application = start();
    let root = page("ItemsRepeater");
    let repeater = root.get_control::<ItemsRepeater>("repeater2");
    // 612 less the right margin of 12: the tiles are max(80, (600 - 3 * 12) / 4) = 141 wide.
    realize(&repeater, 612.0, 400.0);
    for (index, x, y, width) in [
        (0, 0.0, 0.0, 141.0),
        (1, 153.0, 0.0, 141.0),
        (2, 306.0, 0.0, 294.0),
        (3, 0.0, 120.0, 294.0),
        (4, 306.0, 120.0, 141.0),
        (5, 459.0, 120.0, 141.0),
    ] {
        assert_eq!(element(&repeater, index).bounds(), Rect::new(x, y, width, 108.0), "item {index}");
    }
    assert_eq!(tag(&element(&repeater, 0)).as_deref(), Some("Normal"));
    assert_eq!(tag(&element(&repeater, 1)).as_deref(), Some("Accent"));
    assert_eq!(tag(&element(&repeater, 4)).as_deref(), Some("Normal"));

    let layouts = root.get_control::<RadioButtons>("FeedLayouts");
    layouts.set_selected_index(0);
    let grid = repeater.layout().unwrap().cast::<UniformGridLayout>().unwrap();
    assert_eq!((grid.min_item_width(), grid.min_item_height()), (108.0, 108.0));
    layouts.set_selected_index(1);
    assert!(repeater.layout().unwrap().cast::<ActivityFeedLayout>().is_some());
}

#[test]
fn template_selectors_choose_by_parity_and_by_item_type() {
    let _application = start();
    let root = page("ItemsRepeater");
    let template = |key: &str| repeater_samples::page_template(&root, key).unwrap();
    let parity = ParityTemplateSelector {
        normal: Some(template("NormalItemTemplate")),
        accent: Some(template("AccentItemTemplate")),
    };
    let pick = |value: i32| {
        let item: BoxedValue = Rc::new(value);
        parity.select_template(Some(&item)).unwrap()
    };
    assert!(Rc::ptr_eq(&pick(2), parity.normal.as_ref().unwrap()));
    assert!(Rc::ptr_eq(&pick(7), parity.accent.as_ref().unwrap()));
    let types = StringOrIntTemplateSelector {
        string_template: Some(template("StringDataTemplate")),
        int_template: Some(template("IntDataTemplate")),
    };
    let item: BoxedValue = Rc::new("text".to_string());
    let text = types.select_template(Some(&item)).unwrap();
    assert!(Rc::ptr_eq(&text, types.string_template.as_ref().unwrap()));
    let item: BoxedValue = Rc::new(64i32);
    let number = types.select_template(Some(&item)).unwrap();
    assert!(Rc::ptr_eq(&number, types.int_template.as_ref().unwrap()));
    let item: BoxedValue = Rc::new(1.5f64);
    assert!(types.select_template(Some(&item)).is_none());
}

#[test]
fn flow_layout_options_update_the_layout_and_the_realized_count() {
    let _application = start();
    let root = page("ItemsRepeater");
    let repeater = root.get_control::<ItemsRepeater>("FlowRepeater");
    let flow = repeater.layout().unwrap().cast::<FlowLayout>().unwrap();
    let viewer = root.get_control::<ScrollViewer>("FlowScrollViewer");
    let orientations = root.get_control::<RadioButtons>("FlowOrientation");
    orientations.set_selected_index(1);
    assert_eq!(flow.orientation(), Orientation::Vertical);
    assert_eq!(viewer.horizontal_scroll_bar_visibility(), ScrollBarVisibility::Auto);
    assert_eq!(viewer.vertical_scroll_bar_visibility(), ScrollBarVisibility::Disabled);
    orientations.set_selected_index(0);
    assert_eq!(flow.orientation(), Orientation::Horizontal);
    assert_eq!(viewer.horizontal_scroll_bar_visibility(), ScrollBarVisibility::Disabled);
    assert_eq!(viewer.vertical_scroll_bar_visibility(), ScrollBarVisibility::Auto);

    let alignments = root.get_control::<ComboBox>("FlowLineAlignment");
    for (index, alignment) in [
        (1, FlowLayoutLineAlignment::Center),
        (2, FlowLayoutLineAlignment::SpaceBetween),
        (3, FlowLayoutLineAlignment::SpaceEvenly),
        (0, FlowLayoutLineAlignment::Start),
    ] {
        alignments.set_selected_index(index);
        assert_eq!(flow.line_alignment(), alignment);
    }
    root.get_control::<Slider>("FlowItemSpacing").set_value(16.0);
    assert_eq!(flow.min_item_spacing(), 16.0);
    root.get_control::<Slider>("FlowLineSpacing").set_value(20.0);
    assert_eq!(flow.line_spacing(), 20.0);

    realize(&repeater, 520.0, 320.0);
    Dispatcher::ui_thread().run_jobs(None);
    let realized = repeater.realized_elements().len();
    assert!(realized > 0);
    assert_eq!(
        root.get_control::<TextBlock>("RealizedCountText").text().as_deref(),
        Some(format!("Realized elements: {realized} of 500").as_str())
    );
    assert_eq!(element(&repeater, 0).data_context().unwrap().downcast_ref::<String>().map(String::as_str), Some("Item 1"));
}

#[test]
fn mixed_type_repeater_presents_numbers_and_text_with_their_templates() {
    let _application = start();
    let root = page("ItemsRepeater");
    let repeater = root.get_control::<ItemsRepeater>("MixedTypeRepeater");
    realize(&repeater, 812.0, 2000.0);
    for (index, expected) in [(0, "Int"), (1, "String"), (7, "String"), (8, "Int")] {
        assert_eq!(tag(&element(&repeater, index)).as_deref(), Some(expected), "item {index}");
    }
}

#[test]
fn nested_repeaters_present_each_category_with_its_own_items() {
    let _application = start();
    let root = page("ItemsRepeater");
    let outer = root.get_control::<ItemsRepeater>("outerRepeater");
    realize(&outer, 800.0, 1000.0);
    for (index, name, first) in [(0, "Fruits", "Apricots"), (3, "Proteins", "Steak")] {
        let category = element(&outer, index).cast::<StackPanel>().unwrap();
        let title = category.children().get(0).cast::<TextBlock>().unwrap();
        assert_eq!(title.text().as_deref(), Some(name));
        let inner = category.children().get(1).cast::<ItemsRepeater>().unwrap();
        assert_eq!(inner.items_source().unwrap().count(), 7);
        let item = element(&inner, 0);
        assert_eq!(tag(&item).as_deref(), Some("String"));
        assert_eq!(item.data_context().unwrap().downcast_ref::<String>().map(String::as_str), Some(first));
    }
}

#[test]
fn animated_scrolling_items_colour_the_rectangle_and_announce_the_selection() {
    let _application = start();
    let root = page("ItemsRepeater");
    let repeater = root.get_control::<ItemsRepeater>("animatedScrollRepeater");
    let rectangle = root.get_control::<Rectangle>("colorRectangle");
    realize(&repeater, 250.0, 1000.0);
    let first = element(&repeater, 0).cast::<Button>().unwrap();
    let second = element(&repeater, 1).cast::<Button>().unwrap();
    let content = |button: &Button| button.content().and_then(|content| content.downcast_ref::<String>().cloned());
    assert_eq!(content(&*first).as_deref(), Some("Blue"));
    assert_eq!(content(&*second).as_deref(), Some("BlueViolet"));
    assert!(first.background().is_some());
    assert_eq!(AutomationProperties::get_name(&first).as_deref(), Some("Blue"));

    click(&first);
    assert!(rectangle.fill().is_some());
    assert!(rectangle.fill() == first.background());
    assert_eq!(AutomationProperties::get_name(&first).as_deref(), Some("Blue , selected"));
    click(&second);
    assert!(rectangle.fill() == second.background());
    assert_eq!(AutomationProperties::get_name(&first).as_deref(), Some("Blue"));
    assert_eq!(AutomationProperties::get_name(&second).as_deref(), Some("BlueViolet , selected"));

    // Home and End reach the first and the last item; another key is left to the repeater.
    assert!(key_down(&repeater, Key::End));
    assert!(repeater.try_get_element(19).is_some());
    assert!(key_down(&repeater, Key::Home));
    assert!(!key_down(&repeater, Key::A));
}

#[test]
fn animated_scrolling_scale_follows_the_distance_from_the_viewport_centre() {
    let _application = start();
    for (scroll_offset, item_offset, expected) in [(0.0, 77.5, 1.0), (0.0, -10.0, 0.75), (100.0, 177.5, 1.0)] {
        let scale = repeater_samples::item_scale(175.0, scroll_offset, item_offset, 20.0);
        assert!((scale - expected).abs() < 1e-9, "{scroll_offset}, {item_offset}: {scale}");
    }
    let root = page("ItemsRepeater");
    let repeater = root.get_control::<ItemsRepeater>("animatedScrollRepeater");
    realize(&repeater, 250.0, 1000.0);
    repeater_samples::apply_item_scales(&repeater, 175.0, 0.0);
    for index in [0, 5] {
        let item = element(&repeater, index);
        let bounds = item.bounds();
        let expected = repeater_samples::item_scale(175.0, 0.0, bounds.y, bounds.height);
        let matrix = item.render_transform().unwrap().value();
        assert!((matrix.m11 - expected).abs() < 1e-9, "item {index}");
        assert!((matrix.m22 - expected).abs() < 1e-9, "item {index}");
    }
}

#[test]
fn content_heavy_repeater_filters_and_sorts_recipes_in_columns() {
    let _application = start();
    let root = page("ItemsRepeater");
    let repeater = root.get_control::<ItemsRepeater>("VariedImageSizeRepeater");
    let recipes = || -> Vec<Rc<Recipe>> {
        let source = repeater.items_source().unwrap();
        (0..source.count())
            .map(|index| source.get_at(index).unwrap().downcast_ref::<Rc<Recipe>>().cloned().unwrap())
            .collect()
    };
    assert_eq!(recipes().len(), 1000);
    assert_eq!(recipes()[0].num, 0);
    assert_eq!(recipes()[0].name, "Recipe 0");
    assert!(recipes().iter().all(|recipe| (4..=7).contains(&recipe.ing_list.len())));

    // Two 200-pixel columns: each item goes to the lowest column.
    realize(&repeater, 400.0, 600.0);
    let bounds = |index: i32| element(&repeater, index).bounds();
    assert_eq!((bounds(0).x, bounds(0).y, bounds(0).width), (0.0, 0.0, 200.0));
    assert_eq!((bounds(1).x, bounds(1).y, bounds(1).width), (200.0, 0.0, 200.0));
    assert_eq!(bounds(2).y, bounds(0).height.min(bounds(1).height));

    let filter = root.get_control::<TextBox>("FilterRecipes");
    filter.set_text(Some("GARLIC"));
    let filtered = recipes();
    assert!(!filtered.is_empty() && filtered.len() < 1000);
    assert!(filtered.iter().all(|recipe| recipe.ingredients.contains("Garlic")));
    assert!(counts(&filtered).windows(2).all(|pair| pair[0] <= pair[1]));
    click(&root.get_control::<Button>("SortDescending"));
    let descending = recipes();
    assert_eq!(descending.len(), filtered.len());
    assert!(counts(&descending).windows(2).all(|pair| pair[0] >= pair[1]));
    click(&root.get_control::<Button>("SortAscending"));
    assert!(counts(&recipes()).windows(2).all(|pair| pair[0] <= pair[1]));
    filter.set_text(Some(""));
    assert_eq!(recipes().len(), 1000);
}

fn counts(recipes: &[Rc<Recipe>]) -> Vec<usize> {
    recipes.iter().map(|recipe| recipe.ing_list.len()).collect()
}

fn arrange_panel(panel: &LayoutPanel) {
    panel.measure(Size::new(560.0, f64::INFINITY));
    let height = panel.desired_size().height;
    panel.arrange(Rect::new(0.0, 0.0, 560.0, height));
}

#[test]
fn layout_panel_page_arranges_its_children_with_the_featured_tile_layout() {
    let _application = start();
    let root = page("LayoutPanel");
    assert!(root.find_control::<StackPanel>("LayoutPanelExample1").is_some());
    let panel = root.get_control::<LayoutPanel>("LayoutPanel1");
    assert_eq!(panel.children().count(), 9);
    let featured = panel.layout().unwrap().cast::<FeaturedTileLayout>().unwrap();
    assert_eq!(featured.spacing(), 8.0);
    arrange_panel(&panel);
    let bounds = |index: usize| panel.children().get(index).bounds();
    // Inside the border (1) and the padding (12): 534 wide; two columns of (534 - 8) / 2.
    assert_eq!(bounds(0), Rect::new(13.0, 13.0, 534.0, 96.0));
    assert_eq!((bounds(1).x, bounds(1).width), (13.0, 263.0));
    assert_eq!((bounds(2).x, bounds(2).width), (284.0, 263.0));
    // The first row is as high as its taller child (92): the second row starts at 217.
    assert_eq!(bounds(3), Rect::new(13.0, 217.0, 263.0, 80.0));
}

#[test]
fn layout_panel_switches_to_a_stack_and_follows_the_spacing() {
    let _application = start();
    let root = page("LayoutPanel");
    let panel = root.get_control::<LayoutPanel>("LayoutPanel1");
    let layouts = root.get_control::<RadioButtons>("LayoutButtons");
    let bounds = |index: usize| panel.children().get(index).bounds();
    layouts.set_selected_index(1);
    let stack = panel.layout().unwrap().cast::<StackLayout>().unwrap();
    assert_eq!(stack.orientation(), Orientation::Vertical);
    arrange_panel(&panel);
    assert_eq!((bounds(1).x, bounds(1).y, bounds(1).height), (13.0, 117.0, 72.0));
    root.get_control::<Slider>("LayoutSpacing").set_value(16.0);
    assert_eq!(stack.spacing(), 16.0);
    arrange_panel(&panel);
    assert_eq!(bounds(1).y, 125.0);
    layouts.set_selected_index(0);
    let featured = panel.layout().unwrap().cast::<FeaturedTileLayout>().unwrap();
    assert_eq!(featured.spacing(), 16.0);
    arrange_panel(&panel);
    assert_eq!(bounds(3).y, 233.0);
}

#[test]
fn repeater_pages_and_their_handlers_release_their_native_controls() {
    let _application = start();
    for id in ["ItemsRepeater", "LayoutPanel"] {
        let root = page(id);
        let weak = root.downgrade();
        drop(root);
        assert!(weak.upgrade().is_none(), "{id} retained its page");
    }
}
