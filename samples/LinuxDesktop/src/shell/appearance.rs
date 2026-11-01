//! Shell appearances pair desktop geometry with a real FerroUI control theme.
//! A descriptor owns its factory and reserved screen edges. The host and the
//! settings picker both consume this registry.
use ferroui_base::media::Color;
use ferroui_base::styling::{IStyle, ThemeVariant};
use ferroui_base::Ref;
use ferroui_controls::Application;
use ferroui_themes_winui::{DensityStyle, WinUITheme};
use std::cell::{Cell, RefCell};
#[cfg(test)]
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;

#[derive(Clone, Copy)]
pub struct Descriptor {
    pub id: &'static str,
    pub name: &'static str,
    pub insets: (f64, f64),
    pub layout: Layout,
    pub wallpaper: Wallpaper,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Wallpaper {
    Fold,
    Horizon,
    Garden,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ColorMode {
    #[default]
    Light,
    Dark,
    HighContrast,
}

impl ColorMode {
    pub const ALL: [Self; 3] = [Self::Light, Self::Dark, Self::HighContrast];

    pub fn label(self) -> &'static str {
        match self {
            Self::Light => "Light",
            Self::Dark => "Dark",
            Self::HighContrast => "High contrast",
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
            Self::HighContrast => "high-contrast",
        }
    }

    fn variant(self) -> ThemeVariant {
        match self {
            Self::Light => ThemeVariant::light(),
            Self::Dark => ThemeVariant::dark(),
            Self::HighContrast => WinUITheme::high_contrast(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WinUISettings {
    pub mode: ColorMode,
    pub density: DensityStyle,
}

impl WinUISettings {
    const DEFAULT: Self = Self { mode: ColorMode::Light, density: DensityStyle::Normal };

    pub fn toggle_density(self) -> Self {
        let density = match self.density {
            DensityStyle::Normal => DensityStyle::Compact,
            DensityStyle::Compact => DensityStyle::Normal,
        };
        Self { density, ..self }
    }

    fn decode(content: &str) -> Option<Self> {
        let mut words = content.split_whitespace();
        let color_id = words.next()?;
        let mode = ColorMode::ALL.into_iter().find(|mode| mode.id() == color_id)?;
        let density = match words.next()? {
            "compact" => DensityStyle::Compact,
            "normal" => DensityStyle::Normal,
            _ => return None,
        };
        words.next().is_none().then_some(Self { mode, density })
    }

    fn preferred() -> Self {
        crate::services::preferences::config_path("windows-style").ok()
            .and_then(|p| fs::read_to_string(p).ok())
            .and_then(|s| Self::decode(&s)).unwrap_or_default()
    }

    fn save(self) -> io::Result<()> {
        let path = crate::services::preferences::config_path("windows-style")?;
        let density = if self.density == DensityStyle::Compact { "compact" } else { "normal" };
        crate::services::preferences::replace(&path, format!("{} {density}\n", self.mode.id()).as_bytes())
    }
}

#[derive(Clone, Copy)]
pub struct Appearance(&'static Descriptor, Option<WinUISettings>);

#[allow(non_upper_case_globals)]
impl Appearance {
    pub const Windows: Self = Self::new(&Descriptor {
        id: "windows",
        name: "Windows 11",
        insets: (0.0, 52.0),
        layout: Layout::Taskbar,
        wallpaper: Wallpaper::Fold,
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
    }).with_winui(WinUISettings::DEFAULT);
    pub const MacOS: Self = Self::new(&Descriptor {
        id: "macos",
        name: "macOS",
        insets: (34.0, 64.0),
        layout: Layout::MenuBarDock,
        wallpaper: Wallpaper::Horizon,
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
        tint: |c| c,
        style: || ferroui_themes_macos::MacOSTheme::new().as_style(),
        variant: ThemeVariant::dark,
    });
    pub const Material: Self = Self::new(&Descriptor {
        id: "material",
        name: "Material 3",
        insets: (0.0, 72.0),
        layout: Layout::BottomNavigation,
        wallpaper: Wallpaper::Garden,
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
        Self(descriptor, None)
    }
    const fn with_winui(self, settings: WinUISettings) -> Self { Self(self.0, Some(settings)) }

    pub fn winui(self) -> Option<WinUISettings> { self.1 }

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
    pub fn wallpaper(self) -> Wallpaper {
        self.0.wallpaper
    }
    pub const fn palette(self) -> Palette {
        let base = self.descriptor().palette;
        match self.1 {
            Some(WinUISettings { mode: ColorMode::Dark, .. }) => Palette {
                text: "#FFFFFF", muted: "#C8C8C8", accent: "#60CDFF",
                edge: "#484848", surface: "#202020", tile: "#303030",
                selected: "#244767", on_selected: "#FFFFFF",
                bar: "#F3202020", bar_edge: "#30FFFFFF", bar_ink: "#FFFFFF",
                dock_tiles: ["#60CDFF", "#A7CAEB", "#D0D9E1"], ..base
            },
            Some(WinUISettings { mode: ColorMode::HighContrast, .. }) => Palette {
                text: "#FFFFFF", muted: "#FFFFFF", accent: "#FFFF00",
                edge: "#FFFFFF", surface: "#000000", tile: "#000000",
                selected: "#1AEBFF", on_selected: "#000000",
                bar: "#000000", bar_edge: "#FFFFFF", bar_ink: "#FFFFFF",
                dock_tiles: ["#FFFF00", "#1AEBFF", "#FFFFFF"], ..base
            },
            _ => base,
        }
    }
    pub fn tint(self, color: Color) -> Color {
        (self.0.tint)(color)
    }
    pub fn insets(self) -> (f64, f64) {
        let (top, bottom) = self.0.insets;
        if self.winui().is_some_and(|s| s.density == DensityStyle::Compact) {
            (top, 42.0)
        } else { (top, bottom) }
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
        self.id() == other.id() && self.1 == other.1
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
    windows: Cell<WinUISettings>,
}

impl ThemeController {
    pub fn install(app: &Ref<Application>, appearance: Appearance) -> Rc<Self> {
        let windows = WinUISettings::preferred();
        let appearance = if appearance.winui().is_some() { appearance.with_winui(windows) } else { appearance };
        let style = (appearance.0.style)();
        Self::configure(app, appearance, &style);
        app.styles().add(style.clone());
        Rc::new(Self {
            app: app.clone(),
            current: Cell::new(appearance),
            installed: RefCell::new(style),
            windows: Cell::new(windows),
        })
    }

    pub fn current(&self) -> Appearance {
        self.current.get()
    }

    fn configure(app: &Application, appearance: Appearance, style: &Rc<dyn IStyle>) {
        let variant = if let Some(options) = appearance.winui() {
            let theme = style.as_object().and_then(|object| object.downcast_ref::<WinUITheme>())
                .expect("Windows appearance installs WinUITheme");
            theme.set_density_style(options.density);
            options.mode.variant()
        } else {
            (appearance.0.variant)()
        };
        app.set_requested_theme_variant(Some(variant));
    }

    fn apply(&self, appearance: Appearance) {
        let previous = self.current.get();
        if previous == appearance { return; }
        if previous.id() != appearance.id() {
            let style = (appearance.0.style)();
            let styles = self.app.styles();
            let old = self.installed.replace(style.clone());
            if let Some(index) = styles.index_of(old) {
                styles.set(index, style);
            } else {
                styles.add(style);
            }
        }
        Self::configure(&self.app, appearance, &self.installed.borrow());
        self.current.set(appearance);
    }

    pub fn select(&self, appearance: Appearance) -> io::Result<()> {
        let appearance = if appearance.winui().is_some() {
            appearance.with_winui(self.windows.get())
        } else { appearance };
        self.apply(appearance);
        save_preference(&preference_path()?, appearance)
    }

    pub fn set_windows(&self, settings: WinUISettings) -> io::Result<()> {
        let current = self.current();
        if current.winui().is_some() {
            self.windows.set(settings);
            self.apply(current.with_winui(settings));
            settings.save()?;
        }
        Ok(())
    }

}

fn preference_path() -> io::Result<PathBuf> {
    crate::services::preferences::config_path("appearance")
}

fn save_preference(path: &Path, appearance: Appearance) -> io::Result<()> {
    crate::services::preferences::replace(path, format!("{}\n", appearance.id()).as_bytes())
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
        static NIGHT_SHIFT: Descriptor = Descriptor {
            id: "night-shift",
            name: "Night shift",
            insets: (0.0, 62.0),
            palette: Palette {
                accent: "#FF7700",
                ..Appearance::Windows.palette()
            },
            ..*Appearance::Windows.descriptor()
        };
        const CUSTOM: Appearance = Appearance::new(&NIGHT_SHIFT);
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
