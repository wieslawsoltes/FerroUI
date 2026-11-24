//! The SwipeControl page: the source's five examples with their handlers
//! (`swipe_control_samples.rs`).

use crate::swipe_control_samples::{item, items};
use crate::{examples, tests::start};
use ferroui_base::Ref;
use ferroui_controls::{Control, SwipeBehaviorOnInvoked, SwipeControl, SwipeMode, TextBlock};
use ferroui_controls_icons::BitmapIconSource;

fn page() -> Ref<Control> {
    examples::create(examples::find("SwipeControl").unwrap()).unwrap_or_else(|error| panic!("SwipeControl: {error}"))
}

#[test]
fn each_example_has_the_items_and_modes_of_the_source() {
    let _application = start();
    let root = page();
    let mode = |name: &str, right: bool| items(&root, name, right).map(|items| items.mode());
    assert_eq!(mode("RevealSwipe", false), Some(SwipeMode::Reveal));
    assert_eq!(mode("ExecuteSwipe", true), Some(SwipeMode::Execute));
    assert_eq!(mode("ListSwipe", false), Some(SwipeMode::Reveal));
    assert_eq!(mode("GradientSwipe", true), Some(SwipeMode::Execute));
    assert_eq!(mode("IconSwipe", false), Some(SwipeMode::Reveal));
    let texts: Vec<String> = items(&root, "RevealSwipe", false).unwrap().items().to_vec().iter().map(|item| item.text()).collect();
    assert_eq!(texts, ["Accept", "Flag"]);
    let archive = item(&root, "ExecuteSwipe", true, 0).unwrap();
    assert_eq!((archive.text(), archive.behavior_on_invoked()), ("Archive".to_owned(), SwipeBehaviorOnInvoked::Close));
    assert!(archive.icon_source().is_some(), "the ArchiveIcon the sample names is defined");
    let lock = item(&root, "GradientSwipe", true, 0).unwrap();
    assert!(lock.background().is_some_and(|brush| brush.as_linear_gradient_brush().is_some()), "the purple gradient");
}

#[test]
fn invoking_an_item_runs_its_handler() {
    let _application = start();
    let root = page();
    let control = root.get_control::<SwipeControl>("RevealSwipe");
    item(&root, "RevealSwipe", false, 1).unwrap().invoke_swipe(&control);
    assert_eq!(root.get_control::<TextBlock>("RevealOutput").text().as_deref(), Some("Flag was invoked."));
    let execute = root.get_control::<SwipeControl>("ExecuteSwipe");
    item(&root, "ExecuteSwipe", true, 0).unwrap().invoke_swipe(&execute);
    assert_eq!(root.get_control::<TextBlock>("ExecuteOutput").text().as_deref(), Some("Archive was invoked."));
}

#[test]
fn the_custom_icons_are_bitmap_icon_sources_of_an_image_of_the_sample_media() {
    let _application = start();
    let root = page();
    for name in ["ListSwipe", "IconSwipe"] {
        let source = item(&root, name, false, 0)
            .and_then(|item| item.icon_source())
            .and_then(|source| source.cast::<BitmapIconSource>())
            .unwrap_or_else(|| panic!("{name} has a bitmap icon source"));
        assert!(source.show_as_monochrome(), "{name}: monochrome, as the sample does not say otherwise");
        let uri = source.uri_source().unwrap_or_else(|| panic!("{name} has a UriSource"));
        assert!(uri.to_string().ends_with("SampleMedia/folder.png"), "{name}: {uri}");
    }
}
