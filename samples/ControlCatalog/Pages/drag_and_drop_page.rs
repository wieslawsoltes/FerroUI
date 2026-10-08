//! Port of `Pages/DragAndDropPage.xaml.cs`: the class of the document
//! `Pages/DragAndDropPage.xaml`.

use super::DialogsPage;
use crate::markup::{content_page_class, xaml_class};
use ferroui_base::input::{
    DataFormat, DataFormatOf, DataTransfer, DataTransferExtensions, DataTransferItem, DragDrop, DragDropEffects,
    DragEventArgs, IDataTransfer, InputElement, LocalBoxFuture,
};
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::media::{IImage, Stretch};
use ferroui_base::platform::storage::IStorageItem;
use ferroui_base::platform::AssetLoader;
use ferroui_base::utilities::Uri;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref, WeakRef};
use ferroui_controls::{Border, ContentControl, ContentPage, Control, Image, TextBlock, TopLevel};
use mini_mvvm::start_async;
use std::cell::Cell;
use std::rc::Rc;

/// `Environment.NewLine`.
const NEW_LINE: &str = if cfg!(windows) { "\r\n" } else { "\n" };

/// Fills the data transfer of a drag operation (`Func<DataTransfer, Task>`).
type DragDataFactory = Rc<dyn Fn(Rc<DataTransfer>) -> LocalBoxFuture<()>>;

#[repr(C)]
pub struct DragAndDropPage {
    base: ContentPage,
    custom_format: DataFormatOf<String>,
}

content_page_class!(DragAndDropPage);
ferro_class_info!(DragAndDropPage { new: DragAndDropPage::new });
xaml_class!(DragAndDropPage, "/Pages/DragAndDropPage.xaml");

/// A text as the untyped content of a control.
fn text_content(text: &str) -> BoxedValue {
    Rc::new(text.to_string())
}

