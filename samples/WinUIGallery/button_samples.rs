//! The code behind of the pinned Button, RepeatButton, ToggleButton, SplitButton and
//! ToggleSplitButton pages in Rust: the handlers of `ButtonPage.xaml.cs`,
//! `RepeatButtonPage.xaml.cs`, `ToggleButtonPage.xaml.cs`, `SplitButtonPage.xaml.cs` and
//! `ToggleSplitButtonPage.xaml.cs`. The HyperlinkButton page's handler navigates the gallery
//! (`examples::GALLERY_LINKS`). The controls own input, repetition, flyouts, editing and checked
//! state; the sample owns only its output.

use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Color, Colors, IBrush};
use ferroui_base::Ref;
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::documents::CharacterFormat;
use ferroui_controls::primitives::ToggleButton;
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::{
    Border, Button, Control, Image, RepeatButton, RichEditBox, SplitButton, Symbol, SymbolIcon, TextBlock,
    ToggleSplitButton,
};
use std::cell::Cell;
use std::rc::Rc;

/// The image of the source's `Button2`: `/Assets/SampleMedia/Slices.png`.
const SLICES: &str = "ferres://WinUIGallery/Assets/SampleMedia/Slices.png";

pub(crate) fn wire(page_id: &str, root: &Ref<Control>) {
    match page_id {
        "Button" => wire_button(root),
        "RepeatButton" => wire_repeat_button(root),
        "ToggleButton" => wire_toggle_button(root),
        "SplitButton" => wire_split_button(root),
        "ToggleSplitButton" => wire_toggle_split_button(root),
        _ => {}
    }
}

/// `Button_Click` of `ButtonPage.xaml.cs`: the clicked button's name picks its output.
fn wire_button(root: &Ref<Control>) {
    let outputs = Rc::new([
        root.get_control::<TextBlock>("Control1Output").downgrade(),
        root.get_control::<TextBlock>("Control2Output").downgrade(),
    ]);
    for name in ["Button1", "Button2"] {
        let outputs = outputs.clone();
        root.get_control::<Button>(name).click(move |sender, _| {
            let Some(name) = sender.name() else { return };
            let output = match name.as_str() {
                "Button1" => &outputs[0],
                "Button2" => &outputs[1],
                _ => return,
            };
            if let Some(output) = output.upgrade() {
                output.set_text(Some(&format!("You clicked: {name}")));
            }
        });
    }
    let image = root.get_control::<Image>("Button2Image");
    match crate::assets::load_bitmap(SLICES) {
        Ok(bitmap) => image.set_source(Some(Rc::new(bitmap))),
        Err(error) => {
            let diagnostic = root.get_control::<TextBlock>("Button2ImageError");
            diagnostic.set_text(Some(&format!("Slices.png: {error}")));
            diagnostic.set_is_visible(true);
        }
    }
}

ferroui_base::ferro_thread_local! {
    /// The source's `private static int _clicks`: the count is kept when the page is opened again.
    static REPEAT_CLICKS: Cell<u64> = const { Cell::new(0) };
}

/// `RepeatButton_Click` of `RepeatButtonPage.xaml.cs`. The output is a polite live region, which
/// announces the changed text as the source's `LiveRegionChanged` event does.
fn wire_repeat_button(root: &Ref<Control>) {
    let output = root.get_control::<TextBlock>("RepeatButton1Output").downgrade();
    root.get_control::<RepeatButton>("RepeatButton1").click(move |_, _| {
        let clicks = REPEAT_CLICKS.with(|clicks| {
            clicks.set(clicks.get().saturating_add(1));
            clicks.get()
        });
        if let Some(output) = output.upgrade() {
            output.set_text(Some(&format!("Number of clicks: {clicks}")));
        }
    });
}

