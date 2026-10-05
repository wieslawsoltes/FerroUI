//! Every themed control gets its template from the theme: one test per
//! control. The control is created in a `Window` of an application whose
//! theme is the Fluent theme; after the window is shown
//!
//! * the template of the control theme has been applied,
//! * the template parts the class asks for are in the name scope of the
//!   template (the rule of the upstream theme tests: the parts the class
//!   declares itself and the required ones it inherits), with the declared
//!   type,
//! * the setters of the control theme took effect: the brushes the theme
//!   sets from resources (`Background`, `BorderBrush`, `Foreground`) are
//!   resolved, and `Padding`, `BorderThickness`, `MinWidth`, `MinHeight`
//!   and `CornerRadius` have the values of their setters (unless a nested
//!   style of the theme sets the property again).
//!
//! The tests of controls whose document is left out of the theme
//! (`Controls/excluded.txt`), or whose template needs a resource of such a
//! document, are ignored with what they wait for.

use super::support::*;
use ferroui_base::controls::{NameScopeRef, ResourceKey};
use ferroui_base::media::IBrush;
use ferroui_base::metadata::{from_markup_value, MarkupAttributeValue};
use ferroui_base::styling::{ControlTheme, Setter, SetterValue};
use ferroui_base::layout::Layoutable;
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

