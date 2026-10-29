//! Interactive inking sample; works through the same controls on every backend.
use crate::markup::{content_page_class, xaml_class};
use ferroui_base::threading::Dispatcher;
use ferroui_base::Point;
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::{Button, ContentPage, TextBlock, TopLevel};
use ferroui_controls_inking::{CoreInputDeviceTypes, InkCanvas, InkClipboard, InkToolbar};
use std::cell::RefCell;

#[derive(Clone, Copy)]
enum InkAction {
    Clear,
    Save,
    Restore,
    SelectAll,
    Copy,
    Paste,
}

#[repr(C)]
pub struct InkingPage {
    base: ContentPage,
    snapshot: RefCell<Option<Vec<u8>>>,
}
content_page_class!(InkingPage);
ferro_class_info!(InkingPage {
    new: InkingPage::new
});
xaml_class!(InkingPage, "/Pages/InkingPage.xaml");

impl InkingPage {
    pub fn construct() -> Self {
        Self {
            base: ContentPage::construct(),
            snapshot: RefCell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        let canvas = this
            .find_control::<InkCanvas>("InkSurface")
            .expect("InkingPage InkSurface");
        let presenter = canvas.ink_presenter();
        presenter.set_input_device_types(
            CoreInputDeviceTypes::PEN | CoreInputDeviceTypes::MOUSE | CoreInputDeviceTypes::TOUCH,
        );
        this.find_control::<InkToolbar>("Toolbar")
            .expect("InkingPage Toolbar")
            .set_target_ink_canvas(Some(canvas));
        for (name, action) in [
            ("ClearInk", InkAction::Clear),
            ("SaveInk", InkAction::Save),
            ("RestoreInk", InkAction::Restore),
            ("SelectInk", InkAction::SelectAll),
            ("CopyInk", InkAction::Copy),
            ("PasteInk", InkAction::Paste),
        ] {
            let weak = this.downgrade();
            this.find_control::<Button>(name)
                .expect("InkingPage action")
                .click(move |_, _| {
                    if let Some(this) = weak.upgrade() {
                        this.run_action(action);
                    }
                });
        }
        let weak = this.downgrade();
        presenter.stroke_container().changed(move || {
            if let Some(this) = weak.upgrade() {
                this.update_status(None);
            }
        });
        this
    }
    fn run_action(&self, action: InkAction) {
        let Some(canvas) = self.find_control::<InkCanvas>("InkSurface") else {
            return;
        };
        let container = canvas.ink_presenter().stroke_container();
        if matches!(action, InkAction::Copy | InkAction::Paste) {
            let Some(clipboard) =
                TopLevel::get_top_level(Some(self)).and_then(|top| top.clipboard())
            else {
                self.update_status(Some("Clipboard is unavailable"));
                return;
            };
            let service = InkClipboard::new(clipboard);
            let weak = self.to_ref().downgrade();
            let copy = matches!(action, InkAction::Copy);
            let task = if copy {
                service.copy_selected(&container)
            } else {
                let paste = service.paste(container, Point::new(24.0, 24.0));
                Box::pin(async move { paste.await.map(|_| ()) })
            };
            Dispatcher::ui_thread()
                .to_task_scheduler()
                .start_local(async move {
                    let result = task.await;
                    if let Some(this) = weak.upgrade() {
                        match result {
                            Ok(()) => this.update_status(Some(if copy {
                                "Selection copied"
                            } else {
                                "Ink pasted"
                            })),
                            Err(error) => this.update_status(Some(&error.to_string())),
                        }
                    }
                });
            return;
        }
        let result = match action {
            InkAction::Clear => {
                container.clear();
                Ok(())
            }
            InkAction::Save => container.save().map(|bytes| {
                *self.snapshot.borrow_mut() = Some(bytes);
            }),
            InkAction::SelectAll => {
                for stroke in container.get_strokes() {
                    stroke.set_selected(true);
                }
                Ok(())
            }
            _ => {
                let snapshot = self.snapshot.borrow().clone();
                match snapshot {
                    Some(bytes) => container.load(&bytes),
                    None => {
                        self.update_status(Some("Save a snapshot first"));
                        return;
                    }
                }
            }
        };
        match result {
            Ok(()) => self.update_status(None),
            Err(error) => self.update_status(Some(&error.to_string())),
        }
    }
    fn update_status(&self, message: Option<&str>) {
        let Some(status) = self.find_control::<TextBlock>("InkStatus") else {
            return;
        };
        let count = self.find_control::<InkCanvas>("InkSurface").map_or(0, |c| {
            c.ink_presenter().stroke_container().get_strokes().len()
        });
        status.set_text(Some(
            &message
                .map(str::to_owned)
                .unwrap_or_else(|| format!("{count} strokes")),
        ));
    }
}
