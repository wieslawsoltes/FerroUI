//! Minimal reproductions of gaps of the framework found while porting a
//! group of the sample (see `gaps.rs`).

use super::support::*;
use crate::markup::{describe, try_load_text};
use crate::pages::ContextFlyoutPage;
use crate::view_models::{ItemModel, ListBoxPageViewModel};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::{Border, ComboBox, Control, ListBox, Menu, Window};

const XMLNS: &str = "xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'";

/// Shows `control` as the content of a window.
fn show(control: &Ref<Control>) -> Ref<Window> {
    let window = Window::new();
    window.set_width(400.0);
    window.set_height(300.0);
    window.set_content(Some(Control::boxed(control)));
    window.show();
    window
}

#[test]
fn gap_c200_cast_with_a_prefix_in_a_reflection_binding_path() {
    let _app = start_catalog_application();
    // Upstream: `(viewModels:ItemModel).ID` casts the item to the type the prefix names; the
    // prefix is resolved with the namespaces of the document.
    let list_box = from_markup_value::<Ref<ListBox>>(&Some(load_text(&format!(
        "<ListBox {XMLNS} xmlns:viewModels='using:ControlCatalog.ViewModels' x:DataType='viewModels:ListBoxPageViewModel' \
           ItemsSource='{{Binding Items}}' \
           DisplayMemberBinding=\"{{Binding (viewModels:ItemModel).ID, StringFormat='{{}}Item {{0:N0}}'}}\"/>"
    ))))
    .expect("a list box");
    list_box.set_data_context(Some(ListBoxPageViewModel::new() as BoxedValue));
    let window = show(&list_box.clone().upcast());
    assert!(list_box.container_from_index(0).is_some());
    window.close();
    let _ = ItemModel::new(0);
}

#[test]
fn gap_c201_array_list_in_markup() {
    let _app = start_application();
    // Upstream: an `ArrayList` element collects its children (null included) and is the items
    // source of the combo box.
    let combo_box = from_markup_value::<Ref<ComboBox>>(&Some(load_text(&format!(
        "<ComboBox {XMLNS} xmlns:col='using:System.Collections' xmlns:sys='using:System'>\
           <ComboBox.ItemsSource>\
             <col:ArrayList><x:Null /><sys:String>Hello</sys:String><sys:String>World</sys:String></col:ArrayList>\
           </ComboBox.ItemsSource>\
         </ComboBox>"
    ))))
    .expect("a combo box");
    assert_eq!(3, combo_box.item_count());
}

#[test]
fn gap_c202_on_platform_element_with_on_children() {
    let _app = start_application();
    // Upstream: the element form of the extension takes `On` children (its content) and
    // provides the value of the matching option, converted to the type of the property.
    let border = from_markup_value::<Ref<Border>>(&Some(load_text(&format!(
        "<Border {XMLNS}>\
           <Border.Background>\
             <OnPlatform Default='Gray'><On Options='macOS, Linux, Windows' Content='Green' /></OnPlatform>\
           </Border.Background>\
         </Border>"
    ))))
    .expect("a border");
    assert!(border.background().is_some());
}

#[test]
fn gap_c203_flyout_opening_handler() {
    let _app = start_application();
    // Upstream: `Opening` is an `EventHandler`; the page declares
    // `ContextFlyoutPage_Opening(object? sender, EventArgs e)` and tests `e is CancelEventArgs`.
    let xaml = format!(
        "<ContentPage {XMLNS} x:Class='ControlCatalog.Pages.ContextFlyoutPage'>\
           <Border><Border.ContextFlyout><Flyout Opening='ContextFlyoutPage_Opening'/></Border.ContextFlyout></Border>\
         </ContentPage>"
    );
    let page = (ContextFlyoutPage::XAML_CLASS.create_uninitialized)();
    if let Err(error) = try_load_text(&xaml, None, Some(page)) {
        panic!("{}", describe(&error));
    }
}

