//! The code behind of the pinned ListView page in Rust: `ListViewPage.xaml.cs` with its models
//! (`Contact`, `GroupInfoList`, `Message`, and the `CustomDataObject` of the gallery's
//! ConnectedAnimation page) and `ListViewPersistenceHelper`.
//!
//! FerroUI has no ListView: every list of the page is a ListBox. Adaptations: x:Bind in the
//! templates becomes `{Binding}` over the markup types declared here; the contacts of
//! `Assets/SampleMedia/Contacts.txt` are a table; the item collections are `FerroList`s set as the
//! lists' items sources (the source adds the inverted list's messages to `Items`); the grouped list
//! is one list of the group headers followed by their contacts; WinUI's `DragItemsStarting` is a
//! drag started by the pointer on an item, carrying the contacts in an in-process format.

use crate::connected_animation_samples::DESCRIPTIONS;
use ferroui_base::collections::FerroList;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::{
    DataFormat, DataFormatOf, DataTransfer, DataTransferExtensions, DataTransferItem, DragDrop, DragDropEffects,
    DragEventArgs, IDataTransfer, InputElement, PointerEventArgs, PointerPressedEventArgs,
};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::media::{FontWeight, IImage};
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::DateTime;
use ferroui_base::{ferro_markup_type, BoxedValue, Point, Ref, Vector, Visual};
use ferroui_controls::automation::{AccessibilityView, AutomationProperties};
use ferroui_controls::primitives::SelectingItemsControl;
use ferroui_controls::templates::FuncDataTemplate;
use ferroui_controls::{
    Border, Button, ComboBox, Control, ItemsSource, ListBox, MenuFlyout, MenuItem, NumberBox, ScrollViewer,
    SelectionMode, TextBlock, TextBox,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn wire(page_id: &str, root: &Ref<Control>) {
    if page_id == "ListView" {
        register();
        wire_list_view(root);
    }
}

/// The markup types of the page, for the `{Binding}` paths of its templates.
fn register() {
    MarkupType::register_all(&[
        <Contact as MarkupTyped>::MARKUP,
        <GroupInfoList as MarkupTyped>::MARKUP,
        <Message as MarkupTyped>::MARKUP,
        <CustomDataObject as MarkupTyped>::MARKUP,
    ]);
    ValueTypes::register_global(register_value_types);
}

fn register_value_types() {
    ValueTypes::register_reference::<Contact>();
    ValueTypes::register_reference::<GroupInfoList>();
    ValueTypes::register_reference::<Message>();
    ValueTypes::register_reference::<CustomDataObject>();
}

// --- the models of the page -------------------------------------------------------------------

/// The first name, last name and company lines of `Assets/SampleMedia/Contacts.txt`.
pub(crate) const CONTACTS: [(&str, &str, &str); 27] = [
    ("Kendall", "Collins", "Adatum Corporation"),
    ("Henry", "Ross", "Adventure Works Cycles"),
    ("Vance", "DeLeon", "Alpine Ski House"),
    ("Victoria", "Burke", "Bellows College"),
    ("Amber", "Rodriguez", "Best For You Organics Company"),
    ("Amari", "Rivera", "Contoso, Ltd."),
    ("Jessie", "Irwin", "Contoso Pharmaceuticals"),
    ("Quinn", "Campbell", "Contoso Suites"),
    ("Olivia", "Wilson", "Consolidated Messenger"),
    ("Ana", "Bowman", "Fabrikam, Inc."),
    ("Shawn", "Hughes", "Fabrikam Residences"),
    ("Oscar", "Ward", "First Up Consultants"),
    ("Madison", "Butler", "Fourth Coffee"),
    ("Graham", "Barnes", "Graphic Design Institute"),
    ("Anthony", "Ivanov", "Humongous Insurance"),
    ("Michael", "Peltier", "Lamna Healthcare Company"),
    ("Morgan", "Connors", "Liberty's Delightful Sinful Bakery & Cafe"),
    ("Andre", "Lawson", "Lucerne Publishing"),
    ("Preston", "Morales", "Margie's Travel"),
    ("Briana", "Hernandez", "Nod Publishers"),
    ("Nicole", "Wagner", "Northwind Traders"),
    ("Mario", "Rogers", "Proseware, Inc."),
    ("Eugenia", "Lopez", "Relecloud"),
    ("Nathan", "Rigby", "School of Fine Art"),
    ("Ellis", "Turner", "Southridge Video"),
    ("Miguel", "Reyes", "Tailspin Toys"),
    ("Hayden", "Cook", "Tailwind Traders"),
];

/// `Contact`: a contact of the lists.
pub(crate) struct Contact {
    pub(crate) first_name: String,
    pub(crate) last_name: String,
    pub(crate) company: String,
}

impl PartialEq for Contact {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Contact {
    pub(crate) fn new(first_name: &str, last_name: &str, company: &str) -> Rc<Contact> {
        Rc::new(Contact {
            first_name: first_name.to_string(),
            last_name: last_name.to_string(),
            company: company.to_string(),
        })
    }

    /// `Name`: the first name and the last name.
    pub(crate) fn name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }

    /// `GetContactsAsync`: new contacts of the contacts file.
    pub(crate) fn get_contacts() -> Vec<Rc<Contact>> {
        CONTACTS
            .iter()
            .map(|(first, last, company)| Contact::new(first, last, company))
            .collect()
    }

    /// `GetContactsGroupedAsync`: the contacts grouped by the upper-case first letter of their
    /// last name, ordered by that letter.
    pub(crate) fn get_contacts_grouped() -> Vec<(String, Vec<Rc<Contact>>)> {
        let mut groups: Vec<(String, Vec<Rc<Contact>>)> = Vec::new();
        for contact in Contact::get_contacts() {
            let key: String = contact.last_name.chars().take(1).collect::<String>().to_uppercase();
            match groups.iter_mut().find(|(group, _)| *group == key) {
                Some((_, items)) => items.push(contact),
                None => groups.push((key, vec![contact])),
            }
        }
        groups.sort_by(|left, right| left.0.cmp(&right.0));
        groups
    }
}

ferro_markup_type!(class Contact {
    this: Rc<Contact>,
    handles: [Contact, Rc<Contact>, Option<Rc<Contact>>],
    properties: [
        FirstName: String { get: |this: &Rc<Contact>| this.first_name.clone() },
        LastName: String { get: |this: &Rc<Contact>| this.last_name.clone() },
        Company: String { get: |this: &Rc<Contact>| this.company.clone() },
        Name: String { get: |this: &Rc<Contact>| this.name() },
    ],
});

/// `GroupInfoList`: the header of a group of the grouped list, whose contacts follow it.
pub(crate) struct GroupInfoList {
    pub(crate) key: String,
}

impl PartialEq for GroupInfoList {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

ferro_markup_type!(class GroupInfoList {
    this: Rc<GroupInfoList>,
    handles: [GroupInfoList, Rc<GroupInfoList>, Option<Rc<GroupInfoList>>],
    properties: [
        Key: String { get: |this: &Rc<GroupInfoList>| this.key.clone() },
    ],
});

/// `Message`: a message of the inverted list. `MsgDateTime` is the text of the source's
/// `DateTime` (its `ToString()`, the general format of the current culture).
pub(crate) struct Message {
    pub(crate) msg_text: String,
    pub(crate) msg_date_time: String,
    pub(crate) msg_alignment: HorizontalAlignment,
}

impl PartialEq for Message {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

ferro_markup_type!(class Message {
    this: Rc<Message>,
    handles: [Message, Rc<Message>, Option<Rc<Message>>],
    properties: [
        MsgText: String { get: |this: &Rc<Message>| this.msg_text.clone() },
        MsgDateTime: String { get: |this: &Rc<Message>| this.msg_date_time.clone() },
        MsgAlignment: HorizontalAlignment { get: |this: &Rc<Message>| this.msg_alignment },
    ],
});

/// `CustomDataObject`: an item of the list with images. `Image` is the bitmap of the source's
/// `ImageLocation`, decoded from the embedded sample media (`None` when it cannot be decoded).
pub(crate) struct CustomDataObject {
    pub(crate) title: String,
    pub(crate) image_location: String,
    pub(crate) image: Option<Rc<dyn IImage>>,
    pub(crate) views: String,
    pub(crate) likes: String,
    pub(crate) description: &'static str,
}

impl PartialEq for CustomDataObject {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl CustomDataObject {
    /// `GetDataObjects(includeAllItems: false)`: eight items with random views (100 to 998) and
    /// likes (10 to 98), as `Random.Next(min, max)`.
    pub(crate) fn get_data_objects() -> Vec<Rc<CustomDataObject>> {
        let mut seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        let mut next = |min: u32, max: u32| {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            min + ((seed >> 32) as u32) % (max - min)
        };
        (0..8)
            .map(|index| {
                let image_location = format!(
                    "ferres://WinUIGallery/Assets/SampleMedia/LandscapeImage{}.jpg",
                    index + 1
                );
                let image = crate::assets::load_bitmap(&image_location)
                    .ok()
                    .map(|bitmap| Rc::new(bitmap) as Rc<dyn IImage>);
                Rc::new(CustomDataObject {
                    title: format!("Item {}", index + 1),
                    image_location,
                    image,
                    views: next(100, 999).to_string(),
                    likes: next(10, 99).to_string(),
                    description: DESCRIPTIONS[index % DESCRIPTIONS.len()],
                })
            })
            .collect()
    }
}

ferro_markup_type!(class CustomDataObject {
    this: Rc<CustomDataObject>,
    handles: [CustomDataObject, Rc<CustomDataObject>, Option<Rc<CustomDataObject>>],
    properties: [
        Title: String { get: |this: &Rc<CustomDataObject>| this.title.clone() },
        ImageLocation: String { get: |this: &Rc<CustomDataObject>| this.image_location.clone() },
        Image: Option<Rc<dyn IImage>> { get: |this: &Rc<CustomDataObject>| this.image.clone() },
        Views: String { get: |this: &Rc<CustomDataObject>| this.views.clone() },
        Likes: String { get: |this: &Rc<CustomDataObject>| this.likes.clone() },
        Description: String { get: |this: &Rc<CustomDataObject>| this.description.to_string() },
    ],
});

fn boxed<T: PartialEq + 'static>(item: Rc<T>) -> BoxedValue {
    Rc::new(item) as BoxedValue
}

/// The contact of an item of a list, if it is one.
pub(crate) fn contact_of(item: &BoxedValue) -> Option<Rc<Contact>> {
    item.downcast_ref::<Rc<Contact>>().cloned()
}

fn contact_list(contacts: Vec<Rc<Contact>>) -> Rc<FerroList<BoxedValue>> {
    Rc::new(FerroList::from_items(contacts.into_iter().map(boxed)))
}

fn set_items(list: &Ref<ListBox>, items: &Rc<FerroList<BoxedValue>>) {
    list.set_items_source(Some(items.clone().into()));
}

// --- the page ---------------------------------------------------------------------------------

/// The state of `ListViewPage`: the collections of its lists.
pub(crate) struct ListViewPage {
    pub(crate) contacts1: Rc<FerroList<BoxedValue>>,
    pub(crate) contacts2: Rc<FerroList<BoxedValue>>,
    pub(crate) contacts3: Vec<Rc<Contact>>,
    pub(crate) contacts3_filtered: Rc<FerroList<BoxedValue>>,
    pub(crate) contacts4_context_menu: Rc<FerroList<BoxedValue>>,
    pub(crate) messages: Rc<FerroList<BoxedValue>>,
    message_number: Cell<u32>,
    persisted_position: RefCell<Option<(String, f64)>>,
}

impl ListViewPage {
    /// The collections of the constructor and of `OnNavigatedTo`, with the first message of the
    /// inverted list.
    pub(crate) fn new() -> Rc<ListViewPage> {
        let contacts3 = Contact::get_contacts();
        // `contacts3Filtered` holds the very contacts of `contacts3`.
        let contacts3_filtered = Rc::new(FerroList::from_items(contacts3.iter().cloned().map(boxed)));
        let page = Rc::new(ListViewPage {
            contacts1: contact_list(Contact::get_contacts()),
            contacts2: contact_list(vec![
                Contact::new("John", "Doe", "ABC Printers"),
                Contact::new("Jane", "Doe", "XYZ Refrigerators"),
                Contact::new("Santa", "Claus", "North Pole Toy Factory Inc."),
            ]),
            contacts3,
            contacts3_filtered,
            contacts4_context_menu: contact_list(Contact::get_contacts()),
            messages: Rc::new(FerroList::new()),
            message_number: Cell::new(0),
            persisted_position: RefCell::new(None),
        });
        page.add_message(HorizontalAlignment::Right);
        page
    }

    /// `AddItemToEnd` and `MessageReceived`: a new message, sent (right) or received (left).
    pub(crate) fn add_message(&self, alignment: HorizontalAlignment) {
        self.message_number.set(self.message_number.get() + 1);
        self.messages.add(boxed(Rc::new(Message {
            msg_text: format!("Message {}", self.message_number.get()),
            msg_date_time: DateTime::now().to_string(),
            msg_alignment: alignment,
        })));
    }

    /// `OnFilterChanged`: `Remove_NonMatching` then `AddBack_Contacts` (the contacts that match
    /// again are added at the end, as the source does); returns the count it announces.
    pub(crate) fn filter(&self, first: &str, last: &str, company: &str) -> usize {
        let contains = |text: &str, filter: &str| text.to_lowercase().contains(&filter.to_lowercase());
        let filtered: Vec<_> = self
            .contacts3
            .iter()
            .filter(|contact| {
                contains(&contact.first_name, first)
                    && contains(&contact.last_name, last)
                    && contains(&contact.company, company)
            })
            .cloned()
            .collect();
        let shown = |contact: &Rc<Contact>| {
            self.contacts3_filtered
                .to_vec()
                .iter()
                .any(|item| contact_of(item).is_some_and(|item| Rc::ptr_eq(&item, contact)))
        };
        for index in (0..self.contacts3_filtered.count()).rev() {
            let item = contact_of(&self.contacts3_filtered.get(index));
            if !item.is_some_and(|item| filtered.iter().any(|contact| Rc::ptr_eq(contact, &item))) {
                self.contacts3_filtered.remove_at(index);
            }
        }
        for contact in &filtered {
            if !shown(contact) {
                self.contacts3_filtered.add(boxed(contact.clone()));
            }
        }
        filtered.len()
    }

    /// `ContactDeleteMenuItem_Click`: the contact of the item leaves the list.
    pub(crate) fn delete_contact(&self, contact: &Rc<Contact>) {
        if let Some(index) = position(&self.contacts4_context_menu, contact) {
            self.contacts4_context_menu.remove_at(index);
        }
    }

    /// `ListView_Drop`: the contacts go to `index` of the target list (`first` is
    /// DragDropListView) and leave the other list. A drop on the list the contacts come from
    /// moves them there, as WinUI's `CanReorderItems`.
    pub(crate) fn drop_contacts(&self, first: bool, contacts: &[Rc<Contact>], index: usize) {
        let (target, other) = if first {
            (&self.contacts1, &self.contacts2)
        } else {
            (&self.contacts2, &self.contacts1)
        };
        for contact in contacts {
            let mut index = index.min(target.count());
            if let Some(old) = position(target, contact) {
                target.remove_at(old);
                if old < index {
                    index -= 1;
                }
            }
            target.insert(index, boxed(contact.clone()));
            // The source compares the names and the company of the contacts of the other list.
            if let Some(old) = (0..other.count()).find(|&old| {
                contact_of(&other.get(old)).is_some_and(|item| {
                    item.first_name == contact.first_name
                        && item.last_name == contact.last_name
                        && item.company == contact.company
                })
            }) {
                other.remove_at(old);
            }
        }
    }
}

fn position(list: &FerroList<BoxedValue>, contact: &Rc<Contact>) -> Option<usize> {
    (0..list.count()).find(|&index| contact_of(&list.get(index)).is_some_and(|item| Rc::ptr_eq(&item, contact)))
}

/// The selection mode of WinUI's `ListViewSelectionMode` name. FerroUI's SelectionMode has no
/// None (see [`set_selection_none`]); Multiple, whose click toggles an item, is Multiple with Toggle;
/// Extended, with Ctrl+Click and Shift+Click, is Multiple.
pub(crate) fn selection_mode(name: &str) -> Option<SelectionMode> {
    match name {
        "None" | "Single" => Some(SelectionMode::SINGLE),
        "Multiple" => Some(SelectionMode::MULTIPLE | SelectionMode::TOGGLE),
        "Extended" => Some(SelectionMode::MULTIPLE),
        _ => None,
    }
}

/// WinUI's `SelectionMode="None"`: the list's items are neither selected nor hit by the pointer
/// nor focused (the class `selection-none` of the page's style).
pub(crate) fn set_selection_none(list: &Ref<ListBox>, none: bool) {
    if none {
        list.unselect_all();
    }
    list.classes().set("selection-none", none);
}

/// The insertion index of `ListView_Drop`: the item under the drop position (`y` in the items
/// panel, items of `item_height`), and the one after it from its lower half.
pub(crate) fn insertion_index(count: usize, item_height: f64, y: f64) -> usize {
    if count == 0 || item_height <= 0.0 {
        return 0;
    }
    let mut index = ((y / item_height).max(0.0) as usize).min(count - 1);
    if y - index as f64 * item_height > item_height / 2.0 {
        index += 1;
    }
    index.min(count)
}

/// The in-process format of the dragged contacts (the source's `Data.Properties["contacts"]`).
fn contacts_format() -> DataFormatOf<Vec<Rc<Contact>>> {
    DataFormat::create_in_process_format::<Vec<Rc<Contact>>>("WinUIGallery.ListView.contacts")
}

fn wire_list_view(root: &Ref<Control>) {
    // The page's constructor and `OnNavigatedTo`.
    let page = ListViewPage::new();
    let base = root.get_control::<ListBox>("BaseExample");
    set_items(&base, &contact_list(Contact::get_contacts()));
    // `BaseExample_Loaded`: the list takes the focus, so that its first item has it.
    let weak_base = base.downgrade();
    base.loaded(move |_, _| {
        if let Some(base) = weak_base.upgrade() {
            base.focus();
        }
    });

    wire_selection_modes(root);
    wire_drag_and_drop(root, &page);
    wire_grouped(root);
    wire_filter(root, &page);
    wire_messages(root, &page);
    root.get_control::<ListBox>("Control4")
        .set_items_source(Some(ItemsSource::from_items(
            CustomDataObject::get_data_objects()
                .into_iter()
                .map(|item| Some(boxed(item))),
        )));
    wire_context_menu(root, &page);
    wire_scroll_position(root, &page);
    wire_scroll_into_view(root);
}

/// `SelectionModeComboBox_SelectionChanged`.
fn wire_selection_modes(root: &Ref<Control>) {
    let list = root.get_control::<ListBox>("Control2");
    set_items(&list, &contact_list(Contact::get_contacts()));
    let combo_box = root.get_control::<ComboBox>("SelectionModeComboBox");
    let weak_combo_box = combo_box.downgrade();
    let weak_list = list.downgrade();
    combo_box.property_changed(move |change| {
        if change.property() != SelectingItemsControl::selected_index_property().as_property() {
            return;
        }
        let (Some(combo_box), Some(list)) = (weak_combo_box.upgrade(), weak_list.upgrade()) else {
            return;
        };
        let item = combo_box.selected_item();
        let name = item
            .as_ref()
            .and_then(|item| item.downcast_ref::<String>())
            .map(String::as_str)
            .unwrap_or_default();
        if let Some(mode) = selection_mode(name) {
            list.set_selection_mode(mode);
            set_selection_none(&list, name == "None");
        }
    });
}

/// `ListView_DragItemsStarting`, `Source_DragOver`, `Target_DragOver` and `ListView_Drop`.
fn wire_drag_and_drop(root: &Ref<Control>, page: &Rc<ListViewPage>) {
    for (name, first) in [("DragDropListView", true), ("DragDropListView2", false)] {
        let list = root.get_control::<ListBox>(name);
        set_items(&list, if first { &page.contacts1 } else { &page.contacts2 });
        // The drag starts once the pointer pressed on an item moves (WinUI's CanDragItems).
        let pressed: Rc<RefCell<Option<(PointerPressedEventArgs, Point, Rc<Contact>)>>> = Rc::default();
        {
            let pressed = pressed.clone();
            let weak_list = list.downgrade();
            list.add_handler_with(
                InputElement::pointer_pressed_event(),
                move |_, e: &PointerPressedEventArgs| {
                    let Some(list) = weak_list.upgrade() else {
                        return;
                    };
                    let source = e.source().and_then(|source| source.cast::<Visual>());
                    let contact = list
                        .get_container_from_event_source(source.as_ref())
                        .and_then(|container| list.item_from_container(&container))
                        .and_then(|item| contact_of(&item));
                    *pressed.borrow_mut() = match contact {
                        Some(contact) if e.properties().is_left_button_pressed => {
                            Some((e.clone(), e.get_position(Some(&list)), contact))
                        }
                        _ => None,
                    };
                },
                RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE,
                true,
            );
        }
        {
            let pressed = pressed.clone();
            let weak_list = list.downgrade();
            list.add_handler_with(
                InputElement::pointer_moved_event(),
                move |_, e: &PointerEventArgs| {
                    let Some(list) = weak_list.upgrade() else {
                        return;
                    };
                    let moved = pressed.borrow().as_ref().is_some_and(|(_, start, _)| {
                        let point = e.get_position(Some(&list));
                        (point.x - start.x).abs() > 4.0 || (point.y - start.y).abs() > 4.0
                    });
                    if !moved {
                        return;
                    }
                    let Some((trigger, _, contact)) = pressed.borrow_mut().take() else {
                        return;
                    };
                    let item = DataTransferItem::new();
                    item.set(&contacts_format(), Some(vec![contact]));
                    let transfer = DataTransfer::new();
                    transfer.add(item);
                    let transfer: Rc<dyn IDataTransfer> = transfer;
                    let operation = DragDrop::do_drag_drop_async(&trigger, transfer, DragDropEffects::MOVE);
                    drop(Dispatcher::ui_thread().spawn_local(async move {
                        operation.await;
                    }));
                },
                RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE,
                true,
            );
        }
        list.add_handler_with(
            InputElement::pointer_released_event(),
            move |_, _: &ferroui_base::input::PointerReleasedEventArgs| {
                pressed.borrow_mut().take();
            },
            RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE,
            true,
        );
        // `Source_DragOver` and `Target_DragOver`: the operation is a move.
        DragDrop::add_drag_over_handler(&list, |_, e: &DragEventArgs| {
            if e.data_transfer().contains(&contacts_format()) {
                e.set_drag_effects(DragDropEffects::MOVE);
            }
        });
        let page = page.clone();
        let weak_list = list.downgrade();
        DragDrop::add_drop_handler(&list, move |_, e: &DragEventArgs| {
            let Some(list) = weak_list.upgrade() else {
                return;
            };
            let Some(contacts) = e.data_transfer().try_get_value(&contacts_format()) else {
                return;
            };
            let count = list.item_count() as usize;
            let (item_height, y) = match (list.container_from_index(0), list.items_panel_root()) {
                (Some(first_item), Some(panel)) => {
                    let margin = first_item.margin();
                    (
                        first_item.bounds().height + margin.top + margin.bottom,
                        e.get_position(&panel).y,
                    )
                }
                _ => (0.0, 0.0),
            };
            page.drop_contacts(first, &contacts, insertion_index(count, item_height, y));
            e.set_drag_effects(DragDropEffects::MOVE);
        });
    }
}

/// The header of a group, the source's `GroupStyle.HeaderTemplate`: the key in the
/// TitleTextBlockStyle (28, semibold), raw for automation.
fn group_header(group: &GroupInfoList) -> Ref<Control> {
    let text = TextBlock::new();
    text.set_text(Some(&group.key));
    text.set_font_size(28.0);
    text.set_font_weight(FontWeight::SemiBold);
    AutomationProperties::set_accessibility_view(&text, AccessibilityView::Raw);
    let border = Border::new();
    AutomationProperties::set_accessibility_view(&border, AccessibilityView::Raw);
    border.set_child(&text);
    border.upcast()
}

/// The grouped list: each group's header before its contacts, which have the list's item template.
/// The headers are not items one can select or focus.
fn wire_grouped(root: &Ref<Control>) {
    let list = root.get_control::<ListBox>("GroupedListViewCtrl");
    let Some(contact) = list.item_template() else {
        return;
    };
    list.set_item_template(Some(FuncDataTemplate::new(
        |_| true,
        move |data, _| match data.as_ref().and_then(|data| data.downcast_ref::<Rc<GroupInfoList>>()) {
            Some(group) => Some(group_header(group)),
            None => contact.build(data),
        },
        false,
    )));
    let mut items = Vec::new();
    for (key, contacts) in Contact::get_contacts_grouped() {
        items.push(Some(boxed(Rc::new(GroupInfoList { key }))));
        items.extend(contacts.into_iter().map(|contact| Some(boxed(contact))));
    }
    let headers: Vec<bool> = items
        .iter()
        .map(|item| {
            item.as_ref()
                .is_some_and(|item| item.downcast_ref::<Rc<GroupInfoList>>().is_some())
        })
        .collect();
    list.set_items_source(Some(ItemsSource::from_items(items)));
    list.container_prepared(move |args| {
        let is_header = usize::try_from(args.index()).is_ok_and(|index| headers.get(index) == Some(&true));
        let container = args.container();
        container.set_is_hit_test_visible(!is_header);
        container.set_focusable(!is_header);
    })
    .detach();
}

/// `OnFilterChanged` of the three filter boxes, and its announcement of the count (a polite live
/// region, for `UIHelper.AnnounceActionForAccessibility`).
fn wire_filter(root: &Ref<Control>, page: &Rc<ListViewPage>) {
    set_items(
        &root.get_control::<ListBox>("FilteredListView"),
        &page.contacts3_filtered,
    );
    let boxes =
        ["FilterByFirstName", "FilterByLastName", "FilterByCompany"].map(|name| root.get_control::<TextBox>(name));
    let weak_boxes = boxes.clone().map(|text_box| text_box.downgrade());
    let weak_output = root.get_control::<TextBlock>("FilteredCount").downgrade();
    let page = page.clone();
    let update = Rc::new(move || {
        let Some(output) = weak_output.upgrade() else {
            return;
        };
        let [first, last, company] = weak_boxes.clone().map(|text_box| {
            text_box
                .upgrade()
                .and_then(|text_box| text_box.text())
                .unwrap_or_default()
        });
        let found = page.filter(&first, &last, &company);
        output.set_text(Some(&format!("Found {found} contacts")));
    });
    for text_box in boxes {
        let update = update.clone();
        text_box.property_changed(move |change| {
            if change.property() == TextBox::text_property().as_property() {
                update();
            }
        });
    }
}

/// `AddItemToEnd` and `MessageReceived`. The inverted list keeps its last item in view
/// (`ItemsUpdatingScrollMode="KeepLastItemInView"`).
fn wire_messages(root: &Ref<Control>, page: &Rc<ListViewPage>) {
    let list = root.get_control::<ListBox>("InvertedListView");
    set_items(&list, &page.messages);
    set_selection_none(&list, true);
    for (name, alignment) in [
        ("SendMessage", HorizontalAlignment::Right),
        ("ReceiveMessage", HorizontalAlignment::Left),
    ] {
        let page = page.clone();
        let weak_list = list.downgrade();
        root.get_control::<Button>(name).click(move |_, _| {
            let Some(list) = weak_list.upgrade() else {
                return;
            };
            page.add_message(alignment);
            list.scroll_into_view(page.messages.count() as i32 - 1);
        });
    }
}

/// The context menu of the contacts: Delete (`ContactDeleteMenuItem_Click`) removes the contact
/// of the item.
fn wire_context_menu(root: &Ref<Control>, page: &Rc<ListViewPage>) {
    let list = root.get_control::<ListBox>("ContextMenuList");
    set_items(&list, &page.contacts4_context_menu);
    let Some(inner) = list.item_template() else {
        return;
    };
    let page = page.clone();
    list.set_item_template(Some(FuncDataTemplate::new(
        |_| true,
        move |data, _| {
            let grid = inner.build(data)?;
            let delete = MenuItem::new();
            delete.set_header(Some(Rc::new("Delete".to_string()) as BoxedValue));
            let page = page.clone();
            let contact = data.as_ref().and_then(contact_of);
            delete.click(move |_, _| {
                if let Some(contact) = contact.as_ref() {
                    page.delete_contact(contact);
                }
            });
            let menu = MenuFlyout::new();
            menu.items().add(Some(Control::boxed(delete)));
            grid.set_context_flyout(menu);
            Some(grid)
        },
        false,
    )));
}

/// The scroll viewer of a list.
fn scroll_viewer(list: &Ref<ListBox>) -> Option<Ref<ScrollViewer>> {
    list.scroll().and_then(|scroll| scroll.cast::<ScrollViewer>())
}

/// The top of a list's container in its scroll viewer's viewport.
fn top_in_viewport(container: &Ref<Control>, viewer: &Ref<ScrollViewer>) -> Option<f64> {
    container
        .translate_point(Point::new(0.0, 0.0), viewer)
        .map(|point| point.y)
}

/// `ListViewPersistenceHelper.GetRelativeScrollPosition`: the key of the first item that reaches
/// below the top of the viewport, and how far the viewport's top is below the item's top.
pub(crate) fn relative_scroll_position(list: &Ref<ListBox>) -> Option<(String, f64)> {
    let viewer = scroll_viewer(list)?;
    let mut containers = list.get_realized_containers();
    containers.sort_by_key(|container| list.index_from_container(container));
    containers.into_iter().find_map(|container| {
        let top = top_in_viewport(&container, &viewer)?;
        if top + container.bounds().height <= 0.0 {
            return None;
        }
        let key = list.item_from_container(&container)?.downcast_ref::<String>()?.clone();
        Some((key, -top))
    })
}

/// `ListViewPersistenceHelper.SetRelativeScrollPositionAsync`: the item of the key is brought into
/// view, then the viewport's top is put at the saved distance below the item's top.
pub(crate) fn set_relative_scroll_position(list: &Ref<ListBox>, key: &str, offset: f64) {
    let Some(index) = list.items_source().and_then(|items| {
        (0..items.count()).find(|&index| {
            items
                .get_at(index)
                .and_then(|item| item.downcast_ref::<String>().cloned())
                .as_deref()
                == Some(key)
        })
    }) else {
        return;
    };
    scroll_to_top(list, index as i32, offset);
}

/// Scrolls the item of `index` into view, then puts its top `offset` above the viewport's top.
fn scroll_to_top(list: &Ref<ListBox>, index: i32, offset: f64) {
    list.scroll_into_view(index);
    list.update_layout();
    let (Some(viewer), Some(container)) = (scroll_viewer(list), list.container_from_index(index)) else {
        return;
    };
    if let Some(top) = top_in_viewport(&container, &viewer) {
        let current = viewer.offset();
        viewer.set_offset(Vector::new(current.x, (current.y + top + offset).max(0.0)));
    }
}

/// `SavePositionButton_Click` and `RestorePositionButton_Click`.
fn wire_scroll_position(root: &Ref<Control>, page: &Rc<ListViewPage>) {
    let list = root.get_control::<ListBox>("RestoreScrollListView");
    // `contacts5`: each contact's name after its number.
    list.set_items_source(Some(ItemsSource::from_values(
        Contact::get_contacts()
            .iter()
            .enumerate()
            .map(|(index, contact)| format!("{}. {}", index + 1, contact.name())),
    )));
    let restore = root.get_control::<Button>("RestorePositionButton");
    {
        let page = page.clone();
        let weak_list = list.downgrade();
        let weak_restore = restore.downgrade();
        root.get_control::<Button>("SavePositionButton").click(move |_, _| {
            let (Some(list), Some(restore)) = (weak_list.upgrade(), weak_restore.upgrade()) else {
                return;
            };
            *page.persisted_position.borrow_mut() = relative_scroll_position(&list);
            restore.set_is_enabled(true);
        });
    }
    let page = page.clone();
    let weak_list = list.downgrade();
    restore.click(move |_, _| {
        let Some(list) = weak_list.upgrade() else {
            return;
        };
        let position = page.persisted_position.borrow().clone();
        if let Some((key, offset)) = position {
            set_relative_scroll_position(&list, &key, offset);
        }
    });
}

/// `ScrollIntoViewButton_Click`: Default brings the item into view, Leading puts its top at the
/// top of the viewport.
fn wire_scroll_into_view(root: &Ref<Control>) {
    let list = root.get_control::<ListBox>("ScrollIntoViewListView");
    // `contacts6`: ten copies of the contacts, numbered.
    let contacts = Contact::get_contacts();
    list.set_items_source(Some(ItemsSource::from_values(
        (0..10)
            .flat_map(|_| contacts.iter())
            .enumerate()
            .map(|(index, contact)| format!("{}. {}", index + 1, contact.name())),
    )));
    let weak_list = list.downgrade();
    let weak_index = root.get_control::<NumberBox>("ScrollIndexNumberBox").downgrade();
    let weak_alignment = root.get_control::<ComboBox>("ScrollAlignmentComboBox").downgrade();
    root.get_control::<Button>("ScrollIntoViewButton").click(move |_, _| {
        let (Some(list), Some(index), Some(alignment)) =
            (weak_list.upgrade(), weak_index.upgrade(), weak_alignment.upgrade())
        else {
            return;
        };
        let value = index.value();
        if value.is_nan() || value < 0.0 || value as usize >= list.item_count() as usize {
            return;
        }
        let index = value as i32;
        if alignment.selected_index() == 1 {
            scroll_to_top(&list, index, 0.0);
        } else {
            list.scroll_into_view(index);
        }
    });
}
