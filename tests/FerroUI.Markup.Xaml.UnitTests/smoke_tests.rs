//! End-to-end smoke tests of the run-time XAML loader against the real
//! framework: one small document per feature of the minimal viable set.

use std::rc::Rc;

use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::media::Colors;
use ferroui_base::{CornerRadius, Ref, Thickness};
use ferroui_base::layout::Orientation;
use ferroui_controls::{Border, Button, Control, Dock, DockPanel, StackPanel, TextBlock, UserControl};

use crate::support::app::xaml_test_base;
use crate::support::helpers::boxed;
use crate::support::loader::{describe, load_as, load_with_root, try_load};
use crate::support::test_view_model::TestViewModel;
use crate::support::xaml::event_tests::MyButton;

const NS: &str = "xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'";

/// The content of a content control, as a control of class `T`.
fn content<T: ferroui_base::ObjectType>(root: &Ref<UserControl>) -> Ref<T> {
    let content = root.content().expect("the control has content");
    Control::from_boxed(&content).and_then(|control| control.cast::<T>()).expect("the content is of the expected class")
}

#[test]
fn border_with_values_from_text_and_a_child() {
    let _base = xaml_test_base();
    let border = load_as::<Ref<Border>>(&format!(
        "<Border {NS} Background='Red' Padding='1,2,3,4' CornerRadius='5'><TextBlock Text='Hello'/></Border>"
    ));
    assert_eq!(border.padding(), Thickness::new(1.0, 2.0, 3.0, 4.0));
    assert_eq!(border.corner_radius(), CornerRadius::uniform(5.0));
    let background = border.background().expect("a background");
    assert_eq!(background.as_solid_color_brush().map(|brush| brush.color()), Some(Colors::RED));
    let child = border.child().and_then(|child| child.cast::<TextBlock>()).expect("a text block");
    assert_eq!(child.text().as_deref(), Some("Hello"));
}

#[test]
fn panel_with_children_attached_property_enum_and_names() {
    let _base = xaml_test_base();
    let root = load_as::<Ref<UserControl>>(&format!(
        "<UserControl {NS}>
           <DockPanel>
             <StackPanel x:Name='stack' Orientation='Horizontal' DockPanel.Dock='Right'>
               <Button Name='first' Content='One'/>
               <TextBlock x:Name='second' Text='Two'/>
             </StackPanel>
           </DockPanel>
         </UserControl>"
    ));
    let stack = root.find_control::<StackPanel>("stack").expect("the stack panel is found by name");
    assert_eq!(stack.orientation(), Orientation::Horizontal);
    assert_eq!(DockPanel::get_dock(&stack), Dock::Right);
    assert_eq!(stack.children().count(), 2);
    assert!(root.find_control::<Button>("first").is_some());
    assert_eq!(root.find_control::<TextBlock>("second").and_then(|t| t.text()).as_deref(), Some("Two"));
}

#[test]
fn style_with_selector_and_setter() {
    let _base = xaml_test_base();
    let root = load_as::<Ref<UserControl>>(&format!(
        "<UserControl {NS}>
           <UserControl.Styles>
             <Style Selector='TextBlock.big'>
               <Setter Property='FontSize' Value='40'/>
             </Style>
           </UserControl.Styles>
           <TextBlock Name='text' Classes='big'/>
         </UserControl>"
    ));
    assert_eq!(root.styles().count(), 1);
    let text = content::<TextBlock>(&root);
    assert!(text.classes().contains("big"));
}

#[test]
fn static_resource_from_resources() {
    let _base = xaml_test_base();
    let root = load_as::<Ref<UserControl>>(&format!(
        "<UserControl {NS}>
           <UserControl.Resources>
             <SolidColorBrush x:Key='brush'>Blue</SolidColorBrush>
           </UserControl.Resources>
           <Border Name='border' Background='{{StaticResource brush}}'/>
         </UserControl>"
    ));
    let border = content::<Border>(&root);
    let background = border.background().expect("a background");
    assert_eq!(background.as_solid_color_brush().map(|brush| brush.color()), Some(Colors::BLUE));
}

