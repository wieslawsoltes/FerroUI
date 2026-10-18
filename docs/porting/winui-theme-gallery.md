# The WinUI theme gallery

`samples/WinUIThemeGallery` (crate `winui-theme-gallery`) is the gallery of the WinUI theme of the framework (`ferroui-themes-winui`, the port of the WinUI 3 theme of microsoft-ui-xaml): the controls of the framework in that theme, one page per family of controls, each control in its states. It exists to validate the port by eye and by pictures, above all on Windows, where real WinUI 3 (the Windows App SDK) is available and the same pages can be built from native WinUI 3 controls and compared pixel for pixel.

The crate has a library (the gallery) and two binaries:

| Binary | Hosts | What it does |
|---|---|---|
| `winui-theme-gallery` | macOS, Linux, Windows | The gallery in a window of the platform; with `FERROUI_THEME_GALLERY_CAPTURE` the capture mode, which writes a picture of every page in every setting and exits |
| `winui-theme-compare` | Windows | The comparison host: a WinUI 3 window with each page as native WinUI 3 controls beside the same page of the framework, both captured, with their difference |

The comparison host is a binary of the same crate rather than a crate of its own: it needs the pages, their native counterparts and the capture code of the library, and its Windows-only dependencies (`ferroui-winui`, the Windows backend with ANGLE, Skia, HarfBuzz) are target dependencies of the crate, so on another host the binary compiles to a message and the crate builds as everywhere else. Its application manifest (`app.manifest`, per-monitor DPI awareness) is linked into that binary only.

## 1. What the gallery shows

The window has a toolbar, the list of the pages and the selected page:

- **Theme**: `Light`, `Dark`, `HighContrast`. The variant is requested for the application (the window follows it) and for the page. `HighContrast` is a theme variant with the key `HighContrast`, which selects the `HighContrast` theme dictionaries of the theme where the theme has them; where it has none, the resources come from its default dictionaries. The framework itself has no high contrast variant of its own.
- **Density**: `normal`, `compact` (the density style of the theme).
- **Flow direction**: left to right, right to left (of the page).
- **Accent**: the accent colour of the system, or one of five accent colours of Windows, set on the light and the dark palette of the theme.

The pages, with the theme documents (`src/FerroUI.Themes.Fluent/Controls/*.xaml` lists the controls the themes style) each one shows:

