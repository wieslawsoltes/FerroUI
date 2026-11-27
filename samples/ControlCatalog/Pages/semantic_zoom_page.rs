//! Not from upstream: the page of WinUI's SemanticZoom (docs/porting/winui-controls.md, section
//! 10.20), the class of `Pages/SemanticZoomPage.xaml`: the fruits grouped by their first letter in
//! the zoomed-in grid, the letters in the zoomed-out list.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{
    Button, CollectionViewGroup, CollectionViewSource, ContentPage, GridView, ItemsSource, ListView, SemanticZoom,
    TextBlock,
};
use std::rc::Rc;

const FRUITS: &[&str] = &[
    "Apple",
    "Apricot",
    "Avocado",
    "Banana",
    "Blackberry",
    "Blueberry",
    "Cherry",
    "Clementine",
    "Coconut",
    "Cranberry",
    "Date",
    "Dragonfruit",
    "Durian",
    "Elderberry",
    "Fig",
    "Grape",
    "Grapefruit",
    "Guava",
    "Kiwi",
    "Lemon",
    "Lime",
    "Lychee",
    "Mango",
    "Melon",
    "Nectarine",
    "Orange",
    "Papaya",
    "Peach",
    "Pear",
    "Pineapple",
    "Plum",
    "Pomegranate",
    "Raspberry",
    "Strawberry",
    "Tangerine",
    "Watermelon",
];

/// The fruits grouped by their first letter, as the groups of a grouped source.
pub(crate) fn fruit_groups() -> Vec<Rc<CollectionViewGroup>> {
    let mut letters: Vec<char> = FRUITS.iter().filter_map(|fruit| fruit.chars().next()).collect();
    letters.dedup();
    letters
        .into_iter()
        .map(|letter| {
            let items = FRUITS
                .iter()
                .filter(|fruit| fruit.starts_with(letter))
                .map(|fruit| Some(Rc::new(fruit.to_string()) as BoxedValue));
            CollectionViewGroup::new(Some(Rc::new(letter.to_string()) as BoxedValue), ItemsSource::from_items(items))
        })
        .collect()
}

#[repr(C)]
pub struct SemanticZoomPage {
    base: ContentPage,
}

content_page_class!(SemanticZoomPage);
ferro_class_info!(SemanticZoomPage { new: SemanticZoomPage::new });
xaml_class!(SemanticZoomPage, "/Pages/SemanticZoomPage.xaml");

impl SemanticZoomPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        let zoom = this.find_control::<SemanticZoom>("Fruits").expect("SemanticZoomPage Fruits");
        let items = this.find_control::<GridView>("FruitItems").expect("SemanticZoomPage FruitItems");
        let groups = this.find_control::<ListView>("FruitGroups").expect("SemanticZoomPage FruitGroups");
        let source = CollectionViewSource::new();
        source.set_is_source_grouped(true);
        source.set_source(Some(ItemsSource::from_items(
            fruit_groups().into_iter().map(|group| Some(Rc::new(group) as BoxedValue)),
        )));
        items.set_items_source(source.view());
        groups.set_items_source(Some(ItemsSource::from_items(
            source.collection_groups().into_iter().map(|group| Some(Rc::new(group) as BoxedValue)),
        )));
        if let Some(button) = this.find_control::<Button>("ToggleButton") {
            let weak = zoom.downgrade();
            let _ = button.click(move |_, _| {
                if let Some(zoom) = weak.upgrade() {
                    zoom.toggle_active_view();
                }
            });
        }
        if let Some(status) = this.find_control::<TextBlock>("ViewStatus") {
            let weak = status.downgrade();
            zoom.view_change_completed(move |zoom, _| {
                if let Some(status) = weak.upgrade() {
                    let view = if zoom.is_zoomed_in_view_active() { "in" } else { "out" };
                    status.set_text(Some(&format!("Zoomed {view}")));
                }
            })
            .detach();
        }
        this
    }
}

#[cfg(test)]
mod tests {
    use super::fruit_groups;

    fn text(value: &Option<ferroui_base::BoxedValue>) -> String {
        value.as_ref().and_then(|value| value.downcast_ref::<String>().cloned()).unwrap_or_default()
    }

    #[test]
    fn the_fruits_are_grouped_by_their_first_letter() {
        let groups = fruit_groups();
        let letters: Vec<String> = groups.iter().map(|group| text(&group.group())).collect();
        assert_eq!(letters.first().map(String::as_str), Some("A"));
        assert_eq!(letters.last().map(String::as_str), Some("W"));
        let mut sorted = letters.clone();
        sorted.dedup();
        assert_eq!(sorted, letters, "one group per letter");
        for group in &groups {
            let letter = text(&group.group());
            for item in group.group_items().iter() {
                assert!(text(&item).starts_with(&letter), "{} in {letter}", text(&item));
            }
        }
        assert_eq!(groups.iter().map(|group| group.group_items().count()).sum::<usize>(), super::FRUITS.len());
    }
}
