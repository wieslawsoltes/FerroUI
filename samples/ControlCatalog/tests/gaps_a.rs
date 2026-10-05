//! Minimal reproductions of gaps of the framework found while porting a
//! group of the sample (see `gaps.rs`), followed by the tests of the
//! controls of `Controls/` under their control themes (`CustomThemes.xaml`),
//! which need the application of the tests.

#[allow(unused_imports)]
use super::support::*;
use crate::controls::{
    CodeBlock, HomeItemExpander, SampleGalleryPage, SampleGroup, SampleGroups, SampleInfo, SampleSection,
    SelectableButton,
};
use ferroui_base::collections::FerroList;
use ferroui_base::layout::ILayoutManager;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::{BoxedValue, ObjectType, Ref, StaticType, Visual};
use ferroui_base::threading::Dispatcher;
use ferroui_controls::{
    Border, Button, CommandBar, ContentPage, Control, NavigationPage, ScrollViewer, SelectableTextBlock, Window,
};
use std::rc::Rc;

#[allow(dead_code)]
const XMLNS: &str = "xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'";

#[test]
fn gap_c100_setter_value_transform_operations_text() {
    let _app = start_application();
    // The value of a setter of a transform property (CustomThemes.xaml, the theme of CodeBlock).
    load_text(&format!(
        "<Style {XMLNS} Selector='Border'><Setter Property='RenderTransform' Value='rotate(90deg)' /></Style>"
    ));
}

#[test]
fn gap_c100_attribute_transform_operations_text() {
    let _app = start_application();
    let value = Some(load_text(&format!("<Border {XMLNS} RenderTransform='rotate(90deg)' />")));
    let border = from_markup_value::<Ref<Border>>(&value).expect("a border");
    assert!(border.render_transform().is_some());
}

#[test]
#[ignore = "gap C101: a reflection binding cannot resolve `$parent[prefix:Type]` for a type of a `using:` namespace"]
fn gap_c101_reflection_binding_parent_of_prefixed_type() {
    let _app = start_application();
    // CustomThemes.xaml (the theme of SampleGalleryPage) has such bindings; upstream compiles them
    // (compiled bindings are the default of the upstream build), and a reflection binding resolves the
    // type through the namespaces of the document.
    let xaml = format!(
        "<Border {XMLNS} xmlns:controls='using:ControlCatalog.Controls' x:CompileBindings='False'>\
         <controls:SampleSection Description='d'>\
         <Button Tag='{{Binding $parent[controls:SampleSection].Description}}' />\
         </controls:SampleSection></Border>"
    );
    let border = from_markup_value::<Ref<Border>>(&Some(load_text(&xaml))).expect("a border");
    let section = border.child().and_then(|child| child.cast::<SampleSection>()).expect("a section");
    let button = section.content().and_then(|content| Control::from_boxed(&content)).expect("a button");
    let window = show(&border.clone().upcast(), 400.0);
    assert_eq!(Some(String::from("d")), button.tag().and_then(|tag| tag.downcast_ref::<String>().cloned()));
    window.close();
}

#[test]
#[ignore = "gap C102: DataValidationException is not declared for markup"]
fn gap_c102_data_validation_exception_in_markup() {
    let _app = start_application();
    load_text(&format!(
        "<ContentControl {XMLNS} xmlns:data='using:FerroUI.Data'><data:DataValidationException>\
         <x:Arguments><x:String>Enter a valid email address.</x:String></x:Arguments>\
         </data:DataValidationException></ContentControl>"
    ));
}

#[test]
fn gap_c103_plain_property_of_a_class_through_a_base_handle() {
    let _app = start_application();
    let page = SampleGalleryPage::new();
    let property = <SampleGalleryPage as StaticType>::TYPE
        .markup()
        .and_then(|markup| markup.find_property("OpenSampleCommand"))
        .expect("the declared property");
    let get = property.get.expect("a getter");
    // The handle of the class itself is accepted.
    let typed: BoxedValue = Rc::new(page.clone());
    assert!(get(&[Some(typed)]).is_ok());
    // The handle of a base class (what a binding has for an element it found in the tree) is not.
    let base: BoxedValue = Rc::new(page.clone().upcast::<Control>());
    assert!(get(&[Some(base)]).is_ok());
}

