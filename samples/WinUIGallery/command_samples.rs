//! The code behind of the pinned AppBarButton, AppBarToggleButton and CommandBar pages in Rust:
//! the handlers of `AppBarButtonPage.xaml.cs`, `AppBarToggleButtonPage.xaml.cs` and
//! `CommandBarPage.xaml.cs`. WinUI's AppBarButton, AppBarToggleButton and AppBarSeparator are
//! FerroUI's CommandBarButton, CommandBarToggleButton and CommandBarSeparator, and a
//! KeyboardAccelerator is the button's HotKey. The XamlUICommand and StandardUICommand pages
//! (`XamlUICommandPage.xaml.cs`, `StandardUICommandPage.xaml.cs`) use the commands of
//! ferroui-controls. The AppBarSeparator page has no code-behind
//! upstream. The command bar owns its layout and overflow; the sample owns only its commands.

use ferroui_base::collections::FerroList;
use ferroui_base::controls::ResourceKey;
use ferroui_base::input::{
    ContextRequestedEventArgs, ICommand, InputElement, Key, KeyGesture, KeyModifiers, PointerEventArgs, PointerType,
};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Colors, IBrush, IImage};
use ferroui_base::{BoxedValue, Ref, Thickness, WeakRef};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::templates::FuncDataTemplate;
use ferroui_controls_icons::BitmapIcon;
use ferroui_controls::{
    Button, CommandBar, CommandBarButton, CommandBarFlyout, FlyoutShowMode, Image, PlacementMode, CommandBarSeparator, CommandBarToggleButton, Control, Grid, ListBox,
    MenuFlyout, MenuItem, StandardUICommand, StandardUICommandKind, SwipeControl, SwipeItem, SwipeItems, SwipeMode,
    Symbol, SymbolIcon, TextBlock, XamlUICommand,
};
use std::rc::Rc;

/// The bitmap icon of the source's `Button2`: `/Assets/SampleMedia/Slices2.png`.
const SLICES2: &str = "ferres://WinUIGallery/Assets/SampleMedia/Slices2.png";

