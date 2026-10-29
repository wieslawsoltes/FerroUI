//! Every themed control gets its template from the theme: one test per
//! control, as the tests of the Fluent theme
//! (`src/FerroUI.Themes.Fluent/tests/control_templates.rs`). The control is
//! created in a `Window` of an application whose theme is the WinUI theme;
//! after the window is shown
//!
//! * the template of the control theme (or of the theme it is based on) has
//!   been applied,
//! * the template parts the class asks for are in the name scope of the
//!   template, with the declared type,
//! * the setters of the control theme and of its bases took effect: the
//!   brushes it sets from resources (`Background`, `BorderBrush`,
//!   `Foreground`) are resolved, and `Padding`, `BorderThickness`,
//!   `MinWidth`, `MinHeight` and `CornerRadius` have the values of their
//!   setters (unless a nested style sets the property again).
//!
//! A batch that ports a control adds its test here and removes the control
//! from `PENDING_CONTROLS` (tests/support.rs).

use super::support::*;
use ferroui_base::controls::{NameScopeRef, ResourceKey};
use ferroui_base::layout::Layoutable;
use ferroui_base::media::IBrush;
use ferroui_base::metadata::{from_markup_value, MarkupAttributeValue};
use ferroui_base::styling::{ControlTheme, Setter, SetterValue, ThemeVariant};
use ferroui_base::{CornerRadius, FerroObject, Ref, Thickness, TypeInfo};
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::{Application, Border, Control, Window};
use std::cell::RefCell;
use std::rc::Rc;

/// The class of the controls or the dialogs assembly named `name`.
fn control_type(name: &str) -> &'static TypeInfo {
    let mut found = TypeInfo::registered_types().into_iter().filter(|type_| {
        type_.name() == name
            && (type_.module_path().starts_with("ferroui_controls") || type_.module_path().starts_with("ferroui_dialogs"))
    });
    let type_ = found.next().unwrap_or_else(|| panic!("{name} is not a class of the controls or the dialogs assembly"));
    assert!(found.next().is_none(), "{name} is ambiguous");
    type_
}

/// The control theme the application has for `type_`.
fn control_theme(type_: &'static TypeInfo) -> Ref<ControlTheme> {
    let application = Application::current().expect("an application");
    let theme = application.try_get_resource(&ResourceKey::Type(type_), None).flatten();
    from_markup_value::<Ref<ControlTheme>>(&theme).unwrap_or_else(|| panic!("the theme has no control theme for {}", type_.name()))
}

/// The parts of `type_` the theme must define.
fn expected_parts(type_: &'static TypeInfo) -> Vec<(&'static str, Option<&'static str>)> {
    // AutoCompleteBox has the optional part PART_SelectionAdapter, which no template of FerroUI's
    // themes defines (as the tests of the Fluent theme skip it).
    let skipped: &[&str] = if type_.name() == "AutoCompleteBox" { &["PART_SelectionAdapter"] } else { &[] };
    requested_parts(type_)
        .into_iter()
        .filter_map(|part| {
            let Some(MarkupAttributeValue::Str(name)) = part.arguments.first() else { panic!("a part without a name") };
            let type_name = match part.arguments.get(1) {
                Some(MarkupAttributeValue::Type(part_type)) => Some(part_type().name()),
                _ => None,
            };
            (!skipped.contains(name)).then_some((*name, type_name))
        })
        .collect()
}

