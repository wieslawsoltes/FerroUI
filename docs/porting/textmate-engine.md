# TextMate grammar engine port

`ferroui-textmate` implements TextMate grammar execution, continuation state,
themes, packed token metadata, and incremental line invalidation. It is a
standalone Rust crate so the editor and other FerroUI controls can share the
same engine. The implementation is derived from TextMateSharp at
[`e0db45be30278c0f3f80302c31f498c8f8f1c4f2`](https://github.com/danipen/TextMateSharp/tree/e0db45be30278c0f3f80302c31f498c8f8f1c4f2).

This is an adapted implementation with tested behavior, not a claim that every
upstream public member has an identical Rust counterpart. Related internal
classes share Rust modules. The complete source inventory and exact symbol
mappings are maintained in [editor-source-map.md](editor-source-map.md).
Outstanding API and conformance boundaries are listed below.

## Source and ownership boundaries

| Upstream responsibility | Rust implementation | Adaptation |
|---|---|---|
| Raw grammar/rule interfaces, JSON and plist readers | `raw.rs`: `RawGrammar`, `RawRule`, `read_json` | Typed owned values, fallible parsing, JSON comments and trailing commas; XML plist uses a non-fetching parser |
| Rule factory, match/include/begin-end/begin-while/capture rules | `grammar.rs`: `Program`, `Node` | Indexed rule arena and shared lexical repositories replace the managed class graph |
| Grammar and state stack | `grammar.rs`: `Grammar`, `StateStack`, `Token`, `TokenizeLineResult` | Immutable continuation values; a grammar identity prevents accidental cross-grammar state reuse |
| Regex sources, scanners, anchors and backreferences | `regex_source.rs`: `RegexCache` and resolver functions | Cached real Oniguruma patterns; bounded optional pure Rust fallback |
| Scope selector matcher and injection priority | `selector.rs`: `ScopeSelector`, `SelectorPriority` | Parsed expressions for ancestry, alternatives, exclusions and priority |
| Theme readers, theme rules and trie | `theme.rs`: `Theme`, `TokenStyle`, `FontStyle`, `Color` | Deterministic selector cascade with a bounded scope cache rather than the original trie |
| Encoded attributes and binary tokenization | `metadata.rs`, `encoded_tokens.rs` | Explicit bit fields, color IDs, embedded-language and token-type overrides |
| Registry and synchronization registry | `registry.rs`: `Registry` | Explicit registration and shared compiled programs; replacing raw input invalidates future loads |
| Model, line tokens, invalidation and changed ranges | `model.rs`: `TokenizationModel`, `LineTokens`, `TokenizationUpdate` | Caller-scheduled batches and returned ranges replace managed background tasks and listener interfaces |
| Bundled registry options, language packages and themes | `catalog.rs`: `BundledCatalog`, `Language`, `LanguageConfiguration`, `ThemeName` | Compile-time resources, no runtime filesystem dependency |
| Browser regex engine | `browser.rs`, `browser/adapter.js` | Official `vscode-oniguruma` JavaScript/Wasm through `wasm-bindgen` |

The engine does not depend on a control, dispatcher, filesystem, wall clock, or
thread pool. `Rc` and `RefCell` give compiled grammars and regex caches explicit
ownership on the host thread. A worker constructs its own registry and grammar;
grammar handles are not `Send`. The editor translates tokens into FerroUI text
runs and schedules incremental work using its own services.

## Public host contract

```rust
use ferroui_textmate::{BundledCatalog, ThemeName};

let catalog = BundledCatalog::new()?;
let registry = catalog.registry()?;
let grammar = registry.load_grammar("source.rust")?;
let theme = catalog.theme(ThemeName::DarkPlus)?;

let first = grammar.tokenize_line("/* comment", None)?;
let second = grammar.tokenize_line("end */ fn main() {}", Some(&first.rule_stack))?;
for token in &second.tokens {
    let style = theme.match_scopes(&token.scopes);
    // token.start_index..token.end_index addresses UTF-16 code units.
    // Convert style colors and font flags to the host's text properties.
}
# Ok::<(), ferroui_textmate::Error>(())
```

Input is one line without its line delimiter. The engine appends the same
synthetic newline used by the upstream tokenizer and removes it from reported
token bounds. Rust strings are UTF-8, regex capture positions are byte offsets,
and public tokens use UTF-16 code units to match editor document coordinates.
Supplementary Unicode characters therefore occupy two public positions.
Adjacent spans with identical scopes are coalesced. Empty input has one token
covering `0..0`.

`tokenize_line_with_options` accepts a deterministic work budget and an optional
cancellation callback. `stopped_early` means the caller must retry or deliberately
choose a fallback; its partial stack is not a completed continuation state.
`TokenizationModel` never publishes an incomplete line as a valid cache entry.

`Grammar::tokenize_line2` emits alternating UTF-16 start positions and packed
metadata. `GrammarConfiguration` supplies the initial language ID, embedded
language scope prefixes, token-type selectors, and balanced/unbalanced bracket
selectors. `ColorMap` gives stable color IDs; index zero means no color. The
metadata layout matches the upstream 32-bit representation: 8 language bits,
2 token-type bits, 1 balanced-bracket bit, 4 font-style bits, 9 foreground bits,
and 8 background bits. An unrepresentable color index returns an error rather
than truncating.

## Grammar execution

The registry compiles raw grammars into an indexed program. Root rules are
reserved before any grammar is filled so includes can refer to grammars
registered in any order. Nested repositories retain lexical lookup, including
shadowed entries. `$self`, `$base`, `#repository`, an external scope, and an
external `scope#repository` are resolved during include expansion. Cyclic
include graphs are guarded. Missing optional includes follow upstream behavior
and can be inspected through `Grammar::unresolved_includes()`.

For each line, active `while` conditions are checked from the outer frame
inward. A failed condition removes that frame and its descendants. The main
scan selects the earliest matching begin, match, or end rule. Rule order breaks
ties; `applyEndPatternLast` changes end-rule precedence and a left-priority
injection wins a same-position match. Begin and end captures use the rule's
name scopes; nested contents use content scopes. Capture rules can retokenize
their captured subrange with their own patterns. End and while expressions
resolve numeric backreferences against the corresponding begin captures,
escaping captured text as regex literals.

The continuation stack retains dynamic end/while expressions and scope stacks.
`\A` is available only on the first line, and `\G` tracks the active begin/while
anchor. Popping a rule restores its parent's anchor. Per-line anchor locations
are deliberately excluded from continuation-state equality. Regex sources
disable unavailable anchors with the same escaped noncharacter as the upstream
implementation; substituting a different zero-width expression changes how
some real grammar regex comments are parsed.

Zero-width transitions are guarded using the cursor and rule stack, and there
are explicit limits on stack depth and capture retokenization depth. Default
limits are 100,000 scan/search steps, 256 frames, and 32 capture levels per
line. The regex cache is bounded at 8,192 patterns, including dynamically
resolved heredoc terminators. These are deterministic work limits, not elapsed
time limits.

## Native, browser and optional portable engines

| Target/configuration | Engine | Initialization and compatibility |
|---|---|---|
| Native default | `onig` 6.5.3, `onig_sys` 69.9.3, Oniguruma 6.9.10 | Native compilation from the dependency's vendored sources; bindgen is disabled |
| `wasm32-unknown-unknown` default | Official `vscode-oniguruma` 2.0.1, Oniguruma 6.9.5_rev1 | Await `initialize_browser_oniguruma()` once in each JS realm before tokenizing |
| Explicit `portable-regex`, or native without `native-onig` | `fancy-regex` 0.19.2 in Oniguruma mode | Restricted fallback; not full Oniguruma compatibility |

Browser builds embed the official unmodified scanner glue and Wasm binary.
The crate's build script wraps the glue in an inline `wasm-bindgen` module and
the initializer supplies owned Wasm bytes. It does not fetch an asset URL.
Initialization is shared per realm, so repeated calls await the same promise.
Workers initialize independently. Scanner captures cross the JS boundary as
UTF-16 indices and are translated back to byte offsets before scope processing.
Tokenizing before initialization returns an explicit backend initialization
error.

```rust,ignore
#[cfg(target_arch = "wasm32")]
ferroui_textmate::initialize_browser_oniguruma().await?;
// Registry creation and tokenization then use the same API as native builds.
```

The native scanner applies retry and match-stack limits. The official browser
scanner API does not expose equivalent limits, so cooperative cancellation is
observed between searches, not during a single native or Wasm regex search.
Hosts that require isolation from arbitrary third-party grammars can schedule
tokenization in a worker and retain control of worker termination.

The optional portable backend rejects patterns longer than 8,192 bytes or
containing more than 128 opening parentheses before compilation, bounds its
delegated regex allocation, and has a backtracking limit. These restrictions
are observable errors. It is not the default browser path and is not a way to
claim full support for all bundled language grammars. `Grammar::backend()`
reports which engine is active.

## Themes, languages and resources

The catalog contains 51 upstream packages with 72 language contributions,
79 grammar contributions, and 21 theme files. Language contributions can share
grammars; these counts are manifest entries, not a claim of 79 independent
language implementations. Extension lookup accepts a leading dot, filename
lookup prefers an exact name and otherwise the longest extension, and language
IDs are case-insensitive. `language_by_first_line` executes upstream shebang
and other first-line detection patterns through the selected regex backend.
Alias lists, raw language configuration and snippets retain their upstream
values.

`LanguageConfiguration` exposes common comment, bracket and word-pattern
settings, with the complete raw JSON available to host services for additional
settings. Package and grammar resources remain accessible by name. Compiling
without `bundled-grammars` omits embedded resources while retaining runtime
grammar/theme loading from caller-supplied strings.

Themes accept JSON with comments and trailing commas, XML plist `.tmTheme`
files, `tokenColors`/legacy `settings`, string and array selectors, inherited includes through a resolver,
editor colors, and explicit empty font-style resets. Include cycles return an
error. Scope selectors support ordered ancestry, prefix matching, alternatives,
exclusions, and parentheses. Theme rules cascade by specificity and source
order within each inheritance layer. The including theme's rules take
precedence over included rules, preserving TextMateSharp's theme layering.
The resolved style cache is bounded at 16,384 scope stacks.

The original package `cgmanifest.json` files are retained. The crate's
`resources/asset-licenses.json` records all 58 declared source components and
the full notice used for each; it identifies recovered notices where the
original repository or exact historical license was unavailable. Full notices
are stored under `resources/licenses/`. `resources/versions.json` records
versions, resource counts, browser asset hashes, and oracle provenance. The
crate-level `NOTICE.md` includes the TextMateSharp license and links the other
retained notices.

## Incremental invalidation

`TokenizationModel::set_text` recognizes LF, CRLF and CR. `replace_lines` retains
untouched suffix cache entries but makes the changed region and dependent
suffix unavailable to callers. `retokenize(max_lines)` starts from the last
valid predecessor state and returns the range recomputed. It stops after the
requested batch or after both the new end state and tokens converge with the
old suffix entry. A cancelled or budget-exhausted line remains invalid.

This preserves line-state dependency propagation without copying the upstream
background task implementation into a browser-incompatible runtime. The
returned `pending` flag and line range let the host reschedule and invalidate
only the visual lines whose highlights changed. Replacing the grammar
invalidates all cached states. State values from unrelated grammar instances
are rejected.

## Verification and reproduction

`tests/fixtures/upstream-tokens.json` is generated by official
`vscode-textmate` 9.3.2 with `vscode-oniguruma` 2.0.1 against the exact bundled
grammar files. It contains 74 lines across C#, Rust, Python, JavaScript,
TypeScript, JSON, CSS, HTML, Markdown, shell and YAML. Examples include nested
comments, raw/multiline strings, interpolation, heredocs, fenced code, embedded
HTML scripts/styles and supplementary Unicode. The fixture normalizes the
synthetic newline and coalesces equal adjacent scopes, matching the public
Rust token contract. Native differential tests compare every token start,
end and scope as well as the resulting stack depth.

Focused tests cover begin/end and while continuation, UTF-16 capture spans,
dynamic escaped backreferences, include graphs and lexical repositories,
injections, capture retokenization, zero-width progress, cancellation,
incremental convergence, theme inheritance/resets, metadata and encoded
language/token-type overrides. All bundled grammar resources are parsed and
all 21 themes are loaded. Corpus parity is evidence for those cases, not a
proof of every grammar and selector edge case.

```sh
cargo test -p ferroui-textmate --lib
cargo test -p ferroui-textmate --lib --features portable-regex
cargo check -p ferroui-textmate --target wasm32-unknown-unknown

# Regenerate the independent oracle; inspect fixture changes before committing.
cd src/FerroUI.TextMate/tests/oracle
npm ci
node generate.cjs
```

The Wasm conformance example runs the same fixture through the compiled Rust
engine and its real official Wasm scanner. With a `wasm-bindgen` CLI version
matching the workspace dependency:

```sh
cargo build -p ferroui-textmate --example textmate_browser_conformance \
  --target wasm32-unknown-unknown
wasm-bindgen --target experimental-nodejs-module --out-dir /tmp/textmate-wasm \
  target/wasm32-unknown-unknown/debug/examples/textmate_browser_conformance.wasm
node src/FerroUI.TextMate/tests/oracle/check-wasm.mjs \
  /tmp/textmate-wasm/textmate_browser_conformance.js
```

This exercises Wasm execution and JavaScript interop under Node. A browser
rendering or worker-lifecycle test is a separate host integration check.

## Remaining port and verification boundaries

- The managed `IRegistryOptions` lazy-loading callback interface, model listener
  interfaces, and task lifecycle are adapted to explicit registration,
  caller-supplied resolver functions, returned changed ranges, and host
  scheduling. They are not exposed as one-for-one Rust interface names.
- Raw package settings are retained, but the catalog does not automatically
  assign host language IDs or translate every contributed embedded-language
  setting into `GrammarConfiguration`; the host supplies those IDs.
- Theme lookup uses a selector cascade rather than the upstream trie. The
  full upstream theme and selector unit-test suites have not all been
  translated; the source map must not mark trie internals as an exact port.
- Compiled patterns are cached individually. The upstream multi-pattern
  scanner and its complete performance profile have not been reproduced.
  No claim of equivalent throughput on large files is made.
- The corpus verifies 74 lines in 11 language families. Broader generated
  differential tests, all upstream grammar tests, and a browser rendering
  test remain necessary before claiming complete behavioral parity.

These boundaries are tracked explicitly rather than represented by stubs or
silently substituted regex semantics.
