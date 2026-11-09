//! Native desktop data-directory policy. No filesystem access or environment mutation in tests.
use std::{ffi::OsString, io, path::PathBuf};

pub(crate) fn application_data_folder() -> io::Result<PathBuf> {
    folder_for(
        std::env::consts::OS,
        &|name| std::env::var_os(name),
        std::env::home_dir(),
    )
}
fn folder_for(
    platform: &str,
    lookup: &impl Fn(&str) -> Option<OsString>,
    home: Option<PathBuf>,
) -> io::Result<PathBuf> {
    if let Some(path) = lookup("FERROBOARD_DATA_DIR") {
        return absolute(path, "FERROBOARD_DATA_DIR");
    }
    match platform {
        "macos" => Ok(home_folder(home)?.join("Library/Application Support/FerroBoard")),
        "windows" => absolute(
            lookup("LOCALAPPDATA").ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "LOCALAPPDATA is unavailable; set FERROBOARD_DATA_DIR",
                )
            })?,
            "LOCALAPPDATA",
        )
        .map(|path| path.join("FerroBoard")),
        "linux" | "freebsd" | "openbsd" | "netbsd" => {
            if let Some(path) = lookup("XDG_DATA_HOME")
                .map(PathBuf::from)
                .filter(|path| path.is_absolute())
            {
                return Ok(path.join("ferroboard"));
            }
            Ok(home_folder(home)?.join(".local/share/ferroboard"))
        }
        _ => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "No native desktop data-directory policy for this platform; set FERROBOARD_DATA_DIR",
        )),
    }
}
fn absolute(path: OsString, name: &str) -> io::Result<PathBuf> {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        Ok(path)
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{name} must be a nonempty absolute path"),
        ))
    }
}
fn home_folder(home: Option<PathBuf>) -> io::Result<PathBuf> {
    home.filter(|path| path.is_absolute()).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "A user home directory is unavailable; set FERROBOARD_DATA_DIR",
        )
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn base() -> PathBuf {
        std::env::current_dir().unwrap().join("data-path-fixtures")
    }
    fn env(values: &[(&str, OsString)]) -> impl Fn(&str) -> Option<OsString> {
        move |key| {
            values
                .iter()
                .find(|(name, _)| *name == key)
                .map(|(_, value)| value.clone())
        }
    }
    #[test]
    fn explicit_override_wins_on_every_platform_without_home() {
        let path = base().join("explicit");
        for os in ["macos", "windows", "linux", "unknown"] {
            assert_eq!(
                folder_for(
                    os,
                    &env(&[("FERROBOARD_DATA_DIR", path.clone().into_os_string())]),
                    None
                )
                .unwrap(),
                path
            );
        }
    }
    #[test]
    fn invalid_override_is_reported_instead_of_using_another_directory() {
        for value in [OsString::new(), "relative/path".into()] {
            assert_eq!(
                folder_for(
                    "macos",
                    &env(&[("FERROBOARD_DATA_DIR", value)]),
                    Some(base())
                )
                .unwrap_err()
                .kind(),
                io::ErrorKind::InvalidInput
            );
        }
    }
    #[test]
    fn macos_uses_application_support_in_the_user_home() {
        let home = base();
        assert_eq!(
            folder_for(
                "macos",
                &env(&[("XDG_DATA_HOME", home.join("ignored").into_os_string())]),
                Some(home.clone())
            )
            .unwrap(),
            home.join("Library/Application Support/FerroBoard")
        );
    }
    #[test]
    fn windows_uses_localappdata_and_reports_unavailable_or_invalid_paths() {
        let local = base().join("Local");
        assert_eq!(
            folder_for(
                "windows",
                &env(&[("LOCALAPPDATA", local.clone().into_os_string())]),
                None
            )
            .unwrap(),
            local.join("FerroBoard")
        );
        assert_eq!(
            folder_for("windows", &env(&[]), Some(base()))
                .unwrap_err()
                .kind(),
            io::ErrorKind::NotFound
        );
        assert_eq!(
            folder_for(
                "windows",
                &env(&[("LOCALAPPDATA", "relative".into())]),
                Some(base())
            )
            .unwrap_err()
            .kind(),
            io::ErrorKind::InvalidInput
        );
    }
    #[test]
    fn xdg_absolute_override_and_specified_home_fallback() {
        let home = base();
        let data = home.join("xdg");
        assert_eq!(
            folder_for(
                "linux",
                &env(&[("XDG_DATA_HOME", data.clone().into_os_string())]),
                None
            )
            .unwrap(),
            data.join("ferroboard")
        );
        for value in [None, Some(OsString::new()), Some("relative".into())] {
            let values = value
                .map(|value| vec![("XDG_DATA_HOME", value)])
                .unwrap_or_default();
            assert_eq!(
                folder_for("linux", &env(&values), Some(home.clone())).unwrap(),
                home.join(".local/share/ferroboard")
            );
        }
    }
    #[test]
    fn missing_home_and_unknown_platform_do_not_invent_a_storage_location() {
        assert_eq!(
            folder_for("macos", &env(&[]), None).unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
        assert_eq!(
            folder_for("linux", &env(&[]), Some(PathBuf::new()))
                .unwrap_err()
                .kind(),
            io::ErrorKind::NotFound
        );
        assert_eq!(
            folder_for("unknown", &env(&[]), Some(base()))
                .unwrap_err()
                .kind(),
            io::ErrorKind::Unsupported
        );
    }
    #[cfg(unix)]
    #[test]
    fn override_preserves_non_unicode_os_paths() {
        use std::os::unix::ffi::OsStringExt;
        let path = base().join(OsString::from_vec(vec![0xff, b'x']));
        assert_eq!(
            folder_for(
                "macos",
                &env(&[("FERROBOARD_DATA_DIR", path.clone().into_os_string())]),
                None
            )
            .unwrap(),
            path
        );
    }
}
