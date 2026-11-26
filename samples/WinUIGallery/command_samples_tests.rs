//! The AppBarButton, AppBarToggleButton, AppBarSeparator and CommandBar pages: the source's
//! examples with the handlers of their code behind (`command_samples.rs`).

use crate::command_samples::nullable_bool_text;
use crate::{examples, tests::start};
use ferroui_base::input::{Key, KeyGesture, KeyModifiers};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::Ref;
use ferroui_controls::{
    Button, CommandBar, CommandBarButton, CommandBarSeparator, CommandBarToggleButton, Control,
    TextBlock,
};

fn page(id: &str) -> Ref<Control> {
    examples::create(examples::find(id).unwrap()).unwrap_or_else(|error| panic!("{id}: {error}"))
}

fn click(button: &Button) {
    button.raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
}

fn text(root: &Ref<Control>, name: &str) -> Option<String> {
    root.get_control::<TextBlock>(name).text()
}

#[test]
fn app_bar_buttons_write_the_clicked_name_to_their_outputs() {
    let _application = start();
    let root = page("AppBarButton");
    for index in 1..=5 {
        assert_eq!(text(&root, &format!("Control{index}Output")), None);
        click(&root.get_control::<CommandBarButton>(&format!("Button{index}")));
        assert_eq!(
            text(&root, &format!("Control{index}Output")).as_deref(),
            Some(format!("You clicked: Button{index}").as_str())
        );
    }
    // The KeyboardAccelerator is the HotKey; the edit button opens its flyout.
    assert_eq!(
        root.get_control::<CommandBarButton>("Button5").hot_key(),
        Some(KeyGesture::new(Key::S, KeyModifiers::CONTROL))
    );
    assert!(root
        .get_control::<CommandBarButton>("EditButton")
        .flyout()
        .is_some());
}

#[test]
fn app_bar_toggle_buttons_state_is_checked_after_the_click() {
    let _application = start();
    let root = page("AppBarToggleButton");
    for index in 1..=3 {
        let button = root.get_control::<CommandBarToggleButton>(&format!("Button{index}"));
        button.set_is_checked(Some(true));
        click(&button);
        assert_eq!(
            text(&root, &format!("Control{index}Output")).as_deref(),
            Some("IsChecked = True")
        );
    }
    let three_state = root.get_control::<CommandBarToggleButton>("Button4");
    assert!(three_state.is_three_state());
    three_state.set_is_checked(None);
    click(&three_state);
    assert_eq!(
        text(&root, "Control4Output").as_deref(),
        Some("IsChecked = ")
    );
    three_state.set_is_checked(Some(false));
    click(&three_state);
    assert_eq!(
        text(&root, "Control4Output").as_deref(),
        Some("IsChecked = False")
    );
    assert_eq!(nullable_bool_text(Some(true)), "True");
}

#[test]
fn app_bar_separators_divide_the_primary_commands() {
    let _application = start();
    let root = page("AppBarSeparator");
    let commands = root
        .get_control::<CommandBar>("Control1")
        .primary_commands()
        .to_vec();
    assert_eq!(commands.len(), 6);
    let separators: Vec<bool> = commands
        .iter()
        .map(|command| {
            command
                .as_object()
                .is_some_and(|object| object.is::<CommandBarSeparator>())
        })
        .collect();
    assert_eq!(separators, [false, true, false, false, true, false]);
}

#[test]
fn command_bar_options_open_close_and_change_the_secondary_commands() {
    let _application = start();
    let root = page("CommandBar");
    let bar = root.get_control::<CommandBar>("PrimaryCommandBar");
    // OnElementClicked writes the label of the clicked command.
    click(&root.get_control::<CommandBarButton>("editButton"));
    assert_eq!(
        text(&root, "SelectedOptionText").as_deref(),
        Some("You clicked: Edit")
    );
    click(&root.get_control::<CommandBarButton>("settingsButton"));
    assert_eq!(
        text(&root, "SelectedOptionText").as_deref(),
        Some("You clicked: Settings")
    );
    // The KeyboardAccelerators of the constructor are the HotKeys.
    assert_eq!(
        root.get_control::<CommandBarButton>("shareButton")
            .hot_key(),
        Some(KeyGesture::from_key(Key::F4))
    );
    assert_eq!(
        root.get_control::<CommandBarButton>("addButton").hot_key(),
        Some(KeyGesture::new(Key::A, KeyModifiers::CONTROL))
    );
    click(&root.get_control::<Button>("OpenButton"));
    assert!(bar.is_open() && bar.is_sticky());
    click(&root.get_control::<Button>("CloseButton"));
    assert!(!bar.is_open() && !bar.is_sticky());
    assert_eq!(bar.secondary_commands().count(), 1);
    click(&root.get_control::<Button>("AddSecondaryCommandsButton"));
    assert_eq!(bar.secondary_commands().count(), 6);
    // A second click adds nothing: the commands are added while Settings is alone.
    click(&root.get_control::<Button>("AddSecondaryCommandsButton"));
    assert_eq!(bar.secondary_commands().count(), 6);
    click(&root.get_control::<Button>("RemoveSecondaryCommandsButton"));
    assert_eq!(bar.secondary_commands().count(), 1);
}

