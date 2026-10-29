//! Port of `ViewModels/MainWindowViewModel.cs`.

use super::items::{as_item, boxed, index_of, item_list, items_source, text_list, ItemList};
use super::{Command, ItemViewModel, ViewModelBase};
use ferroui_base::data::core::plugins::ObservableValue;
use ferroui_base::input::{DataTransfer, DataTransferItem, ICommand};
use ferroui_base::interactivity::IRoutedEventArgs;
use ferroui_base::reactive::{IObservable, ObservableError};
use ferroui_base::utilities::{DateTimeOffset, Decimal, Uri};
use ferroui_base::{ferro_markup_type, BoxedValue};
use ferroui_controls::platform::Screen;
use ferroui_controls::{ItemsSource, TextBox};
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::{IObserver, Observable, ObservableExt};
use ferroui_reactiveui::{reactive_object, IReactiveObject, RxSchedulers};
use ferroui_xaml_interactions::{
    delay, ActionTask, AsyncDataTransferRef, StorageFileRef, StorageFolderRef, StorageItemRef,
};
use ferroui_xaml_interactions_custom::ChangeSize;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::Duration;

/// The view model of the main window: the state and the commands most pages bind to (the
/// pages inherit it as their data context).
pub struct MainWindowViewModel {
    base: ViewModelBase,
    this: Weak<MainWindowViewModel>,
    value: Cell<i32>,
    container_sample_counter: Cell<i32>,
    scroll_to_item_subject: Rc<Subject<Option<BoxedValue>>>,
    scroll_to_index_subject: Rc<Subject<i32>>,
    stream_source: RefCell<Rc<Subject<Option<BoxedValue>>>>,
    commands: RefCell<Option<Commands>>,
    count: RefCell<i32>,
    timer_count: RefCell<i32>,
    position: RefCell<f64>,
    items: RefCell<Option<ItemList>>,
    suggestions: RefCell<Option<ItemList>>,
    file_items: RefCell<Option<ItemList>>,
    documents_folder_path: RefCell<Option<String>>,
    new_directory_path: RefCell<Option<String>>,
    documents_folder: RefCell<Option<StorageFolderRef>>,
    upload_file_path: RefCell<String>,
    upload_url: RefCell<String>,
    upload_completed: RefCell<bool>,
    upload_status_message: RefCell<String>,
    active_screen_summary: RefCell<String>,
    active_screen_diagnostics: RefCell<String>,
    scroll_demo_items: RefCell<ItemList>,
    scroll_selected_item: RefCell<Option<String>>,
    scroll_target_index: RefCell<i32>,
    max_scroll_index: RefCell<i32>,
    require_shift_for_horizontal_scroll: RefCell<bool>,
    selected_scroll_change_size: RefCell<ChangeSize>,
    text_validation_error: RefCell<Option<String>>,
    is_render_trigger_enabled: RefCell<bool>,
    task_status_message: RefCell<String>,
    task_completed_indicator: RefCell<bool>,
    sample_task: RefCell<Option<ActionTask>>,
    render_trigger_message: RefCell<String>,
    writeable_bitmap_status_message: RefCell<String>,
    is_writeable_bitmap_timer_enabled: RefCell<bool>,
    writeable_bitmap_timer_interval: RefCell<i32>,
    writeable_bitmap_timer_message: RefCell<String>,
    my_string: RefCell<String>,
    focus_flag: RefCell<bool>,
    is_loading: RefCell<bool>,
    progress: RefCell<f64>,
    greeting: RefCell<String>,
    validated_text: RefCell<String>,
    is_text_valid: RefCell<bool>,
    validated_number: RefCell<Decimal>,
    is_number_valid: RefCell<bool>,
    selected_item: RefCell<Option<Rc<ItemViewModel>>>,
    validated_slider: RefCell<f64>,
    is_slider_valid: RefCell<bool>,
    slider_validation_minimum: RefCell<f64>,
    slider_validation_maximum: RefCell<f64>,
    validated_date: RefCell<Option<DateTimeOffset>>,
    is_date_valid: RefCell<bool>,
    validated_item: RefCell<Option<Rc<ItemViewModel>>>,
    is_item_valid: RefCell<bool>,
    can_use_button_picker_commands: Cell<bool>,
    can_use_menu_picker_commands: Cell<bool>,
    can_use_panel_picker_action_command: Cell<bool>,
    active_screen: RefCell<Option<Rc<Screen>>>,
    container_event_log: ItemList,
    screen_events: ItemList,
    scroll_change_size_options: ItemList,
    values: ObservableValue,
    clipboard_data_transfer: AsyncDataTransferRef,
}
reactive_object!(MainWindowViewModel, base);
super::reactive_accessors!(MainWindowViewModel {
    count, set_count, "Count": i32;
    timer_count, set_timer_count, "TimerCount": i32;
    position, set_position, "Position": f64;
    items, set_items, "Items": Option<ItemList>;
    suggestions, set_suggestions, "Suggestions": Option<ItemList>;
    file_items, set_file_items, "FileItems": Option<ItemList>;
    documents_folder_path, set_documents_folder_path, "DocumentsFolderPath": Option<String>;
    new_directory_path, set_new_directory_path, "NewDirectoryPath": Option<String>;
    documents_folder, set_documents_folder, "DocumentsFolder": Option<StorageFolderRef>;
    upload_file_path, set_upload_file_path, "UploadFilePath": String;
    upload_url, set_upload_url, "UploadUrl": String;
    upload_completed, set_upload_completed, "UploadCompleted": bool;
    upload_status_message, set_upload_status_message, "UploadStatusMessage": String;
    active_screen_summary, set_active_screen_summary, "ActiveScreenSummary": String;
    active_screen_diagnostics, set_active_screen_diagnostics, "ActiveScreenDiagnostics": String;
    scroll_demo_items, set_scroll_demo_items, "ScrollDemoItems": ItemList;
    scroll_selected_item, set_scroll_selected_item, "ScrollSelectedItem": Option<String>;
    scroll_target_index, set_scroll_target_index, "ScrollTargetIndex": i32;
    max_scroll_index, set_max_scroll_index, "MaxScrollIndex": i32;
    require_shift_for_horizontal_scroll, set_require_shift_for_horizontal_scroll, "RequireShiftForHorizontalScroll": bool;
    selected_scroll_change_size, set_selected_scroll_change_size, "SelectedScrollChangeSize": ChangeSize;
    text_validation_error, set_text_validation_error, "TextValidationError": Option<String>;
    is_render_trigger_enabled, set_is_render_trigger_enabled, "IsRenderTriggerEnabled": bool;
    task_status_message, set_task_status_message, "TaskStatusMessage": String;
    task_completed_indicator, set_task_completed_indicator, "TaskCompletedIndicator": bool;
    sample_task, set_sample_task, "SampleTask": Option<ActionTask>;
    render_trigger_message, set_render_trigger_message, "RenderTriggerMessage": String;
    writeable_bitmap_status_message, set_writeable_bitmap_status_message, "WriteableBitmapStatusMessage": String;
    is_writeable_bitmap_timer_enabled, set_is_writeable_bitmap_timer_enabled, "IsWriteableBitmapTimerEnabled": bool;
    writeable_bitmap_timer_interval, set_writeable_bitmap_timer_interval, "WriteableBitmapTimerInterval": i32;
    writeable_bitmap_timer_message, set_writeable_bitmap_timer_message, "WriteableBitmapTimerMessage": String;
    my_string, set_my_string, "MyString": String;
    focus_flag, set_focus_flag, "FocusFlag": bool;
    is_loading, set_is_loading, "IsLoading": bool;
    progress, set_progress, "Progress": f64;
    greeting, set_greeting, "Greeting": String;
    validated_text, set_validated_text, "ValidatedText": String;
    is_text_valid, set_is_text_valid, "IsTextValid": bool;
    validated_number, set_validated_number, "ValidatedNumber": Decimal;
    is_number_valid, set_is_number_valid, "IsNumberValid": bool;
    selected_item, set_selected_item, "SelectedItem": Option<Rc<ItemViewModel>>;
    validated_slider, set_validated_slider, "ValidatedSlider": f64;
    is_slider_valid, set_is_slider_valid, "IsSliderValid": bool;
    slider_validation_minimum, set_slider_validation_minimum, "SliderValidationMinimum": f64;
    slider_validation_maximum, set_slider_validation_maximum, "SliderValidationMaximum": f64;
    validated_date, set_validated_date, "ValidatedDate": Option<DateTimeOffset>;
    is_date_valid, set_is_date_valid, "IsDateValid": bool;
    validated_item, set_validated_item, "ValidatedItem": Option<Rc<ItemViewModel>>;
    is_item_valid, set_is_item_valid, "IsItemValid": bool;
});

