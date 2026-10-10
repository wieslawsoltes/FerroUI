// Not from the reference, which has no tests of this class. The dialogs
// run here against a double of the file chooser portal that records what
// it is asked and answers through a request object, as the portal does.

use super::*;
use crate::i_portal_parent_lease::TrivialPortalParentLease;
use crate::test_support::{emit_from_service, log, pump_until, scope, Log, TestConnections};
use ferroui_base::threading::Dispatcher;
use std::cell::{Cell, RefCell};
use std::sync::{Arc, Mutex};
use zbus::zvariant::Structure;

struct Answer {
    version: u32,
    response: u32,
    results: Vec<(String, OwnedValue)>,
    /// Answers with the path of another request than the one that was asked for.
    wrong_path: bool,
}

struct Portal {
    answer: Arc<Mutex<Answer>>,
    log: Log,
}

impl Portal {
    async fn show(
        &self,
        method: &str,
        parent_window: &str,
        title: &str,
        options: HashMap<String, OwnedValue>,
        header: zbus::message::Header<'_>,
        connection: &zbus::Connection,
    ) -> OwnedObjectPath {
        let mut keys: Vec<&str> = options.keys().map(String::as_str).collect();
        keys.sort_unstable();
        log(&self.log, format!("{method}({parent_window}, {title}) with {}", keys.join(", ")));
        for key in ["multiple", "directory", "current_name"] {
            if let Some(value) = options.get(key) {
                log(&self.log, format!("{key} = {:?}", &**value));
            }
        }
        if let Some(folder) = options.get("current_folder") {
            let bytes = <Vec<u8>>::try_from(folder.try_clone().unwrap()).unwrap();
            log(&self.log, format!("current_folder = {:?}", String::from_utf8_lossy(&bytes)));
        }
        for key in ["filters", "current_filter"] {
            if let Some(value) = options.get(key) {
                log(&self.log, format!("{key} : {}", value.value_signature()));
            }
        }
        if let Some(filter) = options.get("current_filter").and_then(|value| parse_current_filter(value)) {
            log(&self.log, format!("current_filter = {filter:?}"));
        }

        let token = <String>::try_from(options.get("handle_token").unwrap().try_clone().unwrap()).unwrap();
        let sender = header
            .sender()
            .map(|sender| sender.as_str().trim_start_matches(':').replace('.', "_"))
            .unwrap_or_default();
        let (response, results, wrong_path) = {
            let answer = self.answer.lock().unwrap();
            let results: HashMap<String, OwnedValue> =
                answer.results.iter().map(|(key, value)| (key.clone(), value.try_clone().unwrap())).collect();
            (answer.response, results, answer.wrong_path)
        };
        let path = request_path(&sender, &token);
        emit_from_service(connection, &path, "org.freedesktop.portal.Request", "Response", &(response, results)).await;
        let path = if wrong_path { format!("{path}_other") } else { path };
        OwnedObjectPath::try_from(path).unwrap()
    }
}

#[zbus::interface(name = "org.freedesktop.portal.FileChooser")]
impl Portal {
    async fn open_file(
        &self,
        parent_window: &str,
        title: &str,
        options: HashMap<String, OwnedValue>,
        #[zbus(header)] header: zbus::message::Header<'_>,
        #[zbus(connection)] connection: &zbus::Connection,
    ) -> OwnedObjectPath {
        self.show("OpenFile", parent_window, title, options, header, connection).await
    }

    async fn save_file(
        &self,
        parent_window: &str,
        title: &str,
        options: HashMap<String, OwnedValue>,
        #[zbus(header)] header: zbus::message::Header<'_>,
        #[zbus(connection)] connection: &zbus::Connection,
    ) -> OwnedObjectPath {
        self.show("SaveFile", parent_window, title, options, header, connection).await
    }

    #[zbus(property, name = "version")]
    fn version(&self) -> u32 {
        self.answer.lock().unwrap().version
    }
}

struct Fixture {
    connections: TestConnections,
    answer: Arc<Mutex<Answer>>,
    log: Log,
}

fn uris(uris: &[&str]) -> (String, OwnedValue) {
    let uris: Vec<String> = uris.iter().map(|uri| uri.to_string()).collect();
    ("uris".to_string(), OwnedValue::try_from(Value::from(uris)).unwrap())
}