/// The constructor and `ToggleButton_Checked`/`ToggleButton_Unchecked` of
/// `ToggleButtonPage.xaml.cs`: the output is "On" while the button is checked, "Off" otherwise.
fn wire_toggle_button(root: &Ref<Control>) {
    let toggle = root.get_control::<ToggleButton>("Toggle1");
    let output = root.get_control::<TextBlock>("Toggle1Output");
    output.set_text(Some(if toggle.is_checked() == Some(true) { "On" } else { "Off" }));
    let weak_toggle = toggle.downgrade();
    let weak_output = output.downgrade();
    toggle.is_checked_changed(move |_, _| {
        let (Some(toggle), Some(output)) = (weak_toggle.upgrade(), weak_output.upgrade()) else {
            return;
        };
        match toggle.is_checked() {
            Some(true) => output.set_text(Some("On")),
            Some(false) => output.set_text(Some("Off")),
            // Neither Checked nor Unchecked is raised for the indeterminate state.
            None => {}
        }
    });
}

fn solid(color: Color) -> Rc<dyn IBrush> {
    Rc::new(ImmutableSolidColorBrush::new(color))
}

fn brush_color(brush: &Rc<dyn IBrush>) -> Option<Color> {
    brush.as_solid_color_brush().map(|brush| brush.color())
}

/// `Document.Selection.CharacterFormat.ForegroundColor = color`.
fn set_selection_foreground(editor: &RichEditBox, color: Color) {
    let _ = editor.text_document().selection().set_character_format(&CharacterFormat {
        foreground: Some(Some(solid(color))),
        ..CharacterFormat::default()
    });
}

/// The swatch buttons of the first flyout (the source's GridView items) and of the second.
pub(crate) const SWATCHES: [&str; 8] = ["Red", "Orange", "Yellow", "Green", "Blue", "Indigo", "Violet", "Gray"];

/// The text the constructor of `SplitButtonPage.xaml.cs` writes into the editor.
pub(crate) const SPLIT_BUTTON_TEXT: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, \
    sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Tempor commodo ullamcorper a lacus.";

/// The constructor and the handlers of `SplitButtonPage.xaml.cs`: `currentColor` is the color of
/// the text, green at first; the primary part gives the text the color of the swatch it shows,
/// a swatch of the flyout (`GridView_ItemClick`) becomes that color, and new text takes it
/// (`MyRichEditBox_TextChanged`). The second button's swatches close its flyout
/// (`RevealColorButton_Click`).
fn wire_split_button(root: &Ref<Control>) {
    let editor = root.get_control::<RichEditBox>("myRichEditBox");
    let current_color = Rc::new(Cell::new(Colors::GREEN));

    // `MyRichEditBox_TextChanged`, wired in the markup of the source before the constructor runs.
    let weak_editor = editor.downgrade();
    let color = current_color.clone();
    editor.text_changed(move |_, _| {
        let Some(editor) = weak_editor.upgrade() else {
            return;
        };
        let foreground = editor
            .text_document()
            .selection()
            .character_format()
            .ok()
            .and_then(|format| format.foreground.flatten())
            .and_then(|brush| brush_color(&brush));
        if foreground != Some(color.get()) {
            set_selection_foreground(&editor, color.get());
        }
    });
    // `Selection.CharacterFormat.ForegroundColor = currentColor` and `Selection.SetText`: the text
    // replaces the selection with the selection's format, and the selection covers it after.
    set_selection_foreground(&editor, current_color.get());
    if let Ok(range) = editor.text_document().selection().range() {
        if range.set_text(SPLIT_BUTTON_TEXT).is_ok() {
            let _ = range.set_character_format(&CharacterFormat {
                foreground: Some(Some(solid(current_color.get()))),
                ..CharacterFormat::default()
            });
            let _ = range.select();
        }
    }

    // `myColorButton_Click`: the color of the swatch the primary part shows.
    let split_button = root.get_control::<SplitButton>("myColorButton");
    let weak_editor = editor.downgrade();
    let color = current_color.clone();
    split_button.click(move |sender, _| {
        let Some(editor) = weak_editor.upgrade() else {
            return;
        };
        let Some(swatch) = sender
            .to_ref()
            .cast::<SplitButton>()
            .and_then(|button| button.content())
            .and_then(|content| Control::from_boxed(&content))
            .and_then(|content| content.cast::<Border>())
            .and_then(|border| border.background())
            .and_then(|background| brush_color(&background))
        else {
            return;
        };
        set_selection_foreground(&editor, swatch);
        color.set(swatch);
    });

    // `GridView_ItemClick`: the clicked swatch colors the text and the primary part.
    let current = root.get_control::<Border>("CurrentColor");
    for name in SWATCHES {
        let button = root.get_control::<Button>(&format!("{name}SwatchButton"));
        let weak_button = button.downgrade();
        let weak_editor = editor.downgrade();
        let weak_current = current.downgrade();
        let weak_split_button = split_button.downgrade();
        let color = current_color.clone();
        button.click(move |_, _| {
            let (Some(button), Some(editor), Some(current)) =
                (weak_button.upgrade(), weak_editor.upgrade(), weak_current.upgrade())
            else {
                return;
            };
            let Some(swatch) = button
                .content()
                .and_then(|content| Control::from_boxed(&content))
                .and_then(|content| content.cast::<Rectangle>())
                .and_then(|rectangle| rectangle.fill())
                .and_then(|fill| brush_color(&fill))
            else {
                return;
            };
            set_selection_foreground(&editor, swatch);
            current.set_background(Some(solid(swatch)));
            editor.focus();
            color.set(swatch);
            // The source hides the flyout after a delay that works around a GridView defect; the
            // swatches here are buttons, so it closes at once.
            if let Some(split_button) = weak_split_button.upgrade() {
                split_button.close_flyout();
            }
        });
    }

    // `RevealColorButton_Click`.
    let reveal = root.get_control::<SplitButton>("myColorButtonReveal");
    for name in SWATCHES.iter().copied().chain(["Black"]) {
        let weak_reveal = reveal.downgrade();
        root.get_control::<Button>(&format!("{name}RevealButton")).click(move |_, _| {
            if let Some(reveal) = weak_reveal.upgrade() {
                reveal.close_flyout();
            }
        });
    }
}