#[test]
fn binding_to_a_view_model_follows_changes() {
    let _base = xaml_test_base();
    let root = load_as::<Ref<UserControl>>(&format!(
        "<UserControl {NS}><TextBlock Name='text' Text='{{Binding String}}'/></UserControl>"
    ));
    let view_model = TestViewModel::new();
    view_model.set_string(Some("first".to_string()));
    root.set_data_context(Some(boxed(view_model.clone())));
    let text = content::<TextBlock>(&root);
    assert_eq!(text.text().as_deref(), Some("first"));
    view_model.set_string(Some("second".to_string()));
    assert_eq!(text.text().as_deref(), Some("second"));
}

#[test]
fn templates_are_deferred_and_instantiated() {
    use ferroui_controls::templates::IDataTemplate;
    let _base = xaml_test_base();
    let root = load_as::<Ref<UserControl>>(&format!(
        "<UserControl {NS}>
           <UserControl.ContentTemplate>
             <DataTemplate><TextBlock Text='templated'/></DataTemplate>
           </UserControl.ContentTemplate>
         </UserControl>"
    ));
    let template: Rc<dyn IDataTemplate> = root.content_template().expect("a template");
    let first = template.build(&None).and_then(|c| c.cast::<TextBlock>()).expect("a text block");
    let second = template.build(&None).and_then(|c| c.cast::<TextBlock>()).expect("a text block");
    assert_eq!(first.text().as_deref(), Some("templated"));
    assert!(first != second, "every instantiation builds new content");
}

#[test]
fn event_handler_on_a_root_instance_does_not_keep_the_root_alive() {
    let _base = xaml_test_base();
    let target = MyButton::new();
    load_with_root("<Button xmlns='https://github.com/ferroui' Click='OnClick'/>", None, boxed(target.clone()));
    target.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert!(target.was_clicked());

    let weak = target.downgrade();
    drop(target);
    assert!(weak.upgrade().is_none(), "the handler keeps the root alive");
}

#[test]
fn an_error_document_is_a_load_exception_with_line_info() {
    let _base = xaml_test_base();
    let error = try_load(&format!("<UserControl {NS}>\n  <Nope/>\n</UserControl>")).err().expect("an error");
    let text = describe(&error);
    assert!(text.contains("Nope"), "{text}");
    let inner = crate::support::loader::xaml_error(&error).expect("the error of the compiler");
    assert_eq!(inner.line_number(), Some(2));
    // A value that cannot be converted.
    let error = try_load(&format!("<Border {NS} Padding='abc'/>")).err().expect("an error");
    assert!(crate::support::loader::xaml_error(&error).and_then(|e| e.line_number()).is_some());
}


#[test]
fn a_style_setter_resolves_a_property_the_class_adds_itself_as_owner_of() {
    use ferroui_base::styling::IStyle;
    let _base = xaml_test_base();
    // `TextBlock.FontSize` is `TextElement.FontSize` with `TextBlock` added as an owner:
    // an instance property of `TextBlock` for the setter.
    let root = load_as::<Ref<UserControl>>(&format!(
        "<UserControl {NS}>
           <UserControl.Styles>
             <Style Selector='TextBlock'><Setter Property='FontSize' Value='40'/></Style>
           </UserControl.Styles>
         </UserControl>"
    ));
    let styles = root.styles();
    assert_eq!(styles.count(), 1);
    let _: Rc<dyn IStyle> = styles.get(0);
}

#[test]
fn a_control_assigned_to_an_untyped_property_is_held_as_the_control_base_class() {
    let _base = xaml_test_base();
    let root = load_as::<Ref<UserControl>>(&format!("<UserControl {NS}><Border/></UserControl>"));
    let boxed = root.content().expect("content");
    // The convention of the framework for controls in untyped values.
    assert!(Control::from_boxed(&boxed).is_some());
    assert!(content::<Border>(&root).child().is_none());
}

