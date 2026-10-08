//! Port of `Pages/ClipboardPage.xaml.cs`: the class of the document
//! `Pages/ClipboardPage.xaml`.

use crate::markup::xaml_class;
use crate::view_models::random::Random;
use ferroui_base::input::platform::{ClipboardError, ClipboardExtensions, IClipboard};
use ferroui_base::input::{
    AsyncDataTransferExtensions, DataFormat, DataFormatOf, DataTransfer, DataTransferItem, IAsyncDataTransfer,
    InputElementImpl,
};
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::media::{Brushes, IBrush, IImage};
use ferroui_base::platform::storage::{IStorageFile, IStorageItem};
use ferroui_base::platform::AssetLoader;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::utilities::Uri;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl,
    VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::documents::Run;
use ferroui_controls::notifications::{Notification, NotificationType, WindowNotificationManager};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ContentPage, ControlImpl, Image, PageImpl, TextBox, TopLevel};
use mini_mvvm::start_async;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// `Environment.NewLine`.
const NEW_LINE: &str = if cfg!(windows) { "\r\n" } else { "\n" };

#[repr(C)]
pub struct ClipboardPage {
    base: ContentPage,
    custom_binary_data_format: DataFormatOf<Rc<[u8]>>,
    notification_manager: RefCell<Option<Ref<WindowNotificationManager>>>,
    clipboard_last_data_object_checker: RefCell<Option<Rc<DispatcherTimer>>>,
    stored_data_transfer: RefCell<Option<Rc<DataTransfer>>>,
    checking_clipboard_data_transfer: Cell<bool>,
    default_image: RefCell<Option<Rc<Bitmap>>>,
}