/// The commands of the view model (created once the view model exists, as their closures hold it
/// weakly).
struct Commands {
    emit_next: Rc<dyn ICommand>,
    emit_error: Rc<dyn ICommand>,
    complete_stream: Rc<dyn ICommand>,
    reset_stream: Rc<dyn ICommand>,
    data_context_changed: Rc<dyn ICommand>,
    initialize: Rc<dyn ICommand>,
    move_left: Rc<dyn ICommand>,
    move_right: Rc<dyn ICommand>,
    reset_move: Rc<dyn ICommand>,
    open_item: Rc<dyn ICommand>,
    open_files: Rc<dyn ICommand>,
    save_file: Rc<dyn ICommand>,
    open_folders: Rc<dyn ICommand>,
    button_picker_can_execute: Rc<Command>,
    menu_picker_can_execute: Rc<Command>,
    panel_picker_action_can_execute: Rc<Command>,
    get_clipboard_text: Rc<dyn ICommand>,
    get_clipboard_data: Rc<dyn ICommand>,
    get_clipboard_formats: Rc<dyn ICommand>,
    upload_completed: Rc<dyn ICommand>,
    begin_upload: Rc<dyn ICommand>,
    text_changed: Rc<dyn ICommand>,
    scroll_to_item: Rc<dyn ICommand>,
    scroll_to_index: Rc<dyn ICommand>,
    start_sample_task: Rc<dyn ICommand>,
    update_render_timestamp: Rc<dyn ICommand>,
    update_manual_writeable_bitmap_message: Rc<dyn ICommand>,
    update_writeable_bitmap_timer_message: Rc<dyn ICommand>,
}

impl PartialEq for MainWindowViewModel {
    /// A view model is a reference type: two are equal when they are the same object.
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// The current local time as the reference formats it with `T` (`DateTime.Now:T`).
fn now(format: &str) -> String {
    DateTimeOffset::now().to_string_with(format)
}

/// `new ItemViewModel(value, color) { Items = [sub items] }`.
fn item_with_sub_items(value: &str, color: &str) -> Rc<ItemViewModel> {
    let item = ItemViewModel::new(value, color);
    item.set_items(Some(item_list((1..=3).map(|index| {
        ItemViewModel::with_value(&format!("{value} Sub Item {index}"))
    }))));
    item
}

impl MainWindowViewModel {
    /// `MainWindowViewModel()`.
    pub fn new() -> Rc<Self> {
        let clipboard_transfer = DataTransfer::new();
        clipboard_transfer.add(DataTransferItem::create_text(Some(
            "Sample clipboard data object",
        )));

        let scroll_demo_items = text_list([
            "Alpha", "Bravo", "Charlie", "Delta", "Echo", "Foxtrot", "Golf", "Hotel", "India",
            "Juliet", "Kilo", "Lima", "Mike", "November", "Oscar", "Papa", "Quebec", "Romeo",
            "Sierra", "Tango", "Uniform", "Victor", "Whiskey", "X-ray", "Yankee", "Zulu",
        ]);
        let max_scroll_index = (scroll_demo_items.count() as i32 - 1).max(0);
        let scroll_selected_item = scroll_demo_items
            .to_vec()
            .first()
            .and_then(|item| item.downcast_ref::<String>().cloned());

        let documents = documents_folder();
        let view_model = Rc::new_cyclic(|this: &Weak<MainWindowViewModel>| {
            // `Values = Observable.Interval(TimeSpan.FromSeconds(1)).Select(_ => _value++)`.
            let counter = this.clone();
            let values: Rc<dyn IObservable<i32>> = Observable::interval_on(
                Duration::from_secs(1),
                RxSchedulers::main_thread_scheduler(),
            )
            .select(move |_| counter.upgrade().map_or(0, |this| this.next_value()));

            Self {
                base: ViewModelBase::new(),
                this: this.clone(),
                value: Cell::new(0),
                container_sample_counter: Cell::new(1),
                scroll_to_item_subject: Subject::new(),
                scroll_to_index_subject: Subject::new(),
                stream_source: RefCell::new(Subject::new()),
                commands: RefCell::new(None),
                count: RefCell::new(0),
                timer_count: RefCell::new(0),
                position: RefCell::new(100.0),
                items: RefCell::new(Some(item_list([
                    item_with_sub_items("First Item", "Red"),
                    item_with_sub_items("Second Item", "Green"),
                    item_with_sub_items("Third Item", "Blue"),
                    item_with_sub_items("Fourth Item", "Orange"),
                    item_with_sub_items("Fifth Item", "Purple"),
                    item_with_sub_items("Sixth Item", "Pink"),
                ]))),
                suggestions: RefCell::new(Some(text_list([
                    "Apple",
                    "Banana",
                    "Cherry",
                    "Date",
                    "Elderberry",
                    "Fig",
                    "Grape",
                ]))),
                file_items: RefCell::new(Some(item_list::<Uri>([]))),
                documents_folder_path: RefCell::new(documents.clone()),
                new_directory_path: RefCell::new(
                    documents.map(|documents| join_path(&documents, "BehaviorsTestDirectory")),
                ),
                // DocumentsFolder is set to null initially. In real applications, you would obtain
                // an IStorageFolder from a previous picker operation or storage provider.
                documents_folder: RefCell::new(None),
                upload_file_path: RefCell::new(String::new()),
                upload_url: RefCell::new(String::new()),
                upload_completed: RefCell::new(false),
                upload_status_message: RefCell::new(
                    "Provide a file path and upload URL, then choose an upload strategy."
                        .to_string(),
                ),
                active_screen_summary: RefCell::new(
                    "Active screen info will appear once the window is visible.".to_string(),
                ),
                active_screen_diagnostics: RefCell::new(String::new()),
                scroll_demo_items: RefCell::new(scroll_demo_items),
                scroll_selected_item: RefCell::new(scroll_selected_item),
                scroll_target_index: RefCell::new(0),
                max_scroll_index: RefCell::new(max_scroll_index),
                require_shift_for_horizontal_scroll: RefCell::new(false),
                selected_scroll_change_size: RefCell::new(ChangeSize::Line),
                text_validation_error: RefCell::new(None),
                is_render_trigger_enabled: RefCell::new(true),
                task_status_message: RefCell::new("Press Start to run a demo task.".to_string()),
                task_completed_indicator: RefCell::new(false),
                sample_task: RefCell::new(None),
                render_trigger_message: RefCell::new("Waiting for trigger...".to_string()),
                writeable_bitmap_status_message: RefCell::new(
                    "Render not requested yet.".to_string(),
                ),
                is_writeable_bitmap_timer_enabled: RefCell::new(false),
                writeable_bitmap_timer_interval: RefCell::new(500),
                writeable_bitmap_timer_message: RefCell::new("Timer disabled.".to_string()),
                my_string: RefCell::new(String::new()),
                focus_flag: RefCell::new(false),
                is_loading: RefCell::new(true),
                progress: RefCell::new(30.0),
                greeting: RefCell::new("Entered text will appear here".to_string()),
                validated_text: RefCell::new(String::new()),
                is_text_valid: RefCell::new(false),
                validated_number: RefCell::new(Decimal::ZERO),
                is_number_valid: RefCell::new(false),
                selected_item: RefCell::new(None),
                validated_slider: RefCell::new(0.0),
                is_slider_valid: RefCell::new(false),
                slider_validation_minimum: RefCell::new(0.0),
                slider_validation_maximum: RefCell::new(100.0),
                validated_date: RefCell::new(Some(DateTimeOffset::now())),
                is_date_valid: RefCell::new(false),
                validated_item: RefCell::new(None),
                is_item_valid: RefCell::new(false),
                can_use_button_picker_commands: Cell::new(true),
                can_use_menu_picker_commands: Cell::new(true),
                can_use_panel_picker_action_command: Cell::new(true),
                active_screen: RefCell::new(None),
                container_event_log: text_list([]),
                screen_events: text_list([]),
                scroll_change_size_options: Rc::new(
                    ferroui_base::collections::FerroList::from_items([
                        Rc::new(ChangeSize::Line) as BoxedValue,
                        Rc::new(ChangeSize::Page) as BoxedValue,
                    ]),
                ),
                values: ObservableValue::new(values),
                clipboard_data_transfer: AsyncDataTransferRef::new(clipboard_transfer),
            }
        });
        view_model.create_commands();
        view_model.try_prepare_sample_upload_defaults();
        view_model
    }

