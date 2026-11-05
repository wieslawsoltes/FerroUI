//! The code behind of the pinned RichEditBox and RichTextBlock pages in Rust: the custom
//! context menu command, the formatting editor (open and save as RTF, bold, italic, font
//! color, find with highlighting) and the text highlighter of the rich text block. The
//! controls own editing, layout, selection and overflow; the sample owns only its UI behavior.

use ferroui_base::controls::ResourceKey;
use ferroui_base::input::{FocusChangedEventArgs, InputElement};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Color, Colors, IBrush};
use ferroui_base::platform::storage::{
    FilePickerFileType, FilePickerOpenOptions, FilePickerSaveOptions, IStorageFile, IStorageProvider,
    WellKnownFolder,
};
use ferroui_base::reactive::Subscription;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{ObjectType, Ref, WeakRef};
use ferroui_controls::documents::{CharacterFormat, TextHighlightRange, TextHighlighter};
use ferroui_controls::primitives::{PopupFlyoutBase, SelectingItemsControl};
use ferroui_controls::rich_text::FindOptions;
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::{
    Button, ComboBox, ComboBoxItem, Control, DropDownButton, MenuFlyout, MenuItem, RichEditBox,
    RichTextBlock, TextBlock, TextBox, TopLevel,
};
use std::cell::{Cell, RefCell};
use std::io::{Read, Write};
use std::rc::Rc;

/// `TextConstants.MaxUnitCount` of the text object model: the scan length that searches the
/// rest of the story.
const MAX_UNIT_COUNT: i32 = 1_073_741_823;

/// The header of the command `Menu_Opening` adds to the menu of `REBCustom`
/// (`StandardUICommandKind.Share`).
pub(crate) const SHARE_COMMAND_HEADER: &str = "Share";

pub(crate) fn wire(root: &Ref<Control>) {
    wire_custom_menu(root);
    wire_custom_editor(root);
    wire_text_highlighting(root);
}

fn find<T: ObjectType>(root: &Ref<Control>, name: &str) -> Option<Ref<T>> {
    root.find_control::<Control>(name)
        .and_then(|control| control.cast())
}

fn solid(color: Color) -> Rc<dyn IBrush> {
    Rc::new(ImmutableSolidColorBrush::new(color))
}

fn brush_color(brush: &Rc<dyn IBrush>) -> Option<Color> {
    brush.as_solid_color_brush().map(|brush| brush.color())
}

/// A color resource of the application or its theme (`App.Current.Resources[key]`), or
/// `fallback` where the theme has none.
fn color_resource(element: &Control, key: &str, fallback: Color) -> Color {
    element
        .try_find_resource(&ResourceKey::from(key), element.actual_theme_variant().as_ref())
        .flatten()
        .and_then(|value| value.downcast_ref::<Color>().copied())
        .unwrap_or(fallback)
}

// --- Customizing the RichEditBox's context menu --------------------------------------------

/// `REBCustom_Loaded`, `REBCustom_Unloaded` and `Menu_Opening`: while the box has a context
/// menu, its `Opening` adds the Share command when the menu opens for `REBCustom`. The menu of
/// the theme is shared by the editors of the page, so the command is removed again when the
/// menu opens for another editor, and it is added once rather than at every opening.
fn wire_custom_menu(root: &Ref<Control>) {
    let Some(custom) = find::<RichEditBox>(root, "REBCustom") else {
        return;
    };
    let share = MenuItem::new();
    share.set_header(Some(Rc::new(SHARE_COMMAND_HEADER.to_string())));
    let opening: Rc<RefCell<Option<Subscription>>> = Rc::new(RefCell::new(None));
    let subscribe = {
        let weak_custom = custom.downgrade();
        let opening = opening.clone();
        Rc::new(move || {
            // Dropping the previous guard unsubscribes from the previous menu.
            let subscription = weak_custom.upgrade().and_then(|custom| {
                let flyout = custom.context_flyout()?;
                let popup = flyout.cast::<PopupFlyoutBase>()?;
                let weak_flyout = flyout.downgrade();
                let weak_custom = custom.downgrade();
                let share = share.clone();
                Some(popup.opening(move |_| {
                    let (Some(flyout), Some(custom)) = (weak_flyout.upgrade(), weak_custom.upgrade()) else {
                        return;
                    };
                    let Some(menu) = flyout.cast::<MenuFlyout>() else {
                        return;
                    };
                    let for_custom = flyout
                        .target()
                        .and_then(|target| target.cast::<RichEditBox>())
                        .is_some_and(|target| target == custom);
                    let items = menu.items();
                    let index = (0..items.count()).find(|&index| {
                        items
                            .get_at(index)
                            .and_then(|item| Control::from_boxed(&item))
                            .and_then(|item| item.cast::<MenuItem>())
                            .is_some_and(|item| item == share)
                    });
                    match (for_custom, index) {
                        (true, None) => {
                            items.add(Some(Control::boxed(share.clone())));
                        }
                        (false, Some(index)) => items.remove_at(index),
                        _ => {}
                    }
                }))
            });
            *opening.borrow_mut() = subscription;
        })
    };
    subscribe();
    custom.property_changed(move |change| {
        if change.property() == Control::context_flyout_property().as_property() {
            subscribe();
        }
    });
}