impl DragAndDropPage {
    pub fn construct() -> Self {
        Self {
            base: ContentPage::construct(),
            custom_format: DataFormat::create_string_application_format("xxx-ferroui-controlcatalog-custom"),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        let text_count = Cell::new(0);

        this.setup_dnd_sync(
            "Text",
            move |d| {
                text_count.set(text_count.get() + 1);
                let text = format!("Text was dragged {} times", text_count.get());
                d.add(DataTransferItem::create(&DataFormat::text(), Some(text)));
            },
            DragDropEffects::COPY | DragDropEffects::MOVE | DragDropEffects::LINK,
        );

        let custom_format = this.custom_format.clone();
        this.setup_dnd_sync(
            "Custom",
            move |d| d.add(DataTransferItem::create(&custom_format, Some(String::from("Test123")))),
            DragDropEffects::COPY | DragDropEffects::MOVE,
        );

        // The handler is held by a descendant of the page: it holds the page weakly.
        let weak = this.downgrade();
        this.setup_dnd(
            "Files",
            Rc::new(move |d: Rc<DataTransfer>| -> LocalBoxFuture<()> {
                let weak = weak.clone();
                Box::pin(async move {
                    let Some(current_file) = std::env::current_exe().ok().filter(|path| path.is_file()) else {
                        return;
                    };
                    let Some(current_file) = current_file.to_str() else {
                        return;
                    };
                    let Some(this) = weak.upgrade() else {
                        return;
                    };
                    let Some(top_level) = TopLevel::get_top_level(Some(&this)) else {
                        return;
                    };

                    let pending = top_level.storage_provider().try_get_file_from_path_str_async(current_file);
                    if let Some(storage_file) = pending.await {
                        let storage_item: Rc<dyn IStorageItem> = storage_file;
                        d.add(DataTransferItem::create(&DataFormat::file(), Some(storage_item)));
                    }
                })
            }),
            DragDropEffects::COPY,
        );

        this.setup_dnd_sync(
            "Bitmap",
            |d| {
                let uri = Uri::absolute("ferres://ControlCatalog/Assets/image1.jpg").unwrap_or_else(|e| panic!("{e}"));
                let mut asset = AssetLoader::open(&uri, None).unwrap_or_else(|e| panic!("{e}"));
                let bitmap = Rc::new(Bitmap::from_stream(&mut asset).unwrap_or_else(|e| panic!("{e}")));
                d.add(DataTransferItem::create(&DataFormat::bitmap(), Some(bitmap)));
            },
            DragDropEffects::COPY,
        );

        this.add_handlers(&this.get_control::<Border>("CopyTarget"), DragDropEffects::COPY);
        this.add_handlers(&this.get_control::<Border>("MoveTarget"), DragDropEffects::MOVE);

        this
    }

    fn drop_state(&self) -> Ref<ContentControl> {
        self.get_control::<ContentControl>("DropState")
    }

    /// `SetupDnd(suffix, Action<DataTransfer> factory, effects)`.
    fn setup_dnd_sync(&self, suffix: &str, factory: impl Fn(&DataTransfer) + 'static, effects: DragDropEffects) {
        self.setup_dnd(
            suffix,
            Rc::new(move |o: Rc<DataTransfer>| -> LocalBoxFuture<()> {
                factory(&o);
                Box::pin(std::future::ready(()))
            }),
            effects,
        );
    }

    /// `SetupDnd(suffix, Func<DataTransfer, Task> factory, effects)`.
    fn setup_dnd(&self, suffix: &str, factory: DragDataFactory, effects: DragDropEffects) {
        let drag_me = self.get_control::<Border>(&format!("DragMe{suffix}"));
        let drag_state = self.get_control::<TextBlock>(&format!("DragState{suffix}"));

        // `DoDrag` (`async void`). The text block is a descendant of the border the
        // handler is added to: the handler holds it weakly.
        let drag_state: WeakRef<TextBlock> = drag_state.downgrade();
        drag_me.add_handler(InputElement::pointer_pressed_event(), move |_, e| {
            let e = e.clone();
            let factory = factory.clone();
            let drag_state = drag_state.clone();
            drop(start_async(async move {
                let drag_data = DataTransfer::new();
                factory(drag_data.clone()).await;

                let data_transfer: Rc<dyn IDataTransfer> = drag_data;
                let result = DragDrop::do_drag_drop_async(&e, data_transfer, effects).await;
                let text = if result == DragDropEffects::MOVE {
                    "Data was moved"
                } else if result == DragDropEffects::COPY {
                    "Data was copied"
                } else if result == DragDropEffects::LINK {
                    "Data was linked"
                } else if result == DragDropEffects::NONE {
                    "The drag operation was canceled"
                } else {
                    "Unknown result"
                };
                if let Some(drag_state) = drag_state.upgrade() {
                    drag_state.set_text(Some(text));
                }
            }));
        });
    }

    /// `AddHandlers(target, allowedEffects)`.
    fn add_handlers(&self, target: &Control, allowed_effects: DragDropEffects) {
        let custom_format = self.custom_format.clone();
        let drag_over = move |e: &DragEventArgs| {
            e.set_drag_effects(e.drag_effects() & allowed_effects);

            // Only allow if the dragged data contains text or filenames.
            let data_transfer = e.data_transfer();
            if !data_transfer.contains(&DataFormat::text())
                && !data_transfer.contains(&DataFormat::file())
                && !data_transfer.contains(&DataFormat::bitmap())
                && !data_transfer.contains(&custom_format)
            {
                e.set_drag_effects(DragDropEffects::NONE);
            }
        };

        // The target is a descendant of the page: its handler holds the content
        // control of the page weakly.
        let custom_format = self.custom_format.clone();
        let drop_state = self.drop_state().downgrade();
        // `Drop` (`async void`).
        let on_drop = move |e: &DragEventArgs| {
            e.set_drag_effects(e.drag_effects() & allowed_effects);

            if e.drag_effects() == DragDropEffects::NONE {
                return;
            }
            let Some(drop_state) = drop_state.upgrade() else {
                return;
            };

            let data_transfer = e.data_transfer();
            if data_transfer.contains(&DataFormat::text()) {
                drop_state.set_content(data_transfer.try_get_text().map(|text| text_content(&text)));
            } else if data_transfer.contains(&DataFormat::file()) {
                let files = data_transfer.try_get_files().unwrap_or_default();
                drop(start_async(async move {
                    let mut content_str = String::new();

                    for item in files {
                        if let Some(file) = item.clone().as_storage_file() {
                            let content = DialogsPage::read_text_from_file(file, 500)
                                .await
                                .unwrap_or_else(|error| panic!("{error}"));
                            content_str +=
                                &format!("File {}:{NEW_LINE}{content}{NEW_LINE}{NEW_LINE}", item.name());
                        } else if let Some(folder) = item.clone().as_storage_folder() {
                            let children_count =
                                folder.get_items_async().await.unwrap_or_else(|error| panic!("{error}")).len();
                            content_str +=
                                &format!("Folder {}: items {children_count}{NEW_LINE}{NEW_LINE}", item.name());
                        }
                    }
                    drop_state.set_content(Some(text_content(&content_str)));
                }));
            } else if data_transfer.contains(&DataFormat::bitmap()) {
                let bitmap = data_transfer.try_get_value(&DataFormat::bitmap());
                let image = Image::new();
                image.set_source(bitmap.map(|bitmap| bitmap as Rc<dyn IImage>));
                image.set_width(400.0);
                image.set_height(300.0);
                image.set_stretch(Stretch::Uniform);
                drop_state.set_content(Some(Control::boxed(image)));
            } else if data_transfer.contains(&custom_format) {
                let value = data_transfer.try_get_value(&custom_format).unwrap_or_default();
                drop_state.set_content(Some(text_content(&format!("Custom: {value}"))));
            }
        };

        let drag_enter = drag_over.clone();
        DragDrop::add_drag_enter_handler(target, move |_, e| drag_enter(e));
        DragDrop::add_drag_over_handler(target, move |_, e| drag_over(e));
        DragDrop::add_drop_handler(target, move |_, e| on_drop(e));
    }
}