fn filter_value(name: &str, extensions: &[(u32, &str)]) -> OwnedValue {
    let extensions: Vec<(u32, String)> = extensions.iter().map(|(style, value)| (*style, value.to_string())).collect();
    OwnedValue::try_from(Value::from(Structure::from((name.to_string(), extensions)))).unwrap()
}

fn fixture(version: u32, results: Vec<(String, OwnedValue)>) -> Fixture {
    let log: Log = Arc::default();
    let answer = Arc::new(Mutex::new(Answer { version, response: 0, results, wrong_path: false }));
    let portal = Portal { answer: answer.clone(), log: log.clone() };
    let connections = TestConnections::new(PORTAL_PATH, move |builder| builder.serve_at(PORTAL_PATH, portal));
    connections.start(PORTAL_NAME);
    Fixture { connections, answer, log }
}

/// Runs a future on the dispatcher of the test and gives its result.
fn wait<T: 'static>(future: impl std::future::Future<Output = T> + 'static) -> T {
    let result = Rc::new(RefCell::new(None));
    let sink = result.clone();
    drop(Dispatcher::ui_thread().invoke_async_task_local(move || async move {
        *sink.borrow_mut() = Some(future.await);
    }));
    pump_until(|| result.borrow().is_some());
    let value = result.borrow_mut().take().unwrap();
    value
}

fn concrete(f: &Fixture, parent: Option<ParentLeaseProvider>) -> Rc<DBusSystemDialog> {
    wait(DBusSystemDialog::try_create_with_connection_async(f.connections.client.clone(), parent))
        .expect("the portal answers for its version")
}

/// The dialog as the framework uses it: through the storage provider contract.
fn dialog(f: &Fixture, parent: Option<ParentLeaseProvider>) -> Rc<dyn IStorageProvider> {
    concrete(f, parent)
}

fn take(log: &Log) -> Vec<String> {
    std::mem::take(&mut *log.lock().unwrap())
}

fn local_path_of(file: &Rc<dyn IStorageFile>) -> Option<String> {
    let item: &dyn IStorageItem = &**file;
    item.try_get_local_path()
}

fn file_type(name: &str, patterns: &[&str], mime_types: &[&str]) -> Rc<FilePickerFileType> {
    let type_ = FilePickerFileType::new(Some(name));
    if !patterns.is_empty() {
        type_.set_patterns(Some(patterns.iter().map(|pattern| pattern.to_string()).collect()));
    }
    if !mime_types.is_empty() {
        type_.set_mime_types(Some(mime_types.iter().map(|mime_type| mime_type.to_string()).collect()));
    }
    type_
}

#[test]
fn file_types_become_filters_of_globs_or_of_mime_types() {
    assert_eq!(try_parse_filters(None, None), None);
    assert_eq!(try_parse_filters(Some(&[]), None), Some((vec![], None)));

    let images = file_type("Images", &["*.ico", "*.png"], &["image/png"]);
    let text = file_type("Text", &[], &["text/plain"]);
    let nothing = file_type("Nothing", &[], &[]);
    let types = [images.clone(), nothing, text.clone()];

    let images_filter = ("Images".to_string(), vec![(0, "*.ico".to_string()), (0, "*.png".to_string())]);
    let text_filter = ("Text".to_string(), vec![(1, "text/plain".to_string())]);
    // Patterns win over MIME types; a type with neither is left out.
    assert_eq!(try_parse_filters(Some(&types), None), Some((vec![images_filter.clone(), text_filter.clone()], None)));
    // The suggested type is found by identity, not by its content.
    assert_eq!(
        try_parse_filters(Some(&types), Some(&text)),
        Some((vec![images_filter.clone(), text_filter.clone()], Some(text_filter.clone())))
    );
    let look_alike = file_type("Text", &[], &["text/plain"]);
    assert_eq!(try_parse_filters(Some(&types), Some(&look_alike)).unwrap().1, None);
}

