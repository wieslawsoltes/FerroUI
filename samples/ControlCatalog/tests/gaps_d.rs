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

/// C401: a reflection binding reads `ProgressBar.TemplateSettings`, as the control theme of
/// the Fluent theme does for the sizes of its indeterminate indicators
/// (`{Binding $parent[ProgressBar].TemplateSettings.ContainerWidth}`). The class declared the
/// property for nothing but its own code, so the binding reported "Could not find a matching
/// property accessor for 'TemplateSettings' on 'ProgressBar'" and the indicators got no size.
#[test]
fn gap_c401_progress_bar_template_settings_in_a_reflection_binding() {
    let _app = start_catalog_application();
    let progress_bar = from_markup_value::<Ref<ferroui_controls::ProgressBar>>(&Some(load_text(&format!(
        "<ProgressBar {XMLNS} x:CompileBindings='False' Tag='{{Binding $self.TemplateSettings}}'/>"
    ))))
    .expect("a progress bar");
    assert!(progress_bar.tag().is_some(), "the binding to the template settings delivers nothing");
}

/// C401, the member of the settings: the path of the control themes, to the end.
#[test]
fn gap_c401_progress_bar_template_settings_member_in_a_reflection_binding() {
    let _app = start_catalog_application();
    let progress_bar = from_markup_value::<Ref<ferroui_controls::ProgressBar>>(&Some(load_text(&format!(
        "<ProgressBar {XMLNS} x:CompileBindings='False' Tag='{{Binding $self.TemplateSettings.ContainerWidth}}'/>"
    ))))
    .expect("a progress bar");
    assert_eq!(Some(0.0), from_markup_value::<f64>(&progress_bar.tag()));
    progress_bar.template_settings().set_container_width(40.0);
    assert_eq!(Some(40.0), from_markup_value::<f64>(&progress_bar.tag()));
}

/// C401 with a compiled binding: the path is typed by the data type, and each segment is a
/// declared member.
#[test]
fn gap_c401_progress_bar_template_settings_in_a_compiled_binding() {
    let _app = start_catalog_application();
    let progress_bar = from_markup_value::<Ref<ferroui_controls::ProgressBar>>(&Some(load_text(&format!(
        "<ProgressBar {XMLNS} x:CompileBindings='True' Tag='{{Binding $self.TemplateSettings.Container2Width}}'/>"
    ))))
    .expect("a progress bar");
    assert_eq!(Some(0.0), from_markup_value::<f64>(&progress_bar.tag()));
    progress_bar.template_settings().set_container2_width(60.0);
    assert_eq!(Some(60.0), from_markup_value::<f64>(&progress_bar.tag()));
}

/// C402: a reflection binding reads a component of a date, as the control theme of the calendar
/// date picker of the Fluent theme does for the day in its button
/// (`{Binding Source={x:Static sys:DateTime.Today}, Path=Day}`). The metadata of `DateTime` had
/// its static members only, and the binding reported "Could not find a matching property
/// accessor for 'Day' on 'System.DateTime'".
#[test]
fn gap_c402_date_time_component_in_a_reflection_binding() {
    use ferroui_base::utilities::DateTime;

    let _app = start_catalog_application();
    let text_block = from_markup_value::<Ref<ferroui_controls::TextBlock>>(&Some(load_text(&format!(
        "<TextBlock {XMLNS} xmlns:sys='using:System' x:CompileBindings='False' \
           Text='{{Binding Source={{x:Static sys:DateTime.Today}}, Path=Day}}'/>"
    ))))
    .expect("a text block");
    assert_eq!(Some(DateTime::today().day().to_string()), text_block.text());
}