| Page (id) | Controls | Native WinUI 3 counterpart |
|---|---|---|
| Buttons (`buttons`) | Button, the accent button, RepeatButton, ToggleButton, HyperlinkButton, DropDownButton, SplitButton, ToggleSplitButton | the same controls; the accent button is `AccentButtonStyle` |
| Check boxes and toggles (`toggles`) | CheckBox, RadioButton, ToggleSwitch | the same controls |
| Text input (`text`) | TextBox (placeholder, text, pointer over, disabled, read-only, password, several lines, validation error), DataValidationErrors, Label, TextBlock, SelectableTextBlock | TextBox, PasswordBox, TextBlock (with `IsTextSelectionEnabled`); the validation error is the critical border brush and the message under the box |
| Numbers and suggestions (`numbers`) | NumericUpDown, ButtonSpinner, AutoCompleteBox | NumberBox with inline spin buttons, a composed spinner, AutoSuggestBox |
| Combo box (`combobox`) | ComboBox, ComboBoxItem | the same controls |
| Lists and trees (`lists`) | ListBox, ListBoxItem, ItemsControl, TreeView, TreeViewItem | ListView, ItemsControl, TreeView with its nodes |
| Sliders, progress and scrolling (`range`) | Slider, ProgressBar, ScrollBar (with its Thumb), ScrollViewer | the same controls (the primitive ScrollBar) |
| Date and time (`datetime`) | Calendar (CalendarItem, CalendarButton, CalendarDayButton), CalendarDatePicker, DatePicker, TimePicker | CalendarView and the same pickers |
| Menus and command bars (`menus`) | Menu, MenuItem, MenuFlyoutPresenter, ContextMenu, MenuScrollViewer, CommandBar with CommandBarButton, CommandBarToggleButton, CommandBarSeparator | MenuBar, MenuFlyoutPresenter with its items (the items of a MenuFlyout of the resources as its `ItemsSource`, so that it shows icons and keyboard accelerator texts as an open menu does, and the narrow padding of a menu opened with the mouse), CommandBar with AppBarButton, AppBarToggleButton, AppBarSeparator |
| Flyouts, tool tips and notifications (`flyouts`) | FlyoutPresenter, ToolTip, NotificationCard, WindowNotificationManager | FlyoutPresenter, ToolTip, InfoBar |
| Tabs (`tabs`) | TabControl, TabItem, TabStrip, TabStripItem | TabView, NavigationView with its pane at the left, SelectorBar |
| Containers (`containers`) | Expander, GroupBox, HeaderedContentControl, Separator, SplitView, GridSplitter, PathIcon, TransitioningContentControl, ThemeVariantScope | Expander, SplitView, PathIcon; the others composed as the WinUI theme builds them (the header bar of an Expander over its content, the header of a text box over its content, the rectangle of MenuFlyoutSeparator, a bar of the strong control brush, an element with a requested theme) |
| Pages and navigation (`navigation`) | TabbedPage, NavigationPage, ContentPage, DrawerPage (its drawer a ListBox with the theme `DrawerPageNavigationViewMenu`, the menu items of NavigationView), CarouselPage, Carousel, PipsPager, RefreshContainer (RefreshVisualizer) | NavigationView (at the top, at the left), a title bar over a page, FlipView, PipsPager (with `PreviousButtonVisibility` and `NextButtonVisibility` `Visible`, the defaults of the framework's PipsPager, where WinUI collapses them), RefreshContainer |
| Table view (`table`) | TableView, TableViewColumnHeader, TableViewRow, TableViewCell | a stand-in: a row of headers over a ListView with the row heights and the cell paddings of TableView (TableView of WinUI is not in the Windows App SDK 1.6; the theme follows the TableView of microsoft-ui-xaml, so the fill of the header band and the grid lines differ by design) |
| Dialogs (`dialogs`) | ManagedFileChooser (without a view model, so its lists are empty and the picture does not depend on the files of the machine) | the layout of the chooser composed of WinUI controls (WinUI has no file chooser of its own) |
| Text calibration (`textcal`) | plain TextBlocks of the default font, one line per size and weight (12 to 32 px, regular and semibold), each at the top of a grid row of a whole number of device-independent pixels | the same grid of TextBlocks; the page measures the weight and the baseline of text per size (`text-rendering.md`, section 10) |

The other theme documents are shown by the gallery's window and popups rather than as elements of a page: `Window` (the window of the gallery), `AdornerLayer` (in every top level), `PopupRoot` and `OverlayPopupHost` (the popups of the drop-downs, flyouts and tool tips the window opens; a picture of a page does not hold its popups, which is why the menus, the flyout and the tool tips are shown in the pages through their presenters), `DatePickerPresenter`, `TimePickerPresenter` and `DateTimePickerShared` (the popups of the pickers), `TextSelectionHandle` (the selection handles of touch input), `EmbeddableControlRoot` (the root of the framework's page in the comparison host) and `WindowDrawnDecorations` (the decorations the framework draws for a window when the platform asks for them, on Wayland without decorations of the compositor).

### Layout

The grids of the pages, whose columns have fixed widths, are aligned to the left on both sides
(`HorizontalAlignment="Left"`). A grid of the framework that is wider than its fixed columns
gives each column one more pixel under layout rounding (phase 5 of its column allocation, as
upstream's and WPF's grid do), which moved the second column of every page 1 px to the right of
WinUI's and widened what it stretches by 1 px; a grid as wide as its columns allocates them
exactly. Aligning a WinUI grid of fixed columns to the left does not change it. The same holds
for rows: the text calibration page, whose grid has rows of fixed heights, aligns it to the top on
both sides (`VerticalAlignment="Top"`); stretched to the height of the page, the framework's grid
gave each row one more device pixel, and its last row was 11 px lower than WinUI's at 200 %
(`text-rendering.md`, section 10.2).

