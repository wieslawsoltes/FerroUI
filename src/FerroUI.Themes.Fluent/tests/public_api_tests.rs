//! Port of `tests/Themes.UnitTests/PublicApiTests.cs`.
//!
//! The managed test asserts that no populate or build method of the
//! compiled documents is public: the documents of the theme are internal
//! (`x:ClassModifier="internal"`), only the entry point is reachable. Here
//! the reachable documents are the ones the document loader table of the
//! assembly answers for.

use super::support::*;
use crate::{assets, FluentTheme, ASSEMBLY};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::utilities::Uri;
use ferroui_base::Ref;
use ferroui_markup_xaml::FerroXamlLoader;

#[test]
fn should_not_include_any_reachable_xaml_files() {
    assert!(!assets::documents().is_empty());
    for (path, content) in assets::documents() {
        let text = std::str::from_utf8(content).expect("UTF-8");
        if *path == "/FluentTheme.xaml" {
            assert!(text.contains("x:Class=\"FerroUI.Themes.Fluent.FluentTheme\""));
        } else {
            assert!(text.contains("x:ClassModifier=\"internal\""), "{path} is not internal");
            assert!(!text.contains("x:Class="), "{path} has a class");
        }
    }
}

#[test]
fn only_the_entry_point_is_built_by_the_document_table() {
    // Without the run-time loader as a fallback, only the document with a class loads by URI.
    crate::register_types();
    let _app = ferroui_controls::testing::UnitTestApplication::start(
        ferroui_controls::testing::TestServices::mock_platform_render_interface(),
    );
    for (path, _) in assets::documents() {
        let uri = Uri::absolute(&format!("ferres://{}{path}", ASSEMBLY.name)).expect("a valid URI");
        let loaded = FerroXamlLoader::load(&uri, None);
        if *path == "/FluentTheme.xaml" {
            let theme = loaded.unwrap_or_else(|error| panic!("{}", describe(&error)));
            assert!(from_markup_value::<Ref<FluentTheme>>(&Some(theme)).is_some());
        } else {
            let error = loaded.err().unwrap_or_else(|| panic!("{path} is reachable"));
            assert!(error.message().starts_with("No precompiled XAML found for"), "{}", error.message());
        }
    }
}