/// The setters of a control theme and of the themes it is based on, the theme's own first: the
/// first setter of a property is the one in effect.
fn setters_of_chain(theme: &Ref<ControlTheme>) -> Vec<(&'static str, SetterValue)> {
    let mut setters = Vec::new();
    let mut current = Some(theme.clone());
    while let Some(theme) = current {
        for setter in theme.setters().snapshot().iter() {
            let Some(setter) = setter.as_any().and_then(|any| any.downcast_ref::<Setter>()) else { continue };
            let (Some(property), Some(value)) = (setter.property(), setter.value()) else { continue };
            if !setters.iter().any(|(name, _): &(&'static str, SetterValue)| *name == property.name()) {
                setters.push((property.name(), value));
            }
        }
        current = theme.based_on();
    }
    setters
}

/// The properties the nested styles of a control theme and of its bases set.
fn set_by_nested_styles(theme: &Ref<ControlTheme>) -> Vec<&'static str> {
    let mut names = Vec::new();
    let mut current = Some(theme.clone());
    while let Some(theme) = current {
        names.extend(
            enumerate_styles(&theme.clone().into())
                .iter()
                .skip(1)
                .flat_map(|style| style.setters().snapshot().iter().cloned().collect::<Vec<_>>())
                .filter_map(|setter| setter.as_any()?.downcast_ref::<Setter>()?.property().map(|property| property.name())),
        );
        current = theme.based_on();
    }
    names
}

fn check_control(name: &str) {
    let _app = start_themed_application();
    let type_ = control_type(name);
    let theme = control_theme(type_);

    let instance = type_.create_instance().unwrap_or_else(|| panic!("{name} has no default constructor"));
    let control = instance.cast::<Control>().unwrap_or_else(|| panic!("{name} is not a control"));
    let templated = control.cast::<TemplatedControl>();

    let name_scope: Rc<RefCell<Option<NameScopeRef>>> = Rc::new(RefCell::new(None));
    if let Some(templated) = &templated {
        let name_scope = name_scope.clone();
        let _ = templated.template_applied(move |_, e| *name_scope.borrow_mut() = Some(e.name_scope().clone()));
    }

    let window = Window::new();
    window.set_content(Some(Control::boxed(&control)));
    window.show();

    // The control theme of the theme is the one the control uses.
    assert!(theme.target_type().is_some_and(|target| std::ptr::eq(target, type_)));

    let setters = setters_of_chain(&theme);
    let setter = |property: &str| setters.iter().find(|(name, _)| *name == property).map(|(_, value)| value.clone());

    // The template of the theme has been applied (for ScrollBar, the template of the nested style
    // of its orientation).
    if setter("Template").is_some() || name == "ScrollBar" {
        let templated = templated.as_ref().unwrap_or_else(|| panic!("{name} has a template but is not templated"));
        assert!(templated.template().is_some(), "{name}: no template");
        assert_eq!(1, control.visual_children_count(), "{name}: the template was not applied");
        assert!(name_scope.borrow().is_some(), "{name}: TemplateApplied was not raised");
    }

    // The template parts.
    for (part, part_type) in expected_parts(type_) {
        let name_scope = name_scope.borrow();
        let name_scope = name_scope.as_ref().unwrap_or_else(|| panic!("{name}: no template name scope"));
        let found = name_scope.find(part).unwrap_or_else(|| panic!("{name}: the part {part} is not in the template"));
        if let Some(part_type) = part_type {
            assert!(
                is_instance_of(&found, part_type),
                "{name}: the part {part} is a {}, not a {part_type}",
                found.get_type().name()
            );
        }
    }

    // The setters of the theme.
    let Some(templated) = templated else { return };
    let brushes: [(&str, Option<Rc<dyn IBrush>>); 3] = [
        ("Background", templated.background()),
        ("BorderBrush", templated.border_brush()),
        ("Foreground", templated.foreground()),
    ];
    for (property, brush) in brushes {
        if matches!(setter(property), Some(SetterValue::BindingBase(_))) {
            assert!(brush.is_some(), "{name}: {property} is set by the theme from a resource but is not resolved");
        }
    }
    let nested = set_by_nested_styles(&theme);
    let value_of = |property: &str| match setter(property) {
        Some(SetterValue::Value(value)) if !nested.contains(&property) => Some(Some(value)),
        _ => None,
    };
    if let Some(expected) = value_of("Padding").and_then(|value| from_markup_value::<Thickness>(&value)) {
        assert_eq!(expected, templated.padding(), "{name}: Padding");
    }
    if let Some(expected) = value_of("BorderThickness").and_then(|value| from_markup_value::<Thickness>(&value)) {
        assert_eq!(expected, templated.border_thickness(), "{name}: BorderThickness");
    }
    if let Some(expected) = value_of("CornerRadius").and_then(|value| from_markup_value::<CornerRadius>(&value)) {
        assert_eq!(expected, templated.corner_radius(), "{name}: CornerRadius");
    }
    let layoutable: &Layoutable = &templated;
    if let Some(expected) = value_of("MinWidth").and_then(|value| from_markup_value::<f64>(&value)) {
        assert_eq!(expected, layoutable.min_width(), "{name}: MinWidth");
    }
    if let Some(expected) = value_of("MinHeight").and_then(|value| from_markup_value::<f64>(&value)) {
        assert_eq!(expected, layoutable.min_height(), "{name}: MinHeight");
    }
}