#[test]
fn elements_are_initialized_between_begin_init_and_end_init() {
    let _base = xaml_test_base();
    // A name is set while the element is being initialized; without `BeginInit` the
    // element would already be styled when it is attached and reject the name.
    let root = load_as::<Ref<UserControl>>(&format!(
        "<UserControl {NS}>
           <UserControl.Styles><Style Selector='TextBlock'/></UserControl.Styles>
           <StackPanel><TextBlock Name='late' Text='x'/></StackPanel>
         </UserControl>"
    ));
    assert!(root.find_control::<TextBlock>("late").is_some());
    // As in the managed original, `EndInit` only initializes an element that is attached
    // to a logical root: a loaded root that is not one stays uninitialized until then.
    assert!(!root.is_initialized());
}

#[test]
fn the_root_is_initialized_as_every_other_element() {
    use crate::support::xaml::initialization_order_tracker::InitializationOrderTracker;
    let _base = xaml_test_base();
    const LOCAL: &str = "xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'";
    // A root the document creates (`Build`): `BeginInit`, the properties, `EndInit`.
    let built = load_as::<Ref<InitializationOrderTracker>>(&format!(
        "<local:InitializationOrderTracker {NS} {LOCAL} Width='100'/>"
    ));
    // The name scope of the root is set after its initialisation (the root object scope).
    assert_eq!(built.order(), ["BeginInit 1", "Property Width Changed", "EndInit 0", "Property NameScope Changed"]);
    assert_eq!(built.init_state(), 0);

    // A root instance the document populates (`Populate`): the same calls on the instance.
    let provided = InitializationOrderTracker::new();
    load_with_root(
        &format!("<local:InitializationOrderTracker {NS} {LOCAL} Height='50'/>"),
        None,
        boxed(provided.clone()),
    );
    assert_eq!(provided.order(), ["BeginInit 1", "Property Height Changed", "EndInit 0", "Property NameScope Changed"]);
}

#[test]
fn a_root_instance_of_a_class_without_a_constructor_in_metadata_is_populated() {
    use ferroui_base::styling::IStyle;
    use ferroui_controls::Application;
    let _app = crate::support::app::styled_window_application();
    // The application of the tests is created by the test services and has no constructor
    // markup could call; populating the instance does not need one.
    let application = Application::current().expect("the application of the test");
    let styles_before = application.styles().count();
    load_with_root(
        &format!(
            "<Application {NS}>
               <Application.Styles><Style Selector='TextBlock'/></Application.Styles>
             </Application>"
        ),
        None,
        boxed(application.clone()),
    );
    let styles = application.styles();
    assert_eq!(styles.count(), styles_before + 1);
    let _: Rc<dyn IStyle> = styles.get(styles_before);
}

// --- includes of documents outside of the group (run-time include fallback) -------------

/// The documents of the assembly `FerroUI.Markup.Xaml.UnitTests.Includes`: an assembly
/// without markup metadata and without compiled markup, whose documents are assets.
mod include_assets {
    pub const ASSEMBLY: &str = "FerroUI.Markup.Xaml.UnitTests.Includes";

    const COLORS: &[u8] = b"<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
  <ResourceDictionary.MergedDictionaries>
    <ResourceInclude Source='Nested/Inner.xaml'/>
  </ResourceDictionary.MergedDictionaries>
  <Color x:Key='IncludedRed'>Red</Color>
</ResourceDictionary>";

    const INNER: &[u8] = b"<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
  <Color x:Key='InnerBlue'>Blue</Color>
</ResourceDictionary>";

    const STYLES: &[u8] = b"<Styles xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
  <Styles.Resources>
    <Color x:Key='StyleGreen'>Green</Color>
  </Styles.Resources>
  <Style Selector='TextBlock.included'>
    <Setter Property='FontSize' Value='40'/>
  </Style>
</Styles>";

