//! The ListView page: the source's ten examples with their code behind (`list_samples.rs`).

use crate::list_samples::{
    contact_of, insertion_index, selection_mode, Contact, CustomDataObject, GroupInfoList, ListViewPage, Message,
};
use crate::{examples, tests::start};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::{Button, ComboBox, Control, ListBox, SelectionMode, TextBlock, TextBox};
use std::rc::Rc;

fn page() -> Ref<Control> {
    examples::create(examples::find("ListView").unwrap()).unwrap_or_else(|error| panic!("ListView: {error}"))
}

fn items(root: &Ref<Control>, name: &str) -> Vec<BoxedValue> {
    root.get_control::<ListBox>(name)
        .items_source()
        .unwrap_or_else(|| panic!("{name} has items"))
        .to_vec()
        .into_iter()
        .flatten()
        .collect()
}

fn names(items: &[BoxedValue]) -> Vec<String> {
    items
        .iter()
        .filter_map(contact_of)
        .map(|contact| contact.name())
        .collect()
}

fn strings(items: &[BoxedValue]) -> Vec<String> {
    items
        .iter()
        .filter_map(|item| item.downcast_ref::<String>().cloned())
        .collect()
}

fn click(root: &Ref<Control>, name: &str) {
    root.get_control::<Button>(name)
        .raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
}

#[test]
fn list_view_lists_get_the_contacts_and_items_of_on_navigated_to() {
    let _application = start();
    let root = page();
    assert_eq!(Contact::get_contacts().len(), 27);
    for name in [
        "BaseExample",
        "Control2",
        "DragDropListView",
        "FilteredListView",
        "ContextMenuList",
    ] {
        let contacts = names(&items(&root, name));
        assert_eq!(contacts.len(), 27, "{name}");
        assert_eq!(contacts[0], "Kendall Collins", "{name}");
        assert_eq!(contacts[26], "Hayden Cook", "{name}");
    }
    assert_eq!(
        names(&items(&root, "DragDropListView2")),
        ["John Doe", "Jane Doe", "Santa Claus"]
    );
    let restore = strings(&items(&root, "RestoreScrollListView"));
    assert_eq!(restore.len(), 27);
    assert_eq!(restore[0], "1. Kendall Collins");
    let scroll = strings(&items(&root, "ScrollIntoViewListView"));
    assert_eq!(scroll.len(), 270);
    assert_eq!(scroll[27], "28. Kendall Collins");
    assert_eq!(scroll[269], "270. Hayden Cook");
    let images: Vec<_> = items(&root, "Control4")
        .iter()
        .filter_map(|item| item.downcast_ref::<Rc<CustomDataObject>>().cloned())
        .collect();
    assert_eq!(images.len(), 8);
    for (index, item) in images.iter().enumerate() {
        assert_eq!(item.title, format!("Item {}", index + 1));
        assert!(item
            .image_location
            .ends_with(&format!("LandscapeImage{}.jpg", index + 1)));
        let views: u32 = item.views.parse().unwrap();
        let likes: u32 = item.likes.parse().unwrap();
        assert!((100..999).contains(&views) && (10..99).contains(&likes));
    }
    // The constructor's AddItemToEnd: the first message, sent.
    let messages = items(&root, "InvertedListView");
    let message = messages[0].downcast_ref::<Rc<Message>>().unwrap();
    assert_eq!(
        (message.msg_text.as_str(), message.msg_alignment),
        ("Message 1", HorizontalAlignment::Right)
    );
}

#[test]
fn list_view_selection_mode_option_sets_the_mode_of_the_list() {
    let _application = start();
    let root = page();
    let list = root.get_control::<ListBox>("Control2");
    let combo_box = root.get_control::<ComboBox>("SelectionModeComboBox");
    assert_eq!(combo_box.selected_index(), 1);
    combo_box.set_selected_index(2);
    assert_eq!(list.selection_mode(), SelectionMode::MULTIPLE | SelectionMode::TOGGLE);
    combo_box.set_selected_index(3);
    assert_eq!(list.selection_mode(), SelectionMode::MULTIPLE);
    list.set_selected_index(0);
    combo_box.set_selected_index(0);
    assert_eq!(list.selection_mode(), SelectionMode::SINGLE);
    assert!(list.classes().contains("selection-none"));
    assert_eq!(list.selected_index(), -1, "None clears the selection");
    combo_box.set_selected_index(1);
    assert!(!list.classes().contains("selection-none"));
    assert_eq!(selection_mode("Extended"), Some(SelectionMode::MULTIPLE));
    assert_eq!(selection_mode("Other"), None);
}