#[test]
fn gap_c103_binding_to_a_plain_property_of_a_named_element() {
    let _app = start_application();
    let xaml = format!(
        "<controls:SampleGalleryPage {XMLNS} xmlns:controls='using:ControlCatalog.Controls' Name='page'>\
         <Button Command='{{Binding #page.OpenSampleCommand}}' /></controls:SampleGalleryPage>"
    );
    let page = from_markup_value::<Ref<SampleGalleryPage>>(&Some(load_text(&xaml))).expect("a page");
    let button = page.content().and_then(|content| Control::from_boxed(&content)).expect("a button");
    let window = show(&page.clone().upcast(), 400.0);
    assert!(button.cast::<Button>().expect("a button").command().is_some());
    window.close();
}

#[test]
fn gap_c104_command_bar_commands_from_markup() {
    let _app = start_application();
    let xaml = format!(
        "<CommandBar {XMLNS}><CommandBar.PrimaryCommands><CommandBarButton Label='A' /><CommandBarSeparator />\
         </CommandBar.PrimaryCommands><CommandBar.SecondaryCommands><CommandBarToggleButton Label='B' />\
         </CommandBar.SecondaryCommands></CommandBar>"
    );
    let bar = from_markup_value::<Ref<CommandBar>>(&Some(load_text(&xaml))).expect("a command bar");
    assert_eq!(2, bar.primary_commands().count());
    assert_eq!(1, bar.secondary_commands().count());

    // `PrimaryCommands` is the content property of the class.
    let xaml = format!("<CommandBar {XMLNS}><CommandBarButton Label='A' /></CommandBar>");
    let bar = from_markup_value::<Ref<CommandBar>>(&Some(load_text(&xaml))).expect("a command bar");
    assert_eq!(1, bar.primary_commands().count());
}

// --- The controls of `Controls/` under their themes (not ports: the upstream sample has no tests). ---

fn show(control: &Ref<Control>, width: f64) -> Ref<Window> {
    let window = Window::new();
    window.set_width(width);
    window.set_height(600.0);
    window.set_content(Some(Control::boxed(control)));
    window.show();
    window
}

fn descendants<T: ObjectType>(root: &Visual) -> Vec<Ref<T>> {
    root.get_visual_descendants().filter_map(|visual| visual.cast::<T>()).collect()
}

fn named<T: ObjectType>(root: &Visual, name: &str) -> Ref<T> {
    root.get_visual_descendants()
        .filter_map(|visual| visual.cast::<Control>())
        .find(|control| control.name().as_deref() == Some(name))
        .and_then(|control| control.cast::<T>())
        .unwrap_or_else(|| panic!("no part {name}"))
}

#[test]
fn sample_section_shows_its_code_and_options_under_its_theme() {
    let _app = start_catalog_application();
    let section = SampleSection::new();
    section.set_header(Some(Rc::new(String::from("Title"))));
    section.set_content(Some(Control::boxed(Border::new())));
    let window = show(&section.clone().upcast(), 900.0);

    let code: Ref<CodeBlock> = named(&section, "PART_Code");
    let options: Ref<Border> = named(&section, "PART_Options");
    assert!(!code.is_visible());
    assert!(!options.is_visible());
    assert!(!section.classes().contains(":narrow"));

    section.set_code(Some("    <Button Content=\"A\"\n            Width=\"10\" />"));
    section.set_options(Some(Control::boxed(Border::new())));
    window.set_width(500.0);
    window.layout_manager().execute_layout_pass();

    assert!(code.is_visible());
    assert!(options.is_visible());
    assert!(section.classes().contains(":narrow"));
    assert_eq!(Some(String::from("<Button Content=\"A\"\n        Width=\"10\" />")), {
        let text: Ref<SelectableTextBlock> = named(&code, "PART_Text");
        text.inlines().and_then(|inlines| inlines.text())
    });
    window.close();
}