// --- A RichEditBox with formatting options -------------------------------------------------

fn wire_custom_editor(root: &Ref<Control>) {
    let Some(editor) = find::<RichEditBox>(root, "editor") else {
        return;
    };
    // `currentColor`: the font color the color buttons chose, green at first.
    let current_color = Rc::new(Cell::new(Colors::GREEN));

    let weak_editor = editor.downgrade();
    root.get_control::<Button>("boldButton").click(move |_, _| {
        if let Some(editor) = weak_editor.upgrade() {
            let _ = editor.toggle_bold();
        }
    });
    let weak_editor = editor.downgrade();
    root.get_control::<Button>("italicButton").click(move |_, _| {
        if let Some(editor) = weak_editor.upgrade() {
            let _ = editor.toggle_italic();
        }
    });

    let font_color_button = root.get_control::<DropDownButton>("fontColorButton");
    for name in [
        "RedColorButton",
        "OrangeColorButton",
        "YellowColorButton",
        "GreenColorButton",
        "BlueColorButton",
        "IndigoColorButton",
        "VioletColorButton",
        "GrayColorButton",
    ] {
        let button = root.get_control::<Button>(name);
        let weak_button = button.downgrade();
        let weak_editor = editor.downgrade();
        let weak_drop_down = font_color_button.downgrade();
        let current_color = current_color.clone();
        button.click(move |_, _| {
            let (Some(button), Some(editor)) = (weak_button.upgrade(), weak_editor.upgrade()) else {
                return;
            };
            // Extract the color of the button that was clicked.
            let Some(color) = button
                .content()
                .and_then(|content| Control::from_boxed(&content))
                .and_then(|content| content.cast::<Rectangle>())
                .and_then(|rectangle| rectangle.fill())
                .and_then(|fill| brush_color(&fill))
            else {
                return;
            };
            let _ = editor.text_document().selection().set_character_format(&CharacterFormat {
                foreground: Some(Some(solid(color))),
                ..CharacterFormat::default()
            });
            if let Some(flyout) = weak_drop_down.upgrade().and_then(|drop_down| drop_down.flyout()) {
                flyout.set_is_open(false);
            }
            editor.focus();
            current_color.set(color);
        });
    }

    // `Editor_TextChanged`: new text takes the current font color.
    let weak_editor = editor.downgrade();
    let color = current_color.clone();
    editor.text_changed(move |_, _| {
        let Some(editor) = weak_editor.upgrade() else {
            return;
        };
        let selection = editor.text_document().selection();
        let foreground = selection
            .character_format()
            .ok()
            .and_then(|format| format.foreground.flatten())
            .and_then(|brush| brush_color(&brush));
        if foreground != Some(color.get()) {
            let _ = selection.set_character_format(&CharacterFormat {
                foreground: Some(Some(solid(color.get()))),
                ..CharacterFormat::default()
            });
        }
    });

    // `Editor_GotFocus`: the focused editor drops the background of the find highlights (WinUI
    // writes the TextControlBackgroundFocused color; clearing the background is the same here,
    // the background of the box shows through).
    let weak_editor = editor.downgrade();
    editor.add_handler(InputElement::got_focus_event(), move |_, _: &FocusChangedEventArgs| {
        if let Some(editor) = weak_editor.upgrade() {
            if let Ok(range) = editor.text_document().document_range() {
                let _ = range.set_character_format(&CharacterFormat {
                    background: Some(None),
                    ..CharacterFormat::default()
                });
            }
        }
    });

    wire_find_box(root, &editor);
    wire_files(root, &editor);
}

