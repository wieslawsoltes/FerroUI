use super::{ManagedFileChooserItemType, ManagedFileChooserNavigationItem};
use ferroui_base::collections::FerroList;
use ferroui_base::platform::storage::file_io::path;
use ferroui_base::utilities::ByteSizeHelper;
use ferroui_controls::platform::MountedVolumeInfo;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// The places the managed file chooser offers as quick links: the folders
/// of the user and the mounted volumes. Each source can be replaced; an
/// instance registered with the locator is used instead of the default one.
///
/// Internal upstream; public here for the platform backends.
pub struct ManagedFileChooserSources {
    get_user_directories: RefCell<Rc<dyn Fn() -> Vec<ManagedFileChooserNavigationItem>>>,
    get_file_system_roots: RefCell<Rc<dyn Fn() -> Vec<ManagedFileChooserNavigationItem>>>,
    get_all_items_delegate: RefCell<Rc<dyn Fn(&ManagedFileChooserSources) -> Vec<ManagedFileChooserNavigationItem>>>,
}

thread_local! {
    static MOUNTED_VOLUMES: FerroList<MountedVolumeInfo> = FerroList::new();
}

/// The special folders of the user the quick links start with, in order
/// (`Environment.SpecialFolder`).
#[derive(Clone, Copy)]
enum SpecialFolder {
    Desktop,
    UserProfile,
    MyDocuments,
    MyMusic,
    MyPictures,
    MyVideos,
}

const FOLDERS: [SpecialFolder; 6] = [
    SpecialFolder::Desktop,
    SpecialFolder::UserProfile,
    SpecialFolder::MyDocuments,
    SpecialFolder::MyMusic,
    SpecialFolder::MyPictures,
    SpecialFolder::MyVideos,
];

fn home_directory() -> Option<PathBuf> {
    let variable = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    std::env::var_os(variable).filter(|home| !home.is_empty()).map(PathBuf::from)
}

/// The directory `key` names for the user: the environment variable of
/// that name, else the entry of the user directories of the desktop
/// (`user-dirs.dirs`), else `fallback` in the home directory.
fn read_xdg_directory(home: &Path, key: &str, fallback: &str) -> PathBuf {
    if let Some(value) = std::env::var_os(key).filter(|value| !value.is_empty()) {
        return PathBuf::from(value);
    }

    let config_home = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|config_home| !config_home.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"));
    if let Ok(content) = std::fs::read_to_string(config_home.join("user-dirs.dirs")) {
        if let Some(directory) = parse_xdg_user_directory(&content, home, key) {
            return directory;
        }
    }

    home.join(fallback)
}

/// Finds `key="value"` in the content of a `user-dirs.dirs` file. The value
/// is an absolute path or a path relative to the home directory
/// (`$HOME/...`).
fn parse_xdg_user_directory(content: &str, home: &Path, key: &str) -> Option<PathBuf> {
    for line in content.lines() {
        let line = line.trim_start();
        let Some(rest) = line.strip_prefix(key) else { continue };
        let Some(rest) = rest.trim_start().strip_prefix('=') else { continue };
        let Some(rest) = rest.trim_start().strip_prefix('"') else { continue };
        let Some(end) = rest.find('"') else { continue };
        let value = &rest[..end];

        if let Some(relative) = value.strip_prefix("$HOME/") {
            return Some(home.join(relative));
        }
        if value.starts_with('/') {
            return Some(PathBuf::from(value));
        }
    }
    None
}

