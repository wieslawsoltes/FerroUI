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
