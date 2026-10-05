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
use ferroui_base::media::{IBrush, VisualBrush};
use ferroui_base::platform::IBitmapImpl;
use ferroui_base::{instantiate, BoxedValue, FerroLocator};
use ferroui_controls::platform::{IPlatformIconLoader, IWindowIconImpl};
use ferroui_controls::{Control, Panel};
use std::io;
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
fn gap_c003_native_menu_item_icon_from_text() {
    let _app = start_catalog_application();
    let xaml = format!("<NativeMenu {XMLNS}><NativeMenuItem Icon='/Assets/github_icon.png' Header='Recent' /></NativeMenu>");
    if let Err(error) = try_load_text(&xaml, Some("/MainWindow.xaml"), None) {
        panic!("{}", describe(&error));
    }
}

#[test]
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

#[test]
fn gap_c009_calendar_week_rule_from_text() {
    let _app = start_catalog_application();
    // As `Pages/CalendarPage.xaml`: the week-number rule with the week numbers shown.
    let xaml = format!("<Calendar {XMLNS} WeekNumberRule='FirstFourDayWeek' IsWeekNumberVisible='True' />");
    if let Err(error) = try_load_text(&xaml, None, None) {
        panic!("{}", describe(&error));
    }
}

#[test]
fn gap_c010_table_view_columns_from_markup() {
    let _app = start_catalog_application();
    let xaml = format!(
        "<TableView {XMLNS}><TableView.Columns><TableViewColumn Header='Country' Width='2*' /></TableView.Columns></TableView>"
    );
    if let Err(error) = try_load_text(&xaml, None, None) {
        panic!("{}", describe(&error));
    }
}

#[test]
fn gap_c006_visual_brush_is_a_composition_render_resource() {
    let _app = start_application();
    // The compositor converts an opacity mask that is not a render resource with
    // `BrushExtensions::to_immutable`, which panics for a visual brush (fixed: a
    // visual brush is a composition render resource).
    let brush: Rc<dyn IBrush> = VisualBrush::new().into();
    assert!(brush.as_composition_render_resource().is_some() || brush.as_mutable_brush().is_some());
}

#[test]
fn gap_c011_multi_page_pages_from_markup() {
    let _app = start_catalog_application();
    for xaml in [
        format!("<TabbedPage {XMLNS}><ContentPage Header='One' /><ContentPage Header='Two' /></TabbedPage>"),
        format!("<CarouselPage {XMLNS}><ContentPage Header='One' /></CarouselPage>"),
    ] {
        if let Err(error) = try_load_text(&xaml, None, None) {
            panic!("{}", describe(&error));
        }
    }
}

/// The table of the elements removed from `App.xaml` matches the document,
/// and the subset loads into an application (TEMPORARY, see `temporary.rs`).
#[test]
fn the_subset_of_the_application_document_loads() {
    // The icon of a native menu item of the document is decoded as a bitmap.
    let _app = start_catalog_application();
    // The tray icon of the document loads its icon through the icon loader of the platform.
    let loader: Rc<dyn IPlatformIconLoader> = Rc::new(TestIconLoader);
    FerroLocator::current_mutable().bind::<dyn IPlatformIconLoader>().to_constant(loader);
    let root: BoxedValue = Rc::new(instantiate(App::construct()));
    if let Err(error) = crate::temporary::load_app_document_subset(root) {
        panic!("{}", describe(&error));
    }
}

/// The table of the elements removed from `MainWindow.xaml` matches the
/// document, and the subset loads into a main window whose content is the
/// real main view (TEMPORARY, see `temporary.rs`).
#[test]
fn the_subset_of_the_main_window_document_loads() {
    let _app = start_catalog_application();
    let window = crate::MainWindow::new();
    let panel = window.content().and_then(|content| Control::from_boxed(&content)).expect("the root panel");
    assert_eq!(1, panel.cast::<Panel>().expect("a panel").children().count());
}

struct TestIconImpl(Vec<u8>);

impl IWindowIconImpl for TestIconImpl {
    fn save(&self, output_stream: &mut dyn io::Write) -> io::Result<()> {
        output_stream.write_all(&self.0)
    }
}

/// An icon loader that keeps the bytes of the icon.
struct TestIconLoader;

impl IPlatformIconLoader for TestIconLoader {
    fn load_icon_from_file(&self, file_name: &str) -> io::Result<Rc<dyn IWindowIconImpl>> {
        Ok(Rc::new(TestIconImpl(file_name.as_bytes().to_vec())))
    }

    fn load_icon_from_stream(&self, stream: &mut dyn io::Read) -> io::Result<Rc<dyn IWindowIconImpl>> {
        let mut data = Vec::new();
        stream.read_to_end(&mut data)?;
        Ok(Rc::new(TestIconImpl(data)))
    }

    fn load_icon_from_bitmap(&self, _bitmap: Rc<dyn IBitmapImpl>) -> Rc<dyn IWindowIconImpl> {
        Rc::new(TestIconImpl(Vec::new()))
    }
}