    pub fn register() {
        ferroui_base::platform::register_assets(
            ASSEMBLY,
            &[("/Colors.xaml", COLORS), ("/Nested/Inner.xaml", INNER), ("/Styles.xaml", STYLES)],
        );
    }
}

fn color_resource(control: &Ref<UserControl>, key: &str) -> Option<ferroui_base::media::Color> {
    let key: ferroui_base::controls::ResourceKey = key.into();
    control.try_get_resource(&key, None).flatten().and_then(|value| value.downcast_ref::<ferroui_base::media::Color>().copied())
}

#[test]
fn style_include_of_an_asset_outside_of_the_group_loads_at_run_time() {
    let _app = crate::support::app::styled_window_application();
    include_assets::register();
    let root = load_as::<Ref<UserControl>>(&format!(
        "<UserControl {NS}>
           <UserControl.Styles>
             <StyleInclude Source='ferres://FerroUI.Markup.Xaml.UnitTests.Includes/Styles.xaml'/>
           </UserControl.Styles>
         </UserControl>"
    ));
    assert_eq!(root.styles().count(), 1);
    // The resources of the included styles are found through the include.
    assert_eq!(color_resource(&root, "StyleGreen"), Some(Colors::GREEN));
}

#[test]
fn resource_include_of_an_asset_outside_of_the_group_loads_at_run_time_with_its_nested_relative_include() {
    let _app = crate::support::app::styled_window_application();
    include_assets::register();
    let root = load_as::<Ref<UserControl>>(&format!(
        "<UserControl {NS}>
           <UserControl.Resources>
             <ResourceDictionary>
               <ResourceDictionary.MergedDictionaries>
                 <ResourceInclude Source='ferres://FerroUI.Markup.Xaml.UnitTests.Includes/Colors.xaml'/>
               </ResourceDictionary.MergedDictionaries>
             </ResourceDictionary>
           </UserControl.Resources>
         </UserControl>"
    ));
    assert_eq!(root.resources().merged_dictionaries().count(), 1);
    assert_eq!(color_resource(&root, "IncludedRed"), Some(Colors::RED));
    // `Nested/Inner.xaml` is relative to the URI of the included document.
    assert_eq!(color_resource(&root, "InnerBlue"), Some(Colors::BLUE));
}

#[test]
fn merge_resource_include_of_an_asset_outside_of_the_group_stays_a_run_time_include() {
    let _app = crate::support::app::styled_window_application();
    include_assets::register();
    // One include is a document of the group (inlined, as at compile time), the other an
    // asset outside of it (left in the merged dictionaries, loaded at run time).
    let documents = vec![
        crate::support::loader::document(
            "ferres://FerroUI.Markup.Xaml.UnitTests/InGroup.xaml",
            &format!("<ResourceDictionary {NS}><Color x:Key='GroupGreen'>Green</Color></ResourceDictionary>"),
        ),
        crate::support::loader::document_without_uri(&format!(
            "<UserControl {NS}>
               <UserControl.Resources>
                 <ResourceDictionary>
                   <ResourceDictionary.MergedDictionaries>
                     <MergeResourceInclude Source='ferres://FerroUI.Markup.Xaml.UnitTests/InGroup.xaml'/>
                     <MergeResourceInclude Source='ferres://FerroUI.Markup.Xaml.UnitTests.Includes/Colors.xaml'/>
                   </ResourceDictionary.MergedDictionaries>
                   <Color x:Key='Own'>White</Color>
                 </ResourceDictionary>
               </UserControl.Resources>
             </UserControl>"
        )),
    ];
    let loaded = crate::support::loader::load_group(documents, None);
    let root = crate::support::loader::group_item::<Ref<UserControl>>(&loaded, 1);
    assert_eq!(root.resources().merged_dictionaries().count(), 1, "only the run-time include is a merged dictionary");
    assert_eq!(color_resource(&root, "GroupGreen"), Some(Colors::GREEN));
    assert_eq!(color_resource(&root, "IncludedRed"), Some(Colors::RED));
    assert_eq!(color_resource(&root, "InnerBlue"), Some(Colors::BLUE));
    assert_eq!(color_resource(&root, "Own"), Some(Colors::WHITE));
}