### States

Each page shows its controls normal and disabled, and, where the control has them, checked, unchecked, indeterminate, selected, expanded, with content, with a placeholder and with a validation error. The pointer-over and pressed states are forced: an element with the class `force-pointerover` gets the pseudo-class `:pointerover`, one with `force-pressed` gets `:pointerover` and `:pressed` (`capture::force_states`), once the page is laid out and its templates are applied, as the control sets them under a pointer. The framework lets any code set a pseudo-class (`pseudo_classes().set(..)`), so the forced states are the states the theme's selectors style. A control resets them on the next pointer event that reaches it, and a button resets `:pressed` when its template is applied again; the capture mode forces them after the layout and draws the page before anything else happens to it.

## 2. Running it on macOS and Linux

```sh
cargo run -p winui-theme-gallery --bin winui-theme-gallery
```

The capture mode writes a picture of every page in every setting and exits:

```sh
FERROUI_THEME_GALLERY_CAPTURE=target/winui-theme-gallery \
  cargo run -p winui-theme-gallery --bin winui-theme-gallery
python3 scripts/winui-theme/report.py target/winui-theme-gallery
```

The pictures are `<page>-<setting>.png`, where the setting is the variant in lower case, then `-compact` and `-rtl` where they apply (`buttons-light.png`, `buttons-dark-compact.png`, `menus-light-rtl.png`). Each is the host of the page (`CaptureHost.xaml`: the page in a theme variant scope on `ApplicationPageBackgroundThemeBrush`) drawn by the framework into a render target bitmap at a fixed size and scale: the frame the framework draws, never a capture of the screen. One line is printed per picture, and the process exits with 1 when anything failed:

- a page that does not load (its constructor panics: a document that does not load);
- a templated control without a template, or whose template made no visual (what a theme without a control theme for that control leaves);
- an error logged while the page was shown: an event of the bindings from the level warning on, an event of any area from the level error on (`capture::ProblemLog`, a log sink of the UI thread that passes every event on to the sink of the process);
- a picture that could not be written.

The options of the capture mode, all optional:

| Variable | Default | What |
|---|---|---|
| `FERROUI_THEME_GALLERY_PAGES` | every page | ids of pages, separated by commas |
| `FERROUI_THEME_GALLERY_VARIANTS` | `Light,Dark` | `Light`, `Dark`, `HighContrast` |
| `FERROUI_THEME_GALLERY_DENSITIES` | `normal,compact` | `normal`, `compact` |
| `FERROUI_THEME_GALLERY_FLOWS` | `ltr` | `ltr`, `rtl` |
| `FERROUI_THEME_GALLERY_SIZE` | `720x960` | the size of the host, in device-independent pixels |
| `FERROUI_THEME_GALLERY_SCALE` | `1` | the scale of the pictures (`2` is 192 DPI) |
| `FERROUI_THEME_GALLERY_SETTLE_MS` | `300` | the wait after a page is shown, and again after its states are forced |
| `FERROUI_THEME_GALLERY_ACCENT` | the platform's | an accent colour, `#RRGGBB` |
| `FERROUI_THEME_GALLERY_TIMEOUT_S` | `900` | a run that has not ended by then is ended with the exit code 3 |

`FERROUI_SMOKE_EXIT_MS` and `FERROUI_SMOKE_SCREENSHOT` work as for the other small samples (`sample_support::smoke_run`).

## 3. Running it on Windows

The gallery and its capture mode run as on the other hosts. The comparison host needs what the WinUI samples need (`winui.md`, section 7): the Windows App Runtime 1.6 on the machine and `Microsoft.WindowsAppRuntime.Bootstrap.dll` of that release beside the executable (`scripts\winui\fetch-windows-app-sdk.ps1 -Destination <target>\debug` puts it there).