    /// The view model as a handle, for the closures of its commands.
    fn this(&self) -> Weak<MainWindowViewModel> {
        self.this.clone()
    }

    /// `_value++` of `Values`.
    fn next_value(&self) -> i32 {
        let value = self.value.get();
        self.value.set(value + 1);
        value
    }

    fn create_commands(&self) {
        let this = self.this();
        let with = move |action: fn(&MainWindowViewModel)| {
            let this = this.clone();
            Command::create(move || {
                if let Some(this) = this.upgrade() {
                    action(&this);
                }
            })
        };
        let this = self.this();
        let with_parameter = move |action: fn(&MainWindowViewModel, Option<&BoxedValue>)| {
            let this = this.clone();
            Command::create_with(move |parameter| {
                if let Some(this) = this.upgrade() {
                    action(&this, parameter);
                }
            })
        };
        let this = self.this();
        let picker = move |can_execute: fn(&MainWindowViewModel) -> bool| {
            let (execute, query) = (this.clone(), this.clone());
            Command::new(
                move |parameter| {
                    if let Some(this) = execute.upgrade() {
                        this.handle_picker_can_execute_result(parameter);
                    }
                },
                move || query.upgrade().is_some_and(|this| can_execute(&this)),
            )
        };

        let commands = Commands {
            emit_next: with(Self::emit_next),
            emit_error: with(Self::emit_error),
            complete_stream: with(Self::complete_stream),
            reset_stream: with(Self::reset_stream),
            data_context_changed: with(|_| Self::data_context_changed()),
            initialize: with(|_| Self::initialize()),
            move_left: with(|this| this.set_position(this.position() - 5.0)),
            move_right: with(|this| this.set_position(this.position() + 5.0)),
            reset_move: with(|this| this.set_position(100.0)),
            open_item: with_parameter(|_, parameter| {
                if let Some(item) = parameter.and_then(as_item::<ItemViewModel>) {
                    Self::open_item(&item);
                }
            }),
            open_files: with_parameter(Self::open_files),
            save_file: with_parameter(Self::save_file),
            open_folders: with_parameter(Self::open_folders),
            button_picker_can_execute: picker(|this| this.can_use_button_picker_commands.get()),
            menu_picker_can_execute: picker(|this| this.can_use_menu_picker_commands.get()),
            panel_picker_action_can_execute: picker(|this| {
                this.can_use_panel_picker_action_command.get()
            }),
            get_clipboard_text: with_parameter(|_, text| {
                Self::get_clipboard_text(
                    text.and_then(|text| text.downcast_ref::<String>().cloned()),
                )
            }),
            get_clipboard_data: with_parameter(|_, data| Self::get_clipboard_data(data)),
            get_clipboard_formats: with_parameter(|_, formats| {
                Self::get_clipboard_formats(formats)
            }),
            upload_completed: with_parameter(Self::on_upload_completed),
            begin_upload: with(Self::begin_upload),
            text_changed: with_parameter(Self::on_text_changed),
            scroll_to_item: with_parameter(|this, item| {
                if let Some(item) = item {
                    this.scroll_to_item_subject.on_next(Some(item.clone()));
                }
            }),
            scroll_to_index: with_parameter(|this, index| {
                let index = index
                    .and_then(|index| index.downcast_ref::<i32>().copied())
                    .unwrap_or_default();
                let clamped = index.clamp(0, this.max_scroll_index());
                this.set_scroll_target_index(clamped);
                this.scroll_to_index_subject.on_next(clamped);
            }),
            start_sample_task: with(Self::start_sample_task_async),
            update_render_timestamp: with(|this| {
                this.set_render_trigger_message(format!("Last trigger tick at {}", now("T")))
            }),
            update_manual_writeable_bitmap_message: with(|this| {
                this.set_writeable_bitmap_status_message(format!(
                    "Manual trigger fired at {}",
                    now("T")
                ))
            }),
            update_writeable_bitmap_timer_message: with(|this| {
                this.set_writeable_bitmap_timer_message(format!("Timer tick at {}", now("T")))
            }),
        };
        *self.commands.borrow_mut() = Some(commands);
    }

    fn command(&self, select: impl Fn(&Commands) -> Rc<dyn ICommand>) -> Rc<dyn ICommand> {
        select(
            self.commands
                .borrow()
                .as_ref()
                .expect("the commands are created with the view model"),
        )
    }

    /// `CanUseButtonPickerCommands`.
    pub fn can_use_button_picker_commands(&self) -> bool {
        self.can_use_button_picker_commands.get()
    }