/// `FindBoxRemoveHighlights`: the whole document goes back to the colors of the box.
pub(crate) fn remove_highlights(editor: &RichEditBox) {
    if let Ok(range) = editor.text_document().document_range() {
        let _ = range.set_character_format(&CharacterFormat {
            background: Some(None),
            foreground: Some(None),
            ..CharacterFormat::default()
        });
    }
}

/// `FindBoxHighlightMatches`: every match of `text` gets the highlight colors of the system.
pub(crate) fn highlight_matches(editor: &RichEditBox, text: &str) {
    remove_highlights(editor);
    let background = color_resource(editor, "SystemColorHighlightColor", Color::from_rgb(0x00, 0x78, 0xD7));
    let foreground = color_resource(editor, "SystemColorHighlightTextColor", Colors::WHITE);
    let format = CharacterFormat {
        background: Some(Some(solid(background))),
        foreground: Some(Some(solid(foreground))),
        ..CharacterFormat::default()
    };
    let document = editor.text_document();
    let Ok(search) = document.get_range(0, 0) else {
        return;
    };
    // Each search starts after the previous match; the bound only guards the loop.
    for _ in 0..=document.text_length() {
        match search.find_text_with_length(text, MAX_UNIT_COUNT, FindOptions::NONE) {
            Ok(length) if length > 0 => {
                let _ = search.set_character_format(&format);
            }
            _ => break,
        }
    }
}

fn wire_find_box(root: &Ref<Control>, editor: &Ref<RichEditBox>) {
    let find_box = root.get_control::<TextBox>("findBox");
    let highlight = {
        let weak_editor = editor.downgrade();
        let weak_find_box = find_box.downgrade();
        Rc::new(move || {
            if let (Some(editor), Some(find_box)) = (weak_editor.upgrade(), weak_find_box.upgrade()) {
                highlight_matches(&editor, &find_box.text().unwrap_or_default());
            }
        })
    };
    let on_focus = highlight.clone();
    find_box.add_handler(InputElement::got_focus_event(), move |_, _: &FocusChangedEventArgs| on_focus());
    let weak_editor = editor.downgrade();
    find_box.add_handler(InputElement::lost_focus_event(), move |_, _: &FocusChangedEventArgs| {
        if let Some(editor) = weak_editor.upgrade() {
            remove_highlights(&editor);
        }
    });
    find_box.property_changed(move |change| {
        if change.property() == TextBox::text_property().as_property() {
            highlight();
        }
    });
}

fn rich_text_file_type() -> Rc<FilePickerFileType> {
    FilePickerFileType::new(Some("Rich Text"))
        .with_patterns(&["*.rtf"])
        .with_apple_uniform_type_identifiers(&["public.rtf"])
        .with_mime_types(&["application/rtf", "text/rtf"])
}

/// `OpenButton_Click` and `SaveButton_Click` with the storage provider of the window in place of
/// the pickers of the Windows App SDK: an `.rtf` file from the documents folder is loaded into the
/// editor, or the editor is saved as RTF. A failure is reported below the editor, where WinUI
/// Gallery shows a message dialog.
fn wire_files(root: &Ref<Control>, editor: &Ref<RichEditBox>) {
    let status = root.get_control::<TextBlock>("FileStatus");
    for (name, save) in [("openFileButton", false), ("saveFileButton", true)] {
        let button = root.get_control::<Button>(name);
        let weak_button = button.downgrade();
        let weak_editor = editor.downgrade();
        let weak_status = status.downgrade();
        button.click(move |_, _| {
            let (Some(button), Some(editor), Some(status)) =
                (weak_button.upgrade(), weak_editor.upgrade(), weak_status.upgrade())
            else {
                return;
            };
            let Some(top_level) = TopLevel::get_top_level(Some(&button)) else {
                status.set_text(Some("This window has no storage service."));
                return;
            };
            let provider = top_level.storage_provider();
            if !(if save { provider.can_save() } else { provider.can_open() }) {
                status.set_text(Some("This platform has no file picker."));
                return;
            }
            let weak_editor = editor.downgrade();
            let weak_status = status.downgrade();
            drop(Dispatcher::ui_thread().start_local(async move {
                let outcome = if save {
                    save_file(provider, weak_editor).await
                } else {
                    open_file(provider, weak_editor).await
                };
                if let (Some(message), Some(status)) = (outcome, weak_status.upgrade()) {
                    status.set_text(Some(&message));
                }
            }));
        });
    }
}

