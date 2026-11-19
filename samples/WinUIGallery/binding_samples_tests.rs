//! The Binding and Templates pages: the source's examples with their code behind
//! (`binding_samples.rs`). The x:Bind of the Binding page's elements is in `x_bind_tests.rs`.

use crate::binding_samples::{code_template, format_date, items, ExampleViewModel, ListDetailItem, GREETING_MESSAGE};
use crate::{examples, tests::start};
use ferroui_base::utilities::DateTimeOffset;
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::{ComboBox, Control, DatePicker, ListBox, RadioButtons, StackPanel, TextBlock, TextBox};
use std::rc::Rc;

fn page(id: &str) -> Ref<Control> {
    examples::create(examples::find(id).unwrap()).unwrap_or_else(|error| panic!("{id}: {error}"))
}

fn text(root: &Ref<Control>, name: &str) -> Option<String> {
    root.get_control::<TextBlock>(name).text()
}

#[test]
fn binding_page_code_behind_sets_the_greeting_the_date_and_the_view_model() {
    let _application = start();
    let root = page("Binding");
    assert_eq!(text(&root, "GreetingMessageText").as_deref(), Some(GREETING_MESSAGE));
    // FormatDate of the date picker's SelectedDate.
    assert_eq!(text(&root, "FormattedDate").as_deref(), Some("No date selected"));
    let date = DateTimeOffset::now();
    root.get_control::<DatePicker>("DatePickerControl")
        .set_selected_date(Some(date));
    assert_eq!(text(&root, "FormattedDate"), Some(format_date(Some(date))));
    assert!(format_date(Some(date)).starts_with("Selected date is: "));
    // The view model is the page's DataContext.
    let model = root
        .data_context()
        .and_then(|data| data.downcast_ref::<Rc<ExampleViewModel>>().cloned());
    let model = model.expect("the view model");
    assert_eq!(model.title, "Welcome to WinUI 3");
    assert_eq!(model.null_string, "");
    let _ = root.get_control::<TextBox>("InputTextBox");
}

#[test]
fn binding_page_list_shows_the_items_and_the_detail_of_the_selection() {
    let _application = start();
    let root = page("Binding");
    let titles: Vec<_> = items().iter().map(|item| item.title.clone()).collect();
    assert_eq!(titles, ["Item 1", "Item 2", "Item 3", "Item 4"]);
    let list = root.get_control::<ListBox>("ListDetailListView");
    assert_eq!(list.selected_index(), 0);
    // `DataContext="{x:Bind ListDetailListView.SelectedItem, Mode=OneWay}"`.
    let detail = root.get_control::<StackPanel>("ListDetail");
    let selected = |detail: &StackPanel| {
        detail
            .data_context()
            .and_then(|data: BoxedValue| data.downcast_ref::<Rc<ListDetailItem>>().map(|item| item.title.clone()))
    };
    assert_eq!(selected(&detail).as_deref(), Some("Item 1"));
    list.set_selected_index(2);
    assert_eq!(selected(&detail).as_deref(), Some("Item 3"));
    assert!(!items()[0].date_created_formatted().is_empty());
}

#[test]
fn templates_page_code_template_and_items_panels_follow_the_source() {
    use ferroui_controls::templates::IDataTemplate;
    let _application = start();
    let root = page("Templates");
    // The constructor's DataTemplate in code: a glyph and the bound text.
    assert!(root
        .get_control::<ComboBox>("CodeTemplateComboBox")
        .item_template()
        .is_some());
    let template: Rc<dyn IDataTemplate> = code_template();
    let panel = template
        .build(&Some(Rc::new("Option 2".to_string()) as BoxedValue))
        .and_then(|panel| panel.cast::<StackPanel>())
        .expect("the template's panel");
    assert_eq!(panel.children().count(), 2);
    assert!(template.match_(None));
    // LayoutSelector_SelectionChanged.
    let list = root.get_control::<ListBox>("MyListView");
    assert!(!list.classes().contains("StackPanel"));
    root.get_control::<RadioButtons>("LayoutSelector").set_selected_index(1);
    assert!(list.classes().contains("StackPanel"));
    root.get_control::<RadioButtons>("LayoutSelector").set_selected_index(0);
    assert!(!list.classes().contains("StackPanel"));
    let _ = root.get_control::<TextBox>("CustomTemplateTextBox");
}

