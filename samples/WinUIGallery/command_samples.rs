//! The code behind of the pinned AppBarButton, AppBarToggleButton and CommandBar pages in Rust:
//! the handlers of `AppBarButtonPage.xaml.cs`, `AppBarToggleButtonPage.xaml.cs` and
//! `CommandBarPage.xaml.cs`. WinUI's AppBarButton, AppBarToggleButton and AppBarSeparator are
//! FerroUI's CommandBarButton, CommandBarToggleButton and CommandBarSeparator, and a
//! KeyboardAccelerator is the button's HotKey. The AppBarSeparator page has no code-behind
//! upstream. The command bar owns its layout and overflow; the sample owns only its commands.

use ferroui_base::input::{Key, KeyGesture, KeyModifiers};
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls_icons::BitmapIcon;
use ferroui_controls::{
    Button, CommandBar, CommandBarButton, CommandBarSeparator, CommandBarToggleButton, Control,
    Image, Symbol, SymbolIcon, TextBlock,
};
use std::rc::Rc;

/// The bitmap icon of the source's `Button2`: `/Assets/SampleMedia/Slices2.png`.
const SLICES2: &str = "ferres://WinUIGallery/Assets/SampleMedia/Slices2.png";

pub(crate) fn wire(page_id: &str, root: &Ref<Control>) {
    match page_id {
        "AppBarButton" => wire_app_bar_button(root),
        "AppBarToggleButton" => wire_app_bar_toggle_button(root),
        "CommandBar" => wire_command_bar(root),
        _ => {}
    }
}

/// The source's `BitmapIcon` of `Button2`, with its `UriSource` Slices2.png, in the foreground's
/// single color (the default of `ShowAsMonochrome`).
fn load_bitmap_icon(root: &Ref<Control>) {
    let icon = root.get_control::<BitmapIcon>("Button2Icon");
    match crate::assets::load_bitmap(SLICES2) {
        Ok(bitmap) => icon.set_source(Some(Rc::new(bitmap))),
        Err(error) => {
            let diagnostic = root.get_control::<TextBlock>("Button2IconError");
            diagnostic.set_text(Some(&format!("Slices2.png: {error}")));
            diagnostic.set_is_visible(true);
        }
    }
}

/// `AppBarButton_Click` of `AppBarButtonPage.xaml.cs`: the clicked button's name is written to its
/// output ("You clicked: Button1"), which a polite live region announces.
fn wire_app_bar_button(root: &Ref<Control>) {
    load_bitmap_icon(root);
    for index in 1..=5 {
        let output = root
            .get_control::<TextBlock>(&format!("Control{index}Output"))
            .downgrade();
        root.get_control::<CommandBarButton>(&format!("Button{index}"))
            .click(move |sender, _| {
                let (Some(name), Some(output)) = (sender.name(), output.upgrade()) else {
                    return;
                };
                output.set_text(Some(&format!("You clicked: {name}")));
            });
    }
}

/// The text of C#'s `bool?.ToString()`: `True`, `False`, or empty for no value.
pub(crate) fn nullable_bool_text(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "True",
        Some(false) => "False",
        None => "",
    }
}

/// `AppBarButton_Click` of `AppBarToggleButtonPage.xaml.cs`: after the click toggles the button,
/// its output states `IsChecked`.
fn wire_app_bar_toggle_button(root: &Ref<Control>) {
    load_bitmap_icon(root);
    for index in 1..=4 {
        let output = root
            .get_control::<TextBlock>(&format!("Control{index}Output"))
            .downgrade();
        let button = root.get_control::<CommandBarToggleButton>(&format!("Button{index}"));
        let weak_button = button.downgrade();
        button.click(move |_, _| {
            let (Some(button), Some(output)) = (weak_button.upgrade(), output.upgrade()) else {
                return;
            };
            output.set_text(Some(&format!(
                "IsChecked = {}",
                nullable_bool_text(button.is_checked())
            )));
        });
    }
}

