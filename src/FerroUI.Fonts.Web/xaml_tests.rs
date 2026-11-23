//! Faces declared in XAML through the run-time loader, and from code.

use crate::{register_types, WebFontCollection, WebFonts};
use ferroui_base::media::fonts::IFontCollection;
use ferroui_base::platform::IFontManagerImpl;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{Dispatcher, UnitTestDispatcherScope};
use ferroui_base::{FerroLocator, Ref};
use ferroui_controls::{Border, Control};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::rc::Rc;

struct Scope {
    locator: Option<Rc<dyn IDisposable>>,
    _dispatcher: UnitTestDispatcherScope,
}

fn scope() -> Scope {
    register_types();
    let dispatcher = Dispatcher::unit_test_scope();
    Control::reset_loaded_queue_for_unit_tests();
    let locator = FerroLocator::enter_scope();
    FerroRuntimeXamlLoader::register();
    let backend: Rc<dyn IFontManagerImpl> = Rc::new(ferroui_vello::FontManagerImpl::new());
    FerroLocator::current_mutable().bind::<dyn IFontManagerImpl>().to_constant(backend);
    Scope { locator: Some(locator), _dispatcher: dispatcher }
}

impl Drop for Scope {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            Dispatcher::reset_for_unit_tests();
        }
        if let Some(locator) = self.locator.take() {
            locator.dispose();
        }
        Control::reset_loaded_queue_for_unit_tests();
    }
}

const DOCUMENT: &str = r#"<Border xmlns="https://github.com/ferroui" xmlns:fonts="using:FerroUI.Fonts.Web">
  <fonts:WebFonts.StyleSheet>
    @font-face {
      font-family: Inter;
      src: url(https://example.test/Inter.woff2) format(woff2);
      font-weight: 100 900;
      font-display: swap;
    }
    @font-face { font-family: Second; src: url(https://example.test/Second.woff2) }
    @font-face { font-family: NoSource }
  </fonts:WebFonts.StyleSheet>
</Border>"#;

#[test]
fn faces_are_declared_in_xaml() {
    let _scope = scope();
    let root = FerroRuntimeXamlLoader::parse_as::<Ref<Border>>(DOCUMENT, None).expect("the document loads");
    assert!(WebFonts::get_style_sheet(&root).contains("@font-face"));
    let collection = WebFontCollection::current();
    let names: Vec<String> = collection.font_families().iter().map(|family| family.name().to_owned()).collect();
    assert_eq!(names, ["Inter", "Second"], "a rule that is not valid is dropped");
}

#[test]
fn an_attribute_works_too() {
    let _scope = scope();
    let document = r#"<Border xmlns="https://github.com/ferroui" xmlns:fonts="using:FerroUI.Fonts.Web"
        fonts:WebFonts.StyleSheet="@font-face { font-family: A; src: url(https://example.test/a.woff2) }" />"#;
    FerroRuntimeXamlLoader::parse_as::<Ref<Border>>(document, None).expect("the document loads");
    assert_eq!(WebFontCollection::current().count(), 1);
}

#[test]
fn a_face_is_declared_from_code() {
    let _scope = scope();
    let collection = WebFontCollection::current();
    let id = collection.declare("Inter").source("url(a.woff2) format(woff2)").weight("100 900").display("swap").add();
    assert!(id.is_ok());
    assert_eq!(collection.count(), 1);
    assert!(collection.declare("Inter").source("url(a)").weight("5000").add().is_err());
    assert!(collection.declare("Inter").add().is_err(), "a face needs a source");
    assert!(collection.declare(" ").source("url(a)").add().is_err(), "and a family");
    assert_eq!(collection.count(), 1);
}