#[test]
fn the_chosen_filter_is_the_file_type_of_the_options_when_it_matches_one() {
    let images = file_type("Images", &["*.ico", "*.png"], &[]);
    let text = file_type("Text", &[], &["text/plain"]);
    let types = [images.clone(), text.clone()];

    let chosen = parse_current_filter(&filter_value("Text", &[(1, "text/plain")])).unwrap();
    assert!(Rc::ptr_eq(&selected_file_type(&chosen, Some(&types)), &text));
    let chosen = parse_current_filter(&filter_value("Images", &[(0, "*.png"), (0, "*.ico")])).unwrap();
    assert!(Rc::ptr_eq(&selected_file_type(&chosen, Some(&types)), &images));

    // Another name, or patterns the type of that name does not have: a new type with what the portal said.
    let chosen = parse_current_filter(&filter_value("Images", &[(0, "*.png")])).unwrap();
    let made = selected_file_type(&chosen, Some(&types));
    assert!(!Rc::ptr_eq(&made, &images));
    assert_eq!(made.name(), "Images");
    assert_eq!(made.patterns().as_deref(), Some(&["*.png".to_string()][..]));
    assert_eq!(made.mime_types().as_deref(), Some(&[][..]));
    let chosen = parse_current_filter(&filter_value("All", &[(0, "*"), (1, "text/x")])).unwrap();
    let made = selected_file_type(&chosen, None);
    assert_eq!((made.name(), made.patterns().unwrap().len(), made.mime_types().unwrap().len()), ("All", 1, 1));

    // Not a filter.
    assert_eq!(parse_current_filter(&Value::from("Text")), None);
}

#[test]
fn without_a_portal_there_is_no_provider() {
    let _scope = scope();
    let connections = TestConnections::new("/org/freedesktop/DBus", Ok);
    assert!(wait(DBusSystemDialog::try_create_with_connection_async(connections.client.clone(), None)).is_none());
}