macro_rules! control_tests {
    ($($test:ident => $name:literal;)*) => {
        $(
            #[test]
            fn $test() {
                check_control($name);
            }
        )*

        /// The controls with a test.
        const TESTED: &[&str] = &[$($name),*];
    };
}

control_tests! {
    auto_complete_box => "AutoCompleteBox";
    button => "Button";
    calendar => "Calendar";
    calendar_button => "CalendarButton";
    calendar_date_picker => "CalendarDatePicker";
    calendar_day_button => "CalendarDayButton";
    calendar_item => "CalendarItem";
    check_box => "CheckBox";
    date_picker => "DatePicker";
    date_picker_presenter => "DatePickerPresenter";
    combo_box => "ComboBox";
    combo_box_item => "ComboBoxItem";
    context_menu => "ContextMenu";
    flyout_presenter => "FlyoutPresenter";
    hyperlink_button => "HyperlinkButton";
    list_box => "ListBox";
    list_box_item => "ListBoxItem";
    menu => "Menu";
    menu_flyout_presenter => "MenuFlyoutPresenter";
    menu_item => "MenuItem";
    radio_button => "RadioButton";
    repeat_button => "RepeatButton";
    scroll_bar => "ScrollBar";
    scroll_viewer => "ScrollViewer";
    text_box => "TextBox";
    theme_variant_scope => "ThemeVariantScope";
    thumb => "Thumb";
    time_picker => "TimePicker";
    time_picker_presenter => "TimePickerPresenter";
    toggle_button => "ToggleButton";
    // B1: primitives and simple content
    data_validation_errors => "DataValidationErrors";
    grid_splitter => "GridSplitter";
    group_box => "GroupBox";
    headered_content_control => "HeaderedContentControl";
    items_control => "ItemsControl";
    label => "Label";
    path_icon => "PathIcon";
    font_icon => "FontIcon";
    symbol_icon => "SymbolIcon";
    progress_bar => "ProgressBar";
    selectable_text_block => "SelectableTextBlock";
    separator => "Separator";
    text_selection_handle => "TextSelectionHandle";
    tool_tip => "ToolTip";
    transitioning_content_control => "TransitioningContentControl";
    // B6: window decorations, notifications, tables and dialogs
    managed_file_chooser => "ManagedFileChooser";
    managed_file_chooser_overwrite_prompt => "ManagedFileChooserOverwritePrompt";
    notification_card => "NotificationCard";
    table_view => "TableView";
    table_view_cell => "TableViewCell";
    table_view_column_header => "TableViewColumnHeader";
    table_view_row => "TableViewRow";
    window_notification_manager => "WindowNotificationManager";
    // B5: navigation, pages, tabs and command bar
    carousel => "Carousel";
    carousel_page => "CarouselPage";
    command_bar => "CommandBar";
    command_bar_button => "CommandBarButton";
    command_bar_separator => "CommandBarSeparator";
    command_bar_toggle_button => "CommandBarToggleButton";
    content_page => "ContentPage";
    drawer_page => "DrawerPage";
    navigation_page => "NavigationPage";
    split_view => "SplitView";
    tab_control => "TabControl";
    tab_item => "TabItem";
    tab_strip => "TabStrip";
    tab_strip_item => "TabStripItem";
    tabbed_page => "TabbedPage";
    // B2: buttons, ranges and expanders
    button_spinner => "ButtonSpinner";
    drop_down_button => "DropDownButton";
    expander => "Expander";
    numeric_up_down => "NumericUpDown";
    pips_pager => "PipsPager";
    refresh_container => "RefreshContainer";
    refresh_visualizer => "RefreshVisualizer";
    slider => "Slider";
    split_button => "SplitButton";
    toggle_switch => "ToggleSwitch";
    tree_view => "TreeView";
    tree_view_item => "TreeViewItem";
}