    /// Sets `CanUseButtonPickerCommands`; the command it enables asks again whether it can execute.
    pub fn set_can_use_button_picker_commands(&self, value: bool) {
        if self.can_use_button_picker_commands.replace(value) != value {
            self.raise_property_changed("CanUseButtonPickerCommands");
            if let Some(commands) = self.commands.borrow().as_ref() {
                commands
                    .button_picker_can_execute
                    .raise_can_execute_changed();
            }
        }
    }

    /// `CanUseMenuPickerCommands`.
    pub fn can_use_menu_picker_commands(&self) -> bool {
        self.can_use_menu_picker_commands.get()
    }

    /// Sets `CanUseMenuPickerCommands`; the command it enables asks again whether it can execute.
    pub fn set_can_use_menu_picker_commands(&self, value: bool) {
        if self.can_use_menu_picker_commands.replace(value) != value {
            self.raise_property_changed("CanUseMenuPickerCommands");
            if let Some(commands) = self.commands.borrow().as_ref() {
                commands.menu_picker_can_execute.raise_can_execute_changed();
            }
        }
    }

    /// `CanUsePanelPickerActionCommand`.
    pub fn can_use_panel_picker_action_command(&self) -> bool {
        self.can_use_panel_picker_action_command.get()
    }

    /// Sets `CanUsePanelPickerActionCommand`; the command it enables asks again whether it can
    /// execute.
    pub fn set_can_use_panel_picker_action_command(&self, value: bool) {
        if self.can_use_panel_picker_action_command.replace(value) != value {
            self.raise_property_changed("CanUsePanelPickerActionCommand");
            if let Some(commands) = self.commands.borrow().as_ref() {
                commands
                    .panel_picker_action_can_execute
                    .raise_can_execute_changed();
            }
        }
    }

    /// `ActiveScreen`.
    pub fn active_screen(&self) -> Option<Rc<Screen>> {
        self.active_screen.borrow().clone()
    }

    /// Sets `ActiveScreen` and describes the screen (`OnActiveScreenChanged`).
    pub fn set_active_screen(&self, value: Option<Rc<Screen>>) {
        if *self.active_screen.borrow() == value {
            return;
        }
        *self.active_screen.borrow_mut() = value.clone();
        self.raise_property_changed("ActiveScreen");
        self.on_active_screen_changed(value.as_deref());
    }

    /// `ContainerEventLog`.
    pub fn container_event_log(&self) -> ItemList {
        self.container_event_log.clone()
    }

    /// `ScreenEvents`.
    pub fn screen_events(&self) -> ItemList {
        self.screen_events.clone()
    }

    /// `Values`: a number every second.
    pub fn values(&self) -> ObservableValue {
        self.values.clone()
    }

    /// `StreamSource`.
    pub fn stream_source(&self) -> Rc<dyn IObservable<Option<BoxedValue>>> {
        self.stream_source.borrow().as_observable()
    }

    /// `ScrollToItemObservable`.
    pub fn scroll_to_item_observable(&self) -> Rc<dyn IObservable<Option<BoxedValue>>> {
        self.scroll_to_item_subject.as_observable()
    }

    /// `ScrollToIndexObservable`.
    pub fn scroll_to_index_observable(&self) -> Rc<dyn IObservable<i32>> {
        self.scroll_to_index_subject.as_observable()
    }

    /// `ScrollChangeSizeOptions`.
    pub fn scroll_change_size_options(&self) -> ItemList {
        self.scroll_change_size_options.clone()
    }

    /// `ClipboardDataTransfer`.
    pub fn clipboard_data_transfer(&self) -> AsyncDataTransferRef {
        self.clipboard_data_transfer.clone()
    }

    fn data_context_changed() {
        println!("DataContextChanged");
    }

    fn initialize() {
        println!("InitializeCommand");
    }

    fn open_item(item: &ItemViewModel) {
        println!("OpenItemCommand: {}", item.value().unwrap_or_default());
    }

    fn add_file_item(&self, uri: Uri) {
        if let Some(file_items) = self.file_items() {
            file_items.add(Rc::new(uri) as BoxedValue);
        }
    }

    /// `OpenFiles(IEnumerable<IStorageItem>)`: the picked files (or files and folders).
    fn open_files(&self, files: Option<&BoxedValue>) {
        let Some(files) = files else { return };
        if let Some(files) = files.downcast_ref::<Vec<StorageFileRef>>() {
            for file in files {
                println!("OpenFilesCommand: {}, {}", file.name(), file.path());
                self.add_file_item(file.path());
            }
        } else if let Some(items) = files.downcast_ref::<Vec<StorageItemRef>>() {
            for item in items {
                println!("OpenFilesCommand: {}, {}", item.name(), item.path());
                self.add_file_item(item.path());
            }
        }
    }

    /// `SaveFile(Uri)`: the picked file (its path).
    fn save_file(&self, file: Option<&BoxedValue>) {
        let Some(file) = file else { return };
        let uri = match (
            file.downcast_ref::<Uri>(),
            file.downcast_ref::<StorageFileRef>(),
        ) {
            (Some(uri), _) => uri.clone(),
            (None, Some(file)) => file.path(),
            (None, None) => return,
        };
        println!("SaveFileCommand: {uri}");
        self.add_file_item(uri);
    }

    /// `OpenFolders(IEnumerable<IStorageFolder>)`: the picked folders.
    fn open_folders(&self, folders: Option<&BoxedValue>) {
        let Some(folders) =
            folders.and_then(|folders| folders.downcast_ref::<Vec<StorageFolderRef>>())
        else {
            return;
        };
        for folder in folders {
            println!("OpenFoldersCommand: {}, {}", folder.name(), folder.path());
            self.add_file_item(folder.path());

            // Set the first folder as DocumentsFolder to demonstrate SuggestedStartLocation usage
            if self.documents_folder().is_none() {
                self.set_documents_folder(Some(folder.clone()));
            }
        }
    }

    /// `HandlePickerCanExecuteResult(object?)`.
    fn handle_picker_can_execute_result(&self, result: Option<&BoxedValue>) {
        let Some(value) = result else { return };
        if value.downcast_ref::<Vec<StorageFolderRef>>().is_some() {
            self.open_folders(result);
        } else if value.downcast_ref::<Vec<StorageFileRef>>().is_some()
            || value.downcast_ref::<Vec<StorageItemRef>>().is_some()
        {
            self.open_files(result);
        } else if value.downcast_ref::<Uri>().is_some()
            || value.downcast_ref::<StorageFileRef>().is_some()
        {
            self.save_file(result);
        } else if let Some(item) = value.downcast_ref::<StorageItemRef>() {
            self.add_file_item(item.path());
        }
    }

    fn get_clipboard_text(text: Option<String>) {
        println!("GetClipboardTextCommand: {}", text.unwrap_or_default());
    }

    fn get_clipboard_data(data: Option<&BoxedValue>) {
        println!(
            "GetClipboardDataCommand: {}",
            ferroui_base::data::core::ValueTypes::to_display_string(data)
        );
    }

    fn get_clipboard_formats(formats: Option<&BoxedValue>) {
        let Some(formats) = formats else { return };
        let names: Option<Vec<String>> =
            formats.downcast_ref::<Vec<String>>().cloned().or_else(|| {
                formats
                    .downcast_ref::<Vec<ferroui_base::input::DataFormat>>()
                    .map(|formats| formats.iter().map(|format| format.to_string()).collect())
            });
        if let Some(names) = names {
            println!("GetClipboardFormatsCommand: {}", names.join(","));
        }
    }

