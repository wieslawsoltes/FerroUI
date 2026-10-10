//! The file dialogs of the desktop, through the file chooser portal (the
//! port of `DBusSystemDialog.cs`).

use crate::dbus_helper::DBusHelper;
use crate::i_portal_parent_lease::IPortalParentLease;
use ferroui_base::input::LocalBoxFuture;
use ferroui_base::platform::storage::file_io::{
    BclStorageFile, BclStorageFolder, BclStorageProvider, FileSystemInfo, StorageProviderHelpers,
};
use ferroui_base::platform::storage::{
    FilePickerFileType, FilePickerOpenOptions, FilePickerSaveOptions, FolderPickerOpenOptions, IStorageFile,
    IStorageFolder, IStorageItem, IStorageProvider, OpenFilePickerResult, SaveFilePickerResult,
};
use ferroui_base::utilities::Uri;
use futures_util::StreamExt;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use zbus::proxy::CacheProperties;
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};
use zbus::Connection;

/// `org.freedesktop.portal.FileChooser` (the proxy the reference
/// generates from `DBusXml/org.freedesktop.portal.FileChooser.xml`), with
/// the members that are called.
#[zbus::proxy(interface = "org.freedesktop.portal.FileChooser", gen_blocking = false, assume_defaults = false)]
pub trait FileChooser {
    fn open_file(
        &self,
        parent_window: &str,
        title: &str,
        options: HashMap<&str, Value<'_>>,
    ) -> zbus::Result<OwnedObjectPath>;

    fn save_file(
        &self,
        parent_window: &str,
        title: &str,
        options: HashMap<&str, Value<'_>>,
    ) -> zbus::Result<OwnedObjectPath>;

    #[zbus(property, name = "version")]
    fn version(&self) -> zbus::Result<u32>;
}

/// `org.freedesktop.portal.Request`: the object a portal call answers
/// through.
#[zbus::proxy(interface = "org.freedesktop.portal.Request", gen_blocking = false, assume_defaults = false)]
pub trait Request {
    #[zbus(signal)]
    fn response(&self, response: u32, results: HashMap<String, OwnedValue>) -> zbus::Result<()>;
}

const PORTAL_NAME: &str = "org.freedesktop.portal.Desktop";
const PORTAL_PATH: &str = "/org/freedesktop/portal/desktop";

const GLOB_STYLE: u32 = 0;
const MIME_STYLE: u32 = 1;

/// A filter as the portal takes it: a name and patterns, each a glob (0)
/// or a MIME type (1).
pub(crate) type PortalFilter = (String, Vec<(u32, String)>);

/// Gives the parent window of a dialog, once per dialog.
pub type ParentLeaseProvider = Rc<dyn Fn() -> LocalBoxFuture<Option<Rc<dyn IPortalParentLease>>>>;

/// The filters of the file types of a dialog, and the one that is
/// suggested (`TryParseFilters`); `None` when the dialog has no file
/// types.
pub(crate) fn try_parse_filters(
    file_types: Option<&[Rc<FilePickerFileType>]>,
    suggested_file_type: Option<&Rc<FilePickerFileType>>,
) -> Option<(Vec<PortalFilter>, Option<PortalFilter>)> {
    // Example: [('Images', [(0, '*.ico'), (1, 'image/png')]), ('Text', [(0, '*.txt')])]
    let file_types = file_types?;

    let mut filters = Vec::new();
    let mut current_filter = None;

    for file_type in file_types {
        let patterns = file_type.patterns().filter(|patterns| !patterns.is_empty());
        let mime_types = file_type.mime_types().filter(|mime_types| !mime_types.is_empty());
        let extensions: Vec<(u32, String)> = if let Some(patterns) = patterns {
            patterns.iter().map(|pattern| (GLOB_STYLE, pattern.clone())).collect()
        } else if let Some(mime_types) = mime_types {
            mime_types.iter().map(|mime_type| (MIME_STYLE, mime_type.clone())).collect()
        } else {
            continue;
        };

        let filter_struct = (file_type.name().to_string(), extensions);
        filters.push(filter_struct.clone());

        if suggested_file_type.is_some_and(|suggested| Rc::ptr_eq(file_type, suggested)) {
            current_filter = Some(filter_struct);
        }
    }

    Some((filters, current_filter))
}