ferro_class!(ClipboardPage: ContentPage);
ferro_impl_classes!(
    ClipboardPage: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    PageImpl
);
ferro_class_info!(ClipboardPage {
    new: ClipboardPage::new,
    markup: {
        methods: [
            fn CopyText(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ClipboardPage>, sender: Option<BoxedValue>, args: Rc<dyn IRoutedEventArgs>| {
                    this.copy_text(&sender, args.as_routed_event_args())
                },
            fn CopyImage(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ClipboardPage>, sender: Option<BoxedValue>, args: Rc<dyn IRoutedEventArgs>| {
                    this.copy_image(&sender, args.as_routed_event_args())
                },
            fn PasteText(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ClipboardPage>, sender: Option<BoxedValue>, args: Rc<dyn IRoutedEventArgs>| {
                    this.paste_text(&sender, args.as_routed_event_args())
                },
            fn PasteImage(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ClipboardPage>, sender: Option<BoxedValue>, args: Rc<dyn IRoutedEventArgs>| {
                    this.paste_image(&sender, args.as_routed_event_args())
                },
            fn CopyFiles(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ClipboardPage>, sender: Option<BoxedValue>, args: Rc<dyn IRoutedEventArgs>| {
                    this.copy_files(&sender, args.as_routed_event_args())
                },
            fn PasteFiles(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ClipboardPage>, sender: Option<BoxedValue>, args: Rc<dyn IRoutedEventArgs>| {
                    this.paste_files(&sender, args.as_routed_event_args())
                },
            fn GetFormats(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ClipboardPage>, sender: Option<BoxedValue>, args: Rc<dyn IRoutedEventArgs>| {
                    this.get_formats(&sender, args.as_routed_event_args())
                },
            fn CopyBinaryData(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ClipboardPage>, sender: Option<BoxedValue>, args: Rc<dyn IRoutedEventArgs>| {
                    this.copy_binary_data(&sender, args.as_routed_event_args())
                },
            fn PasteBinaryData(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ClipboardPage>, sender: Option<BoxedValue>, args: Rc<dyn IRoutedEventArgs>| {
                    this.paste_binary_data(&sender, args.as_routed_event_args())
                },
            fn Clear(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ClipboardPage>, sender: Option<BoxedValue>, args: Rc<dyn IRoutedEventArgs>| {
                    this.clear(&sender, args.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(ClipboardPage, "/Pages/ClipboardPage.xaml");

impl VisualImpl for ClipboardPage {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        let timer = this.clipboard_last_data_object_checker.borrow().clone();
        if let Some(timer) = timer {
            timer.start();
        }
        Self::parent_on_attached_to_visual_tree(this, e);

        let top_level = TopLevel::get_top_level(Some(this)).expect("the page is attached to a top level");
        *this.notification_manager.borrow_mut() = Some(WindowNotificationManager::with_host(Some(&top_level)));
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        let timer = this.clipboard_last_data_object_checker.borrow().clone();
        if let Some(timer) = timer {
            timer.stop();
        }
        Self::parent_on_detached_from_visual_tree(this, e);
    }
}

/// The result of a clipboard operation awaited by an `async void` handler:
/// an exception of such a method is raised on the dispatcher.
///
/// # Panics
/// Panics if the operation failed.
fn raise<T>(result: Result<T, ClipboardError>) -> T {
    result.unwrap_or_else(|error| panic!("{error}"))
}

impl ClipboardPage {
    pub fn construct() -> Self {
        Self {
            base: ContentPage::construct(),
            custom_binary_data_format: DataFormat::create_bytes_application_format("controlcatalog-binary-data"),
            notification_manager: RefCell::new(None),
            clipboard_last_data_object_checker: RefCell::new(None),
            stored_data_transfer: RefCell::new(None),
            checking_clipboard_data_transfer: Cell::new(false),
            default_image: RefCell::new(None),
        }
    }

    /// # Panics
    /// Panics if the default image cannot be opened or decoded.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        // The timer is held by the page: its handler holds the page weakly.
        let weak = this.downgrade();
        let timer =
            DispatcherTimer::with_callback(Duration::from_secs_f64(0.5), DispatcherPriority::DEFAULT, move |_| {
                if let Some(this) = weak.upgrade() {
                    this.check_last_data_object();
                }
            });
        timer.set_is_enabled(false);
        *this.clipboard_last_data_object_checker.borrow_mut() = Some(timer);

        let uri = Uri::absolute("ferres://ControlCatalog/Assets/image1.jpg").unwrap_or_else(|e| panic!("{e}"));
        let mut asset = AssetLoader::open(&uri, None).unwrap_or_else(|e| panic!("{e}"));
        let default_image = Rc::new(Bitmap::from_stream(&mut asset).unwrap_or_else(|e| panic!("{e}")));
        *this.default_image.borrow_mut() = Some(default_image.clone());
        let source: Rc<dyn IImage> = default_image;
        this.clipboard_image().set_source(Some(source));

        this
    }

    fn clipboard_content(&self) -> Ref<TextBox> {
        self.get_control::<TextBox>("ClipboardContent")
    }

    fn clipboard_image(&self) -> Ref<Image> {
        self.get_control::<Image>("ClipboardImage")
    }

    fn owns_clipboard_data_object(&self) -> Ref<Run> {
        self.get_control::<Run>("OwnsClipboardDataObject")
    }

    /// `TopLevel.GetTopLevel(this)?.Clipboard`.
    fn clipboard(&self) -> Option<Rc<dyn IClipboard>> {
        TopLevel::get_top_level(Some(self)).and_then(|top_level| top_level.clipboard())
    }

    fn show_notification(&self, title: &str, message: &str, type_: NotificationType) {
        let notification_manager = self.notification_manager.borrow().clone();
        if let Some(notification_manager) = notification_manager {
            notification_manager.show(
                Notification::empty()
                    .with_title(Some(title.to_string()))
                    .with_message(Some(message.to_string()))
                    .with_type(type_),
            );
        }
    }

    /// `async void`.
    fn copy_text(&self, _sender: &Option<BoxedValue>, _args: &RoutedEventArgs) {
        let Some(clipboard) = self.clipboard() else {
            return;
        };
        let text = self.clipboard_content().text().unwrap_or_default();
        let pending = clipboard.set_text_async(Some(&text));
        drop(start_async(async move {
            raise(pending.await);
        }));
    }

    /// `async void`.
    fn copy_image(&self, _sender: &Option<BoxedValue>, _args: &RoutedEventArgs) {
        let Some(clipboard) = self.clipboard() else {
            return;
        };
        let default_image = self.default_image.borrow().clone();
        let pending = clipboard.set_value_async(&DataFormat::bitmap(), default_image);
        drop(start_async(async move {
            raise(pending.await);
        }));
    }

    /// `async void`.
    fn paste_text(&self, _sender: &Option<BoxedValue>, _args: &RoutedEventArgs) {
        let Some(clipboard) = self.clipboard() else {
            return;
        };
        let this = self.to_ref();
        let pending = clipboard.try_get_text_async();
        drop(start_async(async move {
            let text = raise(pending.await);
            this.clipboard_content().set_text(text.as_deref());
        }));
    }

    /// `async void`.
    fn paste_image(&self, _sender: &Option<BoxedValue>, _args: &RoutedEventArgs) {
        let Some(clipboard) = self.clipboard() else {
            return;
        };
        let this = self.to_ref();
        let pending = clipboard.try_get_data_async();
        drop(start_async(async move {
            let data = raise(pending.await);
            let mut source: Option<Rc<Bitmap>> = None;
            if let Some(data) = data {
                // `using var data`: the data transfer is disposed however the read ends.
                let result = data.try_get_value_async(&DataFormat::bitmap()).await;
                data.dispose();
                source = raise(result);
            }
            this.clipboard_image().set_source(source.map(|bitmap| bitmap as Rc<dyn IImage>));
        }));
    }

    /// `async void`.
    fn copy_files(&self, _sender: &Option<BoxedValue>, _args: &RoutedEventArgs) {
        let Some(top_level) = TopLevel::get_top_level(Some(self)) else {
            return;
        };
        let Some(clipboard) = top_level.clipboard() else {
            return;
        };
        let this = self.to_ref();
        drop(start_async(async move {
            let storage_provider = top_level.storage_provider();
            let text = this.clipboard_content().text().unwrap_or_default();
            let files_path: Vec<&str> = text.split(NEW_LINE).filter(|path| !path.is_empty()).collect();
            if files_path.is_empty() {
                return;
            }
            let mut invalid_file: Vec<&str> = Vec::with_capacity(files_path.len());
            let mut files: Vec<Rc<dyn IStorageFile>> = Vec::with_capacity(files_path.len());

            for path in files_path.iter().copied() {
                let file = storage_provider.try_get_file_from_path_str_async(path).await;
                match file {
                    None => invalid_file.push(path),
                    Some(file) => files.push(file),
                }
            }

            if !invalid_file.is_empty() {
                this.show_notification("Warning", "There is one o more invalid path.", NotificationType::Warning);
            }

            if !files.is_empty() {
                let data_transfer = DataTransfer::new();
                *this.stored_data_transfer.borrow_mut() = Some(data_transfer.clone());
                for file in files {
                    let item: Rc<dyn IStorageItem> = file;
                    data_transfer.add(DataTransferItem::create(&DataFormat::file(), Some(item)));
                }
                let data: Rc<dyn IAsyncDataTransfer> = data_transfer;
                raise(clipboard.set_data_async(Some(data)).await);
                this.show_notification("Success", "Copy completed.", NotificationType::Success);
            } else {
                this.show_notification("Warning", "Any files to copy in Clipboard.", NotificationType::Warning);
            }
        }));
    }

    /// `async void`.
    fn paste_files(&self, _sender: &Option<BoxedValue>, _args: &RoutedEventArgs) {
        let Some(clipboard) = self.clipboard() else {
            return;
        };
        let this = self.to_ref();
        let pending = clipboard.try_get_files_async();
        drop(start_async(async move {
            let files = raise(pending.await);

            let text = match files {
                Some(files) => files
                    .iter()
                    .map(|file| file.try_get_local_path().unwrap_or_else(|| file.name()))
                    .collect::<Vec<_>>()
                    .join(NEW_LINE),
                None => String::new(),
            };
            this.clipboard_content().set_text(Some(&text));
        }));
    }

    /// `async void`.
    fn get_formats(&self, _sender: &Option<BoxedValue>, _args: &RoutedEventArgs) {
        let Some(clipboard) = self.clipboard() else {
            return;
        };
        let this = self.to_ref();
        let pending = clipboard.get_data_formats_async();
        drop(start_async(async move {
            let formats = raise(pending.await);
            let text = formats.iter().map(|format| format.to_string()).collect::<Vec<_>>().join(NEW_LINE);
            this.clipboard_content().set_text(Some(&text));
        }));
    }

    /// `async void`.
    fn copy_binary_data(&self, _sender: &Option<BoxedValue>, _args: &RoutedEventArgs) {
        let Some(clipboard) = self.clipboard() else {
            return;
        };
        let data_transfer = DataTransfer::new();
        *self.stored_data_transfer.borrow_mut() = Some(data_transfer.clone());
        let mut bytes = vec![0u8; 10 * 1024 * 1024];
        let mut random = Random::new();
        for byte in &mut bytes {
            *byte = random.next_max(256) as u8;
        }
        let bytes: Rc<[u8]> = Rc::from(bytes);
        data_transfer.add(DataTransferItem::create(&self.custom_binary_data_format, Some(bytes)));
        let data: Rc<dyn IAsyncDataTransfer> = data_transfer;
        let pending = clipboard.set_data_async(Some(data));
        drop(start_async(async move {
            raise(pending.await);
        }));
    }

    /// `async void`.
    fn paste_binary_data(&self, _sender: &Option<BoxedValue>, _args: &RoutedEventArgs) {
        let Some(clipboard) = self.clipboard() else {
            return;
        };
        let this = self.to_ref();
        let pending = clipboard.try_get_value_async(&self.custom_binary_data_format);
        drop(start_async(async move {
            let bytes = raise(pending.await);
            let text = match bytes {
                None => String::from("<null>"),
                Some(bytes) => format!("{} bytes", bytes.len()),
            };
            this.clipboard_content().set_text(Some(&text));
        }));
    }

    /// `async void`.
    fn clear(&self, _sender: &Option<BoxedValue>, _args: &RoutedEventArgs) {
        let Some(clipboard) = self.clipboard() else {
            return;
        };
        let pending = clipboard.clear_async();
        drop(start_async(async move {
            raise(pending.await);
        }));
    }

    /// The handler of the tick of the timer (`async void`).
    fn check_last_data_object(&self) {
        /// The `finally` block: the check is over however it ends.
        struct Checking(Ref<ClipboardPage>);

        impl Drop for Checking {
            fn drop(&mut self) {
                self.0.checking_clipboard_data_transfer.set(false);
            }
        }

        if self.checking_clipboard_data_transfer.get() {
            return;
        }
        self.checking_clipboard_data_transfer.set(true);
        let checking = Checking(self.to_ref());

        let pending = self.clipboard().map(|clipboard| clipboard.try_get_in_process_data_async());
        drop(start_async(async move {
            let this = &checking.0;

            let mut owns = false;
            if let Some(pending) = pending {
                let data_transfer = raise(pending.await);
                let stored_data_transfer = this.stored_data_transfer.borrow().clone();
                owns = match (data_transfer, stored_data_transfer) {
                    (Some(data_transfer), Some(stored_data_transfer)) => {
                        std::ptr::addr_eq(Rc::as_ptr(&data_transfer), Rc::as_ptr(&stored_data_transfer))
                    }
                    _ => false,
                };
            }

            let owns_clipboard_data_object = this.owns_clipboard_data_object();
            owns_clipboard_data_object.set_text(Some(if owns { "Yes" } else { "No" }));
            let foreground: Rc<dyn IBrush> = if owns { Brushes::green() } else { Brushes::red() };
            owns_clipboard_data_object.set_foreground(Some(foreground));
        }));
    }
}