#[test]
fn folders_can_be_picked_from_version_3_of_the_portal() {
    let _scope = scope();
    let f = fixture(2, vec![uris(&["file:///tmp"])]);
    assert_eq!(concrete(&f, None).version(), 2);
    let old = dialog(&f, None);
    assert!(old.can_open() && old.can_save() && !old.can_pick_folder());
    // Asked anyway: nothing, and the portal is not called.
    assert!(wait(old.open_folder_picker_async(FolderPickerOpenOptions::new())).unwrap().is_empty());
    assert!(take(&f.log).is_empty());

    f.answer.lock().unwrap().version = 4;
    let new = dialog(&f, None);
    assert!(new.can_pick_folder());

    // The portal of WSL lets files be chosen too: only what is a directory comes back.
    let directory = std::env::temp_dir().join(format!("ferroui-freedesktop-folder-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let file = directory.join("file.txt");
    std::fs::write(&file, b"x").unwrap();
    let directory_uri = format!("file://{}", directory.to_string_lossy());
    let file_uri = format!("file://{}", file.to_string_lossy());
    f.answer.lock().unwrap().results = vec![uris(&[&directory_uri, &file_uri])];

    let mut options = FolderPickerOpenOptions::new().with_allow_multiple(true);
    options.set_title(Some("Choose".to_string()));
    options.set_suggested_file_name(Some("here".to_string()));
    let folders = wait(new.open_folder_picker_async(options)).unwrap();
    assert_eq!(folders.len(), 1);
    let item: &dyn IStorageItem = &*folders[0];
    assert_eq!(item.try_get_local_path().map(std::path::PathBuf::from), Some(directory.clone()));
    assert_eq!(
        take(&f.log),
        [
            "OpenFile(, Choose) with current_name, directory, handle_token, multiple",
            "multiple = Bool(true)",
            "directory = Bool(true)",
            "current_name = Str(\"here\")",
        ]
    );
    std::fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn an_open_dialog_sends_its_options_and_gives_the_files_and_the_chosen_type() {
    let _scope = scope();
    let f = fixture(
        4,
        vec![
            uris(&["file:///tmp/a%20b.txt", "file:///x/y.txt"]),
            ("current_filter".to_string(), filter_value("Text", &[(0, "*.txt")])),
        ],
    );
    let leases = Rc::new(Cell::new(0));
    let provider: ParentLeaseProvider = {
        let leases = leases.clone();
        Rc::new(move || {
            leases.set(leases.get() + 1);
            Box::pin(std::future::ready(
                Some(Rc::new(TrivialPortalParentLease::new("x11:1a2b")) as Rc<dyn IPortalParentLease>),
            ))
        })
    };
    let dialog = dialog(&f, Some(provider));

    let text = file_type("Text", &["*.txt"], &[]);
    let images = file_type("Images", &[], &["image/png"]);
    let mut options = FilePickerOpenOptions::new()
        .with_allow_multiple(true)
        .with_file_type_filter(vec![images, text.clone()])
        .with_suggested_file_type(text.clone());
    options.set_title(Some("Open it".to_string()));
    options.set_suggested_file_name(Some("a.txt".to_string()));

    let result = wait(dialog.open_file_picker_with_result_async(options)).unwrap();
    let paths: Vec<Option<String>> = result.files.iter().map(local_path_of).collect();
    assert_eq!(paths, [Some("/tmp/a b.txt".to_string()), Some("/x/y.txt".to_string())]);
    assert!(Rc::ptr_eq(result.selected_file_type.as_ref().unwrap(), &text));
    assert_eq!(leases.get(), 1);
    assert_eq!(
        take(&f.log),
        [
            "OpenFile(x11:1a2b, Open it) with current_filter, current_name, filters, handle_token, multiple",
            "multiple = Bool(true)",
            "current_name = Str(\"a.txt\")",
            "filters : a(sa(us))",
            "current_filter : (sa(us))",
            "current_filter = (\"Text\", [(0, \"*.txt\")])",
        ]
    );

    // Canceled: the portal answers without files.
    {
        let mut answer = f.answer.lock().unwrap();
        answer.response = 1;
        answer.results = vec![];
    }
    let result = wait(dialog.open_file_picker_with_result_async(FilePickerOpenOptions::new())).unwrap();
    assert!(result.files.is_empty());
    assert!(result.selected_file_type.is_none());
    assert_eq!(leases.get(), 2);
    assert_eq!(take(&f.log), ["OpenFile(x11:1a2b, ) with handle_token, multiple", "multiple = Bool(false)"]);
}

#[test]
fn a_save_dialog_gives_one_file_with_the_extension_of_the_chosen_type() {
    let _scope = scope();
    let f = fixture(
        4,
        vec![uris(&["file:///tmp/out"]), ("current_filter".to_string(), filter_value("Text", &[(0, "*.txt")]))],
    );
    let dialog = dialog(&f, None);

    let text = file_type("Text", &["*.txt"], &[]);
    let directory = std::env::temp_dir();
    let start: Rc<dyn IStorageFolder> = BclStorageFolder::new(FileSystemInfo::directory(directory.to_string_lossy().into_owned()));
    let mut options = FilePickerSaveOptions::new().with_file_type_choices(vec![text.clone()]).with_default_extension("txt");
    options.set_suggested_start_location(Some(start));

    let result = wait(dialog.save_file_picker_with_result_async(options)).unwrap();
    assert_eq!(result.file.as_ref().and_then(local_path_of).as_deref(), Some("/tmp/out.txt"));
    assert!(Rc::ptr_eq(result.selected_file_type.as_ref().unwrap(), &text));
    let log = take(&f.log);
    // No "multiple" for a save dialog; the folder is bytes that end with a zero.
    assert_eq!(log[0], "SaveFile(, ) with current_folder, filters, handle_token");
    assert_eq!(log[1], format!("current_folder = {:?}", format!("{}\0", directory.to_string_lossy().trim_end_matches('/'))));

    // Canceled.
    f.answer.lock().unwrap().results = vec![];
    let result = wait(dialog.save_file_picker_with_result_async(FilePickerSaveOptions::new())).unwrap();
    assert!(result.file.is_none());
}

#[test]
fn an_answer_for_another_request_is_a_failure() {
    let _scope = scope();
    let f = fixture(4, vec![uris(&["file:///tmp/a.txt"])]);
    f.answer.lock().unwrap().wrong_path = true;
    let dialog = dialog(&f, None);
    let error = wait(dialog.open_file_picker_with_result_async(FilePickerOpenOptions::new())).err().expect("a failure");
    assert!(error.to_string().starts_with("Portal returned unexpected request path '"), "{error}");
}
