use super::*;
use ferroui_base::input::{
    IPointer, KeyModifiers, Pointer, PointerCaptureLostEventArgs, PointerEventArgs,
    PointerPointProperties, PointerType, PointerUpdateKind, RawInputModifiers,
};
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

#[test]
fn native_library_drag_tracks_its_pointer_and_cancels_filtered_or_lost_gestures() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let library = ComponentLibrary::new();
    let item = item(
        "accent",
        "Accent",
        ComponentLibraryFilter::Resources,
        "Window",
    );
    library.replace_items(vec![item.clone()]);
    let row = library.item_control(&item);
    library.root.children().add(row.clone());
    let window = ferroui_controls::Window::new();
    window.set_width(420.0);
    window.set_height(480.0);
    window.set_content(Some(Control::boxed(library.control())));
    window.show();
    window.update_layout();
    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
    let other = Pointer::new(Pointer::get_next_free_id(), PointerType::Touch, false);
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    let weak = Rc::downgrade(&library);
    library.on_begin_drag(move |item, _| {
        assert_eq!(item.id, "accent");
        observed.set(observed.get() + 1);
        // Hosts may synchronously replace callbacks or refresh their catalog.
        weak.upgrade().unwrap().on_begin_drag(|_, _| {});
    });
    let press = || {
        row.raise_event(&PointerPressedEventArgs::new(
            row.to_ref(),
            pointer.clone(),
            &window,
            Point::new(20.0, 30.0),
            1,
            PointerPointProperties::new(
                RawInputModifiers::LEFT_MOUSE_BUTTON,
                PointerUpdateKind::LeftButtonPressed,
            ),
            KeyModifiers::NONE,
            1,
        ));
    };
    let moved = |pointer: Rc<Pointer>| {
        row.raise_event(&PointerEventArgs::new(
            Some(InputElement::pointer_moved_event()),
            row.to_ref(),
            pointer,
            Some(&window),
            Point::new(80.0, 30.0),
            2,
            PointerPointProperties::new(
                RawInputModifiers::LEFT_MOUSE_BUTTON,
                PointerUpdateKind::Other,
            ),
            KeyModifiers::NONE,
        ));
    };
    press();
    assert!(
        pointer.captured().is_some(),
        "Capture keeps fast drags observable outside the row"
    );
    moved(other);
    assert_eq!(
        calls.get(),
        0,
        "An unrelated touch cannot initiate the mouse gesture"
    );
    assert!(library.press.borrow().is_some());
    library.set_query("nothing matches");
    assert!(pointer.captured().is_none());
    moved(pointer.clone());
    assert_eq!(
        calls.get(),
        0,
        "Filtering cancels the gesture that targeted the retired row"
    );
    library.set_query("");
    press();
    row.raise_event(&PointerCaptureLostEventArgs::new(
        row.to_ref(),
        pointer.clone(),
    ));
    moved(pointer.clone());
    assert_eq!(calls.get(), 0);
    press();
    moved(pointer.clone());
    assert_eq!(calls.get(), 1);
    assert!(library.press.borrow().is_none());
    assert!(
        pointer.captured().is_none(),
        "A host that does not accept the drag retains no capture"
    );
    window.close();
}