    /// `OnUploadCompleted(HttpResponseMessage)`.
    ///
    /// Deviation (DEVIATIONS.md, Behaviors gallery): the upload behaviors of the port send no
    /// request (the framework has no HTTP client), so the response is any value, described as text.
    fn on_upload_completed(&self, response: Option<&BoxedValue>) {
        let status = ferroui_base::data::core::ValueTypes::to_display_string(response);
        println!("UploadCompleted: {status}");
        self.set_upload_completed(true);
        self.set_upload_status_message(
            format!("Upload completed with status {status}")
                .trim()
                .to_string(),
        );
    }

    /// `IncrementCount()`.
    pub fn increment_count(&self) {
        self.set_count(self.count() + 1);
    }

    /// `DecrementCount(object? sender, object parameter)`.
    pub fn decrement_count(&self, _sender: Option<BoxedValue>, _parameter: Option<BoxedValue>) {
        self.set_count(self.count() - 1);
    }

    /// `IncrementTimerCount()`.
    pub fn increment_timer_count(&self) {
        self.set_timer_count(self.timer_count() + 1);
    }

    /// `DecrementTimerCount(object? sender, object parameter)`.
    pub fn decrement_timer_count(
        &self,
        _sender: Option<BoxedValue>,
        _parameter: Option<BoxedValue>,
    ) {
        self.set_timer_count(self.timer_count() - 1);
    }

    /// `OnTextChanged(TextChangedEventArgs)`: the text of the text box that changed.
    fn on_text_changed(&self, args: Option<&BoxedValue>) {
        let source = args
            .and_then(|args| args.downcast_ref::<Rc<dyn IRoutedEventArgs>>())
            .and_then(|args| args.as_routed_event_args().source())
            .and_then(|source| source.cast::<TextBox>());
        if let Some(control) = source {
            self.set_greeting(control.text().unwrap_or_default());
        }
    }

    /// `StartSampleTaskAsync()`: a task of a second and a half, which the `TaskCompletedTrigger`
    /// of the page watches.
    fn start_sample_task_async(&self) {
        self.set_task_completed_indicator(false);
        self.set_task_status_message("Task running...".to_string());

        let work = ActionTask::spawn(async {
            delay(Duration::from_millis(1500)).await;
        });
        self.set_sample_task(Some(work));
        // TaskCompletedTrigger will update state via actions when the task finishes.
    }

    fn begin_upload(&self) {
        self.set_upload_completed(false);
        self.set_upload_status_message(
            "Uploading... (check console output for HTTP response)".to_string(),
        );
    }

    fn try_prepare_sample_upload_defaults(&self) {
        match prepare_sample_upload_file() {
            Ok(path) => self.set_upload_file_path(path),
            Err(message) => {
                println!("Unable to provision sample upload file: {message}");
                self.set_upload_file_path(String::new());
            }
        }

        if self.upload_url().trim().is_empty() {
            self.set_upload_url("https://httpbin.org/post".to_string());
        }
    }

    /// `OnContainerPreparing(object? sender, object? parameter)`.
    pub fn on_container_preparing(
        &self,
        _sender: Option<BoxedValue>,
        parameter: Option<BoxedValue>,
    ) {
        self.append_container_log("PreparingContainer", parameter.as_ref());
    }

    /// `OnContainerPrepared(object? sender, object? parameter)`.
    pub fn on_container_prepared(
        &self,
        _sender: Option<BoxedValue>,
        parameter: Option<BoxedValue>,
    ) {
        self.append_container_log("ContainerPrepared", parameter.as_ref());
    }

    /// `OnContainerClearing(object? sender, object? parameter)`.
    pub fn on_container_clearing(
        &self,
        _sender: Option<BoxedValue>,
        parameter: Option<BoxedValue>,
    ) {
        self.append_container_log("ContainerClearing", parameter.as_ref());
    }

    /// `OnContainerIndexChanged(object? sender, object? parameter)`.
    pub fn on_container_index_changed(
        &self,
        _sender: Option<BoxedValue>,
        parameter: Option<BoxedValue>,
    ) {
        self.append_container_log("ContainerIndexChanged", parameter.as_ref());
    }

    /// `ClearContainerLog()`.
    pub fn clear_container_log(&self) {
        self.container_event_log.clear();
    }

    /// `AddSampleItem()`.
    pub fn add_sample_item(&self) {
        let items = match self.items() {
            Some(items) => items,
            None => {
                let items = item_list::<ItemViewModel>([]);
                self.set_items(Some(items.clone()));
                items
            }
        };

        let counter = self.container_sample_counter.get();
        self.container_sample_counter.set(counter + 1);
        let name = format!("Generated {counter}");
        let color = "Generated";
        let item = ItemViewModel::new(&name, color);
        items.add(boxed(&item));
        self.set_selected_item(Some(item));
        self.set_max_scroll_index((self.scroll_demo_items().count() as i32 - 1).max(0));
        self.set_scroll_target_index(self.scroll_target_index().clamp(0, self.max_scroll_index()));
    }

    /// `RemoveSelectedItem()`.
    pub fn remove_selected_item(&self) {
        let (Some(items), Some(selected_item)) = (self.items(), self.selected_item()) else {
            return;
        };

        let Some(index) = index_of(&items, &selected_item) else {
            return;
        };
        items.remove_at(index);
        let count = items.count();
        self.set_selected_item(if count > 0 {
            as_item::<ItemViewModel>(&items.get(index.min(count - 1)))
        } else {
            None
        });
        self.set_max_scroll_index((self.scroll_demo_items().count() as i32 - 1).max(0));
        self.set_scroll_target_index(self.scroll_target_index().clamp(0, self.max_scroll_index()));
    }

    fn append_container_log(&self, category: &str, parameter: Option<&BoxedValue>) {
        let details = describe_container_parameter(parameter);
        let message = format!(
            "{} - {category}{}",
            now("HH:mm:ss"),
            if details.trim().is_empty() {
                String::new()
            } else {
                format!(": {details}")
            }
        );
        self.container_event_log
            .insert(0, Rc::new(message) as BoxedValue);

        if self.container_event_log.count() > 50 {
            self.container_event_log
                .remove_at(self.container_event_log.count() - 1);
        }
    }

    /// `OnScreensChanged(object? sender, object? parameter)`.
    pub fn on_screens_changed(&self, _sender: Option<BoxedValue>, _parameter: Option<BoxedValue>) {
        self.append_screen_log("ScreensChanged");
    }

    /// `ClearScreenEvents()`.
    pub fn clear_screen_events(&self) {
        self.screen_events.clear();
    }

    fn append_screen_log(&self, category: &str) {
        let entry = format!("{} - {category}", now("T"));
        self.screen_events.insert(0, Rc::new(entry) as BoxedValue);
        if self.screen_events.count() > 50 {
            self.screen_events.remove_at(self.screen_events.count() - 1);
        }
    }

