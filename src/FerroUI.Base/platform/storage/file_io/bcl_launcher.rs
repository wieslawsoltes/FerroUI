use crate::input::LocalBoxFuture;
use crate::logging::{LogEventLevel, Logger};
use crate::platform::storage::{ILauncher, IStorageItem};
use crate::utilities::Uri;
use std::process::{Command, Stdio};
use std::rc::Rc;

/// A launcher that starts the application the operating system associates
/// with a URI or a file, through a process of the system.
///
/// This is an implementation detail of the platform backends.
#[derive(Default)]
pub struct BclLauncher {
    can_open_file_or_directory: Option<Rc<dyn Fn(&str) -> bool>>,
}

impl BclLauncher {
    /// Creates a launcher that opens any file or directory.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a launcher that only opens the local paths `can_open`
    /// accepts (the overridable `CanOpenFileOrDirectory` upstream).
    pub fn with_can_open_file_or_directory(can_open: impl Fn(&str) -> bool + 'static) -> Self {
        Self { can_open_file_or_directory: Some(Rc::new(can_open)) }
    }

    fn can_open_file_or_directory(&self, local_path: &str) -> bool {
        self.can_open_file_or_directory.as_ref().is_none_or(|can_open| can_open(local_path))
    }

    /// The program and the arguments that open `url_or_file` on the
    /// current operating system; `None` where there is no way to do it.
    ///
    /// The argument is passed as it is, as one argument: nothing has to be
    /// escaped.
    pub fn launch_command(url_or_file: &str) -> Option<(&'static str, Vec<String>)> {
        if cfg!(target_os = "linux") {
            // If no associated application/json MimeType is found xdg-open opens return error
            // but it tries to open it anyway using the console editor (nano, vim, other..)
            Some(("xdg-open", vec![url_or_file.to_owned()]))
        } else if cfg!(target_os = "macos") {
            Some(("open", vec![url_or_file.to_owned()]))
        } else if cfg!(target_os = "windows") {
            // Upstream starts the URL or file through the shell of the
            // system; the protocol handler of the shell is the way to do
            // that with a process, without a command interpreter in between.
            Some(("rundll32", vec!["url.dll,FileProtocolHandler".to_owned(), url_or_file.to_owned()]))
        } else {
            None
        }
    }

    fn exec(url_or_file: &str) -> bool {
        let Some((program, arguments)) = Self::launch_command(url_or_file) else {
            return false;
        };

        let child = Command::new(program)
            .args(arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        match child {
            Ok(mut child) => {
                // The process is not waited for by the caller; it is reaped
                // when it exits.
                let _ = std::thread::Builder::new().name("launcher".to_owned()).spawn(move || {
                    let _ = child.wait();
                });
                true
            }
            Err(error) => {
                if let Some(logger) = Logger::try_get(LogEventLevel::Error, "BclLauncher") {
                    logger.log(None, &format!("Exception during BclLauncher.Exec: {error}"));
                }
                false
            }
        }
    }
}

impl ILauncher for BclLauncher {
    fn launch_uri_async(&self, uri: &Uri) -> LocalBoxFuture<bool> {
        let launched = uri.is_absolute_uri() && Self::exec(uri.absolute_uri());
        Box::pin(std::future::ready(launched))
    }

    /// This process based implementation doesn't handle the case, when
    /// there is no app to handle link. It will still return true in this
    /// case.
    fn launch_file_async(&self, storage_item: Rc<dyn IStorageItem>) -> LocalBoxFuture<bool> {
        let launched = match storage_item.try_get_local_path() {
            Some(local_path) if self.can_open_file_or_directory(&local_path) => Self::exec(&local_path),
            _ => false,
        };
        Box::pin(std::future::ready(launched))
    }
}