```
cargo build -p winui-theme-gallery --bins
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\winui\fetch-windows-app-sdk.ps1 -Destination target\debug
target\debug\winui-theme-compare.exe --out target\winui-theme-compare
python scripts\winui-theme\report.py target\winui-theme-compare
```

### 3.1 The comparison

For each page and setting, the host shows the native page and the framework's page at the same size (720 by 960 device-independent pixels by default), in the same requested theme (`RequestedTheme` of WinUI, the theme variant of the framework), the same flow direction and the same density (the compact dictionary of WinUI, `ms-appx:///Microsoft.UI.Xaml/DensityStyles/Compact.xaml`, and the compact density style of the theme), and the accent colour of the system on both sides.

- **The native page** is the page's `Pages/<Page>.winui.xml`, beside its document `Pages/<Page>.xaml`: WinUI XAML with the same controls in the same layout, which the host wraps in a grid with the requested theme, the flow direction, the compact dictionary and the page background (`comparison::native_markup`) and loads with `Microsoft.UI.Xaml.Markup.XamlReader.Load`, as the WinUI samples load their markup. Where WinUI has no counterpart of a control, the native page composes the nearest WinUI controls, and its first comment says so; those parts are expected to differ.
- **The framework's page** is the same page as in the gallery, in its host, in the swap chain panel of `ferroui-winui`.
- **The window** has the two cells side by side when two pages fit the work area of its screen, and one over the other otherwise (the native cell over the panel); a page that does not fit at all fails the run with a message to set `FERROUI_WINUI_COMPARE_SIZE`. The window is placed inside the work area, because the system does not draw the part of a window that lies outside the desktop.

Then the host writes, per page and setting, into the output directory:

| File | What |
|---|---|
| `<page>-<setting>-winui.png` | the native cell, cut out of a picture of the client area of the window: the window printing itself (`PrintWindow` with `PW_RENDERFULLCONTENT`, which includes the composed content of WinUI), never the desktop |
| `<page>-<setting>-ferroui.png` | the framework's page drawn by the framework at the scale of the window (as the capture mode draws it) |
| `<page>-<setting>-ferroui-window.png` | side by side only: the panel cut out of the same picture of the window, which shows that the panel presents what the framework draws |
| `<page>-<setting>-diff.png` | the pixels that differ in red, over the native page in light grey |
| `compare.tsv` | one line per comparison: the page, the setting, the share of the pixels that differ, their number, the size, the pictures and the number of problems of the framework's page |

and prints a line per comparison: `compare buttons-light: 4.12 % of the pixels differ (28474 of 720 by 960 px)`. A pixel differs when a colour channel differs by more than the threshold (24 of 255 by default, `FERROUI_WINUI_COMPARE_THRESHOLD`), which absorbs the differences of anti-aliasing and of the rounding of colours; a pixel only one picture has differs. The process exits with 0 when every comparison was made and with 1 when a page could not be shown or captured on either side, or the framework's page had a problem of the capture mode (section 2). A difference of pixels is a result, not a failure.

The options are those of the capture mode with the prefix `FERROUI_WINUI_COMPARE` (`_PAGES`, `_VARIANTS`, `_DENSITIES`, `_FLOWS`, `_SIZE`, `_SETTLE_MS`, `_TIMEOUT_S`); the scale is the scale of the window's screen. The output directory is `--out <directory>` or `FERROUI_WINUI_COMPARE_OUT`. `FERROUI_WINUI_COMPARE_TEXT_VARIANTS` makes the run a calibration run of the text rendering: every page and setting once per text setting of the list, each into a directory of its name (`text-rendering.md`, section 10; `vm-smoke.ps1 -WinUIThemeGalleryTextVariants`). `HighContrast` is skipped with a line: high contrast is a setting of the system in WinUI, not a theme an element can request, and changing a setting of the system is not what a comparison run may do.

### 3.2 What cannot be forced on the native side