/// `BulletButton_Click` of `ToggleSplitButtonPage.xaml.cs`: the symbol of the clicked button
/// becomes the symbol and the automation name of the toggle split button, which is checked and
/// closes its flyout, and the editor takes the focus. The source also sets the list type of the
/// paragraph (`ParagraphFormat.ListType`) here and in `MyListButton_IsCheckedChanged`; FerroUI's
/// RichEditBox has no list type, so the page is partial.
fn wire_toggle_split_button(root: &Ref<Control>) {
    let list_button = root.get_control::<ToggleSplitButton>("myListButton");
    let symbol_icon = root.get_control::<SymbolIcon>("mySymbolIcon");
    let editor = root.get_control::<RichEditBox>("myRichEditBox");
    for name in ["BulletedListButton", "RomanNumeralsListButton"] {
        let weak_list_button = list_button.downgrade();
        let weak_symbol_icon = symbol_icon.downgrade();
        let weak_editor = editor.downgrade();
        root.get_control::<Button>(name).click(move |sender, _| {
            let (Some(list_button), Some(symbol_icon), Some(editor)) =
                (weak_list_button.upgrade(), weak_symbol_icon.upgrade(), weak_editor.upgrade())
            else {
                return;
            };
            let symbol = sender
                .to_ref()
                .cast::<Button>()
                .and_then(|button| button.content())
                .and_then(|content| Control::from_boxed(&content))
                .and_then(|content| content.cast::<SymbolIcon>())
                .map(|icon| icon.symbol());
            match symbol {
                Some(Symbol::List) => {
                    symbol_icon.set_symbol(Symbol::List);
                    AutomationProperties::set_name(&list_button, Some("Bullets"));
                }
                Some(Symbol::Bullets) => {
                    symbol_icon.set_symbol(Symbol::Bullets);
                    AutomationProperties::set_name(&list_button, Some("Roman Numerals"));
                }
                _ => {}
            }
            list_button.set_is_checked(true);
            list_button.close_flyout();
            editor.focus();
        });
    }
}
