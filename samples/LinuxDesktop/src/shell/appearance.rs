//! Shell appearances pair desktop geometry with a real FerroUI control theme.
//! A descriptor owns its factory and reserved screen edges. The host and the
//! settings picker both consume this registry.
use ferroui_base::media::Color;
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

#[derive(Clone, Copy)]
pub struct Descriptor {
    pub id: &'static str,
    pub name: &'static str,
    pub insets: (f64, f64),
    pub layout: Layout,
    pub palette: Palette,
    pub tint: fn(Color) -> Color,
    pub style: fn() -> Rc<dyn IStyle>,
    pub variant: fn() -> ThemeVariant,
}

#[derive(Clone, Copy)]
pub struct Palette {
    pub text: &'static str,
    pub muted: &'static str,
    pub accent: &'static str,
    pub edge: &'static str,
    pub surface: &'static str,
    pub tile: &'static str,
    pub selected: &'static str,
    pub on_selected: &'static str,
    pub radius: i32,
    pub bar: &'static str,
    pub bar_edge: &'static str,
    pub bar_ink: &'static str,
    pub dock_tiles: [&'static str; 3],
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
        palette: Palette {
            text: "#1C2025",
            muted: "#555E69",
            accent: "#005FB8",
            edge: "#C3CBD6",
            surface: "#F2F5FA",
            tile: "#E7EDF5",
            selected: "#D1E8FC",
            on_selected: "#003868",
            radius: 9,
            bar: "#F5EAF0F7",
            bar_edge: "#F2FFFFFF",
            bar_ink: "#1C2025",
            dock_tiles: ["#0078D4", "#326AAA", "#465E78"],
        },
        tint: |c| Color::from_rgb(c.r.saturating_sub(12), c.b, c.g.saturating_add(49)),
        style: || ferroui_themes_winui::WinUITheme::new().as_style(),
        variant: ThemeVariant::light,
    });
    pub const MacOS: Self = Self(&Descriptor {
        id: "macos",
        name: "macOS",
        insets: (34.0, 64.0),
        layout: Layout::MenuBarDock,
        palette: Palette {
            text: "#FBF7FE",
            muted: "#D4CADA",
            accent: "#DFC2FE",
            edge: "#716477",
            surface: "#342F3C",
            tile: "#423C4B",
            selected: "#78519B",
            on_selected: "#FFFFFF",
            radius: 19,
            bar: "#E8383044",
            bar_edge: "#70F7E9FF",
            bar_ink: "#FBF4FF",
            dock_tiles: ["#635BA4", "#3185B1", "#81617E"],
        },
        tint: |c| Color::from_rgb(c.g.saturating_add(30), c.r, c.b.saturating_add(24)),
        style: || ferroui_themes_macos::MacOSTheme::new().as_style(),
        variant: ThemeVariant::dark,
    });
    pub const Material: Self = Self(&Descriptor {
        id: "material",
        name: "Material 3",
        insets: (0.0, 72.0),
        layout: Layout::BottomNavigation,
        palette: Palette {
            text: "#EFF4E9",
            muted: "#B9C9B9",
            accent: "#A0D5B7",
            edge: "#41564D",
            surface: "#182922",
            tile: "#253A30",
            selected: "#385D48",
            on_selected: "#DBFFE3",
            radius: 28,
            bar: "#F41D3027",
            bar_edge: "#41564D",
            bar_ink: "#D4E7D9",
            dock_tiles: ["#385D48", "#526557", "#546960"],
        },
        tint: |c| c,
        style: || ferroui_themes_material::MaterialTheme::new().as_style(),
        variant: ThemeVariant::dark,
    });
    pub const ALL: &'static [Self] = &[Self::Windows, Self::MacOS, Self::Material];

    pub const fn new(descriptor: &'static Descriptor) -> Self {
        Self(descriptor)
    }
    pub const fn descriptor(self) -> &'static Descriptor {
        self.0
    }

    pub fn parse(id: &str) -> Option<Self> {
        Self::find(Self::ALL, id)
    }
    pub fn find(registry: &[Self], id: &str) -> Option<Self> {
        registry
            .iter()
            .copied()
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
    pub const fn palette(self) -> Palette {
        self.0.palette
    }
    pub fn tint(self, color: Color) -> Color {
        (self.0.tint)(color)
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
        for &appearance in Appearance::ALL {
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

    #[test]
    fn fourth_descriptor_supplies_its_own_palette_on_an_existing_layout() {
        const CUSTOM: Appearance = Appearance::new(&Descriptor {
            id: "night-shift",
            name: "Night shift",
            insets: (0.0, 62.0),
            palette: Palette {
                accent: "#FF7700",
                ..Appearance::Windows.palette()
            },
            ..*Appearance::Windows.descriptor()
        });
        let registry = [
            Appearance::Windows,
            Appearance::MacOS,
            Appearance::Material,
            CUSTOM,
        ];
        let resolved = Appearance::find(&registry, "night-shift").unwrap();
        assert_eq!(resolved.palette().accent, "#FF7700");
        assert_eq!(resolved.insets(), (0.0, 62.0));
        assert_eq!(resolved.layout(), Layout::Taskbar);
    }
}