/// The path of a special folder of the user (`Environment.GetFolderPath`),
/// or an empty string when it has none.
fn get_folder_path(folder: SpecialFolder) -> String {
    let Some(home) = home_directory() else {
        return String::new();
    };

    let path = if cfg!(windows) {
        match folder {
            SpecialFolder::Desktop => home.join("Desktop"),
            SpecialFolder::UserProfile => home,
            SpecialFolder::MyDocuments => home.join("Documents"),
            SpecialFolder::MyMusic => home.join("Music"),
            SpecialFolder::MyPictures => home.join("Pictures"),
            SpecialFolder::MyVideos => home.join("Videos"),
        }
    } else {
        match folder {
            SpecialFolder::UserProfile | SpecialFolder::MyDocuments => home,
            SpecialFolder::Desktop => read_xdg_directory(&home, "XDG_DESKTOP_DIR", "Desktop"),
            SpecialFolder::MyMusic if cfg!(target_os = "macos") => home.join("Music"),
            SpecialFolder::MyMusic => read_xdg_directory(&home, "XDG_MUSIC_DIR", "Music"),
            SpecialFolder::MyPictures if cfg!(target_os = "macos") => home.join("Pictures"),
            SpecialFolder::MyPictures => read_xdg_directory(&home, "XDG_PICTURES_DIR", "Pictures"),
            SpecialFolder::MyVideos => read_xdg_directory(&home, "XDG_VIDEOS_DIR", "Videos"),
        }
    };
    path.to_string_lossy().into_owned()
}

impl Default for ManagedFileChooserSources {
    fn default() -> Self {
        Self::new()
    }
}

impl ManagedFileChooserSources {
    pub fn new() -> Self {
        Self {
            get_user_directories: RefCell::new(Rc::new(Self::default_get_user_directories)),
            get_file_system_roots: RefCell::new(Rc::new(Self::default_get_file_system_roots)),
            get_all_items_delegate: RefCell::new(Rc::new(Self::default_get_all_items)),
        }
    }

    pub fn get_user_directories(&self) -> Rc<dyn Fn() -> Vec<ManagedFileChooserNavigationItem>> {
        self.get_user_directories.borrow().clone()
    }

    pub fn set_get_user_directories(&self, value: Rc<dyn Fn() -> Vec<ManagedFileChooserNavigationItem>>) {
        *self.get_user_directories.borrow_mut() = value;
    }

    pub fn get_file_system_roots(&self) -> Rc<dyn Fn() -> Vec<ManagedFileChooserNavigationItem>> {
        self.get_file_system_roots.borrow().clone()
    }

    pub fn set_get_file_system_roots(&self, value: Rc<dyn Fn() -> Vec<ManagedFileChooserNavigationItem>>) {
        *self.get_file_system_roots.borrow_mut() = value;
    }

    pub fn get_all_items_delegate(&self) -> Rc<dyn Fn(&ManagedFileChooserSources) -> Vec<ManagedFileChooserNavigationItem>> {
        self.get_all_items_delegate.borrow().clone()
    }

    pub fn set_get_all_items_delegate(
        &self,
        value: Rc<dyn Fn(&ManagedFileChooserSources) -> Vec<ManagedFileChooserNavigationItem>>,
    ) {
        *self.get_all_items_delegate.borrow_mut() = value;
    }

    pub fn get_all_items(&self) -> Vec<ManagedFileChooserNavigationItem> {
        (self.get_all_items_delegate())(self)
    }

    /// The mounted volumes, filled by the mounted volume info provider (one
    /// list per thread: the static collection of the original).
    pub fn mounted_volumes() -> FerroList<MountedVolumeInfo> {
        MOUNTED_VOLUMES.with(FerroList::clone)
    }

    pub fn default_get_all_items(sources: &ManagedFileChooserSources) -> Vec<ManagedFileChooserNavigationItem> {
        let mut items = (sources.get_user_directories())();
        items.extend((sources.get_file_system_roots())());
        items
    }

    pub fn default_get_user_directories() -> Vec<ManagedFileChooserNavigationItem> {
        let mut paths: Vec<String> = Vec::new();
        for path in FOLDERS.iter().map(|folder| get_folder_path(*folder)) {
            if !paths.contains(&path) {
                paths.push(path);
            }
        }

        paths
            .into_iter()
            .filter(|d| !d.trim().is_empty())
            .filter(|d| Path::new(d).is_dir())
            .map(|d| ManagedFileChooserNavigationItem {
                item_type: ManagedFileChooserItemType::Folder,
                display_name: Some(path::get_file_name(&d).to_string()),
                path: Some(d),
            })
            .collect()
    }