#[test]
fn gap_c204_menu_under_the_simple_theme() {
    let _app = start_catalog_application();
    // Upstream: the menu shows; the template of the Simple theme finds the control theme
    // `SimpleMenuScrollViewer` of the same theme.
    let menu = from_markup_value::<Ref<Menu>>(&Some(load_text(&format!(
        "<Menu {XMLNS}><MenuItem Header='_First'><MenuItem Header='Item'/></MenuItem></Menu>"
    ))))
    .expect("a menu");
    let window = show(&menu.clone().upcast());
    assert!(menu.is_attached_to_visual_tree());
    window.close();
}

#[test]
fn gap_c207_compiled_stream_binding_of_an_observable() {
    let _app = start_catalog_application();
    // `Pages/ListBoxPage.xaml`: the selection mode is an `IObservable<SelectionMode>` of the view
    // model, streamed by a compiled binding.
    let xaml = format!(
        "<ListBox {XMLNS} xmlns:viewModels='using:ControlCatalog.ViewModels' \
           x:DataType='viewModels:ListBoxPageViewModel' SelectionMode='{{Binding SelectionMode^}}'/>"
    );
    if let Err(error) = try_load_text(&xaml, None, None) {
        panic!("{}", describe(&error));
    }
}

#[test]
#[ignore = "gap C313: the item type of a collection of a view model is not known to markup"]
fn gap_c313_item_type_of_a_collection_of_a_view_model() {
    let _app = start_catalog_application();
    // `Pages/ComboBoxPage.xaml`: the item template and the display member binding have no
    // `x:DataType`; upstream takes the type of the items from the type of the collection the
    // items source is bound to (`InheritDataTypeFromItems`).
    let xaml = format!(
        "<ComboBox {XMLNS} xmlns:viewModels='using:ControlCatalog.ViewModels' \
           x:DataType='viewModels:ComboBoxPageViewModel' ItemsSource='{{Binding Values}}' \
           DisplayMemberBinding='{{Binding Name}}'>\
           <ComboBox.ItemTemplate><DataTemplate><TextBlock Text='{{Binding Id}}'/></DataTemplate></ComboBox.ItemTemplate>\
         </ComboBox>"
    );
    if let Err(error) = try_load_text(&xaml, None, None) {
        panic!("{}", describe(&error));
    }
}

#[test]
fn gap_c208_font_family_name_in_a_compiled_binding() {
    let _app = start_catalog_application();
    // `Pages/ComboBoxPage.xaml`: the item template of the font families shows their names.
    let xaml = format!("<TextBlock {XMLNS} x:DataType='FontFamily' Text='{{Binding Name}}' />");
    if let Err(error) = try_load_text(&xaml, None, None) {
        panic!("{}", describe(&error));
    }
}

#[test]
#[ignore = "gap C209: TextBox.Cut, Copy, Paste and Clear are not declared as methods for markup"]
fn gap_c209_text_box_methods_as_commands() {
    let _app = start_catalog_application();
    // `Pages/ContextFlyoutPage.xaml`: the buttons of the flyout of a text box bind its methods as
    // commands.
    let xaml = format!(
        "<TextBox {XMLNS}><TextBox.Tag><StackPanel>\
           <Button Command='{{Binding $parent[TextBox].Cut}}' IsEnabled='{{Binding $parent[TextBox].CanCut}}' />\
           <Button Command='{{Binding $parent[TextBox].Copy}}' IsEnabled='{{Binding $parent[TextBox].CanCopy}}' />\
           <Button Command='{{Binding $parent[TextBox].Paste}}' IsEnabled='{{Binding $parent[TextBox].CanPaste}}' />\
           <Button Command='{{Binding $parent[TextBox].Clear}}' />\
         </StackPanel></TextBox.Tag></TextBox>"
    );
    if let Err(error) = try_load_text(&xaml, None, None) {
        panic!("{}", describe(&error));
    }
}
