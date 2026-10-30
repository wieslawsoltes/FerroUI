//! Adaptations of the pinned Popup, Clipboard and PullToRefresh code behind.
//! The controls keep their existing platform services and event/deferral APIs.

use ferroui_base::input::platform::ClipboardExtensions;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{BoxedValue, ObjectType, Ref};
use ferroui_controls::primitives::Popup;
use ferroui_controls::pull_to_refresh::{RefreshCompletionDeferral, RefreshContainer};
use ferroui_controls::{Button, Control, ListBox, TextBlock, TextBox, TopLevel};
use std::cell::Cell;
use std::rc::Rc;

pub(crate) fn wire(root: &Ref<Control>) {
    wire_popup(root);
    wire_clipboard(root);
    wire_refresh(root);
}

fn find<T: ObjectType>(root: &Ref<Control>, name: &str) -> Option<Ref<T>> {
    root.find_control::<Control>(name)
        .and_then(|control| control.cast())
}

fn wire_popup(root: &Ref<Control>) {
    let Some(popup) = find::<Popup>(root, "StandardPopup") else {
        return;
    };
    for (name, open) in [("ShowPopup", true), ("ClosePopup", false)] {
        let button = root.get_control::<Button>(name);
        let weak_popup = popup.downgrade();
        button.click(move |_, _| {
            if let Some(popup) = weak_popup.upgrade() {
                if popup.is_open() != open {
                    popup.set_is_open(open);
                }
            }
        });
    }
    let weak_popup = popup.downgrade();
    let weak_output = root.get_control::<TextBlock>("PopupOutput").downgrade();
    popup.property_changed(move |change| {
        if change.property() == Popup::is_open_property().as_property() {
            if let (Some(popup), Some(output)) = (weak_popup.upgrade(), weak_output.upgrade()) {
                output.set_text(Some(if popup.is_open() { "Open" } else { "Closed" }));
            }
        }
    });
}

#[derive(Clone, Copy)]
enum ClipboardOperation {
    Copy,
    Paste,
    Formats,
    Clear,
}

fn wire_clipboard(root: &Ref<Control>) {
    let Some(output) = find::<TextBlock>(root, "ClipboardOutput") else {
        return;
    };
    let busy = Rc::new(Cell::new(false));
    for (name, operation) in [
        ("CopyText", ClipboardOperation::Copy),
        ("PasteText", ClipboardOperation::Paste),
        ("ShowClipboardFormats", ClipboardOperation::Formats),
        ("ClearClipboard", ClipboardOperation::Clear),
    ] {
        let button = root.get_control::<Button>(name);
        let weak_root = root.downgrade();
        let weak_output = output.downgrade();
        let busy = busy.clone();
        button.click(move |_, _| {
            if busy.get() {
                return;
            }
            let (Some(root), Some(output)) = (weak_root.upgrade(), weak_output.upgrade()) else {
                return;
            };
            let Some(clipboard) =
                TopLevel::get_top_level(Some(&root)).and_then(|host| host.clipboard())
            else {
                output.set_text(Some("This window has no clipboard service."));
                return;
            };
            let text = root
                .get_control::<TextBox>("ClipboardText")
                .text()
                .unwrap_or_default();
            let weak_pasted = root.get_control::<TextBox>("PastedText").downgrade();
            let weak_output = output.downgrade();
            let busy = busy.clone();
            busy.set(true);
            output.set_text(Some("Working…"));
            drop(
                Dispatcher::ui_thread()
                    .to_task_scheduler()
                    .start_local(async move {
                        let outcome = match operation {
                            ClipboardOperation::Copy => clipboard
                                .set_text_async(Some(&text))
                                .await
                                .map(|()| "Text copied.".to_string()),
                            ClipboardOperation::Paste => {
                                clipboard.try_get_text_async().await.map(|text| {
                                    if let Some(pasted) = weak_pasted.upgrade() {
                                        pasted.set_text(text.as_deref());
                                    }
                                    if text.is_some() {
                                        "Text pasted.".to_string()
                                    } else {
                                        "The clipboard has no text.".to_string()
                                    }
                                })
                            }
                            ClipboardOperation::Formats => {
                                clipboard.get_data_formats_async().await.map(|formats| {
                                    if formats.is_empty() {
                                        "The clipboard is empty.".to_string()
                                    } else {
                                        format!(
                                            "Available formats: {}",
                                            formats
                                                .iter()
                                                .map(ToString::to_string)
                                                .collect::<Vec<_>>()
                                                .join(", ")
                                        )
                                    }
                                })
                            }
                            ClipboardOperation::Clear => clipboard
                                .clear_async()
                                .await
                                .map(|()| "Clipboard cleared.".to_string()),
                        };
                        busy.set(false);
                        if let Some(output) = weak_output.upgrade() {
                            output.set_text(Some(&outcome.unwrap_or_else(|error| {
                                format!("Clipboard operation failed: {error}")
                            })));
                        }
                    }),
            );
        });
    }
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
