//! Not from upstream: the page of WinUI's AutoSuggestBox (docs/porting/winui-controls.md), the class
//! of `Pages/AutoSuggestBoxPage.xaml`. As in WinUI, the application filters the suggestions when the
//! user types and handles the chosen suggestion and the submitted query.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::{unbox_item, AutoSuggestBox, AutoSuggestionBoxTextChangeReason, ContentPage, ItemsSource, TextBlock};

const STATES: &[&str] = &[
    "Alabama", "Alaska", "Arizona", "Arkansas", "California", "Colorado", "Connecticut", "Delaware",
    "Florida", "Georgia", "Hawaii", "Idaho", "Illinois", "Indiana", "Iowa", "Kansas", "Kentucky",
    "Louisiana", "Maine", "Maryland", "Massachusetts", "Michigan", "Minnesota", "Mississippi",
    "Missouri", "Montana", "Nebraska", "Nevada", "New Hampshire", "New Jersey", "New Mexico",
    "New York", "North Carolina", "North Dakota", "Ohio", "Oklahoma", "Oregon", "Pennsylvania",
    "Rhode Island", "South Carolina", "South Dakota", "Tennessee", "Texas", "Utah", "Vermont",
    "Virginia", "Washington", "West Virginia", "Wisconsin", "Wyoming",
];

#[repr(C)]
pub struct AutoSuggestBoxPage {
    base: ContentPage,
}

content_page_class!(AutoSuggestBoxPage);
ferro_class_info!(AutoSuggestBoxPage { new: AutoSuggestBoxPage::new });
xaml_class!(AutoSuggestBoxPage, "/Pages/AutoSuggestBoxPage.xaml");

/// The states whose names contain every word of `text`, ignoring case.
fn matching_states(text: &str) -> Vec<&'static str> {
    let words: Vec<String> = text.split_whitespace().map(str::to_lowercase).collect();
    if words.is_empty() {
        return Vec::new();
    }
    STATES
        .iter()
        .copied()
        .filter(|state| {
            let state = state.to_lowercase();
            words.iter().all(|word| state.contains(word.as_str()))
        })
        .collect()
}

impl AutoSuggestBoxPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        let states = this.find_control::<AutoSuggestBox>("States").expect("AutoSuggestBoxPage States");
        let weak = states.downgrade();
        states.text_changed(move |args| {
            let Some(states) = weak.upgrade() else { return };
            if args.reason() == AutoSuggestionBoxTextChangeReason::UserInput {
                let matches = matching_states(&states.text());
                let source = if matches.is_empty() { vec!["No results found"] } else { matches };
                states.set_items_source(Some(ItemsSource::from_strs(source)));
            }
        });
        let weak = this.downgrade();
        states.query_submitted(move |args| {
            let Some(this) = weak.upgrade() else { return };
            let chosen = unbox_item::<String>(&args.chosen_suggestion());
            let text = match chosen {
                Some(state) => format!("Chosen: {state}"),
                None => format!("Query: {}", args.query_text()),
            };
            if let Some(status) = this.find_control::<TextBlock>("Submitted") {
                status.set_text(Some(&text));
            }
        });
        this
    }
}

#[cfg(test)]
mod tests {
    use super::matching_states;

    #[test]
    fn the_suggestions_contain_every_typed_word() {
        assert_eq!(matching_states("new y"), vec!["New York"]);
        assert_eq!(matching_states("  "), Vec::<&str>::new());
        assert!(matching_states("carolina").contains(&"South Carolina"));
    }
}
