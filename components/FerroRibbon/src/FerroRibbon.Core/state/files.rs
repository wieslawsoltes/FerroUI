//! State-file persistence with standard futures and platform filesystem I/O.
use super::*;
use crate::RibbonString;
use ferroui_system::threading::CancellationToken;
use std::{
    fs::File,
    future::Future,
    io::{Read, Write},
    path::{Path, PathBuf},
    pin::Pin,
    rc::Rc,
};

/// Single-consumer observation of a state-file operation. Dropping this future
/// does not cancel its already-started filesystem operation.
pub type RibbonStateFileFuture<T> =
    Pin<Box<dyn Future<Output = Result<T, RibbonStateError>> + Send>>;

fn io_error(error: std::io::Error) -> RibbonStateError {
    let kind = match error.kind() {
        std::io::ErrorKind::PermissionDenied => RibbonStateErrorKind::Unauthorized,
        std::io::ErrorKind::InvalidInput => RibbonStateErrorKind::Argument,
        _ => RibbonStateErrorKind::Io,
    };
    RibbonStateError::new(kind, None, error.to_string())
}
fn open_error(error: std::io::Error) -> RibbonStateError {
    if error.kind() == std::io::ErrorKind::IsADirectory {
        RibbonStateError::new(RibbonStateErrorKind::Unauthorized, None, error.to_string())
    } else {
        io_error(error)
    }
}
fn canceled(token: &CancellationToken) -> Result<(), RibbonStateError> {
    if token.is_cancelled() {
        Err(RibbonStateError::new(
            RibbonStateErrorKind::Canceled,
            None,
            "A state-file operation was canceled.",
        ))
    } else {
        Ok(())
    }
}
#[cfg(not(all(target_os = "emscripten", not(target_feature = "atomics"))))]
fn run_file_operation<T: Send + 'static>(
    worker_name: &str,
    operation: impl FnOnce() -> Result<T, RibbonStateError> + Send + 'static,
) -> RibbonStateFileFuture<T> {
    let (sender, receiver) = futures_channel::oneshot::channel();
    let worker = std::thread::Builder::new()
        .name(worker_name.into())
        .spawn(move || {
            let _ = sender.send(operation());
        });
    match worker {
        Ok(worker) => drop(worker),
        Err(error) => return Box::pin(std::future::ready(Err(io_error(error)))),
    }
    Box::pin(async move {
        receiver.await.map_err(|_| {
            RibbonStateError::new(
                RibbonStateErrorKind::Io,
                None,
                "State-file worker stopped before reporting a result",
            )
        })?
    })
}

// The single-threaded browser runtime's virtual filesystem completes standard
// file calls in place. Finish the operation before returning so its effects
// survive an abandoned completion future, just as on the worker path.
#[cfg(all(target_os = "emscripten", not(target_feature = "atomics")))]
fn run_file_operation<T: Send + 'static>(
    _: &str,
    operation: impl FnOnce() -> Result<T, RibbonStateError>,
) -> RibbonStateFileFuture<T> {
    Box::pin(std::future::ready(operation()))
}