WinUI sets the pointer-over and pressed states of a control with `VisualStateManager.GoToState` or under a real pointer; markup cannot set them, and `ferroui-winui` does not bind `VisualStateManager`. The native pages show those states with the brushes WinUI gives the control in that state, set as local values, where the template of the control binds its background, border and foreground to the control and the normal state sets none: the buttons, the repeat button, the toggle button (unchecked), the hyperlink button, the drop-down button, the text box, the combo box and its items, the list items, the menu items, the tab items and the items of the selector bar. Where the template sets those brushes on its own parts (the check box, the radio button, the toggle switch, the slider, the expander, the menu bar), the native entry is in the normal state and its comment says so: its difference in the report is expected. Binding `IVisualStateManagerStatics` in `src/Windows/FerroUI.WinUI/interop/winui.idl` (generated by `scripts/winui/winui-idl.py` from the metadata of the Windows App SDK, `winui.md`, section 3.2) would let the host force the real states.

Dates cannot be set in WinUI markup either (WinUI XAML has no conversion of a text to a date), so the date page shows the calendars and pickers without a selection on both sides, and the calendars show the month of the day of the run. The native side formats dates in the culture of the system (en-US on the runner and in the virtual machine); the framework carries the data of the invariant culture only, so the application of the gallery sets the date patterns of en-US on the invariant culture (`app.rs`, `native_date_culture`): the header of the calendar is "October 2026" on both sides. An empty time picker shows the designator of the current time in the framework and "AM" in WinUI (DEVIATIONS.md, TP-3).

## 4. In the virtual machine

`scripts/windows/vm-smoke.ps1 -WinUIThemeGallery` builds the crate, runs the capture mode and the comparison host in the session of the interactive user, makes the HTML reports where the machine has Python, and copies the pictures, `compare.tsv` and the reports to `vm-smoke-winui-theme\capture` and `vm-smoke-winui-theme\compare` beside the report. From the host:

```
scripts/windows/vm-sync.sh <commit>
# in the machine, from the drive the share is mapped to:
powershell -NoProfile -ExecutionPolicy Bypass -File <S>:\ferroui-vm-windows\src-<commit>\scripts\windows\vm-smoke.ps1 -TargetDir C:\ferroui-target -Jobs 3 -InteractiveUser <user> -SkipTests -WinUIThemeGallery -Report <S>:\ferroui-vm-windows\vm-smoke-report.txt
```

The Windows App Runtime 1.6 must be installed (`-WinUI -InstallWindowsAppRuntime` installs it, with the owner's authorisation). Without Python on the machine, make the reports on the host from the copies:

```sh
python3 scripts/winui-theme/report.py <share>/ferroui-vm-windows/vm-smoke-winui-theme/compare
python3 scripts/winui-theme/report.py <share>/ferroui-vm-windows/vm-smoke-winui-theme/capture
```

### 4.1 In continuous integration

The Windows job of `.github/workflows/ci.yml` runs both modes on the desktop of its runner after
the WinUI step (which installs the Windows App Runtime 1.6 and puts the bootstrap library beside
the executables): the step "WinUI theme gallery (capture, comparison with native WinUI 3;
informative)" builds the two binaries, runs the capture mode into `target/winui-theme/capture`
and the comparison host into `target/winui-theme/compare`, makes both HTML reports, and the
artifact `winui-theme-gallery-windows` keeps the directory. The step does not fail the job while
the comparison is calibrated. It is the path that does not need the virtual machine (the runner
is x64; the virtual machine is ARM64).

## 5. Reading the report

`scripts/winui-theme/report.py <directory>` writes `report.html` into the directory (standard library only; the report names the pictures by their file names, so the directory can be copied with it). For a comparison directory it has:

- a summary by page and by setting: the number of comparisons, the mean and the largest share of differing pixels, the worst first;
- one row per comparison, the largest difference first: the native WinUI 3 page, the framework's page and their difference side by side, with the share of the pixels that differ and the number of problems of the framework's page (their lines are in the log of the run).

How to read it:

