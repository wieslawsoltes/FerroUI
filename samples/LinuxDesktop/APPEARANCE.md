# Desktop appearances

The default Windows 11 appearance uses FerroUI's compiled WinUI 3 theme. Open **Appearance settings** from the taskbar, **Appearance** from the macOS menu bar or **Style** from the Material navigation bar. While the shell has keyboard focus, Ctrl+, opens the chooser. Press a digit from 1 through 9 to select the corresponding registered appearance, or use Tab and Enter. Escape returns to the desktop.

| Appearance | Native control theme | Shell composition |
| --- | --- | --- |
| Windows 11 | WinUI 3, light variant | Centered taskbar, desktop shortcuts and a Start panel with an application grid |
| macOS | macOS, dark variant | Top menu bar, floating Dock and a centered application search panel |
| Material 3 | Material, dark variant with bundled Roboto | Clock home screen, bottom navigation and bottom sheets |

A switch replaces the shell application's installed style in its existing style collection and changes its requested theme variant. Shell labels bind to shared theme resources such as `TextFillColorPrimaryBrush` and `ContentControlThemeFontFamily`. The appearance descriptor supplies brushes for the desktop surface, icon tiles and wallpaper. The shell rebuilds its chrome around the existing search query, catalog, workspace state and window-manager connection. In session mode it also updates the reserved screen edges so maximized clients follow the new layout.

The setting persists as an appearance ID in `$XDG_CONFIG_HOME/ferro-desktop/appearance`, with `$HOME/.config` as the default configuration directory. A persistence error is shown in the chooser after the live change. `--theme=windows`, `--theme=macos` and `--theme=material` select a startup appearance explicitly. Applications launched from the shell continue to draw with their own toolkits and settings.

## Register a fourth appearance

Define a descriptor in `src/shell/appearance.rs` and add its handle to `Appearance::ALL`. Stable IDs identify saved settings and command-line values. Pick a layout, supply its top and bottom screen reservations in device-independent pixels, and provide a native style factory and variant.

```rust
const COBALT: Appearance = Appearance::new(&Descriptor {
    id: "cobalt",
    name: "Cobalt",
    layout: Layout::Taskbar,
    insets: (0.0, 60.0),
    palette: Palette {
        bar: "#F5DBE8FF",
        accent: "#2145AE",
        dock_tiles: ["#2458CF", "#3F6AAA", "#314F71"],
        ..Appearance::Windows.palette()
    },
    tint: |c| Color::from_rgb(c.r / 2, c.b, c.g.saturating_add(35)),
    style: || ferroui_themes_winui::WinUITheme::new().as_style(),
    variant: ThemeVariant::light,
});
```

Include `COBALT` in the registry slice. The chooser, numeric shortcuts, preference reader and command-line parser consume this registry. The profile can use a distinct FerroUI theme crate by supplying its style factory. A new panel placement requires a new layout strategy in `desktop.rs`. The test `fourth_descriptor_supplies_its_own_palette_on_an_existing_layout` exercises registration on a shared layout.