/// The window itself is themed: its template is applied when it is shown
/// and its brushes come from the theme.
#[test]
fn window() {
    let _app = start_themed_application();
    let name_scope: Rc<RefCell<Option<NameScopeRef>>> = Rc::new(RefCell::new(None));
    let window = Window::new();
    let _ = window.template_applied({
        let name_scope = name_scope.clone();
        move |_, e| *name_scope.borrow_mut() = Some(e.name_scope().clone())
    });
    let content = Border::new();
    window.set_content(Some(Control::boxed(&content)));
    window.show();

    assert!(window.template().is_some());
    assert_eq!(1, window.visual_children_count());
    assert!(window.background().is_some());
    assert!(window.foreground().is_some());
    let name_scope = name_scope.borrow();
    let name_scope = name_scope.as_ref().expect("TemplateApplied was raised");
    for (part, _) in expected_parts(Window::TYPE) {
        assert!(name_scope.find(part).is_some(), "Window: the part {part} is not in the template");
    }
    let presenter = name_scope.find("PART_ContentPresenter").expect("the content presenter");
    assert!(content.get_visual_ancestors().any(|ancestor| ancestor.upcast::<FerroObject>().ptr_eq(&presenter)));
}

/// The window follows the theme variants: the background of a window without its backdrop
/// (SolidBackgroundFillColorBaseBrush) differs in the light, the dark and the high contrast
/// variant.
#[test]
fn window_follows_the_theme_variants() {
    let _app = start_themed_application();
    let window = Window::new();
    window.set_content(Some(Control::boxed(&Border::new())));
    window.show();

    let background = || window.background().and_then(|brush| brush.as_solid_color_brush().map(|solid| solid.color()));
    window.set_requested_theme_variant(Some(ThemeVariant::light()));
    let light = background().expect("a solid background");
    window.set_requested_theme_variant(Some(ThemeVariant::dark()));
    let dark = background().expect("a solid background");
    window.set_requested_theme_variant(Some(crate::WinUITheme::high_contrast()));
    let high_contrast = background().expect("a solid background");
    assert_ne!(light, dark);
    assert_ne!(dark, high_contrast);
}

/// The presenters whose template draws the shadow of a popup (DEVIATIONS.md, G-24) do not clip to
/// their bounds, as a templated control does by default: the shadow lies outside them.
#[test]
fn the_presenters_with_a_theme_shadow_do_not_clip_it() {
    let _app = start_themed_application();
    for name in ["MenuFlyoutPresenter", "ContextMenu", "FlyoutPresenter", "ToolTip", "DatePickerPresenter", "TimePickerPresenter"] {
        let instance = control_type(name).create_instance().unwrap_or_else(|| panic!("{name} has no default constructor"));
        let control = instance.cast::<Control>().unwrap_or_else(|| panic!("{name} is not a control"));
        let window = Window::new();
        window.set_content(Some(Control::boxed(&control)));
        window.show();
        assert!(!control.clip_to_bounds(), "{name} clips its shadow");
        window.close();
    }
}

/// The path icon of a menu item, a logical child of the item, is unscaled as WinUI's (PI-3): its
/// size is the extent of its geometry from the origin, which the Viewbox of the item scales to 16.
#[test]
fn the_path_icon_of_a_menu_item_is_unscaled() {
    use ferroui_base::media::StreamGeometry;
    use ferroui_controls::{MenuItem, PathIcon};

    let _app = start_themed_application();
    let icon = PathIcon::new();
    icon.set_data(StreamGeometry::parse("M11,5H13V11H19V13H13V19H11V13H5V11H11Z").expect("a geometry"));
    let item = MenuItem::new();
    item.set_header(Some(Rc::new("New".to_string())));
    item.set_icon(Some(Control::boxed(&icon)));
    let window = Window::new();
    window.set_content(Some(Control::boxed(&item)));
    window.show();
    window.update_layout();

    let layoutable: &Layoutable = &icon;
    assert!(layoutable.width().is_nan() && layoutable.height().is_nan(), "the icon keeps the size of the theme of PathIcon");
    assert_eq!((19.0, 19.0), (icon.bounds().width, icon.bounds().height));
    window.close();
}