/// A command of the bar with a symbol icon and an optional hot key.
fn command(symbol: Symbol, label: &str, hot_key: Option<KeyGesture>) -> Ref<CommandBarButton> {
    let button = CommandBarButton::new();
    button.set_icon(Some(
        Control::boxed(SymbolIcon::with_symbol(symbol)) as BoxedValue
    ));
    button.set_label(Some(label));
    button.set_hot_key(hot_key);
    button
}

/// `AddSecondaryCommands_Click` of `CommandBarPage.xaml.cs`: while the bar has only its Settings
/// command, four buttons and a separator join the secondary commands.
pub(crate) fn add_secondary_commands(bar: &CommandBar) {
    let commands = bar.secondary_commands();
    if commands.count() != 1 {
        return;
    }
    let control = KeyModifiers::CONTROL;
    commands.add(
        command(
            Symbol::Add,
            "Button 1",
            Some(KeyGesture::new(Key::N, control)),
        )
        .as_command_bar_element(),
    );
    commands.add(
        command(
            Symbol::Delete,
            "Button 2",
            Some(KeyGesture::from_key(Key::Delete)),
        )
        .as_command_bar_element(),
    );
    commands.add(CommandBarSeparator::new().as_command_bar_element());
    commands.add(
        command(
            Symbol::FontDecrease,
            "Button 3",
            Some(KeyGesture::new(Key::Subtract, control)),
        )
        .as_command_bar_element(),
    );
    commands.add(
        command(
            Symbol::FontIncrease,
            "Button 4",
            Some(KeyGesture::new(Key::Add, control)),
        )
        .as_command_bar_element(),
    );
}

/// `RemoveSecondaryCommands` of `CommandBarPage.xaml.cs`: the secondary commands go back to
/// Settings alone.
pub(crate) fn remove_secondary_commands(bar: &CommandBar) {
    let commands = bar.secondary_commands();
    while commands.count() > 1 {
        commands.remove_at(commands.count() - 1);
    }
}

/// The code behind of `CommandBarPage.xaml.cs`: the constructor's `AddKeyboardAccelerators` (Add
/// Ctrl+A, Edit Ctrl+E, Share F4; Settings has Ctrl+I in the markup), `OnElementClicked` (the
/// clicked command's label), the Open and Close buttons (IsOpen with IsSticky) and the buttons
/// that add and remove the secondary commands. The source removes them again when the page is
/// left (`OnNavigatingFrom`); the gallery creates the page anew each time it is shown.
fn wire_command_bar(root: &Ref<Control>) {
    let control = KeyModifiers::CONTROL;
    for (name, gesture) in [
        ("editButton", KeyGesture::new(Key::E, control)),
        ("shareButton", KeyGesture::from_key(Key::F4)),
        ("addButton", KeyGesture::new(Key::A, control)),
    ] {
        root.get_control::<CommandBarButton>(name)
            .set_hot_key(Some(gesture));
    }
    let output = root.get_control::<TextBlock>("SelectedOptionText");
    for name in ["addButton", "editButton", "shareButton", "settingsButton"] {
        let button = root.get_control::<CommandBarButton>(name);
        let weak_button = button.downgrade();
        let weak_output = output.downgrade();
        button.click(move |_, _| {
            if let (Some(button), Some(output)) = (weak_button.upgrade(), weak_output.upgrade()) {
                output.set_text(Some(&format!(
                    "You clicked: {}",
                    button.label().unwrap_or_default()
                )));
            }
        });
    }
    let bar = root.get_control::<CommandBar>("PrimaryCommandBar");
    let on_click = |name: &str, action: fn(&CommandBar)| {
        let weak_bar = bar.downgrade();
        root.get_control::<Button>(name).click(move |_, _| {
            if let Some(bar) = weak_bar.upgrade() {
                action(&bar);
            }
        });
    };
    on_click("OpenButton", |bar| {
        bar.set_is_open(true);
        bar.set_is_sticky(true);
    });
    on_click("CloseButton", |bar| {
        bar.set_is_open(false);
        bar.set_is_sticky(false);
    });
    on_click("AddSecondaryCommandsButton", add_secondary_commands);
    on_click("RemoveSecondaryCommandsButton", remove_secondary_commands);
}