/// Returns the message to show, if any.
async fn open_file(
    provider: Rc<dyn IStorageProvider>,
    editor: WeakRef<RichEditBox>,
) -> Option<String> {
    let mut options = FilePickerOpenOptions::new();
    options.set_suggested_start_location(provider.try_get_well_known_folder_async(WellKnownFolder::Documents).await);
    options.set_file_type_filter(Some(vec![rich_text_file_type()]));
    let file = match provider.open_file_picker_async(options).await {
        Ok(files) => files.into_iter().next()?,
        Err(error) => return Some(format!("The file couldn't be opened: {error}")),
    };
    let bytes = match read_all(&file).await {
        Ok(bytes) => bytes,
        Err(error) => return Some(format!("File {} couldn't be opened: {error}", file.name())),
    };
    let editor = editor.upgrade()?;
    match editor.import_rtf(&bytes) {
        Ok(()) => Some(String::new()),
        Err(error) => Some(format!("File {} couldn't be opened: {error}", file.name())),
    }
}

async fn read_all(file: &Rc<dyn IStorageFile>) -> std::io::Result<Vec<u8>> {
    let mut stream = file.open_read_async().await?;
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// Returns the message to show, if any.
async fn save_file(
    provider: Rc<dyn IStorageProvider>,
    editor: WeakRef<RichEditBox>,
) -> Option<String> {
    let mut options = FilePickerSaveOptions::new();
    options.set_suggested_start_location(provider.try_get_well_known_folder_async(WellKnownFolder::Documents).await);
    options.set_suggested_file_name(Some(String::from("New Document")));
    options.set_file_type_choices(Some(vec![rich_text_file_type()]));
    options.set_default_extension(Some(String::from("rtf")));
    let file = match provider.save_file_picker_async(options).await {
        Ok(file) => file?,
        Err(error) => return Some(format!("The file couldn't be saved: {error}")),
    };
    let rtf = editor.upgrade()?.export_rtf();
    let written = match rtf {
        Ok(rtf) => write_all(&file, rtf.as_bytes()).await.map_err(|error| error.to_string()),
        Err(error) => Err(error.to_string()),
    };
    match written {
        Ok(()) => Some(String::new()),
        Err(_) => Some(format!("File {} couldn't be saved.", file.name())),
    }
}

async fn write_all(file: &Rc<dyn IStorageFile>, bytes: &[u8]) -> std::io::Result<()> {
    let mut stream = file.open_write_async().await?;
    stream.write_all(bytes)?;
    stream.flush()
}

// --- RichTextBlock: custom text highlighting -----------------------------------------------

/// `HighlightColorCombobox_SelectionChanged`: the color of the chosen item, by its content, for
/// characters 28 to 39 ("consectetur").
fn wire_text_highlighting(root: &Ref<Control>) {
    let Some(block) = find::<RichTextBlock>(root, "TextHighlightingRichTextBlock") else {
        return;
    };
    let combo = root.get_control::<ComboBox>("HighlightColorCombobox");
    let update = {
        let weak_block = block.downgrade();
        let weak_combo = combo.downgrade();
        move || {
            let (Some(block), Some(combo)) = (weak_block.upgrade(), weak_combo.upgrade()) else {
                return;
            };
            let content = combo
                .selected_item()
                .and_then(|item| Control::from_boxed(&item))
                .and_then(|item| item.cast::<ComboBoxItem>())
                .and_then(|item| item.content())
                .and_then(|content| content.downcast_ref::<String>().cloned())
                .or_else(|| {
                    let names = ["Yellow", "Red", "Blue"];
                    usize::try_from(combo.selected_index())
                        .ok()
                        .and_then(|index| names.get(index))
                        .map(|name| name.to_string())
                });
            let color = match content.as_deref() {
                Some("Red") => Colors::RED,
                Some("Blue") => Colors::BLUE,
                _ => Colors::YELLOW,
            };
            let highlighter = TextHighlighter::new();
            let _ = highlighter.set_background(Some(solid(color)));
            highlighter.ranges().add(TextHighlightRange::new(28, 11));
            let highlighters = block.text_highlighters();
            highlighters.clear();
            highlighters.add(highlighter);
        }
    };
    // The item selected in the markup raises SelectionChanged while the page loads.
    update();
    combo.property_changed(move |change| {
        if change.property() == SelectingItemsControl::selected_index_property().as_property() {
            update();
        }
    });
}