    /// `OnActiveScreenChanged(Screen?)`.
    fn on_active_screen_changed(&self, value: Option<&Screen>) {
        let Some(value) = value else {
            self.set_active_screen_summary(
                "Active screen info will appear once the window is visible.".to_string(),
            );
            self.set_active_screen_diagnostics(String::new());
            return;
        };

        let label = if value.is_primary() {
            "Primary screen"
        } else {
            "Secondary screen"
        };
        self.set_active_screen_summary(label.to_string());

        let scaling = format!("{:.2}", value.scaling());
        let scaling = scaling.trim_end_matches('0').trim_end_matches('.');
        self.set_active_screen_diagnostics(format!(
            "Bounds: {:?}\nWorking area: {:?}\nScaling: {scaling}",
            value.bounds(),
            value.working_area()
        ));
    }

    /// `LoadDataAsync()`: the progress of a load in five steps.
    ///
    /// Deviation (DEVIATIONS.md, Behaviors gallery): the method starts the load and returns; the
    /// task of the reference is the task of the dispatcher it runs on.
    pub fn load_data_async(&self) {
        let this = self.this();
        let _task = ActionTask::spawn(async move {
            let Some(view_model) = this.upgrade() else {
                return;
            };
            view_model.set_is_loading(true);
            view_model.set_progress(0.0);
            drop(view_model);

            for i in (0..=100).step_by(20) {
                delay(Duration::from_millis(100)).await;
                let Some(view_model) = this.upgrade() else {
                    return;
                };
                view_model.set_progress(f64::from(i));
            }

            if let Some(view_model) = this.upgrade() {
                view_model.set_is_loading(false);
            }
        });
    }

    fn emit_next(&self) {
        let value = format!("Value emitted at {}", now("T"));
        self.stream_source
            .borrow()
            .on_next(Some(Rc::new(value) as BoxedValue));
    }

    fn emit_error(&self) {
        self.stream_source.borrow().on_error(simulated_error());
    }

    fn complete_stream(&self) {
        self.stream_source.borrow().on_completed();
    }

    fn reset_stream(&self) {
        *self.stream_source.borrow_mut() = Subject::new();
        self.raise_property_changed("StreamSource");
    }
}

/// `new Exception("Simulated Error")`.
fn simulated_error() -> ObservableError {
    #[derive(Debug)]
    struct SimulatedError;

    impl std::fmt::Display for SimulatedError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("Simulated Error")
        }
    }

    impl std::error::Error for SimulatedError {}

    Rc::new(SimulatedError)
}

/// `DescribeContainerParameter(object?)`: the index, the item and the container of the event
/// arguments of the container events of an items control.
///
/// Deviation (DEVIATIONS.md, Behaviors gallery): the reference reads the properties by reflection;
/// the port reads the event arguments of the container events of the framework.
fn describe_container_parameter(parameter: Option<&BoxedValue>) -> String {
    use ferroui_controls::{
        ContainerClearingEventArgs, ContainerIndexChangedEventArgs, ContainerPreparedEventArgs,
    };

    let Some(args) = parameter else {
        return String::new();
    };
    let context = |container: &ferroui_controls::Control| {
        ferroui_base::data::core::ValueTypes::to_display_string(container.data_context().as_ref())
    };
    let mut parts = Vec::new();
    if let Some(args) = args.downcast_ref::<ContainerPreparedEventArgs>() {
        parts.push(format!("Index={}", args.index()));
        parts.push(format!("Container={}", context(args.container())));
    } else if let Some(args) = args.downcast_ref::<ContainerIndexChangedEventArgs>() {
        parts.push(format!("OldIndex={}", args.old_index()));
        parts.push(format!("NewIndex={}", args.new_index()));
        parts.push(format!("Container={}", context(args.container())));
    } else if let Some(args) = args.downcast_ref::<ContainerClearingEventArgs>() {
        parts.push(format!("Container={}", context(args.container())));
    }
    parts.join(", ")
}

/// `Environment.GetFolderPath(Environment.SpecialFolder.MyDocuments)`.
fn documents_folder() -> Option<String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
        Some(
            std::path::Path::new(&home)
                .join("Documents")
                .to_string_lossy()
                .into_owned(),
        )
    }
    #[cfg(target_arch = "wasm32")]
    {
        None
    }
}

/// `Path.Combine(directory, name)`.
fn join_path(directory: &str, name: &str) -> String {
    std::path::Path::new(directory)
        .join(name)
        .to_string_lossy()
        .into_owned()
}

/// The sample file of the upload behaviors (`TryPrepareSampleUploadDefaults`): a text file in
/// the temporary directory, written once.
fn prepare_sample_upload_file() -> Result<String, String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let path = std::env::temp_dir().join("BehaviorsGalleryUploadSample.txt");
        if !path.exists() {
            std::fs::write(
                &path,
                "Sample upload content generated for the ButtonUploadFileBehavior demo.",
            )
            .map_err(|error| error.to_string())?;
        }
        Ok(path.to_string_lossy().into_owned())
    }
    #[cfg(target_arch = "wasm32")]
    {
        Err("the browser has no files".to_string())
    }
}

type Vm = Rc<MainWindowViewModel>;