- **Red everywhere** (a share near 100 %) is a page whose background, size or position differs: the page background resource of the theme, the size of the host, or a native page that did not load (the log says so). Look at the two pictures before the difference.
- **Red at the edges of controls** is a difference of geometry: a corner radius, a border thickness, a padding or a minimum size of a control theme, or the compact density.
- **Red text** with the same layout is a difference of fonts or of text rendering (the native side renders text with DirectWrite, the framework with its text stack), and is expected in small amounts.
- **The quality of the text** (colour fringes, weight, solidity and vertical position of the rows) is measured with `scripts/text-quality.py <run>/compare`; `text-rendering.md` explains the numbers and what FerroUI's text should give.
- **Red fills** of whole controls are a difference of the brushes of the theme resources in that variant or state.
- **Entries the native page cannot force** (section 3.2) and **controls WinUI does not have** (the first comment of each native page) differ by design.
- The capture of the panel side by side (`-ferroui-window.png`) and the framework's frame (`-ferroui.png`) should look the same; when they do not, the panel presents something else than the framework draws, which is a matter of `ferroui-winui`, not of the theme.

For a directory of the capture mode the report shows every page in every setting, for review by eye.

## 5.1 Results

The comparison on the Windows runner of CI (x64, Windows App SDK 1.6, a work area of 1024 by
768, so the pages are compared at 720 by 660 from their top), share of differing pixels per page
in the light and dark variant:

| Page | First run (run 38928537418) | After the calibration (run 38938301589) |
|---|---|---|
| buttons | 4.46 / 2.36 | 2.43 / 2.35 |
| toggles | 2.52 / 2.10 | 2.10 / 2.10 |
| text | 2.84 / 2.95 | 2.71 / 2.79 |
| numbers | 3.02 / 3.20 | 2.09 / 2.10 |
| combobox | 1.95 / 2.00 | 1.94 / 1.99 |
| lists | 2.09 / 2.12 | 2.09 / 2.12 |
| range | 3.18 / 2.54 | 2.35 / 2.39 |
| datetime | 3.13 / 3.18 | 3.09 / 3.12 |
| menus | 3.54 / 2.63 | 2.98 / 2.45 |
| flyouts | 5.11 / 4.83 | 3.40 / 3.15 |
| tabs | 1.62 / 1.65 | 1.57 / 1.59 |
| containers | 2.78 / 2.80 | 2.61 / 2.63 |
| navigation | 4.16 / 4.20 | 3.13 / 3.17 |
| table | 14.05 / 14.24 | 13.49 / 13.64 |
| dialogs | 0.76 / 0.76 | 0.69 / 0.68 |

About 2 % is the floor of text: the native side draws text with DirectWrite, the framework with
Skia, and the edges of every glyph differ. The table compares the port of WinUI's TableView with
a stand-in (the release 1.6 has no TableView), whose header fill and grid lines differ by design.
The calibration between the two runs: the accent shades of the system, the shadows of popups,
the scroll indicator states, the layout and glyph of InfoBar, the selected item of DrawerPage,
the padding of menu items, the validation errors of NumericUpDown, ButtonSpinner and Slider, the
margins of the inner contents of text boxes, the chevron of ComboBox, the disabled navigation
glyphs of the calendar, and the native pages where they did not show what the framework's did.
The differences that remain are recorded in `winui-theme/DEVIATIONS.md`.

A third calibration followed, from that run and a run at a scaling of 2 in the Windows virtual
machine: the popup shadows drawn (the borders of the presenters, menus, drop-downs and tool tips
drew none, their backgrounds being sized to the inner edge of their borders), the validation
errors of the date and time pickers, the width of the calendar (300) and the rows of its days,
InfoBarPanel for notifications (a card of one line 48 high), path icons unscaled in the icon boxes
of the command bar, menu and tab templates, the carets of the symbol font, the date patterns of
en-US, the grids of the pages aligned to the left, and the native spinner in the compact density.

The comparison in the Windows 11 ARM64 virtual machine (Windows App SDK 1.6, a scaling of 2, the
pages compared at 720 by 912 from their top, pictures of 1440 by 1824 pixels), share of differing
pixels per page in the light and dark variant, before and after the third calibration:

