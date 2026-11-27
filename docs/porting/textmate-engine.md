# TextMate grammar engine port

`ferroui-textmate` implements TextMate grammar execution, continuation state,
themes, packed token metadata, and incremental line invalidation. It is a
standalone Rust crate so the editor and other FerroUI controls can share the
same engine. The implementation is derived from TextMateSharp at
`e0db45be30278c0f3f80302c31f498c8f8f1c4f2`.

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
| Theme readers, theme rules and trie | `theme.rs`: `Theme`, `TokenStyle`, `TokenStyleOverrides`, `FontStyle`, `Color` | Selector cascade with a bounded scope cache and separate host and encoded defaults |
| Encoded attributes and binary tokenization | `metadata.rs`, `encoded_tokens.rs` | Explicit bit fields, color IDs, embedded-language and token-type overrides |
| Registry, host options and synchronization registry | `registry.rs`: `Registry`, `IRegistryOptions`, `IGrammarRepository`, `IThemeProvider`, `DefaultLocator` | Lazy host resource lookup, dependency diagnostics and shared immutable compiled programs |
| Model, line tokens, invalidation and changed ranges | `model.rs`, `model_contract.rs`, `model_events.rs`: `TokenizationModel`, `ITMModel`, `IModelLines`, `ModelTokensChangedEvent` | Caller-scheduled batches and FerroUI event subscriptions replace managed background tasks |
| Borrowed input and tokenizer contract | `line_text.rs`: `LineText`, `ITokenizationSupport` | Borrowed/owned UTF-8 input with checked UTF-16 slice coordinates |
| Bundled registry options, language packages and themes | `catalog.rs`, `package.rs`: `BundledCatalog`, `Language`, `GrammarDefinition`, `GrammarContribution`, `ThemeName` | Typed package metadata, compile-time resources and host-assigned embedded language IDs |
| Filename pattern matching | `filename_glob.rs`: `FilenameGlob`, `FilenameGlobError` | Bounded iterative parser and epsilon-NFA, with checked glob syntax and no filesystem dependency |
| Language configuration and snippet data | `language_configuration.rs`: `Comments`, `Folding`, `Indentation`, `EnterRule`, `AutoPair`, `LanguageSnippets` | Typed settings preserve optional expressions, flags, multi-character pairs and snippet bodies |
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

`LineText` accepts borrowed or owned strings, strips one physical line ending
for the `ITokenizationSupport` adapter, and checks UTF-16 slice boundaries.
Slices that would split a supplementary character return `InvalidOffset`.
`IModelLines` exposes borrowed line text and lengths for host documents and
snapshots; `TokenizationModel::from_lines` takes an owned snapshot of that
source. Later line edits use `replace_lines` to preserve unaffected caches.

`Grammar::tokenize_line2` emits alternating UTF-16 start positions and packed
metadata. `GrammarConfiguration` supplies the initial language ID, embedded
language scope prefixes, token-type selectors, and balanced/unbalanced bracket
selectors. `ColorMap` gives stable color IDs; index zero means no color. The
metadata layout matches the upstream 32-bit representation: 8 language bits,
2 token-type bits, 1 balanced-bracket bit, 4 font-style bits, 9 foreground bits,
and 8 background bits. An unrepresentable color index returns an error rather
than truncating.

## Grammar execution

`Registry::with_options` accepts `Rc<dyn IRegistryOptions>`. The host supplies
raw grammars, target-specific injection names, theme sources and an optional
default theme. Synchronous callbacks can read a browser's asynchronously
preloaded resource map. No host callback runs while a registry map is borrowed.
Root load failures are returned; unavailable optional dependencies are recorded
in `GrammarLoadDiagnostic` while their includes remain inactive. Dependency
discovery traverses rules, repositories, injections and capture subrules.
Dependency and reentrant-load cycles are guarded. `IGrammarRepository` provides
already loaded sources to `Grammar::from_repository`.

The bundled catalog implements the same options contract. Eager `registry()`
loads all grammar sources; `Rc<BundledCatalog>::lazy_registry()` loads only the
requested dependency closure and the catalog's selected default theme. Both
honor package `injectTo` declarations. Manually registered grammars can set
host-specific injection lists with `Registry::set_injections`; without such
a list, registered injection grammars are considered by their selectors.

