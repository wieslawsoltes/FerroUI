# FerroRibbon browser gallery

The WebAssembly entry point opens the gallery used by the desktop sample.
FerroUI uses Inter as the browser's default font and the gallery loads its
bundled Symbols font.

The page displays the original gallery artwork while WebAssembly loads.
FerroUI dismisses the splash after the gallery draws its first frame.

The browser starts with the preferred browser language for UI and number
formatting. Command search, customization sorting and filtering, gallery
filtering, and localized number formats use ICU 68.2 from the pinned
Emscripten SDK. The build embeds the 28,566,176-byte ICU locale data image
and includes `ICU-LICENSE.txt` in the assembled site. The `licenses/`
directory includes the RibbonSpace and FerroUI licenses and the notices of the
direct dependencies.

Run these commands from the root of the FerroUI repository in Bash on Linux or macOS,
including a Linux shell in WSL. The FerroUI setup script uses its pinned Emscripten and
Rust versions. Node.js and npm are also required for the browser bundle.

```sh
scripts/browser/setup.sh
source .tools/env.sh
FERROUI_BROWSER_WORKSPACE="$PWD/components/FerroRibbon" \
  scripts/build-browser.sh ferroribbon-browser
python3 -m http.server 8000 --bind 127.0.0.1 \
  --directory components/FerroRibbon/target/browser/ferroribbon-browser
```

Open [FerroRibbon Gallery](http://127.0.0.1:8000/) in a browser with WebGL 2.

The build script's `--debug` option builds an unoptimized gallery.
The browser binary omits DWARF in that profile to stay within Chromium's
WebAssembly module size limit.