/// The XamlUICommand page: the AppBarButton takes the label, icon, accelerator and description of
/// the page's command, and running it writes the output of `CustomXamlUICommand_ExecuteRequested`.
#[test]
fn xaml_ui_command_gives_the_button_its_ui_and_writes_the_output() {
    use ferroui_controls::{SymbolIcon, ToolTip, XamlUICommand};
    let _application = start();
    let root = page("XamlUICommand");
    let button = root.get_control::<CommandBarButton>("CustomButton");
    assert_eq!(button.label().as_deref(), Some("Custom XamlUICommand"));
    assert_eq!(button.hot_key(), Some(KeyGesture::new(Key::D, KeyModifiers::CONTROL)));
    let icon = button.icon().as_ref().and_then(Control::from_boxed).and_then(|icon| icon.cast::<SymbolIcon>());
    assert_eq!(icon.map(|icon| icon.symbol()), Some(ferroui_controls::Symbol::Favorite));
    let tip = ToolTip::get_tip(&button).and_then(|tip| tip.downcast_ref::<String>().cloned());
    assert_eq!(tip.as_deref(), Some("This is a custom command"));
    assert!(button.command().as_ref().and_then(XamlUICommand::from_command).is_some());
    assert_eq!(text(&root, "XamlUICommandOutput"), None);
    // A click runs the command of the button (Button.OnClick).
    button.on_click();
    assert_eq!(text(&root, "XamlUICommandOutput").as_deref(), Some("You fired the custom command"));
}

/// The StandardUICommand page: the Delete command gives the Edit menu's item its label and
/// accelerator, and deletes the item of its parameter (the swipe item and the hover button), or
/// else the selected item (the menu item).
#[test]
fn standard_ui_command_deletes_the_item_of_the_parameter_or_the_selection() {
    use ferroui_base::BoxedValue;
    use ferroui_controls::{ListBox, MenuItem};
    use std::rc::Rc;
    let _application = start();
    let root = page("StandardUICommand");
    let menu_item = root.get_control::<MenuItem>("DeleteFlyoutItem");
    let header = menu_item.header().and_then(|header| header.downcast_ref::<String>().cloned());
    assert_eq!(header.as_deref(), Some("Delete"));
    assert_eq!(menu_item.input_gesture(), Some(KeyGesture::from_key(Key::Delete)));
    let list = root.get_control::<ListBox>("ListViewRight");
    let items = || -> Vec<String> {
        let view = list.items().view().clone();
        (0..view.count()).filter_map(|index| view.get_at(index)?.downcast_ref::<String>().cloned()).collect()
    };
    assert_eq!(items().len(), 15);
    let command = menu_item.command().expect("the delete command");
    let parameter: BoxedValue = Rc::new("List item 3".to_string());
    command.execute(Some(&parameter));
    assert_eq!(items().len(), 14);
    assert!(!items().contains(&"List item 3".to_string()));
    list.set_selected_index(0);
    command.execute(None);
    assert_eq!(items().first().map(String::as_str), Some("List item 1"));
}

/// The CommandBarFlyout page: a click of the image opens the flyout transient (the bar stays
/// closed), a context request opens it standard (the bar opens with it), both at
/// RightEdgeAlignedTop; a click of a command writes its label, and a secondary command closes the
/// flyout.
#[test]
fn command_bar_flyout_opens_transient_on_click_and_standard_on_context_request() {
    use ferroui_base::controls::ResourceKey;
    use ferroui_base::input::{ContextRequestedEventArgs, InputElement};
    use ferroui_base::threading::Dispatcher;
    use ferroui_controls::{CommandBarFlyout, FlyoutShowMode, PlacementMode, Window};
    let _application = start();
    let root = page("CommandBarFlyout");
    let window = Window::new();
    window.set_content(Some(Control::boxed(root.clone())));
    window.show();
    window.update_layout();
    let flyout = root
        .try_find_resource(&ResourceKey::from("CommandBarFlyout1"), None)
        .flatten()
        .and_then(|value| value.downcast_ref::<Ref<CommandBarFlyout>>().cloned())
        .expect("the flyout resource");
    assert_eq!(flyout.primary_commands().count(), 3);
    assert_eq!(flyout.secondary_commands().count(), 2);
    let image_button = root.get_control::<Button>("myImageButton");
    click(&image_button);
    Dispatcher::ui_thread().run_jobs(None);
    assert!(flyout.is_open());
    assert_eq!(flyout.show_mode(), FlyoutShowMode::Transient);
    assert_eq!(flyout.placement(), PlacementMode::RightEdgeAlignedTop);
    let bar = flyout.command_bar().expect("the bar");
    assert!(!bar.is_open());
    let command = |list: &ferroui_controls::CommandBarElementList, index: usize| {
        list.get(index).as_object().and_then(|object| object.to_ref().cast::<Button>()).expect("a button")
    };
    click(&command(&flyout.primary_commands(), 1));
    assert_eq!(text(&root, "SelectedOptionText").as_deref(), Some("You clicked: Save"));
    assert!(flyout.is_open());
    flyout.hide();

    let mut args = ContextRequestedEventArgs::new();
    args.set_routed_event(Some(InputElement::context_requested_event()));
    image_button.raise_event(&args);
    Dispatcher::ui_thread().run_jobs(None);
    assert!(flyout.is_open());
    assert_eq!(flyout.show_mode(), FlyoutShowMode::Standard);
    assert!(bar.is_open());
    click(&command(&flyout.secondary_commands(), 0));
    assert_eq!(text(&root, "SelectedOptionText").as_deref(), Some("You clicked: Resize"));
    assert!(!flyout.is_open());
    window.close();
}
