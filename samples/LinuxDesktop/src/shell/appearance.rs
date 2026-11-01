//! Shell appearances pair desktop geometry with a real FerroUI control theme.
//! A descriptor owns its factory and reserved screen edges. The host and the
//! settings picker both consume this registry.
use ferroui_base::styling::{IStyle, ThemeVariant};
use ferroui_base::Ref;
use ferroui_controls::Application;
use std::cell::{Cell, RefCell};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

struct Descriptor {
    id: &'static str,
    name: &'static str,
    insets: (f64, f64),
    layout: Layout,
    style: fn() -> Rc<dyn IStyle>,
    variant: fn() -> ThemeVariant,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Layout {
    Taskbar,
    MenuBarDock,
    BottomNavigation,
}

#[derive(Clone, Copy)]
pub struct Appearance(&'static Descriptor);

#[allow(non_upper_case_globals)]
impl Appearance {
    pub const Windows: Self = Self(&Descriptor {
        id: "windows",
        name: "Windows 11",
        insets: (0.0, 52.0),
        layout: Layout::Taskbar,
        style: || ferroui_themes_winui::WinUITheme::new().as_style(),
        variant: ThemeVariant::light,
    });
    pub const MacOS: Self = Self(&Descriptor {
        id: "macos",
        name: "macOS",
        insets: (34.0, 64.0),
        layout: Layout::MenuBarDock,
        style: || ferroui_themes_macos::MacOSTheme::new().as_style(),
        variant: ThemeVariant::dark,
    });
    pub const Material: Self = Self(&Descriptor {
        id: "material",
        name: "Material 3",
        insets: (0.0, 72.0),
        layout: Layout::BottomNavigation,
        style: || ferroui_themes_material::MaterialTheme::new().as_style(),
        variant: ThemeVariant::dark,
    });
    pub const ALL: [Self; 3] = [Self::Windows, Self::MacOS, Self::Material];

    pub fn parse(id: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|appearance| appearance.id() == id)
    }
    pub fn id(self) -> &'static str {
        self.0.id
    }
    pub fn name(self) -> &'static str {
        self.0.name
    }
    pub fn layout(self) -> Layout {
        self.0.layout
    }
    pub fn insets(self) -> (f64, f64) {
        self.0.insets
    }

    pub fn preferred() -> Self {
        preference_path()
            .ok()
            .and_then(|path| fs::read_to_string(path).ok())
            .and_then(|saved| Self::parse(saved.trim()))
            .unwrap_or(Self::Windows)
    }
}

impl PartialEq for Appearance {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}
impl Eq for Appearance {}
impl std::fmt::Debug for Appearance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.id())
    }
}

pub struct ThemeController {
    app: Ref<Application>,
    current: Cell<Appearance>,
    installed: RefCell<Rc<dyn IStyle>>,
}

impl ThemeController {
    pub fn install(app: &Ref<Application>, appearance: Appearance) -> Rc<Self> {
        let style = (appearance.0.style)();
        app.set_requested_theme_variant(Some((appearance.0.variant)()));
        app.styles().add(style.clone());
        Rc::new(Self {
            app: app.clone(),
            current: Cell::new(appearance),
            installed: RefCell::new(style),
        })
    }

    pub fn current(&self) -> Appearance {
        self.current.get()
    }

    pub fn select(&self, appearance: Appearance) -> io::Result<()> {
        if self.current.get() != appearance {
            let style = (appearance.0.style)();
            let styles = self.app.styles();
            let previous = self.installed.replace(style.clone());
            if let Some(index) = styles.index_of(previous) {
                styles.set(index, style);
            } else {
                styles.add(style);
            }
            self.app
                .set_requested_theme_variant(Some((appearance.0.variant)()));
            self.current.set(appearance);
        }
        save_preference(&preference_path()?, appearance)
    }
}

fn preference_path() -> io::Result<PathBuf> {
    let base = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| {
            env::var_os("HOME")
                .map(PathBuf::from)
                .filter(|p| p.is_absolute())
                .map(|p| p.join(".config"))
        })
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "No user configuration directory")
        })?;
    Ok(base.join("ferro-desktop/appearance"))
}

fn save_preference(path: &Path, appearance: Appearance) -> io::Result<()> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let dir = path
        .parent()
        .ok_or_else(|| io::Error::other("Missing configuration directory"))?;
    fs::create_dir_all(dir)?;
    let (mut file, staging) = loop {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let staging = dir.join(format!(".appearance-{}-{id}.tmp", std::process::id()));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staging)
        {
            Ok(file) => break (file, staging),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    };
    let result = writeln!(file, "{}", appearance.id()).and_then(|()| fs::rename(&staging, path));
    if result.is_err() {
        let _ = fs::remove_file(staging);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picker_ids_round_trip_through_the_preference_file() {
        let dir = env::temp_dir().join(format!("ferro-desktop-appearance-{}", std::process::id()));
        fs::create_dir(&dir).unwrap();
        let path = dir.join("appearance");
        for appearance in Appearance::ALL {
            save_preference(&path, appearance).unwrap();
            assert_eq!(
                Appearance::parse(fs::read_to_string(&path).unwrap().trim()),
                Some(appearance)
            );
        }
        assert!(Appearance::parse("missing").is_none());
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
        fs::remove_dir_all(dir).unwrap();
    }
}