pub(crate) fn wire(page_id: &str, root: &Ref<Control>) {
    match page_id {
        "AppBarButton" => wire_app_bar_button(root),
        "AppBarToggleButton" => wire_app_bar_toggle_button(root),
        "CommandBar" => wire_command_bar(root),
        "XamlUICommand" => wire_xaml_ui_command(root),
        "StandardUICommand" => wire_standard_ui_command(root),
        "CommandBarFlyout" => wire_command_bar_flyout(root),
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

/// `CustomXamlUICommand_ExecuteRequested` of `XamlUICommandPage.xaml.cs`: the command of the
/// AppBarButton writes its output, which a polite live region announces (the source's
/// `AnnounceActionForAccessibility`).
fn wire_xaml_ui_command(root: &Ref<Control>) {
    let button = root.get_control::<CommandBarButton>("CustomButton");
    let Some(command) = button.command().as_ref().and_then(XamlUICommand::from_command) else {
        return;
    };
    let output = root.get_control::<TextBlock>("XamlUICommandOutput").downgrade();
    command
        .execute_requested(move |_, _| {
            if let Some(output) = output.upgrade() {
                output.set_text(Some("You fired the custom command"));
            }
        })
        .detach();
}

/// The list of `StandardUICommandPage.xaml.cs`: the texts of its `ListItemData`.
pub(crate) struct StandardUICommandPage {
    collection: Rc<FerroList<BoxedValue>>,
    list: WeakRef<ListBox>,
}

impl StandardUICommandPage {
    /// `DeleteCommand_ExecuteRequested`: the item of the parameter goes, or else the selected one.
    fn delete(&self, parameter: Option<&BoxedValue>) {
        if let Some(text) = parameter.and_then(|parameter| parameter.downcast_ref::<String>()) {
            let items = self.collection.snapshot();
            if let Some(index) = items.iter().position(|item| item.downcast_ref::<String>() == Some(text)) {
                self.collection.remove_at(index);
                return;
            }
        }
        let selected = self.list.upgrade().map_or(-1, |list| list.selected_index());
        if let Ok(index) = usize::try_from(selected) {
            self.collection.remove_at(index);
        }
    }
}

/// The item template of the list (the source's `UserControl` with its `SwipeControl`): the text,
/// the execute SwipeItem of the right side and the AppBarButton shown while a mouse or a pen is
/// over the item, all with the command of the item and its text as the parameter; the context
/// menu of `ListViewRight_ContainerContentChanging` has a MenuFlyoutItem of the command and
/// selects the item when it opens.
fn standard_ui_command_item(list: &WeakRef<ListBox>, data: &BoxedValue, command: &Rc<dyn ICommand>) -> Ref<Control> {
    let text = data.downcast_ref::<String>().cloned().unwrap_or_default();
    let parameter: BoxedValue = Rc::new(text.clone());
    let swipe_item = SwipeItem::new();
    swipe_item.set_background(Some(Rc::new(ImmutableSolidColorBrush::new(Colors::RED)) as Rc<dyn IBrush>));
    swipe_item.set_command(Some(command.clone()));
    swipe_item.set_command_parameter(Some(parameter.clone()));
    let right_items = SwipeItems::with_mode(SwipeMode::Execute);
    right_items.add(swipe_item);

    let label = TextBlock::new();
    label.set_margin(Thickness::uniform(10.0));
    label.set_horizontal_alignment(HorizontalAlignment::Left);
    label.set_vertical_alignment(VerticalAlignment::Center);
    label.set_font_size(18.0);
    label.set_text(Some(&text));
    let hover_button = CommandBarButton::new();
    hover_button.set_name(Some("HoverButton".to_string()));
    hover_button.set_horizontal_alignment(HorizontalAlignment::Right);
    hover_button.set_command(Some(command.clone()));
    hover_button.set_command_parameter(Some(parameter));
    hover_button.set_is_tab_stop(false);
    hover_button.set_is_visible(false);
    let content = Grid::new();
    content.set_vertical_alignment(VerticalAlignment::Center);
    content.children().add(label.upcast::<Control>());
    content.children().add(hover_button.clone().upcast::<Control>());

    let swipe = SwipeControl::new();
    swipe.set_name(Some("ListViewSwipeContainer".to_string()));
    swipe.set_right_items(Some(right_items));
    swipe.set_content(Some(Control::boxed(content)));
    let root = Grid::new();
    AutomationProperties::set_name(&root, Some(&text));
    root.children().add(swipe.upcast::<Control>());
    // ListViewSwipeContainer_PointerEntered and _PointerExited: HoverButtonsShown for a mouse or a
    // pen, HoverButtonsHidden when the pointer leaves.
    let shown = hover_button.downgrade();
    root.add_handler(InputElement::pointer_entered_event(), move |_, e: &PointerEventArgs| {
        if matches!(e.pointer().type_(), PointerType::Mouse | PointerType::Pen) {
            if let Some(button) = shown.upgrade() {
                button.set_is_visible(true);
            }
        }
    });
    let hidden = hover_button.downgrade();
    root.add_handler(InputElement::pointer_exited_event(), move |_, _: &PointerEventArgs| {
        if let Some(button) = hidden.upgrade() {
            button.set_is_visible(false);
        }
    });

    let item = MenuItem::new();
    item.set_command(Some(command.clone()));
    let flyout = MenuFlyout::new();
    flyout.items().add(Some(Control::boxed(item)));
    let list = list.clone();
    let selected = data.clone();
    flyout
        .opened(move || {
            if let Some(list) = list.upgrade() {
                list.set_selected_item(Some(selected.clone()));
            }
        })
        .detach();
    root.set_context_flyout(flyout);
    root.upcast()
}

/// `ControlExample_Loaded` and `ListView_Loaded` of `StandardUICommandPage.xaml.cs`: the Delete
/// command (`StandardUICommand(StandardUICommandKind.Delete)`) with its `ExecuteRequested`
/// handler, set on the Edit menu's item and on the 15 items of the list.
fn wire_standard_ui_command(root: &Ref<Control>) {
    let list = root.get_control::<ListBox>("ListViewRight");
    let page = Rc::new(StandardUICommandPage { collection: Rc::new(FerroList::new()), list: list.downgrade() });
    let delete_command = StandardUICommand::with_kind(StandardUICommandKind::Delete);
    let handler_page = page.clone();
    delete_command.execute_requested(move |_, args| handler_page.delete(args.parameter().as_ref())).detach();
    let command = delete_command.as_command();
    root.get_control::<MenuItem>("DeleteFlyoutItem").set_command(Some(command.clone()));
    for index in 0..15 {
        page.collection.add(Rc::new(format!("List item {index}")) as BoxedValue);
    }
    let weak_list = list.downgrade();
    list.set_item_template(Some(FuncDataTemplate::new(
        |_| true,
        move |data, _| data.as_ref().map(|data| standard_ui_command_item(&weak_list, data, &command)),
        false,
    )));
    list.set_items_source(Some(page.collection.clone().into()));
}

/// The image of the source's `Image1`: `/Assets/SampleMedia/rainier.jpg`.
const RAINIER: &str = "ferres://WinUIGallery/Assets/SampleMedia/rainier.jpg";

/// The code behind of `CommandBarFlyoutPage.xaml.cs`: `OnElementClicked` (the clicked button's
/// label), `MyImageButton_Click` (the flyout transient) and `MyImageButton_ContextRequested` (the
/// flyout standard), both through `ShowMenu` at RightEdgeAlignedTop of the image.
fn wire_command_bar_flyout(root: &Ref<Control>) {
    let image = root.get_control::<Image>("Image1");
    if let Ok(bitmap) = crate::assets::load_bitmap(RAINIER) {
        let source: Rc<dyn IImage> = Rc::new(bitmap);
        image.set_source(Some(source));
    }
    let Some(flyout) = root
        .try_find_resource(&ResourceKey::from("CommandBarFlyout1"), None)
        .flatten()
        .and_then(|value| value.downcast_ref::<Ref<CommandBarFlyout>>().cloned())
    else {
        return;
    };
    let output = root.get_control::<TextBlock>("SelectedOptionText");
    for element in flyout.primary_commands().snapshot().iter().chain(flyout.secondary_commands().snapshot().iter()) {
        let Some(button) = element.as_object().and_then(|object| object.to_ref().cast::<CommandBarButton>()) else {
            continue;
        };
        let weak_button = button.downgrade();
        let weak_output = output.downgrade();
        button.click(move |_, _| {
            if let (Some(button), Some(output)) = (weak_button.upgrade(), weak_output.upgrade()) {
                output.set_text(Some(&format!("You clicked: {}", button.label().unwrap_or_default())));
            }
        });
    }
    let image_button = root.get_control::<Button>("myImageButton");
    let show = move |flyout: &Ref<CommandBarFlyout>, image: &Ref<Image>, transient: bool| {
        flyout.set_show_mode(if transient { FlyoutShowMode::Transient } else { FlyoutShowMode::Standard });
        flyout.set_placement(PlacementMode::RightEdgeAlignedTop);
        flyout.show_at(image);
    };
    let (click_flyout, click_image) = (flyout.clone(), image.downgrade());
    image_button.click(move |_, _| {
        if let Some(image) = click_image.upgrade() {
            show(&click_flyout, &image, true);
        }
    });
    let context_image = image.downgrade();
    image_button.add_handler(InputElement::context_requested_event(), move |_, e: &ContextRequestedEventArgs| {
        if let Some(image) = context_image.upgrade() {
            show(&flyout, &image, false);
            e.set_handled(true);
        }
    });
}