fn absolute(path: &Path) -> Result<PathBuf, RibbonStateError> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        std::env::current_dir()
            .map(|p| p.join(path))
            .map_err(io_error)
    }
}
// These two path operations need the source's read-sharing policy. Native
// Windows share flags and Unix advisory locks stay isolated to this contract;
// this is not a general FileStream/FileShare implementation.
#[cfg(unix)]
fn read_sharing(file: &File) -> std::io::Result<()> {
    match file.try_lock_shared() {
        Ok(()) => Ok(()),
        Err(std::fs::TryLockError::WouldBlock) => Err(std::io::Error::new(
            std::io::ErrorKind::WouldBlock,
            "State file is exclusively locked",
        )),
        // The reference treats advisory locking as best effort on unsupported
        // filesystems. A real conflicting lock, above, must remain an error.
        Err(std::fs::TryLockError::Error(_)) => Ok(()),
    }
}
fn open_read(path: &Path) -> std::io::Result<File> {
    let mut options = File::options();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(1); // FILE_SHARE_READ
    }
    let file = options.open(path)?;
    #[cfg(unix)]
    read_sharing(&file)?;
    Ok(file)
}
fn open_write(path: &Path) -> std::io::Result<File> {
    let mut options = File::options();
    options.write(true).create(true).truncate(false);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(1); // FILE_SHARE_READ
    }
    // Acquire sharing before truncation, so a locked existing .tmp is preserved.
    let file = options.open(path)?;
    #[cfg(unix)]
    read_sharing(&file)?;
    if file.metadata()?.is_file() {
        file.set_len(0)?;
    }
    Ok(file)
}
impl RibbonStateSerializer {
    /// Snapshot, validate and open the temporary file on the caller before
    /// starting the write and replacement.
    pub fn save_async(
        state: Option<&RibbonState>,
        file_path: Option<&Path>,
        token: CancellationToken,
    ) -> RibbonStateFileFuture<()> {
        let prepared = (|| {
            let destination = file_path.ok_or_else(|| {
                RibbonStateError::new(
                    RibbonStateErrorKind::ArgumentNull,
                    Some("path"),
                    "Value cannot be null. (path)",
                )
            })?;
            if destination.as_os_str().is_empty() {
                return Err(RibbonStateError::new(
                    RibbonStateErrorKind::Argument,
                    Some("path"),
                    "The value cannot be an empty path. (path)",
                ));
            }
            let trailing_separator = destination
                .as_os_str()
                .as_encoded_bytes()
                .last()
                .is_some_and(|&c| c == b'/' || (cfg!(windows) && c == b'\\'));
            let directory = if trailing_separator {
                Some(destination)
            } else {
                destination.parent()
            };
            if let Some(directory) = directory.filter(|p| !p.as_os_str().is_empty()) {
                std::fs::create_dir_all(directory).map_err(io_error)?;
            }
            let mut temp = destination.as_os_str().to_os_string();
            temp.push(".tmp");
            let temp = PathBuf::from(temp);
            let json = Self::serialize_default(state)?
                .to_utf8()
                .expect("JSON writer emits ASCII")
                .into_bytes();
            canceled(&token)?;
            let file = open_write(&absolute(&temp)?).map_err(open_error)?;
            Ok((file, json, temp, destination.to_path_buf()))
        })();
        let (file, json, temp, destination) = match prepared {
            Ok(prepared) => prepared,
            Err(error) => return Box::pin(std::future::ready(Err(error))),
        };
        run_file_operation("FerroRibbon state save", move || {
            write_bytes(file, &json, &token)?;
            // Source File.Move follows the completed write without checking
            // cancellation again.
            std::fs::rename(temp, &destination).map_err(|error| {
                // Windows reports access denied when the target is a directory.
                // In the source this is a move conflict.
                if error.kind() == std::io::ErrorKind::PermissionDenied && destination.is_dir() {
                    RibbonStateError::new(RibbonStateErrorKind::Io, None, error.to_string())
                } else {
                    io_error(error)
                }
            })
        })
    }
    pub fn save_async_default(
        state: Option<&RibbonState>,
        path: Option<&Path>,
    ) -> RibbonStateFileFuture<()> {
        Self::save_async(state, path, CancellationToken::none())
    }
    /// The file is opened before returning; replacing its pathname cannot
    /// redirect a started read. Model allocation/deserialization stays on the
    /// future's polling thread, so no Rc models cross a worker boundary.
    pub fn load_async(
        file_path: Option<&Path>,
        token: CancellationToken,
    ) -> RibbonStateFileFuture<Option<Rc<RibbonState>>> {
        let prepared = (|| {
            let Some(path) = file_path else {
                return Ok(None);
            };
            // Source File.Exists precedes cancellation and the read.
            if !path.metadata().is_ok_and(|m| !m.is_dir()) {
                return Ok(None);
            }
            canceled(&token)?;
            let path = match absolute(path) {
                Ok(path) => path,
                Err(_) => return Ok(None),
            };
            match open_read(&path) {
                Ok(file) => Ok(Some(file)),
                Err(error) => {
                    let error = io_error(error);
                    if matches!(
                        error.kind,
                        RibbonStateErrorKind::Io | RibbonStateErrorKind::Unauthorized
                    ) {
                        Ok(None)
                    } else {
                        Err(error)
                    }
                }
            }
        })();
        let file = match prepared {
            Ok(Some(file)) => file,
            Ok(None) => return Box::pin(async { Ok(None) }),
            Err(error) => return Box::pin(async move { Err(error) }),
        };
        let read = run_file_operation("FerroRibbon state load", move || {
            match read_bytes(file, &token) {
                Ok(bytes) => Ok(Some(bytes)),
                Err(error)
                    if matches!(
                        error.kind,
                        RibbonStateErrorKind::Io | RibbonStateErrorKind::Unauthorized
                    ) =>
                {
                    Ok(None)
                }
                Err(error) => Err(error),
            }
        });
        Box::pin(async move {
            let Some(bytes) = read.await? else {
                return Ok(None);
            };
            Self::deserialize(Some(&decode(&bytes)))
        })
    }
    pub fn load_async_default(
        path: Option<&Path>,
    ) -> RibbonStateFileFuture<Option<Rc<RibbonState>>> {
        Self::load_async(path, CancellationToken::none())
    }
}
fn write_bytes(
    mut file: File,
    bytes: &[u8],
    token: &CancellationToken,
) -> Result<(), RibbonStateError> {
    for chunk in bytes.chunks(65536) {
        canceled(token)?;
        file.write_all(chunk).map_err(io_error)?;
    }
    canceled(token)?;
    file.flush().map_err(io_error)
}
fn read_bytes(mut file: File, token: &CancellationToken) -> Result<Vec<u8>, RibbonStateError> {
    let mut result = vec![];
    let mut buffer = [0; 65536];
    loop {
        canceled(token)?;
        let count = file.read(&mut buffer).map_err(io_error)?;
        if count == 0 {
            break;
        }
        result.extend_from_slice(&buffer[..count]);
    }
    Ok(result)
}
fn decode(bytes: &[u8]) -> RibbonString {
    let (bytes, width, big) = if bytes.starts_with(&[0xff, 0xfe, 0, 0]) {
        (&bytes[4..], 4, false)
    } else if bytes.starts_with(&[0, 0, 0xfe, 0xff]) {
        (&bytes[4..], 4, true)
    } else if bytes.starts_with(&[0xff, 0xfe]) {
        (&bytes[2..], 2, false)
    } else if bytes.starts_with(&[0xfe, 0xff]) {
        (&bytes[2..], 2, true)
    } else {
        (
            bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes),
            1,
            false,
        )
    };
    if width == 1 {
        return String::from_utf8_lossy(bytes).as_ref().into();
    }
    if width == 2 {
        let mut units = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| {
                if big {
                    u16::from_be_bytes([c[0], c[1]])
                } else {
                    u16::from_le_bytes([c[0], c[1]])
                }
            })
            .collect::<Vec<_>>();
        if bytes.len() % 2 != 0 {
            units.push(0xfffd);
        }
        return String::from_utf16_lossy(&units).into();
    }
    let mut text = String::new();
    for c in bytes.as_chunks::<4>().0 {
        let n = if big {
            u32::from_be_bytes(*c)
        } else {
            u32::from_le_bytes(*c)
        };
        text.push(char::from_u32(n).unwrap_or(char::REPLACEMENT_CHARACTER));
    }
    if bytes.len() % 4 != 0 {
        text.push(char::REPLACEMENT_CHARACTER);
    }
    text.into()
}