/// The icons of WinUI (`Controls/FontIcon.xaml`, `Controls/SymbolIcon.xaml`) draw their glyph in
/// the font of the resource `SymbolThemeFontFamily` (docs/porting/winui-controls.md, section 5.2):
/// an application that replaces the resource gets its font in both.
#[test]
fn the_icons_draw_in_the_symbol_font_of_the_theme() {
    use ferroui_base::media::FontFamily;
    use ferroui_controls::{FontIcon, StackPanel, Symbol, SymbolIcon, TextBlock};

    let _app = start_themed_application();
    let font_icon = FontIcon::new();
    font_icon.set_glyph("\u{E710}");
    let symbol_icon = SymbolIcon::with_symbol(Symbol::Accept);
    let panel = StackPanel::new();
    panel.children().add(&font_icon);
    panel.children().add(&symbol_icon);
    let window = Window::new();
    window.set_content(Some(Control::boxed(&panel)));
    window.show();
    window.update_layout();

    let text_of = |icon: &TemplatedControl| {
        icon.visual_children_snapshot().expect("the template of the theme")[0].cast::<TextBlock>().expect("PART_Text")
    };
    let theme_font = FontFamily::new("Segoe Fluent Icons,Segoe MDL2 Assets");
    assert_eq!(theme_font, font_icon.font_family());
    assert_eq!(theme_font, text_of(&font_icon).font_family());
    assert_eq!(20.0, text_of(&font_icon).font_size());
    assert_eq!(theme_font, text_of(&symbol_icon).font_family());
    assert_eq!(20.0, text_of(&symbol_icon).font_size());
    assert_eq!(Some(Symbol::Accept.glyph().to_string()), text_of(&symbol_icon).text());

    // An application's symbol font, through the resource.
    let custom = FontFamily::new("Custom Icons");
    window.resources().add("SymbolThemeFontFamily", Some(Rc::new(custom.clone()) as ferroui_base::BoxedValue));
    window.update_layout();
    assert_eq!(custom, text_of(&font_icon).font_family());
    assert_eq!(custom, text_of(&symbol_icon).font_family());
    window.close();
}

/// The keyed template `ScrollViewerScrollBarlessTemplate` (generic.xaml) of `Controls/ScrollViewer.xaml`:
/// a scroll viewer that an application gives it is built with its content presenter and without the
/// scroll bars, which FerroUI's scroll viewer takes as optional parts.
#[test]
fn scroll_viewer_with_the_scroll_bar_less_template() {
    use ferroui_controls::presenters::ScrollContentPresenter;
    use ferroui_controls::templates::IControlTemplate;
    use ferroui_controls::ScrollViewer;

    let _app = start_themed_application();
    let application = Application::current().expect("an application");
    let value = application
        .try_get_resource(&ResourceKey::from("ScrollViewerScrollBarlessTemplate"), None)
        .expect("ScrollViewerScrollBarlessTemplate is a resource of the theme")
        .expect("a template");
    let template = value
        .downcast_ref::<Option<Rc<dyn IControlTemplate>>>()
        .cloned()
        .flatten()
        .or_else(|| from_markup_value::<Rc<dyn IControlTemplate>>(&Some(value.clone())))
        .expect("a control template");

    let scroll_viewer = ScrollViewer::new();
    scroll_viewer.set_template(Some(template));
    let name_scope: Rc<RefCell<Option<NameScopeRef>>> = Rc::new(RefCell::new(None));
    let _ = scroll_viewer.template_applied({
        let name_scope = name_scope.clone();
        move |_, e| *name_scope.borrow_mut() = Some(e.name_scope().clone())
    });
    let content = Border::new();
    scroll_viewer.set_content(Some(Control::boxed(&content)));
    let window = Window::new();
    window.set_content(Some(Control::boxed(&scroll_viewer)));
    window.show();

    assert_eq!(1, scroll_viewer.visual_children_count());
    let name_scope = name_scope.borrow();
    let name_scope = name_scope.as_ref().expect("TemplateApplied was raised");
    let presenter: Ref<FerroObject> = name_scope.find("PART_ContentPresenter").expect("the content presenter");
    assert!(presenter.cast::<ScrollContentPresenter>().is_some(), "PART_ContentPresenter is a {}", presenter.get_type().name());
    assert!(content.get_visual_ancestors().any(|ancestor| ancestor.upcast::<FerroObject>().ptr_eq(&presenter)));
    for part in ["PART_HorizontalScrollBar", "PART_VerticalScrollBar"] {
        let found: Option<Ref<FerroObject>> = name_scope.find(part);
        assert!(found.is_none(), "{part} is in the scroll-bar-less template");
    }
}

