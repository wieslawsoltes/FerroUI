# NOTICE

The Rust sources of this crate and the TypeScript modules under `webapp/modules` are ported from the
`Avalonia.Browser` project of Avalonia (`src/Browser/Avalonia.Browser`, MIT; see the `NOTICE.md` at the root of
the repository). The TypeScript modules follow the upstream modules file by file with the names of the framework
changed and the parts of the threaded mode removed.

`webapp/modules/ferroui/caniuse.ts` contains the mobile browser detection expressions of
http://detectmobilebrowsers.com/ (public domain), as the upstream module does.

Build-time tools, not distributed with the crate: esbuild (MIT) and TypeScript (Apache-2.0), pinned in
`webapp/package.json` and `webapp/package-lock.json`.
