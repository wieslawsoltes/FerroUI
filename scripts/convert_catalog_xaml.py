#!/usr/bin/env python3
"""Converts the markup files of the upstream ControlCatalog sample to FerroUI markup.

The conversion is mechanical and changes names only, in this order:

1. the default XML namespace `https://github.com/avaloniaui` -> `https://github.com/ferroui`
2. the asset scheme `avares://` -> `ferres://`
3. links to the upstream web site (`https://avaloniaui.net/...`, `https://docs.avaloniaui.net/...`)
   -> the repository of this project (`https://github.com/wieslawsoltes/FerroUI`)
4. the root namespace `Avalonia.` -> `FerroUI.` (type, namespace and assembly names)
5. what is left of the upstream name, which may not appear in a markup file of this
   repository: `AVALONIA` -> `FERRO`, `AvaloniaUI` -> `FerroUI`, the name as the prefix of
   an identifier (`AvaloniaFlixHomeView`, `AvaloniaIcon_OnTapped`) -> `Ferro`, the name as a
   word of a text (`Text="Avalonia"`) -> `FerroUI`, and `an avares URI` -> `a ferres URI`

Everything else (elements, attributes, order, whitespace, byte order mark, line ends) is
kept byte for byte. A file that still contains the upstream name after the conversion is
reported and fails the run.

Changes of the port: `ADDITIONS` below lists the lines this project adds to converted documents
(the WinUI theme of the port, the theme of the catalog), each inserted after an anchor line of
the converted text, and `REMOVALS` the lines it takes out of them (the Fluent and Simple themes,
which the catalog has with its feature `classic-themes` only, in `ClassicThemes.xaml`), each with
the reason; the conversion applies the additions and then the removals, so `--check` compares a
document with its conversion and its changes, and fails when an anchor or a removed line is
gone. `PORT_DOCUMENTS` lists the markup files of the catalog that are the port's own, which
`--check` does not report as files without an upstream file. They are recorded in
docs/porting/DEVIATIONS.md (ControlCatalog sample) and samples/ControlCatalog/GAPS.md.

File names: `.axaml` becomes `.xaml`, and the upstream name in a file name is replaced as in
rule 5 (`AvaloniaFlixAppPage.xaml` -> `FerroFlixAppPage.xaml`). Directories are kept.

usage: convert_catalog_xaml.py <upstream sample dir> <target dir> [--check]

`--check` writes nothing and fails when a target file differs from the conversion of its
upstream file, when a converted file is missing, or when the target holds a markup file
that has no upstream file.
"""
import re
import sys
from pathlib import Path

REPOSITORY = b"https://github.com/wieslawsoltes/FerroUI"

FORBIDDEN = re.compile(rb"avalonia|\bavn|axaml|avares", re.IGNORECASE)

# The macOS appearance controls are an addition to the upstream settings page.
MACOS_SETTINGS = b'''
    <Border Classes="card" IsVisible="{Binding MacOSThemeSettingsVisible}">
      <StackPanel Spacing="10">
        <TextBlock Text="macOS 26 Appearance" FontWeight="SemiBold" />
        <TextBlock Text="Preview the control sizes, accent and accessibility settings across the catalog."
                   TextWrapping="Wrap" Foreground="{DynamicResource SubtleForeground}" />
        <Grid ColumnSpacing="10" ColumnDefinitions="*,Auto">
          <Label>Control Size</Label>
          <ComboBox x:Name="MacOSControlSize" Grid.Column="1" SelectedIndex="{Binding MacOSControlSizeIndex}">
            <ComboBoxItem>Mini</ComboBoxItem>
            <ComboBoxItem>Small</ComboBoxItem>
            <ComboBoxItem>Regular</ComboBoxItem>
            <ComboBoxItem>Large</ComboBoxItem>
            <ComboBoxItem>Extra Large</ComboBoxItem>
          </ComboBox>
        </Grid>
        <Grid ColumnSpacing="10" ColumnDefinitions="*,Auto">
          <Label>Accent</Label>
          <ComboBox x:Name="MacOSAccent" Grid.Column="1" SelectedIndex="{Binding MacOSAccentIndex}">
            <ComboBoxItem>System</ComboBoxItem>
            <ComboBoxItem>Blue</ComboBoxItem>
            <ComboBoxItem>Purple</ComboBoxItem>
            <ComboBoxItem>Pink</ComboBoxItem>
            <ComboBoxItem>Red</ComboBoxItem>
            <ComboBoxItem>Orange</ComboBoxItem>
            <ComboBoxItem>Yellow</ComboBoxItem>
            <ComboBoxItem>Green</ComboBoxItem>
            <ComboBoxItem>Graphite</ComboBoxItem>
          </ComboBox>
        </Grid>
        <CheckBox x:Name="MacOSReduceMotion" Content="Reduce Motion" IsChecked="{Binding MacOSReduceMotion}" />
        <CheckBox x:Name="MacOSReduceTransparency" Content="Reduce Transparency" IsChecked="{Binding MacOSReduceTransparency}" />
        <CheckBox x:Name="MacOSIncreaseContrast" Content="Increase Contrast" IsChecked="{Binding MacOSIncreaseContrast}" />
        <CheckBox x:Name="MacOSWindowActive" Content="Active Window" IsChecked="{Binding MacOSWindowActive}" />
      </StackPanel>
    </Border>
'''[1:]