#[test]
fn include_of_a_missing_asset_is_a_load_error_that_names_the_uri() {
    let _app = crate::support::app::styled_window_application();
    include_assets::register();
    for (element, source) in [
        // An assembly the type system does not know.
        ("StyleInclude", "ferres://FerroUI.Markup.Xaml.UnitTests.Includes/Missing.xaml"),
        ("ResourceInclude", "ferres://FerroUI.Markup.Xaml.UnitTests.Includes/Missing.xaml"),
        ("MergeResourceInclude", "ferres://FerroUI.Markup.Xaml.UnitTests.Includes/Missing.xaml"),
        // An assembly with markup metadata but without compiled markup.
        ("StyleInclude", "ferres://FerroUI.Markup.Xaml.UnitTests/Xaml/Missing.xaml"),
    ] {
        let property = if element == "StyleInclude" { "Styles" } else { "Resources" };
        let include = format!("<{element} Source='{source}'/>");
        let body = match element {
            "StyleInclude" => include,
            _ => format!(
                "<ResourceDictionary><ResourceDictionary.MergedDictionaries>{include}</ResourceDictionary.MergedDictionaries><Color x:Key='Own'>Red</Color></ResourceDictionary>"
            ),
        };
        let error = try_load(&format!(
            "<UserControl {NS}>\n<UserControl.{property}>\n{body}\n</UserControl.{property}>\n</UserControl>"
        ))
        .err()
        .unwrap_or_else(|| panic!("{element} of {source} loads"));
        let text = describe(&error);
        // As in the managed original the message has the URI in its canonical form (the
        // host, which is the assembly name, in lower case).
        assert!(text.to_lowercase().contains(&source.to_lowercase()), "{element}: {text}");
        assert!(text.contains("Line 3,"), "{element}: {text}");
    }
}

#[test]
fn a_dictionary_of_theme_dictionaries_knows_its_theme_variant() {
    use ferroui_base::styling::ThemeVariant;
    use ferroui_controls::ThemeVariantScope;
    let _base = xaml_test_base();
    // `x:Key` of an entry of `ThemeDictionaries` is also the `Key` of the dictionary
    // (the theme variant provider transformer): a static resource inside of it is looked
    // up with that variant in the resource nodes above it.
    let scope = load_as::<Ref<ThemeVariantScope>>(&format!(
        "<ThemeVariantScope {NS} RequestedThemeVariant='Light'>
           <ThemeVariantScope.Resources>
             <ResourceDictionary>
               <ResourceDictionary.ThemeDictionaries>
                 <ResourceDictionary x:Key='Light'><Color x:Key='TestColor'>White</Color></ResourceDictionary>
                 <ResourceDictionary x:Key='Dark'><Color x:Key='TestColor'>Black</Color></ResourceDictionary>
               </ResourceDictionary.ThemeDictionaries>
             </ResourceDictionary>
           </ThemeVariantScope.Resources>
           <Border>
             <Border.Resources>
               <ResourceDictionary>
                 <ResourceDictionary.ThemeDictionaries>
                   <ResourceDictionary x:Key='Light'><StaticResource x:Key='Demo' ResourceKey='TestColor'/></ResourceDictionary>
                   <ResourceDictionary x:Key='Dark'><StaticResource x:Key='Demo' ResourceKey='TestColor'/></ResourceDictionary>
                 </ResourceDictionary.ThemeDictionaries>
               </ResourceDictionary>
             </Border.Resources>
           </Border>
         </ThemeVariantScope>"
    ));
    let border = scope.child().and_then(|child| child.cast::<Border>()).expect("the border");
    let dictionaries = border.resources().theme_dictionaries();
    assert_eq!(dictionaries.get(&ThemeVariant::light()).key(), Some(ThemeVariant::light()));
    assert_eq!(dictionaries.get(&ThemeVariant::dark()).key(), Some(ThemeVariant::dark()));
    let key: ferroui_base::controls::ResourceKey = "Demo".into();
    let color = |variant: ThemeVariant| {
        border
            .try_get_resource(&key, Some(&variant))
            .flatten()
            .and_then(|value| value.downcast_ref::<ferroui_base::media::Color>().copied())
    };
    assert_eq!(color(ThemeVariant::light()), Some(Colors::WHITE));
    assert_eq!(color(ThemeVariant::dark()), Some(Colors::BLACK));
}