/// The filter the user chose, from the results of a response.
pub(crate) fn parse_current_filter(value: &Value<'_>) -> Option<PortalFilter> {
    fn unwrap<'a, 'v>(mut value: &'a Value<'v>) -> &'a Value<'v> {
        while let Value::Value(inner) = value {
            value = inner;
        }
        value
    }

    let Value::Structure(filter) = unwrap(value) else {
        return None;
    };
    let Some(Value::Str(name)) = filter.fields().first().map(unwrap) else {
        return None;
    };
    let Some(Value::Array(types)) = filter.fields().get(1).map(unwrap) else {
        return None;
    };
    let mut extensions = Vec::new();
    for t in types.iter() {
        let Value::Structure(t) = unwrap(t) else {
            return None;
        };
        match (t.fields().first().map(unwrap), t.fields().get(1).map(unwrap)) {
            (Some(Value::U32(style)), Some(Value::Str(pattern))) => extensions.push((*style, pattern.as_str().to_string())),
            _ => return None,
        }
    }
    Some((name.as_str().to_string(), extensions))
}

/// The file type of a chosen filter: the one of the options it matches,
/// so that the caller finds its own object again, else a new one.
pub(crate) fn selected_file_type(
    current_filter: &PortalFilter,
    file_types: Option<&[Rc<FilePickerFileType>]>,
) -> Rc<FilePickerFileType> {
    let (name, extensions) = current_filter;
    let mime_types: Vec<String> =
        extensions.iter().filter(|(style, _)| *style == MIME_STYLE).map(|(_, value)| value.clone()).collect();
    let patterns: Vec<String> =
        extensions.iter().filter(|(style, _)| *style != MIME_STYLE).map(|(_, value)| value.clone()).collect();

    // Reuse the file type objects from options
    // so the consuming code can match exactly the
    // file type selected instead of spawning one.
    let existing = file_types.and_then(|file_types| {
        file_types.iter().find(|type_| {
            type_.name() == name
                && (type_.mime_types().is_some_and(|own| own.iter().all(|y| mime_types.contains(y)))
                    || type_.patterns().is_some_and(|own| own.iter().all(|y| patterns.contains(y))))
        })
    });
    match existing {
        Some(existing) => existing.clone(),
        None => {
            let type_ = FilePickerFileType::new(Some(name));
            type_.set_mime_types(Some(mime_types));
            type_.set_patterns(Some(patterns));
            type_
        }
    }
}

/// The local paths of the `uris` of the results of a response; none when
/// the results have none (the dialog was canceled).
pub(crate) fn paths_of_results(results: &HashMap<String, OwnedValue>) -> Vec<String> {
    let Some(uris) = results.get("uris") else {
        return Vec::new();
    };
    let Ok(uris) = <Vec<String>>::try_from(uris.try_clone().unwrap_or_else(|_| OwnedValue::from(0u32))) else {
        return Vec::new();
    };
    uris.iter().filter_map(|uri| Uri::absolute(uri).ok()).map(|uri| uri.local_path()).collect()
}

/// The local path of a folder, when it has one.
fn local_path(folder: &Rc<dyn IStorageFolder>) -> Option<String> {
    let item: &dyn IStorageItem = &**folder;
    item.try_get_local_path()
}

fn failure(message: impl Into<String>) -> std::io::Error {
    std::io::Error::other(message.into())
}

fn dbus_failure(error: zbus::Error) -> std::io::Error {
    std::io::Error::other(error.to_string())
}

/// The path of the request object of a call: the portal derives it from
/// the unique name of the caller and the token of the call. A connection
/// without a bus has no name, and its requests have no element for it
/// (an empty element is not a path; the reference never meets the case).
pub(crate) fn request_path(sender: &str, token: &str) -> String {
    if sender.is_empty() {
        format!("/org/freedesktop/portal/desktop/request/{token}")
    } else {
        format!("/org/freedesktop/portal/desktop/request/{sender}/{token}")
    }
}

static TOKEN_COUNTER: AtomicU64 = AtomicU64::new(0);

/// The storage provider over the file chooser portal.
pub struct DBusSystemDialog {
    connection: Connection,
    file_chooser: FileChooserProxy<'static>,
    parent_lease_provider: Option<ParentLeaseProvider>,
    version: u32,
}

#[derive(Clone, Copy)]
enum Show {
    OpenFile,
    SaveFile,
}

impl DBusSystemDialog {
    /// Creates a portal-backed storage provider if xdg-desktop-portal FileChooser is available.
    ///
    /// `parent_lease_provider` is invoked once per picker call to obtain
    /// the parent-window handle to pass to the portal. The returned lease
    /// is held until the picker call completes and then disposed. May
    /// return `None` to call the portal with an empty parent_window string
    /// (per portal spec: "no parent").
    pub async fn try_create_async(parent_lease_provider: Option<ParentLeaseProvider>) -> Option<Rc<dyn IStorageProvider>> {
        let conn = DBusHelper::default_connection()?;
        let dialog = Self::try_create_with_connection_async(conn, parent_lease_provider).await?;
        Some(dialog)
    }

