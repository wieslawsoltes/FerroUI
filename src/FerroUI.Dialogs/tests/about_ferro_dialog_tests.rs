//! Not from upstream: the upstream project has no tests. The about dialog
//! populates itself from its compiled document, binds its texts to itself,
//! names the project and opens the address of the project when its button is
//! clicked.

use crate::AboutFerroDialog;
use ferroui_base::input::LocalBoxFuture;
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::platform::storage::{ILauncher, IStorageItem};
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::Uri;
use ferroui_base::{Ref, StyledElement};
use ferroui_controls::platform::IWindowImpl;
use ferroui_controls::testing::{MockWindowingPlatform, TestServices, UnitTestApplication};
use ferroui_controls::{Button, TextBlock};
use ferroui_markup_xaml::FerroXamlLoader;
use std::cell::RefCell;
use std::rc::Rc;

/// The elements of the logical tree under `root`, depth first.
fn logical_descendants(root: &StyledElement) -> Vec<Ref<StyledElement>> {
    let mut found = Vec::new();
    for child in root.logical_children().to_vec() {
        found.push(child.clone());
        found.extend(logical_descendants(&child));
    }
    found
}

fn button_of(dialog: &Ref<AboutFerroDialog>) -> Ref<Button> {
    logical_descendants(dialog).iter().find_map(|element| element.cast::<Button>()).expect("the button of the dialog")
}

#[test]
fn the_dialog_loads_its_document_and_binds_its_texts() {
    crate::register_types();
    let _app = UnitTestApplication::start(TestServices::styled_window());

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

    let button = button_of(&dialog);
    assert_eq!(Some("Learn more about FerroUI".to_string()), button.content().and_then(|c| c.downcast_ref::<String>().cloned()));
}

/// A launcher that records the addresses it is asked to open.
#[derive(Default)]
struct RecordingLauncher {
    uris: RefCell<Vec<Uri>>,
}

impl ILauncher for RecordingLauncher {
    fn launch_uri_async(&self, uri: &Uri) -> LocalBoxFuture<bool> {
        self.uris.borrow_mut().push(uri.clone());
        Box::pin(std::future::ready(true))
    }

    fn launch_file_async(&self, _storage_item: Rc<dyn IStorageItem>) -> LocalBoxFuture<bool> {
        Box::pin(std::future::ready(false))
    }
}

/// `Click="Button_OnClick"` of the document: the method of the dialog runs when the
/// button is clicked, and asks the launcher of the platform to open the address of the
/// project.
#[test]
fn clicking_the_button_opens_the_address_of_the_project() {
    crate::register_types();
    let launcher = Rc::new(RecordingLauncher::default());
    let platform = MockWindowingPlatform::with_window_impl({
        let launcher = launcher.clone();
        move || {
            let window = MockWindowingPlatform::create_window_mock();
            window.setup_feature::<dyn ILauncher>(launcher.clone());
            window as Rc<dyn IWindowImpl>
        }
    });
    let _app = UnitTestApplication::start(TestServices::styled_window().with_windowing_platform(platform));

    let dialog = AboutFerroDialog::new();
    let button = button_of(&dialog);
    assert!(launcher.uris.borrow().is_empty());

    button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    Dispatcher::ui_thread().run_jobs(None);

    let expected = Uri::absolute("https://github.com/wieslawsoltes/FerroUI").expect("an absolute URI");
    assert_eq!(vec![expected], *launcher.uris.borrow());
}

/// The loader table of the crate answers a load of the document of the dialog by its
/// URI with a dialog (`!XamlLoader.TryLoad` of upstream's compiler: `new AboutFerroDialog()`).
#[test]
fn loading_the_document_by_its_uri_creates_the_dialog() {
    crate::register_types();
    let _app = UnitTestApplication::start(TestServices::styled_window());

    let uri = Uri::absolute(AboutFerroDialog::DOCUMENT_URI).expect("the URI of the document");
    let loaded = FerroXamlLoader::load(&uri, None).expect("the document is loaded");

    let dialog = from_markup_value::<Ref<AboutFerroDialog>>(&Some(loaded));
    let dialog = dialog.expect("the loader creates the dialog");
    assert_eq!(Some("About FerroUI".to_string()), dialog.title());

    // The document is compiled: it is not an asset of the assembly, its font is.
    let assets = crate::compiled_markup::ASSETS.iter().map(|(path, _)| *path).collect::<Vec<_>>();
    assert_eq!(assets, ["/Assets/Roboto-Light.ttf"]);
}

#[test]
fn the_version_has_the_major_and_minor_components() {
    let version = AboutFerroDialog::version();
    let expected = format!("v{}.{}", env!("CARGO_PKG_VERSION_MAJOR"), env!("CARGO_PKG_VERSION_MINOR"));

    assert_eq!(expected, version);
    assert!(!AboutFerroDialog::is_development_build());
    assert!(AboutFerroDialog::copyright().ends_with(" The FerroUI Project"));
}
