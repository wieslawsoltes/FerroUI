//! Minimal reproductions of the gaps of the framework that keep documents
//! of the sample from loading: one test per gap, each the smallest markup
//! (or code) that shows it. A test is ignored with the gap it reproduces
//! and passes once the gap is closed
//! (`cargo test -p control-catalog -- --ignored gap_`).
//!
//! The gaps found by the page groups are in `gaps_a.rs`, `gaps_b.rs` and
//! `gaps_c.rs`.

use super::support::*;
use crate::markup::{describe, try_load_text};
use crate::App;
use ferroui_base::{instantiate, BoxedValue};
use std::rc::Rc;

const XMLNS: &str = "xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'";

#[test]
fn gap_c001_static_resource_not_found_at_load_time_is_delayed() {
    let _app = start_application();
    // Upstream: a static resource that is not found for a registered property of a control is
    // looked up again when the control is attached to a tree (the unset value is assigned at load
    // time), so a page that names a resource of the application loads on its own.
    load_text(&format!("<Border {XMLNS}><Border Tag='{{StaticResource Missing}}' /></Border>"));
    load_text(&format!("<ContentPage {XMLNS} Theme='{{StaticResource ScrollPage}}' />"));
}

#[test]
#[ignore = "gap C002: NativeMenuItem.Click cannot be assigned a handler from markup"]
fn gap_c002_native_menu_item_click_handler() {
    let _app = start_application();
    let root: BoxedValue = Rc::new(instantiate(App::construct()));
    let xaml = format!(
        "<Application {XMLNS} x:Class='ControlCatalog.App'><NativeDock.Menu><NativeMenu><NativeMenuItem Header='Add' Click='OnDockAddItemClicked' /></NativeMenu></NativeDock.Menu></Application>"
    );
    if let Err(error) = try_load_text(&xaml, None, Some(root)) {
        panic!("{}", describe(&error));
    }
}

#[test]
#[ignore = "gap C003: NativeMenuItem.Icon cannot be assigned from text"]
fn gap_c003_native_menu_item_icon_from_text() {
    let _app = start_catalog_application();
    let xaml = format!("<NativeMenu {XMLNS}><NativeMenuItem Icon='/Assets/github_icon.png' Header='Recent' /></NativeMenu>");
    if let Err(error) = try_load_text(&xaml, Some("/MainWindow.xaml"), None) {
        panic!("{}", describe(&error));
    }
}

#[test]
#[ignore = "gap C004: the item type of a view-model list is not known to a compiled binding of an item template"]
fn gap_c004_item_type_of_a_view_model_list_for_compiled_bindings() {
    let _app = start_catalog_application();
    // Upstream: the data type of an item template is inferred from the element type of the
    // collection its items control is bound to (`IEnumerable<StandardCursorModel>`). The lists of
    // the view models are declared to markup as items sources, which carry no element type.
    let xaml = format!(
        "<ListBox {XMLNS} xmlns:vm='using:ControlCatalog.ViewModels' x:DataType='vm:CursorPageViewModel' ItemsSource='{{Binding StandardCursors}}'><ListBox.ItemTemplate><DataTemplate><TextBlock Text='{{Binding Type}}' /></DataTemplate></ListBox.ItemTemplate></ListBox>"
    );
    if let Err(error) = try_load_text(&xaml, None, None) {
        panic!("{}", describe(&error));
    }
}

#[test]
fn gap_c005_theme_variant_key_in_a_compiled_binding() {
    let _app = start_catalog_application();
    let xaml = format!("<TextBlock {XMLNS} x:DataType='ThemeVariant' Text='{{Binding Key}}' />");
    if let Err(error) = try_load_text(&xaml, None, None) {
        panic!("{}", describe(&error));
    }
}

/// The table of the elements removed from `App.xaml` matches the document,
/// and the subset loads into an application (TEMPORARY, see `temporary.rs`).
#[test]
fn the_subset_of_the_application_document_loads() {
    let _app = start_application();
    let root: BoxedValue = Rc::new(instantiate(App::construct()));
    if let Err(error) = crate::temporary::load_app_document_subset(root) {
        panic!("{}", describe(&error));
    }
}