/// C403: a binding delivers the converter of `DataValidationErrors.ErrorConverter`, as the
/// data validation page binds the converters of its view model. The property holds the
/// nullable form of the delegate, which no conversion produced, and the binding reported
/// "Could not convert 'ErrorConverter' to 'Option<ErrorConverter>'".
#[test]
fn gap_c403_error_converter_from_a_binding() {
    use crate::view_models::DataValidationViewModel;
    use ferroui_controls::{DataValidationErrors, TextBox};

    let _app = start_catalog_application();
    let text_box = from_markup_value::<Ref<TextBox>>(&Some(load_text(&format!(
        "<TextBox {XMLNS} x:CompileBindings='False' DataValidationErrors.ErrorConverter='{{Binding Converter}}'/>"
    ))))
    .expect("a text box");
    let view_model = DataValidationViewModel::new();
    text_box.set_data_context(Some(view_model.clone() as BoxedValue));
    assert_eq!(Some(view_model.converter()), DataValidationErrors::get_error_converter(&text_box));
}

/// C404: a compiled binding whose path is one property of type `object`, as the expander page
/// binds the corner radius of its expanders to a property that holds a corner radius or the
/// unset value. The typed element of such a path delivered the box of the value as the value,
/// and the binding reported "Could not convert 'Rc<dyn AnyValue>' to 'CornerRadius'".
#[test]
fn gap_c404_compiled_binding_to_a_property_of_type_object() {
    use crate::view_models::ExpanderPageViewModel;
    use ferroui_base::CornerRadius;

    let _app = start_catalog_application();
    let border = from_markup_value::<Ref<Border>>(&Some(load_text(&format!(
        "<Border {XMLNS} xmlns:viewModels='using:ControlCatalog.ViewModels' x:CompileBindings='True' \
           x:DataType='viewModels:ExpanderPageViewModel' CornerRadius='{{Binding CornerRadius}}'/>"
    ))))
    .expect("a border");
    let view_model = ExpanderPageViewModel::new();
    border.set_data_context(Some(view_model.clone() as BoxedValue));
    // The unset value: the property keeps its default.
    assert_eq!(CornerRadius::default(), border.corner_radius());
    view_model.set_rounded(true);
    assert_eq!(CornerRadius::uniform(25.0), border.corner_radius());
    view_model.set_rounded(false);
    assert_eq!(CornerRadius::default(), border.corner_radius());
}

/// C405: a two-way binding between a number and a property of an enumeration, as the tab
/// control page and the tree view page bind the selected index of a combo box
/// (`SelectedIndex="{Binding TabPlacement, Mode=TwoWay}"`). Neither conversion existed, and
/// the binding reported "Could not convert 'Top' (Dock) to 'i32'".
#[test]
fn gap_c405_enumeration_bound_to_a_number_in_both_directions() {
    use crate::view_models::TabControlPageViewModel;
    use ferroui_controls::{ComboBox, Dock};

    let _app = start_catalog_application();
    let combo_box = from_markup_value::<Ref<ComboBox>>(&Some(load_text(&format!(
        "<ComboBox {XMLNS} x:CompileBindings='False' SelectedIndex='{{Binding TabPlacement, Mode=TwoWay}}'>\
           <ComboBoxItem>Left</ComboBoxItem>\
           <ComboBoxItem>Bottom</ComboBoxItem>\
           <ComboBoxItem>Right</ComboBoxItem>\
           <ComboBoxItem>Top</ComboBoxItem>\
         </ComboBox>"
    ))))
    .expect("a combo box");
    let view_model = TabControlPageViewModel::new();
    combo_box.set_data_context(Some(view_model.clone() as BoxedValue));
    assert_eq!(view_model.tab_placement() as i32, combo_box.selected_index());
    view_model.set_tab_placement(Dock::Right);
    assert_eq!(Dock::Right as i32, combo_box.selected_index());
    combo_box.set_selected_index(Dock::Bottom as i32);
    assert_eq!(Dock::Bottom, view_model.tab_placement());
}