    pub fn default_get_file_system_roots() -> Vec<ManagedFileChooserNavigationItem> {
        Self::mounted_volumes()
            .to_vec()
            .into_iter()
            .filter_map(|x| {
                let mut display_name = x.volume_label.clone();

                if display_name.is_none() & (x.volume_size_bytes > 0) {
                    display_name = Some(format!("{} Volume", ByteSizeHelper::to_string(x.volume_size_bytes, true)));
                }

                // A volume whose files cannot be listed is left out.
                let path = x.volume_path.clone();
                let files = path.as_deref().map(std::fs::read_dir);
                if !matches!(files, Some(Ok(_))) {
                    return None;
                }

                Some(ManagedFileChooserNavigationItem {
                    item_type: ManagedFileChooserItemType::Volume,
                    display_name,
                    path,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the upstream project has no tests.
    use super::*;

    #[test]
    fn all_items_are_the_user_directories_followed_by_the_roots() {
        let sources = ManagedFileChooserSources::new();
        let item = |name: &str, item_type| ManagedFileChooserNavigationItem {
            display_name: Some(name.to_string()),
            path: Some(format!("/{name}")),
            item_type,
        };
        sources.set_get_user_directories(Rc::new(move || vec![item("home", ManagedFileChooserItemType::Folder)]));
        sources.set_get_file_system_roots(Rc::new(move || vec![item("mnt", ManagedFileChooserItemType::Volume)]));

        let names: Vec<_> = sources.get_all_items().into_iter().map(|i| i.display_name.unwrap()).collect();
        assert_eq!(vec!["home", "mnt"], names);
    }

    #[test]
    fn roots_are_the_listable_mounted_volumes() {
        let root = std::env::temp_dir();
        let volumes = ManagedFileChooserSources::mounted_volumes();
        volumes.clear();
        volumes.add(MountedVolumeInfo {
            volume_label: None,
            volume_path: Some(root.to_string_lossy().into_owned()),
            volume_size_bytes: 1500,
        });
        volumes.add(MountedVolumeInfo {
            volume_label: Some("Missing".to_string()),
            volume_path: Some("/this/volume/does/not/exist".to_string()),
            volume_size_bytes: 0,
        });

        let roots = ManagedFileChooserSources::default_get_file_system_roots();
        volumes.clear();

        assert_eq!(1, roots.len());
        assert_eq!(Some("1.5KB Volume".to_string()), roots[0].display_name);
        assert_eq!(ManagedFileChooserItemType::Volume, roots[0].item_type);
    }

    #[test]
    fn user_directories_exist_and_are_distinct() {
        let directories = ManagedFileChooserSources::default_get_user_directories();
        for (index, directory) in directories.iter().enumerate() {
            let path = directory.path.as_deref().unwrap();
            assert!(Path::new(path).is_dir());
            assert_eq!(ManagedFileChooserItemType::Folder, directory.item_type);
            assert!(directories[index + 1..].iter().all(|other| other.path.as_deref() != Some(path)));
        }
    }

    #[test]
    fn xdg_entries_are_read_relative_to_home_or_absolute() {
        let home = Path::new("/home/user");
        let content = "# comment\nXDG_MUSIC_DIR=\"$HOME/Tunes\"\nXDG_VIDEOS_DIR=\"/media/videos\"\n";

        assert_eq!(Some(PathBuf::from("/home/user/Tunes")), parse_xdg_user_directory(content, home, "XDG_MUSIC_DIR"));
        assert_eq!(Some(PathBuf::from("/media/videos")), parse_xdg_user_directory(content, home, "XDG_VIDEOS_DIR"));
        assert_eq!(None, parse_xdg_user_directory(content, home, "XDG_DESKTOP_DIR"));
    }
}
