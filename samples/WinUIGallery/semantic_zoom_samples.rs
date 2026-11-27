//! The code behind of the pinned SemanticZoom page in Rust: `SemanticZoomPage.xaml.cs` with the
//! groups of `ControlInfoDataSource` (`ControlInfoDataGroup`, `ControlInfoDataItem`).
//!
//! The groups are the categories of the gallery's catalog (`CATALOG`), in the order they first
//! appear, each with its controls; the source reads them from `ControlInfoData.json`. As in the
//! source, the zoomed-in grid shows the `View` of a `CollectionViewSource` with `IsSourceGrouped`
//! and `ItemsPath="Items"`, the zoomed-out list its `View.CollectionGroups`, and `List_GotFocus`
//! brings the semantic zoom into view.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::{FocusChangedEventArgs, InputElement};
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::{ferro_markup_type, BoxedValue, Ref};
use ferroui_controls::{CollectionViewSource, Control, GridView, ItemsSource, ListView, ListViewBase, SemanticZoom};
use std::rc::Rc;

pub(crate) fn wire(page_id: &str, root: &Ref<Control>) {
    if page_id == "SemanticZoom" {
        register();
        wire_semantic_zoom(root);
    }
}

/// The markup types of the page, for the `{Binding}` paths of its templates.
fn register() {
    MarkupType::register_all(&[
        <ControlInfoDataGroup as MarkupTyped>::MARKUP,
        <ControlInfoDataItem as MarkupTyped>::MARKUP,
    ]);
    ValueTypes::register_global(register_value_types);
}

fn register_value_types() {
    ValueTypes::register_reference::<ControlInfoDataGroup>();
    ValueTypes::register_reference::<ControlInfoDataItem>();
}

/// `ControlInfoDataItem`: a control of the gallery.
pub(crate) struct ControlInfoDataItem {
    pub(crate) title: String,
    pub(crate) subtitle: String,
}

impl PartialEq for ControlInfoDataItem {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

ferro_markup_type!(class ControlInfoDataItem {
    this: Rc<ControlInfoDataItem>,
    handles: [ControlInfoDataItem, Rc<ControlInfoDataItem>, Option<Rc<ControlInfoDataItem>>],
    properties: [
        Title: String { get: |this: &Rc<ControlInfoDataItem>| this.title.clone() },
        Subtitle: String { get: |this: &Rc<ControlInfoDataItem>| this.subtitle.clone() },
    ],
});

/// `ControlInfoDataGroup`: a category of the gallery and its controls.
pub(crate) struct ControlInfoDataGroup {
    pub(crate) title: String,
    pub(crate) items: ItemsSource,
}

impl PartialEq for ControlInfoDataGroup {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

ferro_markup_type!(class ControlInfoDataGroup {
    this: Rc<ControlInfoDataGroup>,
    handles: [ControlInfoDataGroup, Rc<ControlInfoDataGroup>, Option<Rc<ControlInfoDataGroup>>],
    properties: [
        Title: String { get: |this: &Rc<ControlInfoDataGroup>| this.title.clone() },
        Items: ItemsSource { get: |this: &Rc<ControlInfoDataGroup>| this.items.clone() },
    ],
});

/// `ControlInfoDataSource.Instance.Groups`: the categories of the catalog with their controls.
pub(crate) fn groups() -> Vec<Rc<ControlInfoDataGroup>> {
    let mut titles: Vec<&'static str> = Vec::new();
    for entry in crate::CATALOG {
        if !entry.category.is_empty() && !titles.contains(&entry.category) {
            titles.push(entry.category);
        }
    }
    titles
        .into_iter()
        .map(|title| {
            let items = crate::CATALOG.iter().filter(|entry| entry.category == title).map(|entry| {
                let item = Rc::new(ControlInfoDataItem {
                    title: entry.title.to_string(),
                    subtitle: entry.subtitle.to_string(),
                });
                Some(Rc::new(item) as BoxedValue)
            });
            Rc::new(ControlInfoDataGroup { title: title.to_string(), items: ItemsSource::from_items(items) })
        })
        .collect()
}

/// The page's `cvsGroups` and the two views; `List_GotFocus` of both views.
fn wire_semantic_zoom(root: &Ref<Control>) {
    let zoom = root.get_control::<SemanticZoom>("Control1");
    let grid = root.get_control::<GridView>("ZoomedInGridView");
    let list = root.get_control::<ListView>("ZoomedOutListView");
    let groups = CollectionViewSource::new();
    groups.set_is_source_grouped(true);
    groups.set_items_path("Items");
    groups.set_source(Some(ItemsSource::from_items(
        self::groups().into_iter().map(|group| Some(Rc::new(group) as BoxedValue)),
    )));
    grid.set_items_source(groups.view());
    list.set_items_source(Some(ItemsSource::from_items(
        groups.collection_groups().into_iter().map(|group| Some(Rc::new(group) as BoxedValue)),
    )));
    for view in [grid.upcast::<ListViewBase>(), list.upcast::<ListViewBase>()] {
        let weak = zoom.downgrade();
        view.add_disposable_handler(InputElement::got_focus_event(), move |_, _: &FocusChangedEventArgs| {
            if let Some(zoom) = weak.upgrade() {
                zoom.bring_into_view();
            }
        })
        .detach();
    }
}