/// The parts of `type_` the theme must define, without the ones the
/// upstream theme tests leave out.
fn expected_parts(type_: &'static TypeInfo) -> Vec<(&'static str, Option<&'static str>)> {
    // AutoCompleteBox has optional PART_SelectionAdapter, that was never defined in templates for "historical reasons".
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

    let setters: Vec<(&'static str, SetterValue)> = theme
        .setters()
        .snapshot()
        .iter()
        .filter_map(|setter| {
            let setter = setter.as_any()?.downcast_ref::<Setter>()?;
            Some((setter.property()?.name(), setter.value()?))
        })
        .collect();
    let setter = |property: &str| setters.iter().find(|(name, _)| *name == property).map(|(_, value)| value.clone());

    // The template of the theme has been applied.
    if setter("Template").is_some() {
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
    // A nested style of the theme may set the property again for the state the control is in
    // (`^[TabStripPlacement=Top]`); only the properties the theme sets once are compared.
    let set_by_nested_styles: Vec<&'static str> = enumerate_styles(&theme.clone().into())
        .iter()
        .skip(1)
        .flat_map(|style| style.setters().snapshot().iter().cloned().collect::<Vec<_>>())
        .filter_map(|setter| setter.as_any()?.downcast_ref::<Setter>()?.property().map(|property| property.name()))
        .collect();
    let value_of = |property: &str| match setter(property) {
        Some(SetterValue::Value(value)) if !set_by_nested_styles.contains(&property) => Some(Some(value)),
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
    ($($test:ident => $name:literal $(, ignore = $reason:literal)?;)*) => {
        $(
            #[test]
            $(#[ignore = $reason])?
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
    button_spinner => "ButtonSpinner";
    calendar => "Calendar";
    calendar_button => "CalendarButton";
    calendar_date_picker => "CalendarDatePicker";
    calendar_day_button => "CalendarDayButton";
    calendar_item => "CalendarItem";
    carousel => "Carousel";
    carousel_page => "CarouselPage";
    check_box => "CheckBox";
    combo_box => "ComboBox";
    combo_box_item => "ComboBoxItem";
    command_bar => "CommandBar";
    command_bar_button => "CommandBarButton";
    command_bar_separator => "CommandBarSeparator";
    command_bar_toggle_button => "CommandBarToggleButton";
    content_page => "ContentPage";
    context_menu => "ContextMenu";
    data_validation_errors => "DataValidationErrors";
    date_picker => "DatePicker";
    date_picker_presenter => "DatePickerPresenter";
    drawer_page => "DrawerPage";
    drop_down_button => "DropDownButton";
    expander => "Expander";
    flyout_presenter => "FlyoutPresenter";
    grid_splitter => "GridSplitter";
    group_box => "GroupBox";
    headered_content_control => "HeaderedContentControl";
    hyperlink_button => "HyperlinkButton";
    items_control => "ItemsControl";
    label => "Label";
    list_box => "ListBox";
    list_box_item => "ListBoxItem";
    managed_file_chooser => "ManagedFileChooser";
    managed_file_chooser_overwrite_prompt => "ManagedFileChooserOverwritePrompt";
    menu => "Menu";
    menu_flyout_presenter => "MenuFlyoutPresenter";
    menu_item => "MenuItem";
    navigation_page => "NavigationPage";
    notification_card => "NotificationCard";
    numeric_up_down => "NumericUpDown";
    path_icon => "PathIcon";
    pips_pager => "PipsPager";
    progress_bar => "ProgressBar";
    radio_button => "RadioButton";
    refresh_container => "RefreshContainer";
    refresh_visualizer => "RefreshVisualizer";
    repeat_button => "RepeatButton";
    scroll_bar => "ScrollBar";
    scroll_viewer => "ScrollViewer";
    selectable_text_block => "SelectableTextBlock";
    separator => "Separator";
    slider => "Slider";
    split_button => "SplitButton";
    split_view => "SplitView";
    tab_control => "TabControl";
    tab_item => "TabItem";
    tab_strip => "TabStrip";
    tab_strip_item => "TabStripItem";
    tabbed_page => "TabbedPage";
    table_view => "TableView";
    table_view_cell => "TableViewCell";
    table_view_column_header => "TableViewColumnHeader";
    table_view_row => "TableViewRow";
    text_box => "TextBox";
    text_selection_handle => "TextSelectionHandle";
    theme_variant_scope => "ThemeVariantScope";
    time_picker => "TimePicker";
    time_picker_presenter => "TimePickerPresenter";
    toggle_button => "ToggleButton";
    toggle_switch => "ToggleSwitch";
    tool_tip => "ToolTip";
    transitioning_content_control => "TransitioningContentControl";
    tree_view => "TreeView";
    tree_view_item => "TreeViewItem";
    window_notification_manager => "WindowNotificationManager";
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
    // The other tests are the ignored ones of the documents that are left out.
    for name in TESTED.iter().filter(|name| !themed.contains(name)) {
        assert!(crate::assets::is_excluded(&format!("/Controls/{name}.xaml")), "{name} has a test but no control theme");
    }
}

/// The items controls of the command bar template present the command lists
/// of the bar: their items sources are bound to `VisiblePrimaryCommands` and
/// `OverflowItems` of the templated parent, plain properties that the
/// binding reads through the root object handle of the templated parent.
#[test]
fn command_bar_presents_its_command_lists() {
    use ferroui_controls::command_bar::{CommandBar, CommandBarButton};
    use ferroui_controls::ItemsControl;

    let _app = start_themed_application();
    let bar = CommandBar::new();
    let name_scope: Rc<RefCell<Option<NameScopeRef>>> = Rc::new(RefCell::new(None));
    let _ = bar.template_applied({
        let name_scope = name_scope.clone();
        move |_, e| *name_scope.borrow_mut() = Some(e.name_scope().clone())
    });
    let primary = CommandBarButton::new();
    bar.primary_commands().add(primary.as_command_bar_element());
    let secondary = CommandBarButton::new();
    bar.secondary_commands().add(secondary.as_command_bar_element());

    let window = Window::new();
    window.set_content(Some(Control::boxed(&bar)));
    window.show();

    let name_scope = name_scope.borrow().clone().expect("the template of the command bar was applied");
    let primary_host = name_scope.find_as::<ItemsControl>("PART_PrimaryCommands").expect("the primary commands host");
    let overflow_presenter =
        name_scope.find_as::<ItemsControl>("PART_OverflowPresenter").expect("the overflow presenter");

    assert_eq!(Some(bar.visible_primary_commands().as_items_source()), primary_host.items_source());
    assert_eq!(Some(bar.overflow_items().as_items_source()), overflow_presenter.items_source());
    assert_eq!(1, primary_host.item_count());
    assert_eq!(1, overflow_presenter.item_count());
    assert!(primary.get_visual_parent().is_some(), "the primary command is presented");
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
