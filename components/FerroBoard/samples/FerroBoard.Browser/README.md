# FerroBoard browser sample

This hosts the existing compiled WinUI/XAML sample, keyboard engine and native
FerroUI controls in its browser platform. It does not implement an HTML keyboard.
Project/crate names use FerroBoard; public controls retain OnScreenKeyboard and
ScreenKeyboardHost.

Activate Emscripten 6.0.10 and put wasm-bindgen 0.2.129 on PATH. From the repository:

```sh
source scripts/env.sh
python3 scripts/build_browser.py --work-dir /Volumes/SSD/temp/ferrokeyboard/browser-work \
  --out /Volumes/SSD/temp/ferrokeyboard/new-browser-site
python3 scripts/check_browser_bundle.py /Volumes/SSD/temp/ferrokeyboard/new-browser-site
python3 -m http.server --bind 127.0.0.1 --directory /Volumes/SSD/temp/ferrokeyboard/new-browser-site
```

Work/output directories must be external to the checkout; output must be new.
Failed attempt logs remain in work/attempts/. Runtime JS builds in an isolated
copy of the exact SDK sources. Shared Cargo versions/checksums remain pinned.
Node/npm and first-use dependency downloads are required. Each dev build records
hashes, sizes, lock and runtime-source inputs.

Persistence uses synchronous browser localStorage under ferroboard.sample., with
the original settings/history/dictionary JSON formats. Denied storage raises
errors through the existing source error boundary; no in-memory success fallback
or virtual filesystem is substituted. local_folder explicitly reports an unsupported
native filesystem. Visibility/page-hide events save data. An options query parameter
passes its decoded string to the existing sample parser.

Selected browser typing, field navigation and persistence checks pass. Mobile system
IME, physical multitouch, broad clipboard/font/emoji/RTL/accessibility, embedded-frame
activation, complete lifecycle and other-browser/platform/release checks remain open.
See docs/NATIVE-BROWSER-HOST.md for exact validation and limitations.