    /// The same over a connection.
    pub async fn try_create_with_connection_async(
        conn: Connection,
        parent_lease_provider: Option<ParentLeaseProvider>,
    ) -> Option<Rc<DBusSystemDialog>> {
        let dbus_file_chooser = FileChooserProxy::builder(&conn)
            .destination(PORTAL_NAME)
            .ok()?
            .path(PORTAL_PATH)
            .ok()?
            .cache_properties(CacheProperties::No)
            .build()
            .await
            .ok()?;

        let version = dbus_file_chooser.version().await.ok()?;

        Some(Rc::new(DBusSystemDialog {
            connection: conn,
            file_chooser: dbus_file_chooser,
            parent_lease_provider,
            version,
        }))
    }

    /// The version of the portal interface.
    pub fn version(&self) -> u32 {
        self.version
    }

    // https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.Request.html
    // Subscribe to the Response signal before making the portal call to avoid a race condition.
    fn create_request_token(connection: &Connection) -> (String, String) {
        let sender = connection
            .unique_name()
            .map(|name| name.as_str().trim_start_matches(':').replace('.', "_"))
            .unwrap_or_default();
        let token = format!(
            "FerroUI_{}_{}",
            std::process::id(),
            TOKEN_COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        (request_path(&sender, &token), token)
    }

    /// Makes a portal call and waits for its response: the request object
    /// is listened to before the call is made, and the call has to answer
    /// with the path of that object.
    async fn call_and_await_response(
        connection: Connection,
        file_chooser: FileChooserProxy<'static>,
        show: Show,
        parent_window: String,
        title: String,
        mut chooser_options: HashMap<&'static str, Value<'static>>,
    ) -> std::io::Result<HashMap<String, OwnedValue>> {
        let (expected_path, token) = Self::create_request_token(&connection);
        chooser_options.insert("handle_token", Value::from(token));

        let request = RequestProxy::builder(&connection)
            .destination(PORTAL_NAME)
            .map_err(dbus_failure)?
            .path(expected_path.clone())
            .map_err(dbus_failure)?
            .cache_properties(CacheProperties::No)
            .build()
            .await
            .map_err(dbus_failure)?;
        let mut responses = request.receive_response().await.map_err(dbus_failure)?;

        let actual_path = match show {
            Show::OpenFile => file_chooser.open_file(&parent_window, &title, chooser_options).await,
            Show::SaveFile => file_chooser.save_file(&parent_window, &title, chooser_options).await,
        }
        .map_err(dbus_failure)?;

        if actual_path.as_str() != expected_path {
            return Err(failure(format!(
                "Portal returned unexpected request path '{}', expected '{expected_path}'.",
                actual_path.as_str()
            )));
        }

        let Some(response) = responses.next().await else {
            return Err(failure("The connection to the portal was closed before it answered."));
        };
        let args = response.args().map_err(dbus_failure)?;
        Ok(args.results)
    }

    #[allow(clippy::too_many_arguments)]
    async fn show_file_picker_async(
        connection: Connection,
        file_chooser: FileChooserProxy<'static>,
        parent_lease_provider: Option<ParentLeaseProvider>,
        suggested_file_type: Option<Rc<FilePickerFileType>>,
        suggested_file_name: Option<String>,
        suggested_start_location: Option<Rc<dyn IStorageFolder>>,
        file_types: Option<Vec<Rc<FilePickerFileType>>>,
        title: Option<String>,
        allow_multiple: Option<bool>,
        show: Show,
    ) -> std::io::Result<(Vec<String>, Option<Rc<FilePickerFileType>>)> {
        let parent_lease = match &parent_lease_provider {
            Some(provider) => provider().await,
            None => None,
        };
        let parent_window = parent_lease.as_ref().map(|lease| lease.handle()).unwrap_or_default();

        let mut chooser_options: HashMap<&'static str, Value<'static>> = HashMap::new();

        if let Some((filters, current_filter)) = try_parse_filters(file_types.as_deref(), suggested_file_type.as_ref()) {
            chooser_options.insert("filters", Value::from(filters));
            if let Some(filter) = current_filter {
                chooser_options.insert("current_filter", Value::from(zbus::zvariant::Structure::from(filter)));
            }
        }

        if let Some(suggested_file_name) = suggested_file_name {
            chooser_options.insert("current_name", Value::from(suggested_file_name));
        }
        if let Some(folder_path) = suggested_start_location.as_ref().and_then(local_path) {
            chooser_options.insert("current_folder", Value::from(format!("{folder_path}\0").into_bytes()));
        }
        if let Some(allow_multiple) = allow_multiple {
            chooser_options.insert("multiple", Value::from(allow_multiple));
        }

        let results =
            Self::call_and_await_response(connection, file_chooser, show, parent_window, title.unwrap_or_default(), chooser_options)
                .await;

        // The lease is held until the call is over, however it ends.
        if let Some(parent_lease) = parent_lease {
            parent_lease.dispose_async().await;
        }
        let results = results?;

        let selected_type = results
            .get("current_filter")
            .and_then(|current_filter| parse_current_filter(current_filter))
            .map(|current_filter| selected_file_type(&current_filter, file_types.as_deref()));

        Ok((paths_of_results(&results), selected_type))
    }
}

impl BclStorageProvider for DBusSystemDialog {
    fn can_open(&self) -> bool {
        true
    }