/// The window decorations are themed (they are created by the framework, not put in a window as
/// content): the template of the theme, or of the theme it is based on, builds the parts the class
/// asks for, with the declared types.
#[test]
fn window_drawn_decorations() {
    use ferroui_controls::chrome::{IWindowDrawnDecorationsTemplate, WindowDrawnDecorations};

    let _app = start_themed_application();
    let theme = control_theme(WindowDrawnDecorations::TYPE);
    assert!(theme.target_type().is_some_and(|target| std::ptr::eq(target, WindowDrawnDecorations::TYPE)));
    let template = setters_of_chain(&theme)
        .into_iter()
        .find(|(name, _)| *name == "Template")
        .and_then(|(_, value)| match value {
            SetterValue::Value(value) => value
                .downcast_ref::<Option<Rc<dyn IWindowDrawnDecorationsTemplate>>>()
                .cloned()
                .flatten()
                .or_else(|| from_markup_value::<Rc<dyn IWindowDrawnDecorationsTemplate>>(&Some(value.clone()))),
            _ => None,
        })
        .expect("WindowDrawnDecorations: a decorations template");
    let built = template.build_typed();
    let name_scope = built.name_scope();
    for (part, part_type) in expected_parts(WindowDrawnDecorations::TYPE) {
        let found = name_scope
            .find(part)
            .unwrap_or_else(|| panic!("WindowDrawnDecorations: the part {part} is not in the template"));
        if let Some(part_type) = part_type {
            assert!(
                is_instance_of(&found, part_type),
                "WindowDrawnDecorations: the part {part} is a {}, not a {part_type}",
                found.get_type().name()
            );
        }
    }
}