ferro_markup_type!(class MainWindowViewModel {
    this: Rc<MainWindowViewModel>,
    handles: [MainWindowViewModel, Rc<MainWindowViewModel>, Option<Rc<MainWindowViewModel>>],
    constructors: [() => MainWindowViewModel::new],
    properties: [
        StreamSource: Rc<dyn IObservable<Option<BoxedValue>>> { get: |this: &Vm| this.stream_source() },
        EmitNextCommand: Rc<dyn ICommand> { get: |this: &Vm| this.command(|c| c.emit_next.clone()) },
        EmitErrorCommand: Rc<dyn ICommand> { get: |this: &Vm| this.command(|c| c.emit_error.clone()) },
        CompleteStreamCommand: Rc<dyn ICommand> { get: |this: &Vm| this.command(|c| c.complete_stream.clone()) },
        ResetStreamCommand: Rc<dyn ICommand> { get: |this: &Vm| this.command(|c| c.reset_stream.clone()) },
        Count: i32 { get: |this: &Vm| this.count(), set: |this: &Vm, value: i32| this.set_count(value) },
        TimerCount: i32 { get: |this: &Vm| this.timer_count(), set: |this: &Vm, value: i32| this.set_timer_count(value) },
        Position: f64 { get: |this: &Vm| this.position(), set: |this: &Vm, value: f64| this.set_position(value) },
        Items: Option<ItemsSource> { get: |this: &Vm| items_source(&this.items()) },
        Suggestions: Option<ItemsSource> { get: |this: &Vm| items_source(&this.suggestions()) },
        FileItems: Option<ItemsSource> { get: |this: &Vm| items_source(&this.file_items()) },
        DocumentsFolderPath: Option<String> {
            get: |this: &Vm| this.documents_folder_path(),
            set: |this: &Vm, value: Option<String>| this.set_documents_folder_path(value)
        },
        NewDirectoryPath: Option<String> {
            get: |this: &Vm| this.new_directory_path(),
            set: |this: &Vm, value: Option<String>| this.set_new_directory_path(value)
        },
        DocumentsFolder: Option<StorageFolderRef> {
            get: |this: &Vm| this.documents_folder(),
            set: |this: &Vm, value: Option<StorageFolderRef>| this.set_documents_folder(value)
        },
        CanUseButtonPickerCommands: bool {
            get: |this: &Vm| this.can_use_button_picker_commands(),
            set: |this: &Vm, value: bool| this.set_can_use_button_picker_commands(value)
        },
        CanUseMenuPickerCommands: bool {
            get: |this: &Vm| this.can_use_menu_picker_commands(),
            set: |this: &Vm, value: bool| this.set_can_use_menu_picker_commands(value)
        },
        CanUsePanelPickerActionCommand: bool {
            get: |this: &Vm| this.can_use_panel_picker_action_command(),
            set: |this: &Vm, value: bool| this.set_can_use_panel_picker_action_command(value)
        },
        UploadFilePath: String {
            get: |this: &Vm| this.upload_file_path(),
            set: |this: &Vm, value: String| this.set_upload_file_path(value)
        },
        UploadUrl: String { get: |this: &Vm| this.upload_url(), set: |this: &Vm, value: String| this.set_upload_url(value) },
        UploadCompleted: bool {
            get: |this: &Vm| this.upload_completed(),
            set: |this: &Vm, value: bool| this.set_upload_completed(value)
        },
        UploadStatusMessage: String {
            get: |this: &Vm| this.upload_status_message(),
            set: |this: &Vm, value: String| this.set_upload_status_message(value)
        },
        ActiveScreen: Option<Rc<Screen>> {
            get: |this: &Vm| this.active_screen(),
            set: |this: &Vm, value: Option<Rc<Screen>>| this.set_active_screen(value)
        },
        ActiveScreenSummary: String {
            get: |this: &Vm| this.active_screen_summary(),
            set: |this: &Vm, value: String| this.set_active_screen_summary(value)
        },
        ActiveScreenDiagnostics: String {
            get: |this: &Vm| this.active_screen_diagnostics(),
            set: |this: &Vm, value: String| this.set_active_screen_diagnostics(value)
        },
        ScrollDemoItems: ItemsSource { get: |this: &Vm| ItemsSource::from(this.scroll_demo_items()) },
        ScrollSelectedItem: Option<String> {
            get: |this: &Vm| this.scroll_selected_item(),
            set: |this: &Vm, value: Option<String>| this.set_scroll_selected_item(value)
        },
        ScrollTargetIndex: i32 {
            get: |this: &Vm| this.scroll_target_index(),
            set: |this: &Vm, value: i32| this.set_scroll_target_index(value)
        },
        MaxScrollIndex: i32 { get: |this: &Vm| this.max_scroll_index(), set: |this: &Vm, value: i32| this.set_max_scroll_index(value) },
        ScrollToItemObservable: Rc<dyn IObservable<Option<BoxedValue>>> { get: |this: &Vm| this.scroll_to_item_observable() },
        ScrollToIndexObservable: Rc<dyn IObservable<i32>> { get: |this: &Vm| this.scroll_to_index_observable() },
        RequireShiftForHorizontalScroll: bool {
            get: |this: &Vm| this.require_shift_for_horizontal_scroll(),
            set: |this: &Vm, value: bool| this.set_require_shift_for_horizontal_scroll(value)
        },
        SelectedScrollChangeSize: ChangeSize {
            get: |this: &Vm| this.selected_scroll_change_size(),
            set: |this: &Vm, value: ChangeSize| this.set_selected_scroll_change_size(value)
        },
        ScrollChangeSizeOptions: ItemsSource { get: |this: &Vm| ItemsSource::from(this.scroll_change_size_options()) },
        TextValidationError: Option<String> {
            get: |this: &Vm| this.text_validation_error(),
            set: |this: &Vm, value: Option<String>| this.set_text_validation_error(value)
        },
        IsRenderTriggerEnabled: bool {
            get: |this: &Vm| this.is_render_trigger_enabled(),
            set: |this: &Vm, value: bool| this.set_is_render_trigger_enabled(value)
        },
        TaskStatusMessage: String {
            get: |this: &Vm| this.task_status_message(),
            set: |this: &Vm, value: String| this.set_task_status_message(value)
        },
        TaskCompletedIndicator: bool {
            get: |this: &Vm| this.task_completed_indicator(),
            set: |this: &Vm, value: bool| this.set_task_completed_indicator(value)
        },
        SampleTask: Option<ActionTask> {
            get: |this: &Vm| this.sample_task(),
            set: |this: &Vm, value: Option<ActionTask>| this.set_sample_task(value)
        },
        RenderTriggerMessage: String {
            get: |this: &Vm| this.render_trigger_message(),
            set: |this: &Vm, value: String| this.set_render_trigger_message(value)
        },
        WriteableBitmapStatusMessage: String {
            get: |this: &Vm| this.writeable_bitmap_status_message(),
            set: |this: &Vm, value: String| this.set_writeable_bitmap_status_message(value)
        },
        IsWriteableBitmapTimerEnabled: bool {
            get: |this: &Vm| this.is_writeable_bitmap_timer_enabled(),
            set: |this: &Vm, value: bool| this.set_is_writeable_bitmap_timer_enabled(value)
        },
        WriteableBitmapTimerInterval: i32 {
            get: |this: &Vm| this.writeable_bitmap_timer_interval(),
            set: |this: &Vm, value: i32| this.set_writeable_bitmap_timer_interval(value)
        },
        WriteableBitmapTimerMessage: String {
            get: |this: &Vm| this.writeable_bitmap_timer_message(),
            set: |this: &Vm, value: String| this.set_writeable_bitmap_timer_message(value)
        },
        ContainerEventLog: ItemsSource { get: |this: &Vm| ItemsSource::from(this.container_event_log()) },
        ScreenEvents: ItemsSource { get: |this: &Vm| ItemsSource::from(this.screen_events()) },
        MyString: String { get: |this: &Vm| this.my_string(), set: |this: &Vm, value: String| this.set_my_string(value) },
        FocusFlag: bool { get: |this: &Vm| this.focus_flag(), set: |this: &Vm, value: bool| this.set_focus_flag(value) },
        IsLoading: bool { get: |this: &Vm| this.is_loading(), set: |this: &Vm, value: bool| this.set_is_loading(value) },
        Progress: f64 { get: |this: &Vm| this.progress(), set: |this: &Vm, value: f64| this.set_progress(value) },
        Greeting: String { get: |this: &Vm| this.greeting(), set: |this: &Vm, value: String| this.set_greeting(value) },
        ValidatedText: String {
            get: |this: &Vm| this.validated_text(),
            set: |this: &Vm, value: String| this.set_validated_text(value)
        },
        IsTextValid: bool { get: |this: &Vm| this.is_text_valid(), set: |this: &Vm, value: bool| this.set_is_text_valid(value) },
        ValidatedNumber: Decimal {
            get: |this: &Vm| this.validated_number(),
            set: |this: &Vm, value: Decimal| this.set_validated_number(value)
        },
        IsNumberValid: bool { get: |this: &Vm| this.is_number_valid(), set: |this: &Vm, value: bool| this.set_is_number_valid(value) },
        SelectedItem: Option<Rc<ItemViewModel>> {
            get: |this: &Vm| this.selected_item(),
            set: |this: &Vm, value: Option<Rc<ItemViewModel>>| this.set_selected_item(value)
        },
        ValidatedSlider: f64 {
            get: |this: &Vm| this.validated_slider(),
            set: |this: &Vm, value: f64| this.set_validated_slider(value)
        },
        IsSliderValid: bool { get: |this: &Vm| this.is_slider_valid(), set: |this: &Vm, value: bool| this.set_is_slider_valid(value) },
        SliderValidationMinimum: f64 {
            get: |this: &Vm| this.slider_validation_minimum(),
            set: |this: &Vm, value: f64| this.set_slider_validation_minimum(value)
        },
        SliderValidationMaximum: f64 {
            get: |this: &Vm| this.slider_validation_maximum(),
            set: |this: &Vm, value: f64| this.set_slider_validation_maximum(value)
        },
        ValidatedDate: Option<DateTimeOffset> {
            get: |this: &Vm| this.validated_date(),
            set: |this: &Vm, value: Option<DateTimeOffset>| this.set_validated_date(value)
        },
        IsDateValid: bool { get: |this: &Vm| this.is_date_valid(), set: |this: &Vm, value: bool| this.set_is_date_valid(value) },
        ValidatedItem: Option<Rc<ItemViewModel>> {
            get: |this: &Vm| this.validated_item(),
            set: |this: &Vm, value: Option<Rc<ItemViewModel>>| this.set_validated_item(value)
        },
        IsItemValid: bool { get: |this: &Vm| this.is_item_valid(), set: |this: &Vm, value: bool| this.set_is_item_valid(value) },
        Values: ObservableValue { get: |this: &Vm| this.values() },
        DataContextChangedCommand: Rc<dyn ICommand> { get: |this: &Vm| this.command(|c| c.data_context_changed.clone()) },
        InitializeCommand: Rc<dyn ICommand> { get: |this: &Vm| this.command(|c| c.initialize.clone()) },
        MoveLeftCommand: Rc<dyn ICommand> { get: |this: &Vm| this.command(|c| c.move_left.clone()) },
        MoveRightCommand: Rc<dyn ICommand> { get: |this: &Vm| this.command(|c| c.move_right.clone()) },
        ResetMoveCommand: Rc<dyn ICommand> { get: |this: &Vm| this.command(|c| c.reset_move.clone()) },
        OpenItemCommand: Rc<dyn ICommand> { get: |this: &Vm| this.command(|c| c.open_item.clone()) },
        OpenFilesCommand: Rc<dyn ICommand> { get: |this: &Vm| this.command(|c| c.open_files.clone()) },
        SaveFileCommand: Rc<dyn ICommand> { get: |this: &Vm| this.command(|c| c.save_file.clone()) },
        BeginUploadCommand: Rc<dyn ICommand> { get: |this: &Vm| this.command(|c| c.begin_upload.clone()) },
        OpenFoldersCommand: Rc<dyn ICommand> { get: |this: &Vm| this.command(|c| c.open_folders.clone()) },
        ButtonPickerCanExecuteCommand: Rc<dyn ICommand> {
            get: |this: &Vm| this.command(|c| c.button_picker_can_execute.clone() as Rc<dyn ICommand>)
        },
        MenuPickerCanExecuteCommand: Rc<dyn ICommand> {
            get: |this: &Vm| this.command(|c| c.menu_picker_can_execute.clone() as Rc<dyn ICommand>)
        },
        PanelPickerActionCanExecuteCommand: Rc<dyn ICommand> {
            get: |this: &Vm| this.command(|c| c.panel_picker_action_can_execute.clone() as Rc<dyn ICommand>)
        },
        GetClipboardTextCommand: Rc<dyn ICommand> { get: |this: &Vm| this.command(|c| c.get_clipboard_text.clone()) },
        GetClipboardDataCommand: Rc<dyn ICommand> { get: |this: &Vm| this.command(|c| c.get_clipboard_data.clone()) },
        GetClipboardFormatsCommand: Rc<dyn ICommand> { get: |this: &Vm| this.command(|c| c.get_clipboard_formats.clone()) },
        UploadCompletedCommand: Rc<dyn ICommand> { get: |this: &Vm| this.command(|c| c.upload_completed.clone()) },
        ClipboardDataTransfer: AsyncDataTransferRef { get: |this: &Vm| this.clipboard_data_transfer() },
        TextChangedCommand: Rc<dyn ICommand> { get: |this: &Vm| this.command(|c| c.text_changed.clone()) },
        ScrollToItemCommand: Rc<dyn ICommand> { get: |this: &Vm| this.command(|c| c.scroll_to_item.clone()) },
        ScrollToIndexCommand: Rc<dyn ICommand> { get: |this: &Vm| this.command(|c| c.scroll_to_index.clone()) },
        StartSampleTaskCommand: Rc<dyn ICommand> { get: |this: &Vm| this.command(|c| c.start_sample_task.clone()) },
        UpdateRenderTimestampCommand: Rc<dyn ICommand> { get: |this: &Vm| this.command(|c| c.update_render_timestamp.clone()) },
        UpdateManualWriteableBitmapMessageCommand: Rc<dyn ICommand> {
            get: |this: &Vm| this.command(|c| c.update_manual_writeable_bitmap_message.clone())
        },
        UpdateWriteableBitmapTimerMessageCommand: Rc<dyn ICommand> {
            get: |this: &Vm| this.command(|c| c.update_writeable_bitmap_timer_message.clone())
        },
    ],
    methods: [
        fn IncrementCount() => |this: &Vm| this.increment_count(),
        fn DecrementCount(Option<BoxedValue>, Option<BoxedValue>) =>
            |this: &Vm, sender: Option<BoxedValue>, parameter: Option<BoxedValue>| this.decrement_count(sender, parameter),
        fn IncrementTimerCount() => |this: &Vm| this.increment_timer_count(),
        fn DecrementTimerCount(Option<BoxedValue>, Option<BoxedValue>) =>
            |this: &Vm, sender: Option<BoxedValue>, parameter: Option<BoxedValue>| this.decrement_timer_count(sender, parameter),
        fn OnContainerPreparing(Option<BoxedValue>, Option<BoxedValue>) =>
            |this: &Vm, sender: Option<BoxedValue>, parameter: Option<BoxedValue>| this.on_container_preparing(sender, parameter),
        fn OnContainerPrepared(Option<BoxedValue>, Option<BoxedValue>) =>
            |this: &Vm, sender: Option<BoxedValue>, parameter: Option<BoxedValue>| this.on_container_prepared(sender, parameter),
        fn OnContainerClearing(Option<BoxedValue>, Option<BoxedValue>) =>
            |this: &Vm, sender: Option<BoxedValue>, parameter: Option<BoxedValue>| this.on_container_clearing(sender, parameter),
        fn OnContainerIndexChanged(Option<BoxedValue>, Option<BoxedValue>) =>
            |this: &Vm, sender: Option<BoxedValue>, parameter: Option<BoxedValue>| this.on_container_index_changed(sender, parameter),
        fn ClearContainerLog() => |this: &Vm| this.clear_container_log(),
        fn AddSampleItem() => |this: &Vm| this.add_sample_item(),
        fn RemoveSelectedItem() => |this: &Vm| this.remove_selected_item(),
        fn OnScreensChanged(Option<BoxedValue>, Option<BoxedValue>) =>
            |this: &Vm, sender: Option<BoxedValue>, parameter: Option<BoxedValue>| this.on_screens_changed(sender, parameter),
        fn ClearScreenEvents() => |this: &Vm| this.clear_screen_events(),
        fn LoadDataAsync() => |this: &Vm| this.load_data_async(),
    ],
    notify_property_changed: MainWindowViewModel,
});
