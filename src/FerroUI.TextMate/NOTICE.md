# FerroUI.TextMate third-party notices

The grammar, state, theme, metadata, registry, and incremental tokenization
implementation is adapted from TextMateSharp, pinned to commit
`e0db45be30278c0f3f80302c31f498c8f8f1c4f2`:
https://github.com/danipen/TextMateSharp

The Rust implementation reorganizes related source types into modules and
replaces managed runtime dependencies with explicit ownership and host driven
scheduling. See `docs/porting/textmate-engine.md` for the implementation and
validation record and `docs/porting/editor-source-map.md` for the source map.

The bundled language packages and themes are copied from that same revision.
Their original component manifests are retained. `resources/asset-licenses.json`
records all 58 declared source components, their pinned revisions, notice
locations, and any case where the notice had to be recovered from a different
revision. Full notices are under `resources/licenses/`. Copyright and license
headers within the resources are retained.

The two theme regression fixtures under `tests/fixtures/theme-dark-*.json`
are copied from `src/TextMateSharp.Tests/Resources` at the same pinned revision.
They derive from Microsoft's default editor themes. The full Microsoft MIT
notice is retained in `resources/licenses/microsoft--vscode--main.md`; the
fixture paths and hashes are recorded in `resources/versions.json`.

The browser scanner bundles the unmodified JavaScript and WebAssembly artifacts
from `vscode-oniguruma` 2.0.1, distributed by Microsoft under the MIT license.
`browser/vendor/LICENSE.txt` and `browser/vendor/NOTICES.txt` contain the full
Microsoft and Oniguruma notices. `resources/versions.json` records exact asset
hashes and upstream versions. Native builds link Oniguruma through `onig` 6.5.3
and its transitive dependency `onig_sys`; those crates retain their own notices.

## TextMateSharp

MIT License

Copyright (c) 2021 Daniel Peñalba

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
