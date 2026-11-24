//! The code behind of the pinned Popup and Clipboard pages (the handlers of `PopupPage.xaml.cs`
//! and `ClipboardPage.xaml.cs`) and an adaptation of the PullToRefresh code behind.
//! The controls keep their existing platform services and event/deferral APIs.

use ferroui_base::input::platform::{ClipboardContentOptions, ClipboardExtensions, IClipboard};
use ferroui_base::input::{DataFormat, DataPackageOperation, DataTransfer, DataTransferItem, IAsyncDataTransfer};
use ferroui_base::media::IImage;
use ferroui_base::platform::storage::{FilePickerOpenOptions, IStorageItem};
use ferroui_base::reactive::{IDisposable, Subscription};
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
use ferroui_base::{BoxedValue, ObjectType, Ref, WeakRef};
use ferroui_controls::primitives::{Popup, ToggleButton};
use ferroui_controls::pull_to_refresh::{RefreshCompletionDeferral, RefreshContainer};
use ferroui_controls::{
    Button, Control, Image, ListBox, RichEditBox, TextBlock, TextBox, ToggleSwitch, TopLevel,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

pub(crate) fn wire(root: &Ref<Control>) {
    wire_popup(root);
    wire_clipboard(root);
    wire_refresh(root);
}

fn find<T: ObjectType>(root: &Ref<Control>, name: &str) -> Option<Ref<T>> {
    root.find_control::<Control>(name)
        .and_then(|control| control.cast())
}

/// `ShowPopupOffsetClicked`, `ClosePopupClicked` and `PopupClosed` of `PopupPage.xaml.cs`: the
/// buttons open and close the popup, and the light-dismiss option is disabled while it is open.
fn wire_popup(root: &Ref<Control>) {
    let Some(popup) = find::<Popup>(root, "StandardPopup") else {
        return;
    };
    let toggle = root.get_control::<ToggleSwitch>("IsLightDismissEnabledToggleSwitch");
    for (name, open) in [("ShowPopup", true), ("ClosePopup", false)] {
        let button = root.get_control::<Button>(name);
        let weak_popup = popup.downgrade();
        let weak_toggle = toggle.downgrade();
        button.click(move |_, _| {
            let (Some(popup), Some(toggle)) = (weak_popup.upgrade(), weak_toggle.upgrade()) else {
                return;
            };
            if popup.is_open() != open {
                popup.set_is_open(open);
            }
            toggle.set_is_enabled(!open);
        });
    }
    let weak_toggle = toggle.downgrade();
    popup
        .closed(move || {
            if let Some(toggle) = weak_toggle.upgrade() {
                toggle.set_is_enabled(true);
            }
        })
        .detach();
}

/// The source of the image example: `ms-appx:///Assets/SampleMedia/rainier.jpg`.
pub(crate) const RAINIER: &str = "ferres://WinUIGallery/Assets/SampleMedia/rainier.jpg";

/// The text the constructor of `ClipboardPage.xaml.cs` puts in the editor.
pub(crate) const COPIED_TEXT: &str = "This text will be copied to the clipboard.";

/// The message of a page whose window has no clipboard service.
pub(crate) const NO_CLIPBOARD: &str = "This window has no clipboard service.";

/// Runs `operation` with the clipboard of the window of `root` when `button` is clicked; without
/// a clipboard service, `output` says so.
fn on_clipboard_click<F>(
    root: &Ref<Control>,
    button: &str,
    output: &str,
    operation: impl Fn(Ref<Control>) -> F + 'static,
) where
    F: std::future::Future<Output = ()> + 'static,
{
    let weak_root = root.downgrade();
    let output = output.to_string();
    root.get_control::<Button>(button).click(move |_, _| {
        let Some(root) = weak_root.upgrade() else {
            return;
        };
        if clipboard_of(&root).is_none() {
            let output = root.get_control::<TextBlock>(&output);
            output.set_text(Some(NO_CLIPBOARD));
            output.set_is_visible(true);
            return;
        }
        drop(Dispatcher::ui_thread().start_local(operation(root)));
    });
}

/// The RequestedOperation of the data on the clipboard, by WinUI's names.
async fn requested_operation(clipboard: &Rc<dyn IClipboard>) -> String {
    let Ok(Some(data)) = clipboard.try_get_data_async().await else {
        return "None".to_string();
    };
    let operation = DataPackageOperation::of_async(data.clone()).await.unwrap_or_default();
    data.dispose();
    let names: Vec<&str> = [
        (DataPackageOperation::COPY, "Copy"),
        (DataPackageOperation::MOVE, "Move"),
        (DataPackageOperation::LINK, "Link"),
    ]
    .into_iter()
    .filter(|(flag, _)| operation.contains(*flag))
    .map(|(_, name)| name)
    .collect();
    if names.is_empty() { "None".to_string() } else { names.join(", ") }
}

fn clipboard_of(root: &Ref<Control>) -> Option<Rc<dyn IClipboard>> {
    TopLevel::get_top_level(Some(root)).and_then(|host| host.clipboard())
}

fn set_status(root: &WeakRef<Control>, name: &str, text: &str) {
    if let Some(root) = root.upgrade() {
        let output = root.get_control::<TextBlock>(name);
        output.set_text(Some(text));
        output.set_is_visible(true);
    }
}

/// The handlers of `ClipboardPage.xaml.cs` over the window's clipboard service:
/// `CopyText_Click` with its confirmation, shown for two seconds; `PasteText_Click`;
/// `CopyImage_Click` and `PasteImage_Click` with the source's rainier.jpg; `CopyFiles_Click`, with
/// the files of the window's storage provider, and `PasteFiles_Click`; `CopyWithOptions_Click`,
/// with its history and roaming options (Windows has them; elsewhere they do nothing); `ShowFormats_Click` and
/// `ClearClipboard_Click`; and `ContentChangedToggle_Toggled`, over the clipboard's
/// `content_changed`, which reads nothing until the application asks.
fn wire_clipboard(root: &Ref<Control>) {
    let Some(editor) = find::<RichEditBox>(root, "richEditBox") else {
        return;
    };
    let _ = editor.set_text(COPIED_TEXT);
    if let Ok(bitmap) = crate::assets::load_bitmap(RAINIER) {
        root.get_control::<Image>("SourceImage")
            .set_source(Some(Rc::new(bitmap)));
    }
    // IsHistoryEnabled and IsRoamingEnabled: false where the platform has neither (only Windows
    // has them).
    let (history, roaming) = clipboard_of(root)
        .map(|clipboard| (clipboard.is_history_enabled(), clipboard.is_roaming_enabled()))
        .unwrap_or_default();
    let yes_no = |on: bool| if on { "Yes" } else { "No" };
    root.get_control::<TextBlock>("HistoryEnabledText")
        .set_text(Some(&format!("Clipboard history enabled: {}", yes_no(history))));
    root.get_control::<TextBlock>("RoamingEnabledText")
        .set_text(Some(&format!("Clipboard roaming enabled: {}", yes_no(roaming))));

    on_clipboard_click(root, "CopyText", "ConfirmationTextBlock", |root| {
        let clipboard = clipboard_of(&root);
        let text = root.get_control::<RichEditBox>("richEditBox").text();
        let weak = root.downgrade();
        async move {
            let Some(clipboard) = clipboard else { return };
            match clipboard.set_text_async(Some(&text)).await {
                Ok(()) => {
                    set_status(&weak, "ConfirmationTextBlock", "Text copied to clipboard!");
                    // The source hides the confirmation again after two seconds.
                    let weak = weak.clone();
                    DispatcherTimer::run_once(
                        move || {
                            if let Some(root) = weak.upgrade() {
                                root.get_control::<TextBlock>("ConfirmationTextBlock")
                                    .set_is_visible(false);
                            }
                        },
                        Duration::from_secs(2),
                        DispatcherPriority::NORMAL,
                    )
                    .detach();
                }
                Err(error) => {
                    set_status(&weak, "ConfirmationTextBlock", &format!("Clipboard operation failed: {error}"))
                }
            }
        }
    });
    on_clipboard_click(root, "PasteText", "PasteClipboard2", |root| {
        let clipboard = clipboard_of(&root);
        let weak = root.downgrade();
        async move {
            let Some(clipboard) = clipboard else { return };
            match clipboard.try_get_text_async().await {
                Ok(Some(text)) => set_status(&weak, "PasteClipboard2", &text),
                Ok(None) => {}
                Err(error) => set_status(&weak, "PasteClipboard2", &format!("Clipboard operation failed: {error}")),
            }
        }
    });
    on_clipboard_click(root, "CopyImage", "ImageStatusText", |root| {
        let clipboard = clipboard_of(&root);
        let bitmap = crate::assets::load_bitmap(RAINIER).ok().map(Rc::new);
        let weak = root.downgrade();
        async move {
            let Some(clipboard) = clipboard else { return };
            let copied = match bitmap {
                Some(bitmap) => clipboard.set_bitmap_async(Some(bitmap)).await.is_ok(),
                None => false,
            };
            let message = if copied { "Image copied to clipboard." } else { "Error copying image to clipboard." };
            set_status(&weak, "ImageStatusText", message);
        }
    });
    on_clipboard_click(root, "PasteImage", "ImageStatusText", |root| {
        let clipboard = clipboard_of(&root);
        let weak = root.downgrade();
        async move {
            let Some(clipboard) = clipboard else { return };
            let result = clipboard.try_get_bitmap_async().await;
            let Some(root) = weak.upgrade() else { return };
            let pasted = root.get_control::<Image>("PastedImage");
            match result {
                Ok(Some(bitmap)) => {
                    pasted.set_source(Some(bitmap as Rc<dyn IImage>));
                    pasted.set_is_visible(true);
                    set_status(&weak, "ImageStatusText", "Image pasted from clipboard.");
                }
                Ok(None) => {
                    pasted.set_is_visible(false);
                    set_status(&weak, "ImageStatusText", "Bitmap format is not available in the clipboard.");
                }
                Err(error) => set_status(&weak, "ImageStatusText", &format!("Error pasting image: {error}")),
            }
        }
    });
    on_clipboard_click(root, "CopyFiles", "FilesStatusText", |root| {
        let clipboard = clipboard_of(&root);
        let storage = TopLevel::get_top_level(Some(&root)).map(|host| host.storage_provider());
        let weak = root.downgrade();
        async move {
            let Some(clipboard) = clipboard else { return };
            let Some(storage) = storage.filter(|storage| storage.can_open()) else {
                set_status(&weak, "FilesStatusText", "This platform has no file picker.");
                return;
            };
            let options = FilePickerOpenOptions::new().with_allow_multiple(true);
            let files = match storage.open_file_picker_async(options).await {
                Ok(files) if !files.is_empty() => files,
                Ok(_) => return,
                Err(error) => {
                    set_status(&weak, "FilesStatusText", &format!("Error copying files to clipboard: {error}"));
                    return;
                }
            };
            let count = files.len();
            let files: Vec<Rc<dyn IStorageItem>> = files.into_iter().map(|file| file as Rc<dyn IStorageItem>).collect();
            let data = DataTransfer::new();
            for file in &files {
                data.add(DataTransferItem::create(&DataFormat::file(), Some(file.clone())));
            }
            // RequestedOperation = DataPackageOperation.Copy, where the platform has a format for it.
            if let Some(operation) = DataPackageOperation::COPY.to_data_transfer_item(&files) {
                data.add(operation);
            }
            let message = match clipboard.set_data_async(Some(data as Rc<dyn IAsyncDataTransfer>)).await {
                Ok(()) => format!("{count} file(s) copied to clipboard."),
                Err(_) => "Error copying files to clipboard.".to_string(),
            };
            set_status(&weak, "FilesStatusText", &message);
        }
    });
    on_clipboard_click(root, "PasteFiles", "FilesStatusText", |root| {
        let clipboard = clipboard_of(&root);
        let weak = root.downgrade();
        async move {
            let Some(clipboard) = clipboard else { return };
            let message = match clipboard.try_get_files_async().await {
                Ok(Some(files)) => {
                    let mut output = format!("File(s) on clipboard ({}):", files.len());
                    for file in &files {
                        output.push_str(&format!("\n  \u{2022} {}", file.name()));
                    }
                    output.push_str(&format!("\nRequested operation: {}", requested_operation(&clipboard).await));
                    output
                }
                Ok(None) => "StorageItems format is not available in the clipboard.".to_string(),
                Err(error) => format!("Error pasting files: {error}"),
            };
            set_status(&weak, "FilesStatusText", &message);
        }
    });
    on_clipboard_click(root, "CopyWithOptions", "HistoryRoamingStatusText", |root| {
        let clipboard = clipboard_of(&root);
        let text = root
            .get_control::<TextBox>("HistoryRoamingTextBox")
            .text()
            .unwrap_or_default();
        let options = ClipboardContentOptions {
            is_allowed_in_history: root.get_control::<ToggleSwitch>("AllowHistoryToggle").is_checked() == Some(true),
            is_roaming_allowed: root.get_control::<ToggleSwitch>("AllowRoamingToggle").is_checked() == Some(true),
        };
        let weak = root.downgrade();
        async move {
            let Some(clipboard) = clipboard else { return };
            if text.is_empty() {
                set_status(&weak, "HistoryRoamingStatusText", "Please enter text to copy.");
                return;
            }
            let data = DataTransfer::new();
            data.add(DataTransferItem::create_text(Some(&text)));
            let message = match clipboard.set_data_with_options_async(Some(data), options).await {
                Ok(()) => "Text copied to clipboard. History and roaming follow the selected options.".to_string(),
                Err(_) => "Error copying content to clipboard.".to_string(),
            };
            set_status(&weak, "HistoryRoamingStatusText", &message);
        }
    });
    on_clipboard_click(root, "ShowClipboardFormats", "OtherOperationsStatusText", |root| {
        let clipboard = clipboard_of(&root);
        let weak = root.downgrade();
        async move {
            let Some(clipboard) = clipboard else { return };
            let message = match clipboard.get_data_formats_async().await {
                Ok(formats) if !formats.is_empty() => {
                    let mut output = "Available formats on the clipboard:".to_string();
                    for format in formats.iter() {
                        output.push_str(&format!("\n  \u{2022} {format}"));
                    }
                    output
                }
                Ok(_) => "The clipboard is empty.".to_string(),
                Err(error) => format!("Clipboard operation failed: {error}"),
            };
            set_status(&weak, "OtherOperationsStatusText", &message);
        }
    });
    on_clipboard_click(root, "ClearClipboard", "OtherOperationsStatusText", |root| {
        let clipboard = clipboard_of(&root);
        let weak = root.downgrade();
        async move {
            let Some(clipboard) = clipboard else { return };
            let message = match clipboard.clear_async().await {
                Ok(()) => "Clipboard has been cleared.".to_string(),
                Err(error) => format!("Error clearing clipboard: {error}"),
            };
            set_status(&weak, "OtherOperationsStatusText", &message);
        }
    });
    let toggle = root.get_control::<ToggleSwitch>("ContentChangedToggle");
    let weak_toggle = toggle.downgrade();
    let weak_root = root.downgrade();
    // The subscription of the monitor while the switch is on.
    let monitor: Rc<RefCell<Option<Subscription>>> = Rc::default();
    toggle.property_changed(move |change| {
        if change.property() != ToggleButton::is_checked_property().as_property() {
            return;
        }
        let Some(toggle) = weak_toggle.upgrade() else { return };
        if toggle.is_checked() != Some(true) {
            drop(monitor.take());
            set_status(&weak_root, "OtherOperationsStatusText", "Stopped monitoring clipboard changes.");
            return;
        }
        let Some(clipboard) = weak_root.upgrade().and_then(|root| clipboard_of(&root)) else {
            set_status(&weak_root, "OtherOperationsStatusText", "This window has no clipboard.");
            return;
        };
        let changed_root = weak_root.clone();
        let subscription = clipboard.content_changed(Rc::new(move || {
            set_status(&changed_root, "OtherOperationsStatusText", "Clipboard content changed.");
        }));
        monitor.replace(Some(subscription));
        let message = if clipboard.is_content_changed_supported() {
            "Monitoring clipboard changes."
        } else {
            "Monitoring the clipboard changes of this application: this platform does not report the changes of other applications."
        };
        set_status(&weak_root, "OtherOperationsStatusText", message);
    });
}

/// Balances the acquired deferral even if the page disappears before the work
/// reaches the dispatcher. The native deferral has no separate Dispose method.
struct CompleteOnDrop(Rc<RefreshCompletionDeferral>);

impl Drop for CompleteOnDrop {
    fn drop(&mut self) {
        self.0.complete();
    }
}

fn wire_refresh(root: &Ref<Control>) {
    let Some(refresh) = find::<RefreshContainer>(root, "Refresh") else {
        return;
    };
    let items = root.get_control::<ListBox>("RefreshItems");
    let output = root.get_control::<TextBlock>("RefreshOutput");
    for name in ["Button", "TextBlock", "Slider", "ComboBox", "CheckBox"] {
        items
            .items()
            .add(Some(Rc::new(name.to_string()) as BoxedValue));
    }
    let weak_items = items.downgrade();
    let weak_output = output.downgrade();
    refresh.refresh_requested(move |_, args| {
        let complete = CompleteOnDrop(args.get_deferral());
        if let Some(output) = weak_output.upgrade() {
            output.set_text(Some("Refreshing…"));
        }
        let weak_items = weak_items.clone();
        let weak_output = weak_output.clone();
        Dispatcher::ui_thread().post_local(
            move || {
                if let Some(items) = weak_items.upgrade() {
                    items
                        .items()
                        .insert(0, Some(Rc::new("NewControl".to_string()) as BoxedValue));
                }
                if let Some(output) = weak_output.upgrade() {
                    output.set_text(Some("NewControl inserted; refresh completed."));
                }
                drop(complete);
            },
            DispatcherPriority::BACKGROUND,
        );
    });
    let weak_refresh = refresh.downgrade();
    root.get_control::<Button>("RequestRefresh")
        .click(move |_, _| {
            if let Some(refresh) = weak_refresh.upgrade() {
                refresh.request_refresh();
            }
        });
}
