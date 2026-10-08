//! Minimal reproductions of gaps of the framework found while porting the
//! navigation page samples (see `gaps.rs`).

use super::support::*;
use ferroui_base::media::ImageBrush;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::{Border, ContentPage, Control, Window};
use ferroui_markup_xaml::markup_extensions::DynamicResourceExtension;
use std::rc::Rc;

const XMLNS: &str = "xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'";

#[test]
fn gap_c400_image_brush_source_from_text() {
    let _app = start_catalog_application();
    let border = from_markup_value::<Ref<Border>>(&Some(load_text(&format!(
        "<Border {XMLNS}>\
           <Border.Background>\
             <ImageBrush Source='ferres://ControlCatalog/Assets/CurvedHeader/featured.jpg' Stretch='UniformToFill'/>\
           </Border.Background>\
         </Border>"
    ))))
    .expect("a border");
    let brush = border
        .background()
        .and_then(|brush| brush.as_object().and_then(|object| object.to_ref().cast::<ImageBrush>()))
        .expect("an image brush");
    assert!(brush.source().is_some());
}

// --- C314: what keeps a page alive after it left its host ---
//
// A dynamic resource holds the host it looks the resource up from: the element it is the value
// of, or the element it is declared under. The element owns the dynamic resource (through its
// values, or through the object the resource is a value of), so the two keep each other alive.
// The control theme of a page sets its background and its foreground from dynamic resources.

fn run_jobs() {
    Dispatcher::ui_thread().run_jobs(None);
}

/// Shows the control as the content of a window, takes it out again and returns the window.
fn show_and_remove(control: Ref<Control>) -> Ref<Window> {
    let window = Window::new();
    window.set_content(Some(Control::boxed(control)));
    window.show();
    run_jobs();
    window.set_content(None);
    run_jobs();
    window
}

#[test]
fn gap_c314_dynamic_resource_bound_to_an_element() {
    let _app = start_catalog_application();
    let border = Border::new();
    let key: BoxedValue = Rc::new(String::from("ThemeBackgroundBrush"));
    let resource = DynamicResourceExtension::with_resource_key(Some(key));
    border.bind_binding(Border::background_property().as_property(), &*resource.as_binding());
    drop(resource);

    let weak = border.downgrade();
    drop(border);
    assert!(weak.upgrade().is_none());
}

#[test]
fn gap_c314_dynamic_resource_of_an_object_under_an_element() {
    let _app = start_catalog_application();
    let border = from_markup_value::<Ref<Border>>(&Some(load_text(&format!(
        "<Border {XMLNS}>\
           <Border.Background>\
             <SolidColorBrush Color='{{DynamicResource ThemeAccentColor}}'/>\
           </Border.Background>\
         </Border>"
    ))))
    .expect("a border");

    let weak = border.downgrade();
    drop(border);
    assert!(weak.upgrade().is_none());
}

#[test]
fn gap_c314_page_with_a_control_theme_that_left_the_tree() {
    let _app = start_catalog_application();
    let page = ContentPage::new();
    let window = show_and_remove(page.clone().upcast());

    let weak = page.downgrade();
    drop(page);
    assert!(weak.upgrade().is_none());
    window.close();
}

/// The counterpart of the reproductions above: a control without a control theme, and so
/// without a dynamic resource, is freed after it was shown. Not a gap.
#[test]
fn control_without_a_dynamic_resource_that_left_the_tree_is_freed() {
    let _app = start_catalog_application();
    let border = Border::new();
    let window = show_and_remove(border.clone().upcast());

    let weak = border.downgrade();
    drop(border);
    assert!(weak.upgrade().is_none());
    window.close();
}