// --- the cache of transformed documents ---------------------------------------------------

#[test]
fn a_document_loaded_again_is_interpreted_again_from_its_transformed_form() {
    let _base = xaml_test_base();
    let xaml = format!(
        "<UserControl {NS}>
           <UserControl.Resources><SolidColorBrush x:Key='brush'>Blue</SolidColorBrush></UserControl.Resources>
           <Border Name='border' Background='{{StaticResource brush}}'><TextBlock Text='a'/></Border>
         </UserControl>"
    );
    let first = load_as::<Ref<UserControl>>(&xaml);
    let second = load_as::<Ref<UserControl>>(&xaml);
    // Every load builds its own objects, name scope and resources.
    assert!(first != second);
    let (first_border, second_border) = (content::<Border>(&first), content::<Border>(&second));
    assert!(first_border != second_border);
    assert!(first.find_control::<Border>("border") == Some(first_border.clone()));
    assert!(second.find_control::<Border>("border") == Some(second_border.clone()));
    first_border.set_padding(Thickness::uniform(3.0));
    assert_eq!(second_border.padding(), Thickness::uniform(0.0));
    // Another text is another document; a failed document is not kept.
    let changed = load_as::<Ref<UserControl>>(&xaml.replace("Text='a'", "Text='b'"));
    let text = content::<Border>(&changed).child().and_then(|child| child.cast::<TextBlock>()).expect("the text");
    assert_eq!(text.text().as_deref(), Some("b"));
    for _ in 0..2 {
        assert!(try_load(&format!("<UserControl {NS}><Nope/></UserControl>")).is_err());
    }
    // The same text with another root instance type or in design mode is transformed anew.
    let design = crate::support::loader::try_load_design_mode(&xaml).expect("the document loads in design mode");
    assert!(crate::support::loader::try_cast::<Ref<UserControl>>(&design).is_some());
}

#[test]
fn loading_theme_documents_again_skips_parsing_and_transforming() {
    use ferroui_markup_xaml_loader::FerroXamlIlRuntimeCompiler;
    use std::time::Instant;
    let _app = crate::support::app::styled_window_application();
    let documents = || {
        vec![
            crate::support::loader::document("ferres://Cache/Base.xaml", include_str!("theme_readiness/Base.xaml")),
            crate::support::loader::document("ferres://Cache/Button.xaml", include_str!("theme_readiness/Button.xaml")),
            crate::support::loader::document("ferres://Cache/CheckBox.xaml", include_str!("theme_readiness/CheckBox.xaml")),
            crate::support::loader::document("ferres://Cache/ListBox.xaml", include_str!("theme_readiness/ListBox.xaml")),
            crate::support::loader::document(
                "ferres://Cache/ScrollViewer.xaml",
                include_str!("theme_readiness/ScrollViewer.xaml"),
            ),
        ]
    };
    FerroXamlIlRuntimeCompiler::clear_cache();
    let start = Instant::now();
    let first = crate::support::loader::load_group(documents(), None);
    let cold = start.elapsed();
    let start = Instant::now();
    let second = crate::support::loader::load_group(documents(), None);
    let warm = start.elapsed();
    eprintln!("theme documents: first load {cold:?}, second load {warm:?}");
    assert_eq!(first.len(), second.len());
    // Not a benchmark: the second load only interprets, which is far cheaper than
    // parsing and transforming five theme documents.
    assert!(warm < cold, "first load {cold:?}, second load {warm:?}");
}