# The lines this project adds to converted documents: (path relative to the sample, anchor line of
# the converted text, the lines inserted after it, why).
ADDITIONS = [
    (
        "Pages/SettingsPage.xaml",
        b'                  ToolTip.Tip="Window State"/>\n      </Grid>\n    </Border>\n',
        MACOS_SETTINGS,
        "live macOS 26 theme preview settings, visible when the optional macos-theme is selected",
    ),
    (
        "App.xaml",
        b"      <!-- Styles attached dynamically depending on current theme (simple or fluent) -->\n",
        b"      <!-- Not in the original: the WinUI theme of the port, the theme of the catalog. The Fluent\n"
        b"           and Simple themes and their styles are in ClassicThemes.xaml, with the feature\n"
        b"           `classic-themes` of the catalog only. -->\n"
        b'      <WinUITheme x:Key="WinUITheme" />\n',
        "the WinUI theme of the port (ferroui-themes-winui), the theme of the catalog",
    ),
    (
        "App.xaml",
        b'      <StyleInclude x:Key="ColorPickerFluent" Source="ferres://FerroUI.Controls.ColorPicker/Themes/Fluent/Fluent.xaml" />\n',
        b'      <StyleInclude x:Key="TreeDataGridFluent" Source="ferres://FerroUI.Controls.TreeDataGrid/Themes/Fluent.xaml" />\n',
        "the styles of the TreeDataGrid of the port, for the page of its demo",
    ),
    (
        "Pages/SettingsPage.xaml",
        b"        <Label>Theme</Label>\n",
        b"        <!-- Not in the original: the themes the catalog is built with (App.AvailableThemes), in\n"
        b"             place of the Fluent and Simple items, which need the feature `classic-themes`. -->\n",
        "the theme selector of the settings page lists the themes the catalog is built with",
    ),
    (
        "Pages/SettingsPage.xaml",
        b'                  SelectedItem="{x:Static local:App.CurrentTheme}"\n',
        b'                  ItemsSource="{x:Static local:App.AvailableThemes}"\n',
        "the theme selector of the settings page lists the themes the catalog is built with",
    ),
]

# The lines this project removes from converted documents (after the additions): (path relative to
# the sample, the lines, why). Each is in the converted text once.
REMOVALS = [
    (
        "App.xaml",
        b'      <FluentTheme x:Key="FluentTheme" />\n      <SimpleTheme x:Key="SimpleTheme" />\n',
        "the Fluent and Simple themes are in ClassicThemes.xaml, with the feature classic-themes only",
    ),
    (
        "App.xaml",
        b'      <StyleInclude x:Key="ColorPickerSimple" Source="ferres://FerroUI.Controls.ColorPicker/Themes/Simple/Simple.xaml" />\n',
        "the Simple styles of the colour picker are in ClassicThemes.xaml, with the feature classic-themes only",
    ),
    (
        "Pages/SettingsPage.xaml",
        b"          <ComboBox.Items>\n"
        b"            <models:CatalogTheme>Fluent</models:CatalogTheme>\n"
        b"            <models:CatalogTheme>Simple</models:CatalogTheme>\n"
        b"          </ComboBox.Items>\n",
        "the theme selector of the settings page lists the themes the catalog is built with (ItemsSource)",
    ),
]