| Page | Before (`b3a3ea90`) | After (`e72b8415`) |
|---|---|---|
| buttons | 1.33 / 1.29 | 1.33 / 1.29 |
| toggles | 1.19 / 1.19 | 1.19 / 1.19 |
| text | 1.71 / 1.72 | 1.67 / 1.68 |
| numbers | 1.06 / 1.06 | 1.00 / 1.00 |
| combobox | 1.04 / 1.06 | 0.99 / 1.00 |
| lists | 1.17 / 1.17 | 1.05 / 1.08 |
| range | 1.28 / 1.25 | 1.12 / 1.19 |
| datetime | 2.10 / 2.10 | 1.87 / 1.81 |
| menus | 1.62 / 1.27 | 1.52 / 1.18 |
| flyouts | 1.87 / 1.81 | 1.60 / 1.53 |
| tabs | 0.90 / 0.93 | 0.91 / 0.93 |
| containers | 1.71 / 1.70 | 1.57 / 1.55 |
| navigation | 1.65 / 1.63 | 1.48 / 1.46 |
| table | 9.45 / 9.53 | 9.45 / 9.53 |
| dialogs | 0.37 / 0.35 | 0.37 / 0.35 |

At a scaling of 2 the edges of glyphs are a smaller share of the picture, so the floor of text is
about 1 %.

The same comparison on 2026-10-18 (`03828103`), after the text calibration of
`text-rendering.md` and the scaled path icons, and with the calendar's day items at the padding of
CDB-10 (the text of the days had been 2 px below WinUI's: datetime 1.30 / 1.31 before):

| Page | Light / dark |
|---|---|
| buttons | 0.53 / 0.38 |
| toggles | 0.53 / 0.45 |
| text | 0.86 / 0.72 |
| numbers | 0.54 / 0.44 |
| combobox | 0.56 / 0.51 |
| lists | 0.54 / 0.45 |
| range | 0.60 / 0.50 |
| datetime | 0.93 / 0.87 |
| menus | 0.87 / 0.77 |
| flyouts | 0.82 / 0.65 |
| tabs | 0.54 / 0.51 |
| containers | 0.62 / 0.46 |
| navigation | 0.66 / 0.48 |
| table | 9.17 / 9.20 |
| dialogs | 0.15 / 0.11 |

What remains is mostly the edges of glyphs, the stand-in of the table, and the menus: a menu item
of WinUI is 29 px high where FerroUI's is 28, because WinUI lays out the clipped text of a menu item
(`MenuFlyoutItemTextTrimming` Clip) in another text mode than a plain text block, whose line height
the framework does not reproduce.

## 6. Files

| File | What |
|---|---|
| `samples/WinUIThemeGallery/theme.rs` | the one place of the code that names the theme crate (`use ferroui_themes_winui::{..}`): creation, density, accent, the variants (the markup of the flyouts page names the `InfoBarPanel` of the theme, `using:FerroUI.Themes.WinUI.Primitives`, for the content of its notification cards) |
| `samples/WinUIThemeGallery/app.rs`, `App.xaml` | the application: the theme first, then the styles of its document |
| `samples/WinUIThemeGallery/main_window.rs`, `MainWindow.xaml` | the window: the toolbar, the list, the page |
| `samples/WinUIThemeGallery/capture_host.rs`, `CaptureHost.xaml` | the host of a page, shared by the window, the capture mode and the comparison host |
| `samples/WinUIThemeGallery/capture.rs` | the settings, the options, the forced states, the checks, the pictures and the capture mode |
| `samples/WinUIThemeGallery/comparison.rs` | the wrapper of a native page and the comparison of two pictures |
| `samples/WinUIThemeGallery/Pages/` | the pages: `<Page>.xaml` with its class, and `<Page>.winui.xml`, its native counterpart |
| `samples/WinUIThemeGallery/compare/` | the comparison host: the WinUI application, its window, the run and the capture of the window |
| `scripts/winui-theme/report.py` | the HTML report |
| `scripts/windows/vm-smoke.ps1 -WinUIThemeGallery` | the run in the virtual machine |