/// Not from upstream: the file chooser of the theme presents its view model:
/// the quick links, the entries of the folder and the selection.
#[test]
fn managed_file_chooser_presents_its_view_model() {
    use ferroui_base::metadata::from_markup_value;
    use ferroui_base::threading::Dispatcher;
    use ferroui_controls::{ListBox, TextBox};
    use ferroui_dialogs::internal::{ManagedFileChooserItemViewModel, ManagedFileChooserViewModel};
    use ferroui_dialogs::{ManagedFileChooser, ManagedFileDialogOptions};

    let _app = start_themed_application();
    let root = std::env::temp_dir().join(format!("ferroui-theme-chooser-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("folder")).unwrap();
    std::fs::write(root.join("file.txt"), b"text").unwrap();

    let model = ManagedFileChooserViewModel::new(ManagedFileDialogOptions::new());
    model.navigate(Some(&root.to_string_lossy()), None);
    let chooser = ManagedFileChooser::new();
    chooser.set_data_context(Some(model.clone() as ferroui_base::BoxedValue));
    let applied: Rc<RefCell<Option<NameScopeRef>>> = Rc::new(RefCell::new(None));
    {
        let applied = applied.clone();
        let _ = chooser.template_applied(move |_, e| *applied.borrow_mut() = Some(e.name_scope().clone()));
    }
    let window = Window::new();
    window.set_content(Some(Control::boxed(&chooser)));
    window.show();
    window.update_layout();

    let name_scope = applied.borrow().clone().expect("TemplateApplied was raised");
    let find = |name: &str| -> Ref<FerroObject> {
        name_scope.find(name).unwrap_or_else(|| panic!("the part {name} is not in the template"))
    };

    let files = find("PART_Files").cast::<ListBox>().expect("a list box");
    let entries: Vec<String> = files
        .items()
        .to_vec()
        .iter()
        .filter_map(|item| from_markup_value::<Rc<ManagedFileChooserItemViewModel>>(item))
        .map(|item| item.display_name().unwrap())
        .collect();
    assert_eq!(vec!["folder", "file.txt"], entries);

    let quick_links = find("PART_QuickLinks").cast::<ListBox>().expect("a list box");
    assert_eq!(model.quick_links().count(), quick_links.items().count());

    let location = find("Location").cast::<TextBox>().expect("a text box");
    assert_eq!(Some(root.to_string_lossy().into_owned()), location.text());

    // A selection of the list box is the selection of the view model.
    files.set_selected_index(1);
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(Some("file.txt".to_string()), model.file_name());

    window.close();
    let _ = std::fs::remove_dir_all(&root);
}

/// Not from upstream: a press on a quick link navigates to it, a double press
/// on an entry of the folder opens it (a folder) or completes with it (a
/// file), and a single press on an entry only selects it.
#[test]
fn managed_file_chooser_opens_quick_links_and_double_pressed_entries() {
    use ferroui_base::input::{
        KeyModifiers, Pointer, PointerPointProperties, PointerPressedEventArgs, PointerType,
        PointerUpdateKind, RawInputModifiers,
    };
    use ferroui_base::{FerroLocator, Point};
    use ferroui_controls::ListBox;
    use ferroui_dialogs::internal::{
        ManagedFileChooserItemType, ManagedFileChooserNavigationItem, ManagedFileChooserSources,
        ManagedFileChooserViewModel,
    };
    use ferroui_dialogs::{ManagedFileChooser, ManagedFileDialogOptions};

    let _app = start_themed_application();
    let root = std::env::temp_dir().join(format!("ferroui-theme-chooser-press-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("folder").join("inner")).unwrap();
    std::fs::write(root.join("file.txt"), b"text").unwrap();
    let path = |name: &str| root.join(name).to_string_lossy().into_owned();

    let sources = ManagedFileChooserSources::new();
    let quick_link = path("folder");
    sources.set_get_all_items_delegate(Rc::new(move |_| {
        vec![ManagedFileChooserNavigationItem {
            display_name: Some("Folder".to_string()),
            path: Some(quick_link.clone()),
            item_type: ManagedFileChooserItemType::Folder,
        }]
    }));
    FerroLocator::current_mutable().bind::<ManagedFileChooserSources>().to_constant(Rc::new(sources));

    let model = ManagedFileChooserViewModel::new(ManagedFileDialogOptions::new());
    let completed = Rc::new(RefCell::new(Vec::new()));
    {
        let completed = completed.clone();
        let _ = model.complete_requested(move |items| completed.borrow_mut().push(items.to_vec()));
    }
    model.navigate(Some(&root.to_string_lossy()), None);
    let chooser = ManagedFileChooser::new();
    chooser.set_data_context(Some(model.clone() as ferroui_base::BoxedValue));
    let applied: Rc<RefCell<Option<NameScopeRef>>> = Rc::new(RefCell::new(None));
    {
        let applied = applied.clone();
        let _ = chooser.template_applied(move |_, e| *applied.borrow_mut() = Some(e.name_scope().clone()));
    }
    let window = Window::new();
    window.set_content(Some(Control::boxed(&chooser)));
    window.show();
    window.update_layout();
    let name_scope = applied.borrow().clone().expect("TemplateApplied was raised");
    let list = |name: &str| name_scope.find(name).and_then(|part| part.cast::<ListBox>()).expect("a list box");

    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
    let press = |target: &Ref<Control>, click_count: i32| {
        let args = PointerPressedEventArgs::new(
            target.clone(),
            pointer.clone(),
            &window,
            Point::default(),
            1,
            PointerPointProperties::new(RawInputModifiers::LEFT_MOUSE_BUTTON, PointerUpdateKind::LeftButtonPressed),
            KeyModifiers::NONE,
            click_count,
        );
        target.raise_event(&args);
        args.handled()
    };

    // A single press on an entry of the folder is left to the list box.
    let file = list("PART_Files").container_from_index(1).expect("the container of file.txt");
    assert!(!press(&file, 1));
    assert!(completed.borrow().is_empty());

    // A double press on a file completes with it.
    assert!(press(&file, 2));
    assert_eq!(vec![vec![path("file.txt")]], *completed.borrow());

    // A double press on a folder opens it.
    let folder = list("PART_Files").container_from_index(0).expect("the container of the folder");
    assert!(press(&folder, 2));
    assert_eq!(Some(path("folder")), model.location());

    // A single press on a quick link navigates to it.
    model.navigate(Some(&root.to_string_lossy()), None);
    window.update_layout();
    let link = list("PART_QuickLinks").container_from_index(0).expect("the container of the quick link");
    assert!(press(&link, 1));
    assert_eq!(Some(path("folder")), model.location());

    window.close();
    let _ = std::fs::remove_dir_all(&root);
}

/// Every control with a default control theme in the theme (a type-keyed
/// control theme whose target can be created and put in a window) has a
/// test above.
#[test]
fn every_themed_control_has_a_test() {
    let _app = start_application();
    let theme = create_attached_theme();
    // Top levels, layers and window decorations are created by the framework, not put in a window
    // as content.
    let not_content =
        ["Window", "EmbeddableControlRoot", "PopupRoot", "OverlayPopupHost", "AdornerLayer", "WindowDrawnDecorations"];
    let mut themed: Vec<&str> = enumerate_resources(&theme.as_style())
        .into_iter()
        .filter(|entry| entry.control_theme().is_some())
        .filter_map(|entry| entry.type_key().map(|type_| type_.name()))
        .filter(|name| !not_content.contains(name))
        .collect();
    themed.sort_unstable();
    for name in &themed {
        assert!(TESTED.contains(name), "{name} has a control theme but no test");
    }
    for name in TESTED {
        assert!(themed.contains(name), "{name} has a test but no control theme");
    }
}

/// A path icon of the framework's applications (the navigation of the ControlCatalog: a 16 by 16
/// icon of a geometry of 24 units in a button) is scaled into its box (PI-2), not drawn unscaled
/// and cut as the icons of the templates of PI-3.
#[test]
fn a_path_icon_in_a_button_is_scaled_into_its_box() {
    use ferroui_base::media::StreamGeometry;
    use ferroui_base::Visual;
    use ferroui_controls::shapes::Path;
    use ferroui_controls::{Button, PathIcon};

    let _app = start_themed_application();
    let icon = PathIcon::new();
    icon.set_data(StreamGeometry::parse("M2,2H22V22H2Z").expect("a geometry"));
    let layoutable: &Layoutable = &icon;
    layoutable.set_width(16.0);
    layoutable.set_height(16.0);
    let button = Button::new();
    button.set_content(Some(Control::boxed(&icon)));
    let window = Window::new();
    window.set_content(Some(Control::boxed(&button)));
    window.show();
    window.update_layout();

    let visual: &Visual = &icon;
    let names: Vec<String> = visual.get_self_and_visual_descendants().map(|d| d.get_type().name().to_string()).collect();
    assert!(visual.get_visual_descendants().any(|d| d.cast::<ferroui_controls::Viewbox>().is_some()), "the icon has the template of PI-2: {names:?}");
    let path = visual.get_visual_descendants().find_map(|d| d.cast::<Path>()).unwrap_or_else(|| panic!("the path of the icon: {names:?}"));
    assert_eq!(ferroui_base::media::Stretch::Uniform, path.stretch(), "the geometry is scaled");
    window.close();
}
