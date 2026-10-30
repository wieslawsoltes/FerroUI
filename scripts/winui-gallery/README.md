# WinUI API and Gallery scanner

`scan.py` compares the public API of WinUI (microsoft-ui-xaml) and the pages of WinUI Gallery with FerroUI's sources, at the pins of `docs/porting/winui-gallery/data/sources.lock.json` (docs/porting/winui-controls.md, sections 1 and 10.8). It is a source scanner: a same-name type or member is a *candidate*, never proof of behaviour, which the tests of the controls and of `samples/WinUIGallery` establish.

```sh
# The pinned checkouts (microsoft-ui-xaml needs only controls/dev, controls/idl and
# dxaml/xcp/tools/XCPTypesAutoGen).
python3 scripts/winui-gallery/scan.py --winui-root ../microsoft-ui-xaml --gallery-root ../WinUI-Gallery
python3 scripts/winui-gallery/scan.py --check --winui-root ../microsoft-ui-xaml --gallery-root ../WinUI-Gallery

# The event-target table of the WinUI XAML importer, from the scanned API.
python3 scripts/winui-gallery/event_targets.py --check

python3 -m unittest discover -s scripts/winui-gallery -p 'test_*.py'
```

Committed (`docs/porting/winui-gallery/data/`): `sources.lock.json` (the two pins), the reviewed map `api-mappings.json` (hand-maintained: type and member mappings, overloads, XAML feature dispositions with their implementation and test anchors), and the generated `summary.json` and `REPORT.md`. The inventories and maps (`upstream-api.json`, `gallery-pages.json`, `gallery-usage.json`, `source-files.json`, `api-map.json`, `gallery-page-map.json`, `gallery-xaml-map.json`, `xaml-feature-map.json`) are written to `--output-dir` (`target/winui-gallery-scan` by default) and are not committed (O8). A checkout that is not at its pin, or has changed tracked files, is refused.

The page states come from `samples/WinUIGallery/port-status.json`: `pending`, `partial` or `ported`. A page cannot be `verified`, which would need a record of a test run (per-run evidence, O8).

The workflow `.github/workflows/winui-gallery-scan.yml` checks out the pins, runs the tests, `scan.py --check` and `event_targets.py --check`, puts the report in the run summary and keeps the inventories as an artifact. On a pull request it runs for the paths of the scanner and fails on a stale report; at night it reports the drift that changes of the controls make, without failing.
