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

// --- C317 to C320: what a visit to a page of the catalog retained ---
//
// Found with the tours of `catalog_tour.rs`: every visit to a page left the page, or most of
// what it showed, alive. Each reproduction is one of the cycles: the managed original has the
// same references and its collector frees them.

/// Loads a control and returns what remains of it once the caller lets go of it.
fn load_and_release(xaml: &str) -> ferroui_base::WeakRef<Control> {
    let control = from_markup_value::<Ref<Control>>(&Some(load_text(xaml))).expect("a control");
    let weak = control.downgrade();
    drop(control);
    weak
}

/// C317: the deferred content of a template holds the root of its document and the resource
/// nodes above its declaration, which own the template.
#[test]
fn gap_c317_element_with_a_template_declared_under_it() {
    let _app = start_catalog_application();
    let weak = load_and_release(&format!(
        "<ItemsControl {XMLNS}>\
           <ItemsControl.ItemTemplate>\
             <DataTemplate><TextBlock/></DataTemplate>\
           </ItemsControl.ItemTemplate>\
         </ItemsControl>"
    ));
    assert!(weak.upgrade().is_none());
}

/// C317: the deferred content of a template holds the name scope of its document, which holds
/// the named elements: here the element that owns the template.
#[test]
fn gap_c317_named_element_with_a_template_declared_under_it() {
    let _app = start_catalog_application();
    let panel = from_markup_value::<Ref<ferroui_controls::StackPanel>>(&Some(load_text(&format!(
        "<StackPanel {XMLNS}>\
           <ItemsControl Name='list'>\
             <ItemsControl.ItemTemplate>\
               <DataTemplate><TextBlock/></DataTemplate>\
             </ItemsControl.ItemTemplate>\
           </ItemsControl>\
         </StackPanel>"
    ))))
    .expect("a panel");
    let list = panel.children().get(0).downgrade();
    let weak = panel.downgrade();
    drop(panel);
    assert!(weak.upgrade().is_none());
    assert!(list.upgrade().is_none());
}

/// C317: what the deferred content holds weakly is there when the template is built: the
/// resources of the element above the declaration, and the elements its document names.
#[test]
fn gap_c317_template_finds_the_resources_and_the_names_of_its_document() {
    let _app = start_catalog_application();
    let panel = from_markup_value::<Ref<ferroui_controls::StackPanel>>(&Some(load_text(&format!(
        "<StackPanel {XMLNS} Name='root' Tag='named'>\
           <StackPanel.Resources>\
             <SolidColorBrush x:Key='Declared' Color='Red'/>\
           </StackPanel.Resources>\
           <ContentControl Content='content'>\
             <ContentControl.ContentTemplate>\
               <DataTemplate>\
                 <Border Background='{{StaticResource Declared}}' Tag='{{ReflectionBinding #root.Tag}}'/>\
               </DataTemplate>\
             </ContentControl.ContentTemplate>\
           </ContentControl>\
         </StackPanel>"
    ))))
    .expect("a panel");
    let window = Window::new();
    window.set_content(Some(Control::boxed(&panel)));
    window.show();
    run_jobs();

    let border = panel
        .get_visual_descendants()
        .find_map(|visual| visual.cast::<Border>())
        .expect("the border of the template");
    assert!(border.background().is_some());
    let tag = border.tag().and_then(|tag| tag.downcast_ref::<String>().cloned());
    assert_eq!(Some(String::from("named")), tag);
    window.close();
}

/// C318: the node of a binding of `DataContext` holds the parent of its element, which owns the
/// element.
#[test]
fn gap_c318_element_whose_data_context_is_bound() {
    let _app = start_catalog_application();
    let panel = from_markup_value::<Ref<ferroui_controls::StackPanel>>(&Some(load_text(&format!(
        "<StackPanel {XMLNS}>\
           <Border DataContext='{{ReflectionBinding}}'/>\
         </StackPanel>"
    ))))
    .expect("a panel");
    let window = show_and_remove(panel.clone().upcast());
    let child = panel.children().get(0).downgrade();
    let weak = panel.downgrade();
    drop(panel);
    assert!(weak.upgrade().is_none());
    assert!(child.upgrade().is_none());
    window.close();
}

/// C319: the node of a binding that locates an element (by name, as an ancestor) holds the
/// element it found, whose tree owns the target of the binding.
#[test]
fn gap_c319_element_bound_to_an_element_above_it() {
    let _app = start_catalog_application();
    for path in ["#owner.Tag", "$parent[Border].Tag", "$parent.Tag"] {
        let panel = from_markup_value::<Ref<ferroui_controls::StackPanel>>(&Some(load_text(&format!(
            "<StackPanel {XMLNS}>\
               <Border Name='owner' Tag='above'>\
                 <TextBlock Tag='{{ReflectionBinding {path}}}'/>\
               </Border>\
             </StackPanel>"
        ))))
        .expect("a panel");
        let window = Window::new();
        window.set_content(Some(Control::boxed(&panel)));
        window.show();
        run_jobs();
        let owner = panel.children().get(0).cast::<Border>().expect("the border");
        let bound = owner.child().expect("the text block");
        let tag = bound.tag().and_then(|tag| tag.downcast_ref::<String>().cloned());
        assert_eq!(Some(String::from("above")), tag, "{path}");
        window.set_content(None);
        run_jobs();

        let (owner, bound, weak) = {
            let weak = (owner.downgrade(), bound.downgrade(), panel.downgrade());
            drop((owner, bound, panel));
            weak
        };
        assert!(weak.upgrade().is_none(), "{path}");
        assert!(owner.upgrade().is_none(), "{path}");
        assert!(bound.upgrade().is_none(), "{path}");
        window.close();
    }
}

/// Open: the name scope of a document holds the elements it names, and the root of the
/// document holds the name scope, so a root that has a name keeps itself alive. The managed
/// original has both references. Reported by the tours as what remains.
#[test]
#[ignore = "open: a named root and its name scope hold each other"]
fn open_named_root_of_a_document() {
    let _app = start_catalog_application();
    let weak = load_and_release(&format!("<StackPanel {XMLNS} Name='root'/>"));
    assert!(weak.upgrade().is_none());
}

/// C320: a binding entry of a value store and the observable it is subscribed to hold each
/// other. The presenter of a scroll viewer binds its content to the content of the viewer.
#[test]
fn gap_c320_content_of_a_scroll_viewer_that_was_dropped_while_shown() {
    let _app = start_catalog_application();
    let viewer = from_markup_value::<Ref<ferroui_controls::ScrollViewer>>(&Some(load_text(&format!(
        "<ScrollViewer {XMLNS}><Border/></ScrollViewer>"
    ))))
    .expect("a scroll viewer");
    let content = from_markup_value::<Ref<Control>>(&viewer.content()).expect("the content").downgrade();
    let window = Window::new();
    window.set_content(Some(Control::boxed(&viewer)));
    window.show();
    run_jobs();

    // The window is closed with the viewer in it: nothing detaches the parts of the viewer
    // one by one, they go with the tree.
    let weak = viewer.downgrade();
    drop(viewer);
    window.close();
    run_jobs();
    drop(window);
    assert!(weak.upgrade().is_none());
    assert!(content.upgrade().is_none());
}