The registry compiles raw grammars into an indexed program. Root rules are
reserved before any grammar is filled so includes can refer to grammars
registered in any order. Nested repositories retain lexical lookup, including
shadowed entries. `$self`, `$base`, `#repository`, an external scope, and an
external `scope#repository` resolve through the compiled graph. The scanner
skips an unresolved include and also skips a rule when every declared
alternative is unavailable. This prevents begin/end and begin/while rules
with missing children from claiming subsequent lines. Intentionally empty
rules, mixed alternatives and recursive rules remain usable. Availability is
computed once for the active grammar and its `$base`, and grammar clones share
the result. External grammar names are available through
`Grammar::unresolved_includes()`.

A comparison with the pinned tokenizer matched all 20 available token streams,
including missing local and external fragments, begin/while rules, mixed
alternatives and deliberate empty rules. The pinned scanner raised an invalid
cast on one additional line after a malformed injection. The Rust scanner
kept that line in the host scope and highlighted the host match. Reloading
the registry with the missing external fragment enabled the rule in the new
grammar instance.

The loader selects one scanning form for each compiled rule. A rule with
`match` uses its match captures even when the source also supplies `begin`.
A begin rule with `while` continues until its condition fails, even when the
source also supplies `end`. The pinned compiler and the Rust regressions
agree on token ranges, scopes and continuation depth for both combinations.
Pinned rule factory: `src/TextMateSharp/Internal/Rules/RuleFactory.cs`

For each line, active `while` conditions are checked from the outer frame
inward. A successful condition may match ahead of the cursor. The frame's
content scopes cover the gap and its while captures, and the match end sets
the next anchor. A failed condition removes that frame and its descendants. The main
scan selects the earliest matching begin, match, or end rule. Rule order breaks
ties; `applyEndPatternLast` changes end-rule precedence and a left-priority
injection wins a same-position match. Begin and end captures use the rule's
name scopes; nested contents use content scopes. Capture rules can retokenize
their captured subrange with their own patterns. End and while expressions
resolve numeric backreferences against the corresponding begin captures,
escaping captured text as regex literals.

The continuation stack retains dynamic end/while expressions and scope stacks.
`\A` is available on the first line until the scan advances. `\G` tracks the
active begin or while anchor. A capture starts at its source offset with its
own anchor state. `\A` is available there only at the start of the document,
and `\G` becomes available when a begin rule inside the capture establishes
an anchor. Popping a rule restores its parent's anchor. Per-line anchor locations
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
| FerroUI Emscripten browser host | Same official browser scanner | Flat `raw_module` import and `OUT_DIR/browser-site` assets follow the existing browser site builder; application linking/rendering requires the host SDK |
| Explicit `portable-regex`, or native without `native-onig` | `fancy-regex` 0.19.2 in Oniguruma mode | Restricted fallback; not full Oniguruma compatibility |

Browser builds embed the official unmodified scanner glue and Wasm binary.
For `wasm32-unknown-unknown`, the crate's build script wraps the glue in an
inline `wasm-bindgen` module. For Emscripten, it emits the same glue and adapter
as `OUT_DIR/browser-site/ferroui-textmate-oniguruma.js`, along with the complete
scanner license and third-party notices. The binding uses a flat `raw_module`
import, matching FerroUI's existing browser interop. `scripts/build-browser.sh`
copies those assets and `scripts/browser/combine-site.mjs` shares one module
between the plain and threaded application variants. Inline snippet directory
imports would not satisfy that site's flat shared-import contract.
The initializer supplies owned Wasm bytes; it does not fetch an asset URL.
Initialization is shared per realm, so repeated calls await the same promise.
Workers initialize independently. An attempted search before initialization
returns an explicit backend initialization error.

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

### Browser scan input and offsets

The first regex search of a line creates one Oniguruma string for that scan.
Subsequent rules share its input and offset tables. A capture retokenizes its
bounded prefix with a separate input, so end anchors and lookarounds keep the
capture's original limits. Rust drops these inputs when their scans return.
First-line language detection also shares one input across its candidates.
The adapter converts the byte cursor into the scanner's UTF-16 coordinates
and returns flat byte ranges for captures. The grammar produces public UTF-16
token positions as before.

### Grouped browser rule scans

The browser tokenizer compiles each eligible ordered host or injection rule
list into one Oniguruma scanner. The returned pattern index identifies the
original rule and its capture ranges. Cache keys include the grammar root,
source rule, list kind and resolved `\A` and `\G` choices. The group cache
retains up to 8,192 compiled pattern slots, alongside the existing individual
expression cache.

