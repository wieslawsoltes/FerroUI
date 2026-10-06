# NOTICE

This crate is a port of the browser host of the ControlCatalog sample of the upstream project
(Avalonia, `samples/ControlCatalog.Browser`, MIT; see the `NOTICE.md` at the root of the repository).

- `program.rs` ports `Program.cs`.
- `embed_sample_browser.rs` ports `EmbedSample.Browser.cs`, and `wwwroot/embed.js` is the upstream file of the
  same name.
- `wwwroot/index.html`, `wwwroot/app.css` and `wwwroot/main.js` port the files of the same name. The host
  page keeps the licence header of the upstream page (.NET Foundation, MIT). The script `main.js` starts
  the WebAssembly module of this crate instead of the .NET runtime.
- `wwwroot/favicon.svg` is original artwork of this project: the neutral placeholder mark the published
  site shows in place of the logo of the upstream project (the splash screen of `index.html` draws the
  same mark). The upstream `favicon.ico` is not copied.

Not ported: `Properties/launchSettings.json` (settings of the .NET development server).