/// C406: a public method of a control bound as a command, as the control themes bind the items
/// of the context menu of a scroll bar (`Command="{Binding $parent[ScrollBar].LineDown}"`), the
/// scroll buttons of the menu scroll viewer and the copy item of the selectable text block.
/// Found by the audit of the binding paths of the themes (`theme_binding_paths` of the XAML test
/// crate): the classes declared no method for markup, so the bindings found no member and the
/// items did nothing.
#[test]
fn gap_c406_methods_of_the_scrolling_controls_as_commands() {
    use ferroui_base::metadata::into_markup_value;
    use ferroui_controls::primitives::ScrollBar;
    use ferroui_controls::{Button, ScrollViewer, SelectableTextBlock};

    let _app = start_catalog_application();
    let button_bound_to = |method: &str, source: BoxedValue| {
        let button = from_markup_value::<Ref<Button>>(&Some(load_text(&format!(
            "<Button {XMLNS} x:CompileBindings='False' Command='{{Binding {method}}}'/>"
        ))))
        .expect("a button");
        button.set_data_context(Some(source));
        button
    };

    let scroll_bar = ScrollBar::new();
    scroll_bar.set_maximum(100.0);
    scroll_bar.set_small_change(5.0);
    scroll_bar.set_large_change(20.0);
    let source = into_markup_value(scroll_bar.clone()).expect("the scroll bar as a value");
    for (method, value) in [
        ("LineDown", 5.0),
        ("LineRight", 10.0),
        ("PageDown", 30.0),
        ("PageRight", 50.0),
        ("LineUp", 45.0),
        ("LineLeft", 40.0),
        ("PageUp", 20.0),
        ("PageLeft", 0.0),
        ("ScrollToEnd", 100.0),
        ("ScrollToHome", 0.0),
    ] {
        let button = button_bound_to(method, source.clone());
        let command = button.command().unwrap_or_else(|| panic!("ScrollBar.{method} is no command"));
        command.execute(None);
        assert_eq!(value, scroll_bar.value(), "{method}");
    }
    assert!(button_bound_to("ScrollHere", source.clone()).command().is_some());

    let scroll_viewer = into_markup_value(ScrollViewer::new()).expect("the scroll viewer as a value");
    for method in [
        "LineUp", "LineDown", "LineLeft", "LineRight", "PageUp", "PageDown", "PageLeft", "PageRight", "ScrollToHome",
        "ScrollToEnd",
    ] {
        assert!(button_bound_to(method, scroll_viewer.clone()).command().is_some(), "ScrollViewer.{method}");
    }

    let text_block = into_markup_value(SelectableTextBlock::new()).expect("the text block as a value");
    for method in ["Copy", "SelectAll", "ClearSelection"] {
        assert!(button_bound_to(method, text_block.clone()).command().is_some(), "SelectableTextBlock.{method}");
    }
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

/// C321: the name scope of a document holds the elements it names, and the root of the
/// document holds the name scope, so a root that has a name kept itself alive. The managed
/// original has both references. The scope holds the element it is attached to weakly, and
/// finds it by its name as long as it is alive; the other names are held as before.
#[test]
fn gap_c321_named_root_of_a_document() {
    let _app = start_catalog_application();
    let weak = load_and_release(&format!("<StackPanel {XMLNS} Name='root'/>"));
    assert!(weak.upgrade().is_none());

    let root = from_markup_value::<Ref<Control>>(&Some(load_text(&format!(
        "<StackPanel {XMLNS} Name='root'><Border Name='child'/></StackPanel>"
    ))))
    .expect("a control");
    let scope = ferroui_base::controls::NameScope::get_name_scope(&root).expect("the name scope of the root");
    assert!(scope.find("root").is_some_and(|found| found == root));
    let child = scope.find("child").expect("the named child").downgrade();
    let weak = root.downgrade();
    drop(root);
    assert!(weak.upgrade().is_none());
    // The scope outlives its root here: the root is gone, the child is the scope's.
    assert!(scope.find("root").is_none());
    assert!(child.upgrade().is_some());
    drop(scope);
    assert!(child.upgrade().is_none());
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

// --- C322 to C330: what a visit still retained after C317 to C321 ---
//
// Found with the revisits of `catalog_tour.rs` and the markup probe (`catalog_markup_survivors`),
// with the weak references of the object model marked for the allocation trace. Each is a cycle
// of references the managed original has too.

/// Shows the control in a window, takes it out, lets go of it and returns what remains.
fn show_and_release(control: Ref<Control>) -> ferroui_base::WeakRef<Control> {
    let weak = control.downgrade();
    let window = show_and_remove(control);
    assert!(weak.upgrade().is_none());
    window.close();
    weak
}

/// C322: `DataValidationErrors.Owner` is the control the errors are shown in, which owns the
/// errors control through its template.
#[test]
fn gap_c322_owner_of_data_validation_errors() {
    let _app = start_catalog_application();
    let owner = ferroui_controls::ContentControl::new();
    let errors = ferroui_controls::DataValidationErrors::new();
    errors.set_owner(Some(owner.clone().upcast()));
    owner.set_content(Some(Control::boxed(&errors)));
    assert!(errors.owner().is_some_and(|found| found == owner));
    let weak = owner.downgrade();
    drop(owner);
    assert!(weak.upgrade().is_none());
    assert!(errors.owner().is_none());
}

/// C322: the themes make the owner the data context of a part of the template of the errors
/// control (`DataContext="{TemplateBinding Owner}"`), and the bindings of that part read the
/// owner through it.
#[test]
fn gap_c322_owner_as_the_data_context_of_a_template_part() {
    let _app = start_catalog_application();
    let control = from_markup_value::<Ref<ferroui_controls::ContentControl>>(&Some(load_text(&format!(
        "<ContentControl {XMLNS} Tag='of the owner'>\
           <ContentControl.Template>\
             <ControlTemplate>\
               <DataValidationErrors>\
                 <DataValidationErrors.Template>\
                   <ControlTemplate>\
                     <Border DataContext='{{TemplateBinding Owner}}' Tag='{{ReflectionBinding Tag}}'/>\
                   </ControlTemplate>\
                 </DataValidationErrors.Template>\
               </DataValidationErrors>\
             </ControlTemplate>\
           </ContentControl.Template>\
         </ContentControl>"
    ))))
    .expect("a content control");
    let window = Window::new();
    window.set_content(Some(Control::boxed(&control)));
    window.show();
    run_jobs();
    let part = control
        .get_visual_descendants()
        .find_map(|visual| visual.cast::<Border>())
        .expect("the part of the template of the errors control");
    let tag = part.tag().and_then(|tag| tag.downcast_ref::<String>().cloned());
    assert_eq!(Some(String::from("of the owner")), tag);
    let part = {
        let weak = part.downgrade();
        drop(part);
        weak
    };
    window.set_content(None);
    run_jobs();
    let weak = control.downgrade();
    drop(control);
    assert!(weak.upgrade().is_none());
    assert!(part.upgrade().is_none());
    window.close();
}

// C323 (the type resolver of a reflection binding declared in a template holds the context of the
// build of the template) is reproduced by `catalog_tour::gap_c323_controls_of_the_compiled_theme_are_freed`:
// it needs a template of compiled markup, which the Fluent theme of the tours has.

/// C324: the disposable of a routed event handler holds the element the handler was added to. A
/// slider keeps the disposable of the handler of its own pointer events.
#[test]
fn gap_c324_element_that_keeps_the_disposable_of_its_own_handler() {
    let _app = start_catalog_application();
    show_and_release(ferroui_controls::Slider::new().upcast());
}

/// C325: a dynamic resource declared as the value of a setter is anchored to the element the
/// style is declared under, which owns the style.
#[test]
fn gap_c325_dynamic_resource_in_a_style_declared_under_an_element() {
    let _app = start_catalog_application();
    let control = from_markup_value::<Ref<Control>>(&Some(load_text(&format!(
        "<Border {XMLNS}>\
           <Border.Styles>\
             <Style Selector='Border'>\
               <Setter Property='Tag' Value='{{DynamicResource NoSuchResource}}'/>\
             </Style>\
           </Border.Styles>\
         </Border>"
    ))))
    .expect("a control");
    show_and_release(control);
}

/// C326: the buttons page of the sample is its own data context (`DataContext = this`), and its
/// document binds to it.
#[test]
fn gap_c326_page_that_is_its_own_data_context() {
    use ferroui_base::data::core::ValueTypes;
    let _app = start_catalog_application();
    let page = crate::pages::ButtonsPage::new();
    let context = page.data_context().expect("the data context of the page");
    assert!(ValueTypes::as_object(&*context).is_some_and(|object| object == page));
    let window = Window::new();
    window.set_content(Some(Control::boxed(&page)));
    window.show();
    run_jobs();
    // `Command="{Binding $parent[controls:SamplePage].((pages:ButtonsPage)DataContext).CountCommand}"`.
    let bound = page
        .get_visual_descendants()
        .filter_map(|visual| visual.cast::<ferroui_controls::Button>())
        .filter(|button| button.get_value(ferroui_controls::Button::command_property()).is_some())
        .count();
    assert_eq!(1, bound);
    window.set_content(None);
    run_jobs();
    let weak = page.downgrade();
    drop((page, context));
    assert!(weak.upgrade().is_none());
    window.close();
}

/// C327: the observer of a binding of a local value and the observable it is subscribed to hold
/// each other after the object of the binding was dropped. The default data template binds the
/// text of its text block to an observable of the data context of the block: every such block
/// left the observable, the observer and its own memory, which their weak references held.
#[test]
fn gap_c327_local_value_binding_of_an_object_that_was_dropped() {
    use ferroui_base::data::BindingPriority;
    use ferroui_base::{FerroObject, FerroObjectExtensions};
    let _app = start_catalog_application();
    let source = Border::new();
    let target = Border::new();
    let source_object: &FerroObject = &source;
    let target_object: &FerroObject = &target;
    let subscribers = source_object.property_changed_subscriber_count();
    let observable = FerroObjectExtensions::get_observable(source_object, Control::tag_property());
    let binding =
        FerroObjectExtensions::bind_typed(target_object, Control::tag_property(), observable, BindingPriority::LocalValue);
    source.set_tag(Some(Rc::new(String::from("value")) as BoxedValue));
    assert!(target.tag().is_some());
    assert_eq!(subscribers + 1, source_object.property_changed_subscriber_count());

    // The target goes without the binding being disposed, as an element goes with its tree.
    drop(binding);
    drop(target);
    assert_eq!(subscribers, source_object.property_changed_subscriber_count());
}

/// C328: a view model of the sample keeps an observable of its own properties
/// (`SelectionMode = this.WhenAnyValue(..)`), which held the view model.
#[test]
fn gap_c328_view_model_with_an_observable_of_its_own_properties() {
    let _app = start_catalog_application();
    let view_model = crate::view_models::ListBoxPageViewModel::new();
    let weak = Rc::downgrade(&view_model);
    drop(view_model);
    assert!(weak.upgrade().is_none());
}

/// C329: the path of a compiled binding to a named element holds the name scope. The binding of a
/// setter of a style declared under a named element is held by that element, which the scope
/// holds.
#[test]
fn gap_c329_compiled_binding_to_a_name_in_a_style_under_a_named_element() {
    let _app = start_catalog_application();
    let root = from_markup_value::<Ref<ferroui_controls::Panel>>(&Some(load_text(&format!(
        "<Panel {XMLNS}>\
           <StackPanel Name='named' Tag='of the named element'>\
             <StackPanel.Styles>\
               <Style Selector='Border'>\
                 <Setter Property='Tag' Value='{{CompiledBinding #named.Tag}}'/>\
               </Style>\
             </StackPanel.Styles>\
             <Border/>\
           </StackPanel>\
         </Panel>"
    ))))
    .expect("a panel");
    let window = Window::new();
    window.set_content(Some(Control::boxed(&root)));
    window.show();
    run_jobs();
    let (named, bound) = {
        let named = root.children().get(0).cast::<ferroui_controls::StackPanel>().expect("the named panel");
        let bound = named.children().get(0);
        let tag = bound.tag().and_then(|tag| tag.downcast_ref::<String>().cloned());
        assert_eq!(Some(String::from("of the named element")), tag);
        (named.downgrade(), bound.downgrade())
    };
    window.set_content(None);
    run_jobs();
    let weak = root.downgrade();
    drop(root);
    assert!(weak.upgrade().is_none());
    assert!(named.upgrade().is_none());
    assert!(bound.upgrade().is_none());
    window.close();
}

/// C330: a visual subscribes to the changes of the values of its render-affecting properties, and
/// a value that many visuals share (the geometry of an icon that is a resource) kept the entry of
/// every visual that ever drew it, with the memory of the visual its weak reference held. The
/// managed original subscribes with a weak event, whose list is compacted.
#[test]
fn gap_c330_shared_geometry_of_a_visual_that_was_dropped() {
    use ferroui_base::media::{Geometry, RectangleGeometry};
    let _app = start_catalog_application();
    let geometry: Ref<Geometry> = RectangleGeometry::new().upcast();
    let subscribers = geometry.changed_subscriber_count();
    let icon = ferroui_controls::PathIcon::new();
    icon.set_data(&geometry);
    assert_eq!(subscribers + 1, geometry.changed_subscriber_count());
    drop(icon);
    assert_eq!(subscribers, geometry.changed_subscriber_count());
}

/// C332: a part of a template whose command is a method of the templated control. The scroll
/// buttons of the menu scroll viewer of the themes bind their commands to the methods of the
/// scroll viewer (`Command="{Binding LineUp, RelativeSource={RelativeSource TemplatedParent}}"`,
/// declared for markup with C406). The delegate the binding reads has the scroll viewer as its
/// target, and so has the command made from it, which the button has as the value of its
/// command: the scroll viewer, its template, the button, its command, the scroll viewer. The
/// managed original has the same references and its collector frees them; here every menu that
/// was opened stayed, with its items. The delegate of a binding source has the source weakly.
#[test]
fn gap_c332_part_of_a_template_with_a_command_of_a_method_of_the_templated_control() {
    use ferroui_controls::{RepeatButton, ScrollViewer};
    let _app = start_catalog_application();
    let viewer = from_markup_value::<Ref<ScrollViewer>>(&Some(load_text(&format!(
        "<ScrollViewer {XMLNS} Theme='{{StaticResource SimpleMenuScrollViewer}}' Width='100' Height='40'>\
           <StackPanel><TextBlock Height='30'>One</TextBlock><TextBlock Height='30'>Two</TextBlock></StackPanel>\
         </ScrollViewer>"
    ))))
    .expect("a scroll viewer");
    let window = Window::new();
    window.set_content(Some(Control::boxed(&viewer)));
    window.show();
    run_jobs();
    // The two scroll buttons have their commands, and the commands scroll.
    let buttons: Vec<Ref<RepeatButton>> =
        viewer.get_visual_descendants().filter_map(|visual| visual.cast::<RepeatButton>()).collect();
    assert_eq!(2, buttons.len());
    let commands: Vec<_> =
        buttons.iter().filter_map(|button| button.get_value(ferroui_controls::Button::command_property())).collect();
    assert_eq!(2, commands.len());
    assert_eq!(0.0, viewer.offset().y);
    commands[1].execute(None);
    assert!(viewer.offset().y > 0.0, "the command of the lower button scrolls a line down");
    commands[0].execute(None);
    assert_eq!(0.0, viewer.offset().y);

    window.set_content(None);
    run_jobs();
    let weak = viewer.downgrade();
    drop((viewer, buttons));
    assert!(weak.upgrade().is_none(), "the scroll viewer is alive after it left the tree and was dropped");
    // A command that outlives the scroll viewer does nothing.
    assert!(!commands[0].can_execute(None));
    commands[1].execute(None);
    window.close();
}