#[test]
fn list_view_drop_moves_contacts_between_and_within_the_lists() {
    let _application = start();
    // The insertion index of ListView_Drop: the item under the point, the next from its lower half.
    assert_eq!(insertion_index(0, 40.0, 100.0), 0);
    assert_eq!(insertion_index(3, 40.0, 10.0), 0);
    assert_eq!(insertion_index(3, 40.0, 30.0), 1);
    assert_eq!(insertion_index(3, 40.0, 500.0), 3);
    let page = ListViewPage::new();
    let first = page.contacts1.get(0);
    let santa = contact_of(&page.contacts2.get(2)).unwrap();
    // From the second list to the top of the first.
    page.drop_contacts(true, &[santa.clone()], 0);
    assert_eq!(names(&page.contacts1.to_vec())[0], "Santa Claus");
    assert_eq!(names(&page.contacts2.to_vec()), ["John Doe", "Jane Doe"]);
    // From the first list to the end of the second.
    let kendall = contact_of(&first).unwrap();
    page.drop_contacts(false, &[kendall], 2);
    assert_eq!(
        names(&page.contacts2.to_vec()),
        ["John Doe", "Jane Doe", "Kendall Collins"]
    );
    assert_eq!(page.contacts1.count(), 27);
    // Within a list: the contact moves.
    page.drop_contacts(true, &[santa], 3);
    let first_names = names(&page.contacts1.to_vec());
    assert_eq!(first_names.iter().filter(|name| *name == "Santa Claus").count(), 1);
    assert_eq!(first_names[2], "Santa Claus");
}

#[test]
fn list_view_grouped_list_shows_each_header_before_its_contacts() {
    let _application = start();
    let root = page();
    let items = items(&root, "GroupedListViewCtrl");
    assert_eq!(items.len(), 27 + 11);
    let keys: Vec<_> = items
        .iter()
        .filter_map(|item| item.downcast_ref::<Rc<GroupInfoList>>().map(|group| group.key.clone()))
        .collect();
    assert_eq!(keys, ["B", "C", "D", "H", "I", "L", "M", "P", "R", "T", "W"]);
    assert!(items[0].downcast_ref::<Rc<GroupInfoList>>().is_some());
    assert_eq!(
        names(&items[1..5]),
        ["Victoria Burke", "Ana Bowman", "Madison Butler", "Graham Barnes"]
    );
    assert!(items[5].downcast_ref::<Rc<GroupInfoList>>().is_some());
}

#[test]
fn list_view_filters_remove_and_add_back_contacts_and_announce_the_count() {
    let _application = start();
    let root = page();
    root.get_control::<TextBox>("FilterByFirstName").set_text(Some("an"));
    let shown = names(&items(&root, "FilteredListView"));
    assert!(shown
        .iter()
        .all(|name| name.split(' ').next().unwrap().to_lowercase().contains("an")));
    assert_eq!(
        root.get_control::<TextBlock>("FilteredCount").text().as_deref(),
        Some(format!("Found {} contacts", shown.len()).as_str())
    );
    root.get_control::<TextBox>("FilterByCompany")
        .set_text(Some("FABRIKAM"));
    assert_eq!(names(&items(&root, "FilteredListView")), ["Ana Bowman"]);
    root.get_control::<TextBox>("FilterByCompany").set_text(Some(""));
    root.get_control::<TextBox>("FilterByFirstName").set_text(Some(""));
    let all = names(&items(&root, "FilteredListView"));
    assert_eq!(all.len(), 27);
    // AddBack_Contacts adds the contacts that match again at the end.
    assert_eq!(all[0], "Ana Bowman");
    assert_eq!(
        root.get_control::<TextBlock>("FilteredCount").text().as_deref(),
        Some("Found 27 contacts")
    );
}