The tokenizer compares the list winner with the current end rule and with
injection winners using the same precedence rules. An injection hit at the
cursor charges work through its winning rule. The ordinary scan handles
cancellation callbacks, limited remaining work, scanner errors and lists that
use `\K`, where the reported capture start can differ from the scanner's
internal match start. While conditions and dynamic end expressions still use
individual searches.

The Wasm differential compares tokens, continuation frames and early-stop state
with grouped scans enabled and with the individual scan forced by a cancellation
callback. Fixtures cover equal-position rules, left-priority injections, end
ordering, Unicode captures, anchors, `\K` and malformed expressions across
41 step budgets. Both upstream token-stream corpora also pass.

### Native line search reuse

The native scan retains results on each regex input. A future hit can serve
later cursors through the native engine's actual match start. A search that
finds no match can serve later cursors through the end of that input. Each
retokenized capture uses a separate cache for its bounded prefix.

The cache shares capture ranges and reuses one native capture region. Its
8,192-slot limit charges one slot per result and one more per retained capture
range. Keys use the source pattern. The cache checks first-line state for
patterns containing `\A` and contiguous-anchor state for patterns containing
`\G`. Other patterns keep their cached hits or misses through those state
changes. An active `\G` restricts reuse
to the original cursor. The scanner uses the engine's start offset to account
for `\K`. Callouts such as `(*SKIP)` run through Oniguruma on every search.
Search errors propagate immediately. A reused search still counts against the
tokenizer's work limit and checks cancellation at the original boundary.

The native tests cover reused and fresh searches across `\G`, `\K`, lookbehind,
optional captures, zero-width matches and `(*SKIP)`.

### Native ordered rule lists

