//! The Color page on the real WinUI theme.

use crate::color_sections::COLOR_SECTIONS;
use crate::{examples, tests::start_winui};
use ferroui_controls::primitives::TabStrip;
use ferroui_controls::{Border, Control, StackPanel, Window, WrapPanel};

#[test]
fn each_section_shows_its_groups_brushes_and_usage_notes() {
    let _application = start_winui();
    let root = examples::create(examples::find("Color").unwrap()).unwrap();
    assert_eq!(COLOR_SECTIONS.len(), 6);
    // The brushes come from the theme of the window the page is in.
    let window = Window::new();
    window.set_content(Some(Control::boxed(&root)));
    window.show();
    let strip = root.get_control::<TabStrip>("ColorSections");
    for (index, section) in COLOR_SECTIONS.iter().enumerate() {
        strip.set_selected_index(index as i32);
        let host = root.get_control::<StackPanel>("ColorGroups");
        assert_eq!(host.children().count(), section.groups.len() * 3, "{}", section.title);
        for (position, group) in section.groups.iter().enumerate() {
            let tiles = host.children().get(position * 3 + 2).cast::<WrapPanel>().unwrap();
            assert_eq!(tiles.children().count(), group.brushes.len(), "{}", group.name);
            for (place, info) in group.brushes.iter().enumerate() {
                let tile = tiles.children().get(place).cast::<StackPanel>().unwrap();
                let swatch = tile.children().get(0).cast::<Border>().unwrap();
                assert_eq!(swatch.background().is_some(), info.key.is_some(), "{}", info.name);
            }
        }
    }
}

#[test]
fn the_table_has_every_brush_of_the_source_once() {
    let mut names: Vec<&str> = COLOR_SECTIONS
        .iter()
        .flat_map(|section| section.groups.iter())
        .flat_map(|group| group.brushes.iter().map(|info| info.name))
        .collect();
    assert_eq!(names.len(), 106);
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), 106);
}

#[test]
fn only_the_two_mica_brushes_are_not_in_the_theme() {
    let missing: Vec<&str> = COLOR_SECTIONS
        .iter()
        .flat_map(|section| section.groups.iter())
        .flat_map(|group| group.brushes.iter())
        .filter(|info| info.key.is_none())
        .map(|info| info.name)
        .collect();
    assert_eq!(missing, ["MicaBackgroundFillColorBaseBrush", "MicaBackgroundFillColorBaseAltBrush"]);
}