#[test]
fn xaml_styles_apply_the_keyed_and_the_implicit_style() {
    use ferroui_base::media::FontWeight;
    use ferroui_controls::{Button, Window};
    let _application = start();
    let root = page("XamlStyles");
    let window = Window::new();
    window.set_content(Some(Control::boxed(root.clone())));
    window.show();
    window.update_layout();
    // The keyed style: the buttons of the class take its MinWidth; the default button does not,
    // and the third button's own Background is its local value.
    assert_ne!(root.get_control::<Button>("DefaultStyleButton").min_width(), 200.0);
    assert_eq!(root.get_control::<Button>("CustomStyleButton").min_width(), 200.0);
    let overridden = root.get_control::<Button>("OverriddenStyleButton");
    assert_eq!(overridden.min_width(), 200.0);
    assert!(overridden.classes().contains("CustomButtonStyle"));
    // The implicit style of the panel's TextBlocks.
    for name in ["ImplicitText1", "ImplicitText2"] {
        let text = root.get_control::<TextBlock>(name);
        assert_eq!(text.font_size(), 16.0, "{name}");
        assert_eq!(text.font_weight(), FontWeight::Bold, "{name}");
        assert_eq!(text.font_family().name(), "Consolas", "{name}");
    }
    // The page's own text is outside the implicit style's panel.
    assert_ne!(
        root.get_control::<StackPanel>("Example2")
            .children()
            .get(0)
            .cast::<TextBlock>()
            .unwrap()
            .font_size(),
        16.0
    );
    window.close();
}

#[test]
fn xaml_resources_resolve_at_each_level_and_follow_the_theme() {
    use crate::binding_samples::{theme_image_uri, ACCENT_BRUSHES};
    use ferroui_base::controls::ResourceKey;
    use ferroui_base::media::immutable::ImmutableSolidColorBrush;
    use ferroui_base::media::{Color, IBrush};
    use ferroui_base::styling::ThemeVariant;
    use ferroui_controls::{Application, Border, Grid, Image, Window};
    fn color(brush: Option<Rc<dyn IBrush>>) -> Option<Color> {
        brush.and_then(|brush| brush.as_solid_color_brush().map(|brush| brush.color()))
    }
    let _application = start();
    let root = page("XamlResources");
    // The application-, page- and control-level resources of the first example.
    assert_eq!(
        color(root.get_control::<Border>("AppLevelPanel").background()),
        Some(Color::from_rgb(0x00, 0x78, 0xd4))
    );
    assert_eq!(
        color(root.get_control::<Border>("PageLevelPanel").background()),
        Some(Color::from_rgb(0xa9, 0x4d, 0xc1))
    );
    assert_eq!(
        color(root.get_control::<TextBlock>("PageLevelText").foreground()),
        Some(Color::from_rgb(0xff, 0xff, 0xff))
    );
    assert_eq!(
        color(root.get_control::<Border>("ControlLevelPanel").background()),
        Some(Color::from_rgb(0xe2, 0x24, 0x1a))
    );
    assert_eq!(text(&root, "ControlLevelText").as_deref(), Some("Using control-level resources"));
    // StaticResource and ThemeResource of the same theme brush.
    assert!(root.get_control::<Grid>("StaticResourceGrid").background().is_some());
    assert!(theme_image_uri(Some(&ThemeVariant::light())).ends_with("SampleMedia/Light_Image.png"));
    assert!(theme_image_uri(Some(&ThemeVariant::dark())).ends_with("SampleMedia/Dark_Image.png"));
    // The theme's image where the platform decodes it (the test platform may not).
    let decoded = crate::assets::load_bitmap(theme_image_uri(None)).is_ok();
    assert_eq!(root.get_control::<Image>("ThemeImage").source().is_some(), decoded);

    // In a window: the ThemeDictionaries of the third example follow the requested theme, and
    // SetThemeResourceBinding follows a change of the resource where the one-time lookup keeps
    // the value it read.
    let window = Window::new();
    window.set_content(Some(Control::boxed(root.clone())));
    window.show();
    window.update_layout();
    let host = root.get_control::<Grid>("ThemeDictionaryHost");
    host.set_value(ThemeVariant::requested_theme_variant_property(), Some(ThemeVariant::light()));
    assert_eq!(text(&root, "ThemeStringText").as_deref(), Some("Light theme"));
    assert_eq!(
        color(root.get_control::<Border>("ThemeDictionaryPanel").background()),
        Some(Color::from_rgb(0xee, 0xee, 0xee))
    );
    host.set_value(ThemeVariant::requested_theme_variant_property(), Some(ThemeVariant::dark()));
    assert_eq!(text(&root, "ThemeStringText").as_deref(), Some("Dark theme"));
    assert_eq!(
        color(root.get_control::<Border>("ThemeDictionaryPanel").background()),
        Some(Color::from_rgb(0x33, 0x33, 0x33))
    );

    let (background, _) = ACCENT_BRUSHES;
    let one_time = root.get_control::<Border>("OneTimeLookupBorder");
    let bound = root.get_control::<Border>("ThemeResourceBindingBorder");
    let before = color(one_time.background());
    assert!(before.is_some(), "the one-time lookup read the application's brush");
    assert_eq!(color(bound.background()), before);
    let green = Color::from_rgb(0, 128, 0);
    Application::current().unwrap().resources().set(
        ResourceKey::from(background),
        Some(Rc::new(Rc::new(ImmutableSolidColorBrush::new(green)) as Rc<dyn IBrush>) as BoxedValue),
    );
    assert_eq!(color(bound.background()), Some(green), "the resource binding follows the resource");
    assert_eq!(color(one_time.background()), before, "the one-time lookup keeps its value");
    window.close();
}
