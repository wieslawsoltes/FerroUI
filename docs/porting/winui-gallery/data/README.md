# WinUI gallery data

| File | What it is | Made by |
|---|---|---|
| `upstream-api.json` | The public API declarations of WinUI 3 (types, bases, members and events of `controls/dev` and the XAML object model of `dxaml/xcp/tools/XCPTypesAutoGen`), read lexically from microsoft-ui-xaml at `a71c42bd3e352060e5f5ecf3b5e1a98f8a745809` (SHA-256 `18174690ec96912a937edd09c4df5a4bbf0c76d91485aab21141451277ac18ab`). It is not a WinMD ABI, a signature check or a behaviour certification. | The scanner of the gallery port (`scripts/winui-gallery/scan.py`, W10 of #286) |

`scripts/winui-gallery/event_targets.py` reads `upstream-api.json` and writes the event-target table of the importer, `src/Markup/FerroUI.Markup.Xaml.WinUI/importer/event_targets.rs`. `--check` verifies that the table is up to date.