Eligible native rule lists cache their compiled expressions together and
reuse the line's capture cache. The first rule that matches at the cursor
can win immediately. If every candidate is farther ahead, the earliest
match wins and equal positions keep grammar order. This follows the
[ordered scanner path in vscode-oniguruma 2.0.1](https://github.com/microsoft/vscode-oniguruma/blob/v2.0.1/src/onig.cc#L238-L274).
The native implementation uses the existing bounded Oniguruma searches.

The tokenizer still charges the established rule-list work budget.
Cancellation callbacks, groups containing `\K`, and malformed expressions
use the ordinary path. Regex limits still apply to each executed search.
Native differential tests cover anchors, Unicode captures, equal-position
injections, continuation stacks, malformed expressions, and 41 work budgets.

### Editor line-length limit

The editor skips TextMate scans for physical
lines longer than 3,000 UTF-16 code units, the visual-line cutoff for optional
generators. This check covers visible and idle highlighting, prospective
character classification, and the preview for optional paste indentation. An
oversized paste preview uses ordinary paste.

The installation retains the verified prefix. It renders the oversized
line and its suffix as plain text because the skipped line may change the
continuation state. Shortening the line lets the installation resume
highlighting. Hosts can change the cutoff with
`TextMateInstallation::set_max_tokenized_line_length` without rescanning
the verified prefix. Raising the cutoff lets the scan converge on cached
states, up to the next line that exceeds the new cutoff. Standalone
callers can pass full physical lines to `Grammar::tokenize_line_with_options`.

Expensive scans within the cutoff and precise highlighting of minified files
remain part of the tokenizer performance work.

The regression tests cover the exact UTF-16 boundary, recovery of multiline
state, changes to the cutoff, and clipboard paste and undo.

### Retrying lines that reach a limit

The installation remembers physical line, tokenizer-work and regex-work
limits at the line where tokenization stopped. Redraws and edits that
invalidate only later lines reuse that stop. Changing the failed line or
its incoming grammar state permits a new scan. A new grammar, work budget
or line limit also permits a retry. Cancellation leaves a line eligible
for another scan.

### Portable fallback

The optional portable backend limits patterns to 16 KiB of source and 1,024
opening parentheses. Fancy Regex applies its own nesting limit. The adapter
also bounds delegated regex allocation and backtracking. These limits report
errors when exceeded. The source and group bounds admit the bundled HTML
named-entity pattern and the JavaScript expressions used inside
`<script>` blocks. Some bundled C++ and Objective-C expressions exceed the
portable limits. `Grammar::backend()` reports the selected engine.

## Themes, languages and resources

The catalog contains 51 upstream packages with 72 language contributions,
79 grammar contributions, and 21 theme files. Language contributions can share
grammars; these counts are manifest entries, not a claim of 79 independent
language implementations. Extension lookup accepts a leading dot. Filename
lookup prefers exact basenames, then the longest matching filename pattern,
then the longest extension. All 15 bundled filename patterns are compiled when
the catalog is constructed. Patterns containing `/` inspect the full path;
other patterns inspect the basename. Both input path separators are recognized,
and comparisons fold ASCII case independently of the host operating system.
Equal-length filename patterns and shared extensions use deterministic
language-ID order. Once filename lookup chooses a suffix, it uses the same
language preference as direct extension lookup. Filename patterns identify
`tsconfig.app.json` as JSON with comments, `compose.dev.yaml` as Docker Compose,
and `.git/config` as Properties.
`language_by_first_line` executes upstream shebang
and other first-line detection patterns through the selected regex backend.
Alias lists, MIME types, configuration resource paths and snippets retain their
upstream values. Typed `GrammarDefinition` values preserve package provenance,
engines, scripts, repository and contribution metadata; unrecognized package
fields remain available in their `other` maps.

`FilenameGlob` supports the documented [VS Code glob operators](https://code.visualstudio.com/docs/editor/glob-patterns):
`*`, `?`, whole-component `**`, brace alternatives, and positive/negative
character classes and ranges. Nested brace groups use an explicit parse stack;
alternatives are never expanded into combinations. Compiled matching uses an
epsilon-NFA with work bounded by path length times pattern size and no recursive
backtracking. Patterns are limited to 8,192 UTF-8 bytes and 128 nested groups.
Malformed classes, reversed ranges, unmatched groups and backslash syntax return
structured errors with byte offsets. A character class cannot consume a path
separator. Brackets quote special characters, including `[[]` and `[]]`.
`?` consumes one Unicode scalar value; the optional insensitive comparison
folds ASCII, rather than claiming JavaScript UTF-16 or full Unicode case-folding
equivalence. Matching reads neither the filesystem nor the host OS. These
explicit rules also apply when applications use `FilenameGlob` directly.

### Host-supplied language packages

`BundledCatalog::with_packages` accepts extension manifests and their UTF-8
resources. The application can read a directory on desktop or preload files
in a browser. Each package participates in normal language detection,
configuration, snippets and embedded-language lookup. The lazy registry loads
its grammar, included scopes and declared injections as needed.

FerroUI keeps its catalog's existing language-ID precedence in both lookup
paths. The Rust regression registers the packages in reverse order and checks a
shared suffix, a longer compound suffix, mixed case, and Unicode filenames.
Pinned extension lookup: `src/TextMateSharp.Grammars/RegistryOptions.cs` (lines 153-174)

```rust
use ferroui_textmate::{BundledCatalog, LanguagePackage, ThemeName};
use std::rc::Rc;

let manifest = r#"{"contributes":{
    "languages":[{"id":"sample","extensions":[".sample"]}],
    "grammars":[{"language":"sample","scopeName":"source.sample","path":"./syntax.json"}]
}}"#;
let syntax = r#"{"scopeName":"source.sample","patterns":[{"match":"word","name":"keyword.sample"}]}"#;
let catalog = Rc::new(BundledCatalog::with_packages(
    ThemeName::DarkPlus,
    &[LanguagePackage {
        name: "sample",
        manifest,
        resources: &[("syntax.json", syntax)],
    }],
)?);
let scope = catalog.scope_by_extension(".sample").expect("sample package");
let grammar = catalog.clone().lazy_registry()?.load_grammar(scope)?;
```

Package names are their directory keys beneath `grammars/`. Supplying a package
with the same key replaces that bundled package. Duplicate supplied directories,
ambiguous grammar scopes and resource paths outside the package produce errors.
Resource paths accept either slash, and grammar and package lookups keep the
declared case.

The pinned QML fixture produced the same language discovery, configuration
presence and token spans from the reference file and directory loaders and the
FerroUI catalog. The four-line comparison applied FerroUI's documented
physical-line clipping and adjacent-scope coalescing to the reference tokens.
Pinned package tests: `src/TextMateSharp.Grammars.Tests/GrammarTests.cs`

The Wasm suite also discovered and tokenized a host-supplied package.

`LanguageConfiguration` exposes comments, brackets, folding markers, indentation
rules, enter actions, automatic/surrounding pairs and word expressions. Regex
settings accept string and `{pattern, flags}` forms. The Rust ECMAScript matcher
evaluates the word, Enter, indentation and conditional opener expressions in
UTF-16. Matching observes the declared flags, including Unicode sets, and
reports invalid expressions. Enter, indentation and opener checks start at
offset zero on every call. The configured word lookup scans the physical line,
retains the declared flags, advances past empty matches, and returns a UTF-16
range. If a package omits its word pattern, the host retains its normal word
navigation policy. Typed snippets preserve multiple prefixes, string/array
bodies, description and scope lists. The raw JSON remains available to hosts.

Native and linked browser configuration expressions use a 50 ms match budget.
On bare `wasm32-unknown-unknown`, the standard library has no monotonic clock,
so matching runs without a timer. Word lookup also limits the number of
matches it examines. TextMate grammar rules and first-line detection use the
selected scanner. See the [JavaScript word and caret reference](https://github.com/microsoft/vscode/blob/main/src/vs/editor/common/core/wordHelper.ts).

The editor can install automatic folding with
`TextEditor::install_language_folding`. It uses the document language's
markers and tab-aware indentation. Off-side languages leave trailing blank
lines outside indentation folds. Nested explicit markers include the closing
line and use the text after the opening match as the fold title. Markers use
the same ECMAScript engine as the other language-configuration expressions.
The provider scans a document snapshot and prefers shallower ranges when the
5,000-fold limit is reached. See the [indentation folding reference](https://github.com/microsoft/vscode/blob/main/src/vs/editor/contrib/folding/browser/indentRangeProvider.ts).

Automatic folding owns the manager while enabled. It coalesces updates after
document, language and editor-option changes and reconciles the new ranges with
existing anchors to retain collapsed state. Replacing the document creates a
manager for the new document. Hosts can use `install_folding` to manage ranges
directly. The EditorDemo uses the automatic provider.

Hosts register contributed snippets with `TextMateInstallation::set_package_snippets`
using the same numeric IDs as their grammar configuration. The editor resolves
the active language at the caret before providing native completion items.
Each prefix can trigger the same snippet, and accepting a completion replaces
its tracked UTF-16 range in one undo step. A named snippet can use the editor's
current selection, including selected-text defaults in Markdown and JavaScript.
The pinned corpus prepares with numeric stops, nested fields, mirrors, escaped
characters and selected-text variables. See [native snippet lifecycle](editor-snippets.md).

An editor host can pass a package's configuration to
`TextMateInstallation::set_language_configuration` for the root language or
use `set_language_configurations` with the grammar metadata and the rules for
each embedded language ID. On Enter, the first rule whose before-selection,
after-selection and previous-line expressions match supplies the action.
The editor limits each expression to text in the applicable language scope
and filters configured delimiters inside comment, string, and regex tokens
from that context. The editor uses its configured tabs or spaces and each
selection's native document delimiter. `indentOutdent` leaves the caret on the
inner line. All carets share one protected edit and undo step. Theme changes
retain the rules. When replacing the grammar, the host can supply the new
language's configuration.
The JavaScript and HTML samples use the bundled JavaScript rules to expand
and continue documentation comments in standalone code and `<script>` blocks.
[Language configuration guide](https://code.visualstudio.com/api/language-extensions/language-configuration-guide)

When no explicit Enter rule matches, the default indentation strategy uses
configured bracket pairs and then the package's four indentation patterns.
An opener can indent the next line or insert an inner line and align the
closing bracket. The regex rules increase or decrease indentation, apply
an increase for one line, or exclude a line from inheritance. Inheritance
uses up to 128 cached lines in the same language. The editor updates only
leading whitespace owned by that caret and keeps the native undo and
read-only checks. Custom strategies such as C# control inferred indentation
unless they opt in to the language rules.
[Reference Enter indentation](https://github.com/microsoft/vscode/blob/main/src/vs/editor/common/languages/autoIndent.ts)

Reindent uses the same bundled patterns on selected logical lines, or on the
whole document when no text is selected. Selected ranges inherit from the
preceding line. Embedded languages track their own indentation and resume the
enclosing language's level when their scope ends. The command preserves lines
starting inside multiline strings. It tokenizes through the last requested line
before committing a single protected, undoable batch. Custom strategies such
as C# retain control of their Reindent behavior.
[Reference Reindent behavior](https://github.com/microsoft/vscode/blob/main/src/vs/editor/contrib/indentation/common/indentation.ts)

Ordinary typing applies the decrease rule when the new character makes a line
match it. The editor can also return to the base level when a bracket opener
replaces one-line indentation. Only the leading whitespace and the typed text
are edited, and the edit shares an undo step with any generated single-character
pair. Multi-character openers retain a separate undo step for the closer.
The planner checks the active language and prospective token type and needs a
compatible change at each caret. Protected prefixes and input-method commits
use ordinary insertion.
[Reference indentation during typing](https://github.com/microsoft/vscode/blob/main/src/vs/editor/common/languages/autoIndent.ts)

Set `EditorOptions::auto_indent_on_paste` to apply the language policy to a
plain-text clipboard paste into a single physical selection. The editor
anchors the block in the destination's language and shifts subsequent lines
by the same column offset to preserve their relative indentation. The
clipboard's line endings are retained. Set `auto_indent_on_paste_within_string`
to `false` to preserve text pasted inside strings and multiline string lines.
Paste indentation defaults to off. Planning yields between tokenization
batches, checks the original edit target after each yield, and commits the
paste and indentation as one undo step.
[Reference paste indentation](https://github.com/microsoft/vscode/blob/main/src/vs/editor/contrib/indentation/browser/indentation.ts)

The same installation enables configured typing pairs. The editor finds the
longest opener and closer shared by the active language rules at all carets.
It checks the
configured following characters and `beforeText` expression, and evaluates
`notIn` against TextMate's comment, string and regex token types. Brackets supply
missing auto-closing pairs, and auto-closing pairs supply missing surround
rules. An explicit empty list disables that fallback. If the grammar state for
a conditional pair is outside the typing work budget, the typed character is
inserted on its own.

Generated closers follow text changes through document anchors. Typing a
generated single-character closer at its anchor advances the caret. Backspace
between adjacent generated single-character pairs deletes both characters in
one protected edit. Surrounding inserts at the selection boundaries, preserving
the directed UTF-16 selection and anchors within its contents. The browser and
desktop input-method paths that identify commits send their committed text
through ordinary insertion.

Completing a multi-character opener records the typed text and generated
closer as separate undo steps. Undo removes the closer first and leaves the
opener and its caret in place. Redo restores the caret before the closer.
The follow-up checks that the document, editor binding, selection and language
configuration still match and that protection allows the insertion. Typing
inside a transaction started by the host preserves the host's undo group.
[Reference pair behavior](https://github.com/microsoft/vscode/blob/main/src/vs/editor/common/cursor/cursorTypeEditOperations.ts)

For brackets declared by each active language, automatic closing checks for
a compatible unmatched closer anywhere at or after the caret. A closer can
match an enclosing opener across lines. Comments, strings and regex tokens
are skipped, and brackets from different languages are matched independently.
If any caret can reuse a closer, the editor inserts the typed
opener at every caret. The parser carries an opening stack across cached
lines and reuses the unchanged suffix when the grammar and bracket states
converge. It defers the document-wide decision while the background tokenizer
is catching up. [Reference bracket matching](https://github.com/microsoft/vscode/blob/main/src/vs/editor/common/model/bracketPairsTextModelPart/bracketPairsTree/bracketPairsTree.ts)

`GrammarContribution::configuration` and the catalog's `grammar_configuration`
translate embedded-language names, token-type overrides and balanced/unbalanced
bracket scopes to `GrammarConfiguration`. The host provides the numeric language
IDs; unknown languages retain zero or are omitted from embedded overrides.
Encoded tokens and editor input share scope resolution. When the token cache
cannot resolve a scope within the typing budget or a mapped language has no
installed rules, the editor uses ordinary input.
Package and grammar resources remain accessible by name. Enable the engine's
`bundled-grammars` feature to link the optional `ferroui-textmate-grammars`
crate. The default engine loads grammars and themes from caller-supplied strings.

Themes accept JSON with comments and trailing commas, XML plist `.tmTheme`
files, `tokenColors` and legacy `settings`, string and array selectors, inherited
includes through a resolver, editor colors, and explicit empty font-style
resets. Both lists contribute to one theme layer. Entry positions are counted
within each list, with `tokenColors` applied later when the positions tie.
A missing scope sets defaults. An explicit empty string or empty array produces
no rules. A whitespace-only selector or an empty element in a scope array sets
defaults alongside any named selectors in that array. Include cycles return an
error.

Scope selectors support ordered ancestry, prefix matching, alternatives,
whole-path exclusions, and parentheses. Callers can use `matches_with` to supply
a predicate for each ordered scope path. Within a theme layer, styling the
innermost scope takes precedence. Rules targeting that same scope are ranked by
the number of dot-separated target segments, then by parent count and parent
selector length from nearest to furthest, then by source order. Ancestor
positions determine whether a path matches. The including theme's rules take
precedence over included rules. The scope style cache is bounded at 16,384
scope stacks.

`Theme::style_overrides` combines explicit token defaults and matching scope
rules. The native editor applies only supplied properties so its foreground,
background and typeface can supply missing values. EditorDemo reads the theme's
UI palette to configure the canvas. `Theme::defaults` and `Theme::match_scopes`
resolve a complete style for callers, falling back to the UI palette and then
to black on white. Binary tokenization starts with the root theme's explicit
token defaults over black on white. Included scope rules still contribute to
the encoded style. Binary tokenization visits scopes from outermost to
innermost. At each scope it takes the first eligible theme layer, applies one
winning rule, and carries remaining properties forward from the enclosing
scope. The layer index preserves dotted-prefix inheritance, including when
the same parent condition is redeclared at a more specific dotted target.
The encoded cache keeps up to 16,384 scope prefixes so nearby tokens can reuse
inherited values.

The original package `cgmanifest.json` files are retained in the grammar crate. Its
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

`TokenizationModel::default()` starts with highlighting disabled, and
`grammar()` returns `Option<&Grammar>`. Edits still update the model while
highlighting is disabled. `clear_grammar()` discards cached tokens and states,
cancels pending model work and publishes one change for the full line range.
Clearing an already disabled model is a no-op. Setting a grammar marks the
first line pending, and setting a clone of the active grammar preserves
its cached states.

`tokens_changed` and `add_model_tokens_changed_listener` publish completed
ranges through `ferroui-system::events::Event`. Keep the returned
`ModelSubscription` guard alive for as long as the listener is needed.
Events carry zero-based half-open ranges, revision, line count and pending
status; they do not create a strong reference cycle back to the model. A regex
failure after some completed lines still publishes those completed ranges and
leaves the failing line invalid. Cancellation publishes no partial line.
`force_tokenization` advances through a requested line with the same bounded
per-line work policy. Model destruction releases owned caches and listeners;
the host owns worker/dispatcher scheduling and shutdown.

The editor's `TextMateInstallation` applies the same convergence rule to
persistent document snapshots. It maps batches of edits into the old and new
line spans, retains the aligned suffix and hides invalid tokens until a valid
continuation state is restored. Grammar changes reset states, while theme
changes notify the view without rescanning text. Six integration tests include
line insertion/removal, multiline-state propagation, CRLF edits, cancellation,
theme/grammar notifications and 200 deterministic multi-edit batches compared
with fresh tokenization.

`TextArea::apply_visual_line_highlighting` consumes the installation's explicit
token styles during visual-line construction. It clips each UTF-16 range to
its logical line and projects supplied foreground, background, weight,
italics, underline, and strikethrough to existing FerroUI text-run properties.
Applications can use `DocumentColorizingTransformer` for additional
document-offset styling.

## Tests and reproduction

`tests/fixtures/upstream-tokens.json` is generated by official
`vscode-textmate` 9.3.2 with `vscode-oniguruma` 2.0.1 against the exact bundled
grammar files. It contains 74 lines across C#, Rust, Python, JavaScript,
TypeScript, JSON, CSS, HTML, Markdown, shell and YAML. Examples include nested
comments, raw/multiline strings, interpolation, heredocs, fenced code, embedded
HTML scripts/styles and supplementary Unicode. The fixture normalizes the
synthetic newline and coalesces equal adjacent scopes, matching the public
Rust token contract. Native differential tests compare every token start,
end and scope as well as the resulting stack depth.

`tests/fixtures/catalog-tokens.json` exercises ten mixed editing lines for each
of the 76 distinct bundled scopes. Native Rust and Rust/Wasm match all 760
lines against the same independent oracle. The shared test checks that the
fixture covers every scope contributed by the catalog. FerroUI loads
`source.js.regexp` from its bundled `.tmLanguage` plist.

Focused tests cover begin/end and while continuation, UTF-16 capture spans,
dynamic escaped backreferences, include graphs and lexical repositories,
injections, capture retokenization, zero-width progress, cancellation,
incremental convergence, theme inheritance/resets, metadata and encoded
language/token-type overrides. All bundled grammar resources are parsed and
all 21 themes are loaded. Corpus parity is evidence for those cases, not a
proof of every grammar and selector edge case.

The matcher regression test translates all 25 cases in the pinned upstream
`MatcherTests` suite. Seven upstream theme color regressions use their actual
test theme fixtures. Additional host contract tests exercise lazy dependency
loading, injection target isolation, explicit listener disposal, model error
recovery, typed language/package settings and configuration-to-metadata mapping.
The deterministic large-set ordering rule is in
[the runtime contracts](textmate-runtime-contracts.md). Rust regressions cover whole-path exclusions, recovery
from an unclosed group, and a caller-supplied name predicate. A grammar
injection test verifies the observable token scopes across nested regions.

```sh
cargo test -p ferroui-textmate --lib
cargo test -p ferroui-textmate --lib --features portable-regex
cargo check -p ferroui-textmate --target wasm32-unknown-unknown
cargo check -p ferroui-edit-core -p ferroui-edit --lib --target wasm32-unknown-unknown
node src/FerroUI.TextMate/tests/oracle/check-browser-packaging.mjs

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
rendering or worker-lifecycle test is a separate host integration check. The
packaging regression compiles and executes the actual Rust build script for both
browser packaging modes, composes minimal application shells using the
production site builder, and executes the emitted official scanner. It does not
link an Emscripten application.

## Pinned TextMateSharp grammar cases

The TextMateSharp test suite loads its own C#, CSS, Batch File and TypeScript
grammar snapshots. FerroUI runs the same eight tokenizer cases against those
six original grammar and injection files. The expected output was captured
using the pinned test registry. The comparison checks UTF-16 ranges, complete
scope stacks, and completion of each of the eleven lines, including the
multiline C# case and the TypeScript decorator template. It also checks the
Dark VS foreground rule for the reference CSS property.
Pinned grammar tests: `src/TextMateSharp.Tests/Internal/Grammars/GrammarTests.cs`

The six inputs retain the original bytes, including line endings. The
fixture notice records the component revisions and licenses. The hashes for
the inputs, token stream, and four component notices are recorded in
`resources/versions.json` and checked by `scripts/edit-port/check_assets.py`.

## Remaining port boundaries

- Registry, repository, line, tokenizer and model interfaces have Rust host
  contracts. Managed stream overloads, mutable internal model-line objects,
  background tokenizer threads are not reproduced as identical object APIs.
  Resource I/O, scheduling and shutdown belong to the
  host; the engine owns immutable compiled programs and bounded batches.
- Language indentation patterns are active for Enter, typing, Reindent,
  and opt-in paste. Package snippets use the host's numeric language IDs.
  They resolve document context and accept host-provided variable values on
  named insertion and completion. Choice fields use the native completion list
  and existing linked editing. Declarative variable transforms run during
  expansion, and placeholder transforms run on field exit. Snippet transforms
  use an ECMAScript matcher in the editor core.
- macOS native input carries the input-method origin for commits that arrive
  after preedit clears. Live typing with composing input sources remains open.
- Rendered styles use a per-property cascade and encoded styles use a scope
  index. Rust regressions cover defaults, competing layers, dotted-prefix
  inheritance and competing parent paths; see
  [encoded scope inheritance](textmate-runtime-contracts.md#encoded-scope-inheritance).
  Custom themes and composition of styles in the live editor need
  broader reference coverage.
- Browser rule lists use the upstream multi-pattern scanner. Native groups
  reuse bounded per-expression searches and stop at the first eligible
  match at the cursor. The portable backend searches rules individually.
- The token corpus holds 74 language-specific lines and 760 mixed editing
  inputs covering all 76 bundled scopes. The complete upstream grammar tests and a
  wider set of language-specific inputs remain open.

These boundaries are tracked explicitly rather than represented by stubs or
silently substituted regex semantics.