#[test]
fn sample_gallery_page_shows_a_card_per_sample_under_its_theme() {
    let _app = start_catalog_application();
    let page = SampleGalleryPage::new();
    page.set_description(Some("The samples."));
    page.set_samples(FerroList::from_items([
        SampleInfo::new(SampleGroups::FEATURES, "Second", "", || Border::new().upcast()),
        SampleInfo::new(SampleGroups::OVERVIEW, "First", "", || Border::new().upcast()),
        SampleInfo::new(SampleGroups::FEATURES, "Third", "", || Border::new().upcast()),
    ]));
    let window = show(&page.clone().upcast(), 900.0);

    let groups = descendants::<SampleGroup>(&page);
    assert_eq!(2, groups.len());
    assert_eq!(1, descendants::<Button>(&groups[0]).len());
    assert_eq!(2, descendants::<Button>(&groups[1]).len());
    // Without a navigation host the command does nothing.
    let button = &descendants::<Button>(&groups[0])[0];
    let command = button.command().expect("the command of the page");
    command.execute(button.command_parameter().as_ref());
    window.close();
}

#[test]
fn home_item_expander_shows_its_content_when_it_can_expand_under_its_theme() {
    let _app = start_catalog_application();
    let xaml = format!(
        "<controls:HomeItemExpander {XMLNS} xmlns:controls='using:ControlCatalog.Controls' \
         Theme='{{StaticResource HomeSectionExpander}}' Header='Section' IsSelected='True'>\
         <Border /></controls:HomeItemExpander>"
    );
    let expander = from_markup_value::<Ref<HomeItemExpander>>(&Some(load_text(&xaml))).expect("an expander");
    let window = show(&expander.clone().upcast(), 400.0);

    let header: Ref<SelectableButton> = named(&expander, "ExpanderHeader");
    let content: Ref<Border> = named(&expander, "ExpanderContent");
    assert!(header.is_selected());
    assert!(header.classes().contains(":selected"));
    assert!(!content.is_visible());

    header.set_is_checked(Some(true));
    assert!(expander.is_expanded());
    assert!(!content.is_visible());
    expander.set_can_expand(true);
    assert!(content.is_visible());
    window.close();
}

#[test]
fn sample_gallery_page_opens_a_sample_on_its_navigation_host() {
    let _app = start_application();
    let navigation = NavigationPage::new();
    let window = show(&navigation.clone().upcast(), 600.0);
    let page = SampleGalleryPage::new();
    let _ = navigation.push_async_with_transition(page.clone(), None);
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(1, navigation.stack_depth());

    let sample = SampleInfo::new(SampleGroups::OVERVIEW, "Opened", "", || Border::new().upcast());
    let parameter: BoxedValue = Rc::new(sample);
    page.open_sample_command().execute(Some(&parameter));
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(2, navigation.stack_depth());
    let opened = navigation.current_page().expect("the page of the sample");
    assert_eq!(Some(String::from("Opened")), opened.header().and_then(|header| header.downcast_ref::<String>().cloned()));

    // A factory that fails gives a page that says so.
    let failing = SampleInfo::new(SampleGroups::OVERVIEW, "Broken", "", || panic!("no such sample"));
    let parameter: BoxedValue = Rc::new(failing);
    page.open_sample_command().execute(Some(&parameter));
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(3, navigation.stack_depth());
    let content = navigation
        .current_page()
        .and_then(|page| page.cast::<ContentPage>())
        .and_then(|page| page.content())
        .and_then(|content| Control::from_boxed(&content))
        .expect("the content of the page");
    assert!(content.is::<ScrollViewer>());
    window.close();
}
