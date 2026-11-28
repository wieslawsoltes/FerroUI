//! Independent reusable-library JSON consumer used by the unchanged-C# cross-check.
use ferroribbon_core::collections::RibbonItems;
use ferroribbon_core::{RibbonDisplayMode, state::*};
use std::io::{self, BufRead};
fn main() {
    if std::env::args().any(|a| a == "--emit") {
        let state = RibbonState::new();
        state.set_selected_tab_id(Some("insert & é😀".into()));
        state.set_display_mode(RibbonDisplayMode::Simplified);
        state.set_recent_search_ids(Some(RibbonItems::from_values(vec![
            Some("bold".into()),
            Some("paste".into()),
        ])));
        let group = RibbonCustomGroup::new();
        group.set_id(Some("custom.tools".into()));
        group.set_label(Some("Tools".into()));
        group.item_ids().unwrap().add(Some("bold".into()));
        state
            .customization()
            .unwrap()
            .custom_groups()
            .unwrap()
            .add(Some(group));
        println!(
            "{}",
            RibbonStateSerializer::serialize(Some(&state), false)
                .unwrap()
                .to_utf8()
                .unwrap()
        );
        return;
    }
    for line in io::stdin().lock().lines() {
        let line = line.unwrap();
        let text = line.into();
        let state = RibbonStateSerializer::deserialize(Some(&text)).unwrap();
        let json = state
            .map(|s| {
                RibbonStateSerializer::serialize(Some(&s), false)
                    .unwrap()
                    .to_utf8()
                    .unwrap()
            })
            .unwrap_or_else(|| "null".into());
        println!("{json}");
    }
}
