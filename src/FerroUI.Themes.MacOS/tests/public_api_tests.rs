//! The rule of `tests/Themes.UnitTests/PublicApiTests.cs` of the upstream project (as the tests
//! of the Fluent theme): the documents of the theme are internal
//! (`x:ClassModifier="internal"`), only the entry point is reachable. Here the reachable
//! documents are the ones the document loader table of the assembly answers for.

use super::support::*;
use crate::{assets, MacOSTheme, ASSEMBLY};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::utilities::Uri;
use ferroui_base::Ref;
use ferroui_markup_xaml::FerroXamlLoader;

#[test]
fn should_not_include_any_reachable_xaml_files() {
    assert!(!assets::documents().is_empty());
    for (path, content) in assets::documents() {
        let text = std::str::from_utf8(content).expect("UTF-8");
        if *path == "/MacOSTheme.xaml" {
            assert!(text.contains("x:Class=\"FerroUI.Themes.MacOS.MacOSTheme\""));
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
        if *path == "/MacOSTheme.xaml" {
            let theme = loaded.unwrap_or_else(|error| panic!("{}", describe(&error)));
            assert!(from_markup_value::<Ref<MacOSTheme>>(&Some(theme)).is_some());
        } else {
            let error = loaded.err().unwrap_or_else(|| panic!("{path} is reachable"));
            assert!(error.message().starts_with("No precompiled XAML found for"), "{}", error.message());
        }
    }
}

/// Inherited documents preserve their upstream provenance. Newly authored macOS documents
/// name the macOS theme design record instead of claiming to be original WinUI resources.
#[test]
fn every_document_names_its_source() {
    for (path, content) in assets::documents() {
        let text = std::str::from_utf8(content).expect("UTF-8");
        let start = text.trim_start_matches('\u{feff}').trim_start();
        assert!(start.starts_with("<!--"), "{path} does not start with a comment");
        let comment = &start[..start.find("-->").unwrap_or_else(|| panic!("{path}: an unterminated comment"))];
        if comment.contains("microsoft-ui-xaml") {
            assert!(comment.contains("7e4b0c2096be402f51cc8f7d203ebf67cff183d0"), "{path}: the inherited source pin is missing");
        } else {
            assert!(comment.contains("docs/porting/macos26-theme.md"), "{path}: the macOS design record is missing");
        }
    }
}
