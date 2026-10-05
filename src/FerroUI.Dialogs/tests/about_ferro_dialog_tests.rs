//! Not from upstream: the upstream project has no tests. The about dialog
//! loads its document, binds its texts to itself and names the project.

use crate::AboutFerroDialog;
use ferroui_base::{Ref, StyledElement};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Button, TextBlock};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;

/// The elements of the logical tree under `root`, depth first.
fn logical_descendants(root: &StyledElement) -> Vec<Ref<StyledElement>> {
    let mut found = Vec::new();
    for child in root.logical_children().to_vec() {
        found.push(child.clone());
        found.extend(logical_descendants(&child));
    }
    found
}

#[test]
fn the_dialog_loads_its_document_and_binds_its_texts() {
    crate::register_types();
    let _app = UnitTestApplication::start(TestServices::styled_window());
    FerroRuntimeXamlLoader::register();

    let dialog = AboutFerroDialog::new();

    assert_eq!(Some("About FerroUI".to_string()), dialog.title());
    assert_eq!(430.0, dialog.min_width());
    assert_eq!(475.0, dialog.max_height());
    assert!(dialog.data_context().is_some());

    let elements = logical_descendants(&dialog);
    let texts: Vec<String> =
        elements.iter().filter_map(|element| element.cast::<TextBlock>()).map(|text| text.text().unwrap_or_default()).collect();
    assert!(texts.contains(&"FerroUI".to_string()), "{texts:?}");
    assert!(texts.contains(&AboutFerroDialog::version()), "{texts:?}");
    assert!(texts.contains(&AboutFerroDialog::copyright()), "{texts:?}");

    let button = elements.iter().find_map(|element| element.cast::<Button>()).expect("the button of the dialog");
    assert_eq!(Some("Learn more about FerroUI".to_string()), button.content().and_then(|c| c.downcast_ref::<String>().cloned()));
}

#[test]
fn the_version_has_the_major_and_minor_components() {
    let version = AboutFerroDialog::version();
    let expected = format!("v{}.{}", env!("CARGO_PKG_VERSION_MAJOR"), env!("CARGO_PKG_VERSION_MINOR"));

    assert_eq!(expected, version);
    assert!(!AboutFerroDialog::is_development_build());
    assert!(AboutFerroDialog::copyright().ends_with(" The FerroUI Project"));
}