# The markup files of the catalog that are the port's own: (path relative to the sample, why).
PORT_DOCUMENTS = [
    ("ClassicThemes.xaml", "the Fluent and Simple themes of the original, with the feature classic-themes only"),
    ("Pages/TreeDataGridPage.xaml", "the page of the TreeDataGrid demo of the port"),
    ("Pages/InkingPage.xaml", "the page of the inking controls of the port"),
    ("Pages/IconElementPage.xaml", "the page of the icons of WinUI (SymbolIcon, FontIcon; docs/porting/winui-controls.md)"),
]


def add(relative: str, data: bytes) -> bytes:
    """The converted text with the additions and the removals of the port to the document."""
    # Line ends of the document: the anchors and the lines are written with `\n`.
    crlf = b"\r\n" in data

    def with_line_ends(text: bytes) -> bytes:
        return text.replace(b"\n", b"\r\n") if crlf else text

    for path, anchor, lines, _why in ADDITIONS:
        if path != relative:
            continue
        anchor_text, lines_text = with_line_ends(anchor), with_line_ends(lines)
        if data.count(anchor_text) != 1:
            raise SystemExit(f"the anchor of an addition to {relative} is not in its conversion once: {anchor!r}")
        data = data.replace(anchor_text, anchor_text + lines_text)
    for path, lines, _why in REMOVALS:
        if path != relative:
            continue
        lines_text = with_line_ends(lines)
        if data.count(lines_text) != 1:
            raise SystemExit(f"the lines of a removal from {relative} are not in its conversion once: {lines!r}")
        data = data.replace(lines_text, b"")
    return data


def convert(data: bytes) -> bytes:
    data = data.replace(b"https://github.com/avaloniaui", b"https://github.com/ferroui")
    data = data.replace(b"avares://", b"ferres://")
    data = re.sub(rb"https://(?:docs\.)?avaloniaui\.net[^\"&<\s]*", REPOSITORY, data)
    data = data.replace(b"Avalonia.", b"FerroUI.")
    data = data.replace(b"AVALONIA", b"FERRO")
    data = data.replace(b"AvaloniaUI", b"FerroUI")
    data = re.sub(rb"Avalonia(?=[A-Z_])", b"Ferro", data)
    data = data.replace(b"Avalonia", b"FerroUI")
    data = data.replace(b"an avares URI", b"a ferres URI")
    return data


def convert_name(name: str) -> str:
    name = re.sub(r"\.axaml$", ".xaml", name)
    name = re.sub(r"Avalonia(?=[A-Z_])", "Ferro", name)
    return name.replace("Avalonia", "FerroUI")


def is_markup(path: Path) -> bool:
    return path.suffix in (".xaml", ".axaml")


def main() -> int:
    args = [a for a in sys.argv[1:] if a != "--check"]
    check = "--check" in sys.argv[1:]
    if len(args) != 2:
        print(__doc__)
        return 2
    upstream, target = Path(args[0]), Path(args[1])

    files = []
    for path in sorted(upstream.rglob("*")):
        relative = path.relative_to(upstream)
        if path.is_file() and is_markup(path) and not {"bin", "obj"} & set(relative.parts):
            files.append((path, target / relative.parent / convert_name(relative.name)))

    failed = False
    destinations = set()
    for source, destination in files:
        if destination in destinations:
            print(f"two upstream files convert to {destination}")
            failed = True
        destinations.add(destination)
        converted = add(destination.relative_to(target).as_posix(), convert(source.read_bytes()))
        if FORBIDDEN.search(converted) or FORBIDDEN.search(str(destination.relative_to(target)).encode()):
            print(f"upstream name left in {destination}: {FORBIDDEN.findall(converted)[:5]}")
            failed = True
        if check:
            if not destination.exists():
                print(f"missing: {destination}")
                failed = True
            elif destination.read_bytes() != converted:
                print(f"differs from upstream: {destination}")
                failed = True
        else:
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(converted)

    port_documents = {target / path for path, _why in PORT_DOCUMENTS}
    if check and target.exists():
        for path in sorted(target.rglob("*.xaml")):
            if "target" not in path.relative_to(target).parts and path not in destinations and path not in port_documents:
                print(f"no upstream file for: {path}")
                failed = True

    print(f"{len(files)} files {'checked' if check else 'converted'}")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