#[test]
fn list_view_messages_are_sent_and_received_with_their_alignment() {
    let _application = start();
    let root = page();
    click(&root, "ReceiveMessage");
    click(&root, "SendMessage");
    let messages: Vec<_> = items(&root, "InvertedListView")
        .iter()
        .map(|item| {
            let message = item.downcast_ref::<Rc<Message>>().unwrap();
            (message.msg_text.clone(), message.msg_alignment)
        })
        .collect();
    assert_eq!(
        messages,
        [
            ("Message 1".to_string(), HorizontalAlignment::Right),
            ("Message 2".to_string(), HorizontalAlignment::Left),
            ("Message 3".to_string(), HorizontalAlignment::Right),
        ]
    );
    let list = root.get_control::<ListBox>("InvertedListView");
    assert!(
        list.classes().contains("selection-none"),
        "the source's SelectionMode None"
    );
}

#[test]
fn list_view_context_menu_delete_removes_the_contact() {
    let _application = start();
    let page = ListViewPage::new();
    let ross = contact_of(&page.contacts4_context_menu.get(1)).unwrap();
    page.delete_contact(&ross);
    let left = names(&page.contacts4_context_menu.to_vec());
    assert_eq!(left.len(), 26);
    assert!(!left.contains(&"Henry Ross".to_string()));
}

/// In a window with the WinUI theme: the context menu of an item, scrolling into view and the saved
/// scroll position.
#[cfg(feature = "theme")]
#[test]
fn list_view_context_menu_and_scrolling_act_on_the_realized_items() {
    use ferroui_controls::{MenuItem, NumberBox, ScrollViewer, Window};
    let _application = crate::tests::start_winui();
    let root = page();
    let window = Window::new();
    window.set_width(1200.0);
    window.set_height(4000.0);
    window.set_content(Some(Control::boxed(root.clone())));
    window.show();
    window.update_layout();
    // The Delete item of the first contact's context menu.
    let list = root.get_control::<ListBox>("ContextMenuList");
    let container = list.container_from_index(0).expect("the first contact is realized");
    let content = container
        .get_visual_descendants()
        .filter_map(|visual| visual.cast::<Control>())
        .find(|control| control.context_flyout().is_some())
        .expect("the item's grid has the context menu");
    let menu = content
        .context_flyout()
        .unwrap()
        .cast::<ferroui_controls::MenuFlyout>()
        .unwrap();
    let delete = menu
        .items()
        .view()
        .get_at(0)
        .and_then(|item| Control::from_boxed(&item))
        .and_then(|item| item.cast::<MenuItem>());
    delete
        .expect("the Delete item")
        .raise_event(&RoutedEventArgs::with_event(&MenuItem::click_event()));
    assert_eq!(names(&items(&root, "ContextMenuList")).len(), 26);
    // ScrollIntoView with the Leading alignment puts the item at the top of the viewport.
    let scroll = root.get_control::<ListBox>("ScrollIntoViewListView");
    root.get_control::<NumberBox>("ScrollIndexNumberBox")
        .set_numeric_value(100.0);
    root.get_control::<ComboBox>("ScrollAlignmentComboBox")
        .set_selected_index(1);
    click(&root, "ScrollIntoViewButton");
    window.update_layout();
    let viewer = scroll
        .scroll()
        .and_then(|scroll| scroll.cast::<ScrollViewer>())
        .unwrap();
    assert!(viewer.offset().y > 0.0);
    let item = scroll.container_from_index(100).expect("the item is realized");
    let top = item
        .translate_point(ferroui_base::Point::new(0.0, 0.0), &viewer)
        .unwrap()
        .y;
    assert!(top.abs() < 1.0, "the item's top is at the viewport's top: {top}");
    // Save, scroll away, restore.
    let restore = root.get_control::<ListBox>("RestoreScrollListView");
    let restore_viewer = restore
        .scroll()
        .and_then(|scroll| scroll.cast::<ScrollViewer>())
        .unwrap();
    restore.scroll_into_view(20);
    window.update_layout();
    let saved = restore_viewer.offset().y;
    assert!(saved > 0.0, "the list scrolled to its 21st item");
    assert!(!root.get_control::<Button>("RestorePositionButton").is_enabled());
    click(&root, "SavePositionButton");
    assert!(root.get_control::<Button>("RestorePositionButton").is_enabled());
    restore.scroll_into_view(0);
    window.update_layout();
    assert!(restore_viewer.offset().y < saved);
    click(&root, "RestorePositionButton");
    window.update_layout();
    assert!(
        (restore_viewer.offset().y - saved).abs() < 1.0,
        "restored {} for {saved}",
        restore_viewer.offset().y
    );
}
