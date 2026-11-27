//! The code behind of the pinned ContentDialog page in Rust: `ShowDialog_Click` and
//! `ShowDialogNoDefault_Click` of `ContentDialogPage.xaml.cs`, with the content of
//! `ContentDialogContent.xaml`.

use ferroui_base::media::TextWrapping;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::{
    Button, CheckBox, ContentDialog, ContentDialogButton, ContentDialogResult, Control, StackPanel, TextBlock,
};
use std::rc::Rc;

/// What an example's dialog says and what its result texts are.
pub(crate) struct DialogSpec {
    pub(crate) title: &'static str,
    pub(crate) primary: &'static str,
    pub(crate) secondary: &'static str,
    pub(crate) default_button: ContentDialogButton,
    pub(crate) on_primary: &'static str,
    pub(crate) on_secondary: &'static str,
}

pub(crate) const SAVE: DialogSpec = DialogSpec {
    title: "Save your work?",
    primary: "Save",
    secondary: "Don't Save",
    default_button: ContentDialogButton::Primary,
    on_primary: "User saved their work",
    on_secondary: "User did not save their work",
};

pub(crate) const REPLACE: DialogSpec = DialogSpec {
    title: "Replace file?",
    primary: "Replace",
    secondary: "Keep",
    default_button: ContentDialogButton::None,
    on_primary: "User replaced the file",
    on_secondary: "User kept the file",
};

/// The text of a result.
pub(crate) fn result_text(spec: &DialogSpec, result: ContentDialogResult) -> &'static str {
    match result {
        ContentDialogResult::Primary => spec.on_primary,
        ContentDialogResult::Secondary => spec.on_secondary,
        ContentDialogResult::None => "User cancelled the dialog",
    }
}

/// `ContentDialogContent.xaml`.
fn dialog_content() -> Ref<Control> {
    let panel = StackPanel::new();
    let text = TextBlock::new();
    text.set_text(Some("Lorem ipsum dolor sit amet, adipisicing elit."));
    text.set_text_wrapping(TextWrapping::Wrap);
    panel.children().add(text);
    let check = CheckBox::new();
    check.set_content(Some(Rc::new("Upload your content to the cloud.".to_string()) as BoxedValue));
    panel.children().add(check);
    panel.upcast()
}

fn show(button: &Ref<Button>, output: &Ref<TextBlock>, spec: &'static DialogSpec) {
    let dialog = ContentDialog::new();
    dialog.set_title(Some(Rc::new(spec.title.to_string()) as BoxedValue));
    dialog.set_primary_button_text(Some(spec.primary));
    dialog.set_secondary_button_text(Some(spec.secondary));
    dialog.set_close_button_text(Some("Cancel"));
    dialog.set_default_button(spec.default_button);
    dialog.set_content(Some(Control::boxed(dialog_content())));
    // `dialog.RequestedTheme = stackPanel.ActualTheme`.
    dialog.set_value(ThemeVariant::requested_theme_variant_property(), button.actual_theme_variant());
    let weak_output = output.downgrade();
    dialog
        .closed(move |args| {
            if let Some(output) = weak_output.upgrade() {
                output.set_text(Some(result_text(spec, args.result())));
            }
        })
        .detach();
    // The button of a page in a window; a second click while a dialog is open does nothing.
    drop(dialog.show_async(button));
}

pub(crate) fn wire(root: &Ref<Control>) {
    for (name, result, spec) in
        [("ShowDialog", "DialogResult", &SAVE), ("ShowDialogNoDefault", "DialogResultNoDefault", &REPLACE)]
    {
        let Some(button) = root.find_control::<Button>(name) else {
            return;
        };
        let output = root.get_control::<TextBlock>(result);
        let weak_button = button.downgrade();
        let weak_output = output.downgrade();
        button.click(move |_, _| {
            if let (Some(button), Some(output)) = (weak_button.upgrade(), weak_output.upgrade()) {
                show(&button, &output, spec);
            }
        });
    }
}
