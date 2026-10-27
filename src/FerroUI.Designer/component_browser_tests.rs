use super::*;
use ferroui_controls::testing::{TestServices, UnitTestApplication};

fn item(
    id: &str,
    title: &str,
    filter: ComponentLibraryFilter,
    detail: &str,
) -> ComponentLibraryItem {
    ComponentLibraryItem {
        id: id.into(),
        title: title.into(),
        kind: filter.title().into(),
        detail: detail.into(),
        filter,
        node: Some(id.len() as u64),
        asset_id: None,
        swatch: None,
        usages: 0,
    }
}

#[test]
fn library_filters_names_types_scope_and_preserves_selection_without_commands() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let library = ComponentLibrary::new();
    library.replace_items(vec![
        item(
            "brush",
            "Accent",
            ComponentLibraryFilter::Resources,
            "SolidColorBrush Window",
        ),
        item(
            "theme",
            "Primary button",
            ComponentLibraryFilter::Themes,
            "ControlTheme Button Document",
        ),
        item(
            "template",
            "Card",
            ComponentLibraryFilter::Templates,
            "DataTemplate Sales",
        ),
    ]);
    let selections = Rc::new(Cell::new(0));
    let counter = selections.clone();
    library.on_select(move |_| counter.set(counter.get() + 1));
    assert!(library.select("theme"));
    assert_eq!(selections.get(), 1);
    library.set_query("button DOCUMENT");
    assert_eq!(library.visible_items().len(), 1);
    library.set_filter(ComponentLibraryFilter::Resources);
    assert!(library.visible_items().is_empty());
    assert_eq!(library.selected_item().unwrap().id, "theme");
    library.set_query("");
    library.set_filter(ComponentLibraryFilter::All);
    assert_eq!(library.list.selected_index(), 2);
    assert_eq!(
        selections.get(),
        1,
        "Filtering and list restoration are navigation, not commands"
    );
}

#[test]
fn library_replacement_retains_identity_even_when_labels_and_order_change() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let library = ComponentLibrary::new();
    library.replace_items(vec![
        item("one", "Accent", ComponentLibraryFilter::Resources, "Window"),
        item("two", "Primary", ComponentLibraryFilter::Themes, "Window"),
    ]);
    library.select("two");
    library.replace_items(vec![
        item(
            "two",
            "AAA renamed",
            ComponentLibraryFilter::Themes,
            "Window",
        ),
        item("one", "Accent", ComponentLibraryFilter::Resources, "Window"),
    ]);
    assert_eq!(library.selected_item().unwrap().title, "AAA renamed");
    assert_eq!(library.list.selected_index(), 0);
}