    fn can_save(&self) -> bool {
        true
    }

    fn can_pick_folder(&self) -> bool {
        self.version >= 3
    }

    fn open_file_picker_with_result_async(
        &self,
        options: FilePickerOpenOptions,
    ) -> LocalBoxFuture<std::io::Result<OpenFilePickerResult>> {
        let picker = Self::show_file_picker_async(
            self.connection.clone(),
            self.file_chooser.clone(),
            self.parent_lease_provider.clone(),
            options.suggested_file_type().cloned(),
            options.suggested_file_name().map(str::to_string),
            options.suggested_start_location().cloned(),
            options.file_type_filter().map(<[_]>::to_vec),
            options.title().map(str::to_string),
            Some(options.allow_multiple()),
            Show::OpenFile,
        );
        Box::pin(async move {
            let (paths, selected_type) = picker.await?;

            let files = paths
                .into_iter()
                .map(|path| BclStorageFile::new(FileSystemInfo::file(path)) as Rc<dyn IStorageFile>)
                .collect();
            Ok(OpenFilePickerResult { files, selected_file_type: selected_type })
        })
    }

    fn save_file_picker_with_result_async(
        &self,
        options: FilePickerSaveOptions,
    ) -> LocalBoxFuture<std::io::Result<SaveFilePickerResult>> {
        let picker = Self::show_file_picker_async(
            self.connection.clone(),
            self.file_chooser.clone(),
            self.parent_lease_provider.clone(),
            options.suggested_file_type().cloned(),
            options.suggested_file_name().map(str::to_string),
            options.suggested_start_location().cloned(),
            options.file_type_choices().map(<[_]>::to_vec),
            options.title().map(str::to_string),
            None,
            Show::SaveFile,
        );
        Box::pin(async move {
            let (paths, selected_type) = picker.await?;
            let path = paths.into_iter().next();

            // WSL2 freedesktop automatically adds extension from selected file type, but we can't pass "default ext". So apply it manually.
            let path = StorageProviderHelpers::name_with_extension(
                path.as_deref(),
                options.default_extension(),
                selected_type.as_deref(),
            );
            let file = path.map(|path| BclStorageFile::new(FileSystemInfo::file(path)) as Rc<dyn IStorageFile>);

            Ok(SaveFilePickerResult { file, selected_file_type: selected_type })
        })
    }

    fn open_folder_picker_async(
        &self,
        options: FolderPickerOpenOptions,
    ) -> LocalBoxFuture<std::io::Result<Vec<Rc<dyn IStorageFolder>>>> {
        if self.version < 3 {
            return Box::pin(std::future::ready(Ok(Vec::new())));
        }

        let (connection, file_chooser, parent_lease_provider) =
            (self.connection.clone(), self.file_chooser.clone(), self.parent_lease_provider.clone());
        Box::pin(async move {
            let parent_lease = match &parent_lease_provider {
                Some(provider) => provider().await,
                None => None,
            };
            let parent_window = parent_lease.as_ref().map(|lease| lease.handle()).unwrap_or_default();

            let mut chooser_options: HashMap<&'static str, Value<'static>> = HashMap::new();
            chooser_options.insert("directory", Value::from(true));
            chooser_options.insert("multiple", Value::from(options.allow_multiple()));

            if let Some(current_name) = options.suggested_file_name() {
                chooser_options.insert("current_name", Value::from(current_name.to_string()));
            }
            if let Some(folder_path) = options.suggested_start_location().and_then(local_path) {
                chooser_options.insert("current_folder", Value::from(format!("{folder_path}\0").into_bytes()));
            }

            let results = Self::call_and_await_response(
                connection,
                file_chooser,
                Show::OpenFile,
                parent_window,
                options.title().unwrap_or_default().to_string(),
                chooser_options,
            )
            .await;
            if let Some(parent_lease) = parent_lease {
                parent_lease.dispose_async().await;
            }

            Ok(paths_of_results(&results?)
                .into_iter()
                // WSL2 freedesktop allows to select files as well in directory picker, filter it out.
                .filter(|path| std::path::Path::new(path).is_dir())
                .map(|path| BclStorageFolder::new(FileSystemInfo::directory(path)) as Rc<dyn IStorageFolder>)
                .collect())
        })
    }
}

#[cfg(test)]
mod tests;
