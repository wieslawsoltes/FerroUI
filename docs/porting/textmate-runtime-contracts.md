# TextMate runtime contracts and helper audit

This audit uses TextMateSharp revision
`e0db45be30278c0f3f80302c31f498c8f8f1c4f2`.
It distinguishes an upstream responsibility implemented by an existing Rust
contract from an upstream public helper API that Rust does not expose. Adding
a source alias to an adapted contract does not establish exact API or behavioral
parity. The generated [source map](editor-source-map.md) remains authoritative
for accepted mappings; the tables below provide member-level evidence for
reviewing its remaining runtime entries.

## Standard token types: a behavioral correction

The pinned
`BasicScopeAttributesProvider.ToStandardTokenType`
uses the first match of `\b(comment|string|regex|meta\.embedded)\b` anywhere
inside each scope name. Testing only the beginning of the scope loses types
such as `punctuation.definition.string.begin` and `meta.comment.block`.
`EncodedTokenAttributes::infer_token_type` now preserves this behavior:

- Within one scope, the first matching boundary word wins. `comment.string`
  means Comment; `string.meta.embedded` means String.
- The nearest recognized inner scope overrides its ancestors. An unrecognized
  inner scope inherits its parent's type.
- `meta.embedded` is an explicit Other value, not an absent value. It resets an
  enclosing Comment or String type. A still deeper String scope can override
  that reset.
- Matching is case sensitive. Underscores, letters and decimal digits do not
  create boundaries within an identifier.

Boundary classification follows
[`.NET RegexCharClass.IsBoundaryWordChar`](https://github.com/dotnet/runtime/blob/v8.0.0/src/libraries/System.Text.RegularExpressions/src/System/Text/RegularExpressions/RegexCharClass.cs):
Unicode categories `L`, `Mn`, `Nd`, `Pc`, plus U+200C and U+200D, are word
characters. `Mc`, `Me`, `Nl` and `No` are not. This is narrower than the Rust
regex library's default Unicode `\w`. .NET examines UTF-16 code units, so the
surrogates representing a supplementary scalar are non-word characters even
when the scalar is a letter. The Rust boundary helper preserves that distinction
while retaining valid UTF-8 input.

The implementation scans ASCII scope names directly. It uses the existing
`fancy-regex` dependency's explicit category class only for non-ASCII BMP
characters, with one lazily compiled expression and no new dependency or
browser scanner initialization. Unicode category data comes from the locked
Rust regex dependency; a different .NET runtime's Unicode database can differ
for characters introduced or reclassified between versions. These tests do not
claim an exhaustive comparison of every Unicode code point against every .NET
runtime.

Focused regressions cover the first-match rule, inheritance and embedded reset,
both sides of non-ASCII boundaries, supplementary characters, and the resulting
packed tokens from `Grammar::tokenize_line2`. All three fail with the previous
prefix-only implementation and pass with this correction.

```sh
cargo test --locked -p ferroui-textmate --lib token_type
cargo test --locked -p ferroui-textmate --lib --features portable-regex token_type
```

## Grammar execution and compiler helpers

Paths in the Rust column are relative to `src/FerroUI.TextMate`.

| Upstream type and members | Existing Rust destination | Mapping boundary |
| --- | --- | --- |
| `Grammar.Injection`: priority, rule ID, grammar and `Match(scopes)` | `grammar.rs`: `Program::compile`, its `(ScopeSelector, RuleId)` injection entries; `selector.rs`: `ScopeSelector::matches`, `SelectorPriority`; `Grammar::scan` | Ownership is represented by `Node::owner` and the program's roots. Local declarations and configured external lists retain their order within each priority. Pinned reference cases cover equal-position matches at and ahead of the cursor. |
| Internal `Grammar.GrammarRepository`: `Lookup`, `Injections` | `registry.rs`: `IGrammarRepository`, `Registry::lookup`, `Registry::injections`, `Grammar::from_repository`; `grammar.rs`: `Program::roots` | Repository lookup and dependency collection happen before returning a compiled grammar. The nested forwarding object is not part of the Rust API. |
| `IGrammarRegistry.GetExternalGrammar` | `registry.rs`: `Registry::load_dependencies`, `Grammar::from_repository`; `grammar.rs`: `Program::collect` | External rules and `$base` are resolved through the compiled arena and lexical repositories. Custom implementations of the managed compiler interface are not accepted. |
| `IRuleRegistry.GetRule`, `RegisterRule`; `IRuleFactoryHelper` | `grammar.rs`: `Program::reserve`, `Program::fill`, indexed `Program::nodes` | Two-phase reservation and filling supports recursive references. Rust accepts owned `RawGrammar`/`RawRule` values rather than externally subclassed compiled Rule objects or a rule-factory callback. |
| `MatchResult.CaptureIndexes`, `MatchedRuleId` | `grammar.rs`: `Match`; `regex_source.rs`: `Captures`; `Grammar::consider` | The private Rust match owns checked optional byte ranges and an arena ID. Public token bounds are converted to UTF-16 separately. |
| `MatchInjectionsResult.IsPriorityMatch` | `grammar.rs`: `Grammar::scan`, `selector.rs`: `SelectorPriority::Left` | The tokenizer chooses the injection candidate first. Only a left-priority injection wins when its start ties the host's best match or end rule. |
| `LineTokenizer.WhileStack`, `WhileCheckResult`: stack, rule, cursor, anchor and first-line flag | `grammar.rs`: `Frame`, `Grammar::tokenize_line_with_options`, `Grammar::scan` | The local state, cursor and anchor carry the helper members. They are not independently constructible objects. |
| `RegExpSourceListAnchorCache.A0_G0`, `A0_G1`, `A1_G0`, `A1_G1` | `regex_source.rs`: `resolve_anchors`, `RegexCache::search_group`, `RegexInput`, `LineSearch`, `RuleGroup`; `browser.rs`: `BrowserRegex::search_group` | Browser and native rule-list keys include both anchor choices. The browser scanner returns the winning rule and byte captures. Native groups reuse future matches and misses from bounded per-expression searches and can stop at the first match at the cursor. The portable engine searches expressions individually. |
| `MetadataConsts` masks and offsets | `metadata.rs`: `EncodedTokenAttributes` constructors/accessors | The packed layout is implemented directly. No constants-only wrapper is required; its boundary values have round-trip tests. |
| `OptionalStandardTokenType.NotSet` and concrete types | `metadata.rs`: `scope_token_type -> Option<StandardTokenType>` | `None` is absence/inheritance; `Some(Other)` is the explicit reset. The managed numeric sentinel 8 is not placed in the two-bit token field. |
| `IStateStack.Depth`, `RuleId`, `EndRule` | `grammar.rs`: `StateStack`, private `Frame` | Immutable continuations expose the active program-local rule ID and resolved end expression. Rust `depth()` counts nested frames. Upstream Depth includes the root. |

The compiled Rust `Grammar` retains the source name and file types alongside its
scope name. Compilation completes before the registry returns the handle.
Regex patterns are cached lazily during tokenization.

### Capture lists and grammar metadata

`captures`, `beginCaptures`, `endCaptures`, and `whileCaptures` accept numbered
dictionaries or arrays. Arrays start at group 1. Missing or null overrides
inherit shared captures, and explicit empty dictionaries or arrays suppress
them. Compiled grammar extensions strip one leading dot, matching the pinned
raw grammar reader.

The typed end-pattern precedence flag accepts `true` or integer `1`. The pinned
JSON reader forwards strings, objects and arrays to its raw grammar model.
FerroUI also decodes the Boolean and integer flag values from JSON and plist. An
unchanged pinned runtime confirmed the eight-line capture sequence, extension
normalization, and end-pattern tie handling when the typed flag was supplied to
the compiled grammar. Pinned raw grammar tests: `src/TextMateSharp.Tests/Internal/Grammars/Parser/RawTests.cs`

### Capture placeholders in scope names

The scope-name expander accepts numeric references such as `$1` and the
braced `:/upcase` and `:/downcase` commands. It strips leading dots from a
captured scope fragment. A group that exists but did not match contributes
empty text. A reference beyond the available groups stays literal, as does
malformed braced syntax. A later valid reference still expands when an earlier
braced expression is malformed or unclosed.

The shared native and Wasm regression compares the complete lists, including
capture zero, both case commands and the text following malformed syntax. Case
conversion uses Rust's Unicode casing and the placeholder parser uses ASCII
capture indices.

### Capture overrides

`beginCaptures`, `endCaptures`, and `whileCaptures` inherit the rule's
`captures` when omitted or set to JSON `null`. An explicit empty dictionary
suppresses the shared captures. A populated dictionary supplies the complete
override for that phase. In Rust, `RawRule` uses `None` for inheritance and
`Some(map)` for an explicit override. Optional grammar and rule collections
without inheritance use their empty defaults when set to `null`.

The shared regression checks normalized UTF-16 ranges, complete scope stacks and
continuation depth with native Oniguruma, the portable backend, and the browser
Oniguruma scanner through Rust/Wasm.

### Nested while continuations

The tokenizer emits the gap with the frame's content scopes and applies capture
scopes on top. The last successful condition anchors the ordinary scan at its
match end. When the inner condition fails, the outer content continues. When the
outer condition fails, the inner frame closes with it. The shared regression
also checks a supplementary character before the first match and validates the
UTF-16 ranges, ordered scopes, and continuation depths on each line. Pinned tokenizer: `src/TextMateSharp/Internal/Grammars/LineTokenizer.cs`

### Nested and overlapping captures

Captures emit in group order. Plain capture scopes use a stack, and a
retokenized capture starts with the surrounding rule's scopes plus its own
`name` and `contentName`. Plain scopes resume after that scan. Once a capture
emits a range, later captures can style only the unemitted suffix. A later
scan still sees the original text prefix for assertions such as lookbehind.

The pinned runtime confirmed the complete scope stacks for a plain parent
with a retokenized child, two nested retokenized captures, and two overlapping
retokenized captures. The larger corpus still matched all 5,694 lines in
UTF-16 ranges, ordered scopes and continuation depth.

### Zero-width rule recovery

The scanner records where each frame opened on the current line. When a
zero-width end immediately follows its opener at the same cursor, the frame
continues with its closing-delimiter scopes. A recursive zero-width begin
keeps the existing frame. A standalone zero-width match first moves the
cursor if its position is ahead, then exits one frame if it repeats there.
Inside a retokenized capture, that exit restores the surrounding rule's
scopes for the rest of the capture.

The frame entry positions reset on the next line and are excluded from
continuation equality. These paths complete the line with the same fallback
as the pinned tokenizer. The shared fixture compares seven lines across four
grammars with native Oniguruma, the portable backend and browser Oniguruma
through Rust/Wasm. It includes a dynamic end built from a captured delimiter.
The larger reference corpus continued to match all 5,694 lines.

### Captured delimiters in free-spacing rules

Dynamic `end` and `while` expressions substitute the same captured text with
regex literal escaping. Punctuation includes commas. Captured tabs and other
whitespace that an extended-mode parser may skip use explicit escapes.
This preserves the distinction between a captured delimiter and nearby
text with different indentation or spacing.

The pinned runtime dropped a raw captured tab under `(?x)`. The same
runtime matched the intended end and while delimiters when the tab was
written as an explicit escape. FerroEdit applies that escaping during
substitution. The shared regression also checks a captured nonbreaking
space through the portable backend and browser Oniguruma.
Pinned backreference resolver: `src/TextMateSharp/Internal/Rules/RegExpSource.cs`

## Raw values, selectors and theme helpers

| Upstream type and members | Existing Rust destination | Mapping boundary |
| --- | --- | --- |
| `IRawCaptures`: capture-key enumeration and `GetCapture` | `raw.rs`: `RawRule::{captures,begin_captures,end_captures,while_captures}`; `Program::fill` | Owned maps provide enumeration and lookup. Optional overrides distinguish inheritance from an explicit empty map. The compiler converts numeric keys to ordered capture entries. |
| `IRawRepository`: `GetProp`, `Merge`, `$self`/`$base` getters/setters | `raw.rs`: `RawRule::repository`, `RawGrammar::repository`; `Program::fill`, `Program::collect` | Lexical maps are cloned/extended and shared through `Rc`; owner and base rule IDs replace synthetic raw `$self` and `$base` entries. The mutable managed repository facade is not exposed. |
| `IMatchesName.Match`, `MatcherBuilder.Results`, and `MatcherBuilder.Tokenizer.Next` | `selector.rs`: `ScopeSelector::{parse_many,matches,matches_with}`, `Expr::rank`, `lex`, `Parser` | An ordered path is one operand. `matches_with` accepts a Rust closure over the entire path and caller data. `parse_many` keeps the priority on each comma branch. The lexer retains FerroUI's extended scope syntax. |
| `PListObject`: array/map accumulation, parent and pending key | `raw.rs`: `read_plist`, `PlistValue`, `plist_text` | XML plist dictionaries are read in declaration order, including injection selectors. Values feed owned Rust types for grammars and raw themes. |
| `SimpleJSON` nodes, enumerators and primitive wrappers | `serde_json::Value` through `raw.rs::read_json` | This is dependency reuse, not a port of SimpleJSON's operators, lazy mutation and iterator APIs. A local wrapper for each external JSON type would add no runtime functionality. |
| `IRawTheme`: name, include, settings/tokenColors and GUI colors | `RawTheme::parse`, `Theme::from_raw_with_resolver`, `Registry::load_raw_theme`, `Registry::compile_theme` | The parsed document retains its fields before include resolution. The compiler resolves resources through the caller or registry and loads both rule lists in the same layer. |
| `IRawThemeSetting`: name, scope and setting | `RawTheme::document`, `Theme::apply_rules`, private `Rule`, `ScopeSelector` | Names, string and array scopes, style fields and other parsed values stay editable in the raw document. Compiled rules use the existing selector and style types. |
| `IThemeSetting`: font style, background and foreground | `theme.rs`: `TokenStyle`, `TokenStyleOverrides`, `FontStyle`, `Color`, private `Rule` | Optional rule fields preserve inheritance and explicit font reset for native rendering. Encoded style resolution uses the root theme's token defaults. |
| Internal `ParsedTheme`: defaults, parsed rule order and lookup cache | `theme.rs`: `Theme::{defaults,match_scopes}`, private `Rule`, scope cache | Selector specificity ranks target depth, parent count and parent lengths within a target scope. FerroUI resolves style fields through its cascade. Full trie candidate ordering and inherited-style parity remain open. |
| `StringUtils`: indexed slicing, hex validation and string/array comparison | `line_text.rs`: `LineText::slice`; `theme.rs`: `Color::parse`; Rust comparisons in selector/theme processing | Checked UTF-16 slicing rejects a split surrogate pair. Color parsing requires a complete supported hex form; upstream validation accepts some longer strings with valid prefixes. Rust ordering does not reproduce culture-dependent `string.CompareTo`. These are explicit adaptations. |
| `GrammarNames.SupportedGrammars` | `FerroUI.TextMate.Grammars/build.rs` generates the resource inventory; `catalog.rs`: `BundledCatalog` | The 51 actual package resources drive enumeration. The internal managed spelling/casing array is not a separate Rust API. |

`RawTheme::parse` accepts the same JSON, JSONC and XML plist inputs as the
compiled theme loader. A host can edit the Serde document, serialize it as
JSON and compile it without a text round trip. The parsed values preserve
unknown keys, rule display names, nulls and array order. Input comments,
source formatting and alternate XML spelling are discarded. The regression
edits a JSONC rule, serializes and reparses it, and resolves a plist include
to check the effective foreground and font style. A registry test checks
resolution of a dependency introduced by a raw edit.
Pinned theme reader: `src/TextMateSharp/Internal/Themes/reader/ThemeReader.cs`

The shared XML decoder joins text around comments and CDATA in keys and
strings. Spaces around numbers are accepted. Finite real numbers are kept
as JSON numbers, and date and base64 data values remain strings in the raw
theme. Non-finite real values produce a parse diagnostic. Regressions compile
a grammar whose key and regex span multiple XML nodes and verify that plist
metadata survives raw-theme serialization before the theme is compiled.
[Property list XML tags](https://developer.apple.com/library/archive/documentation/CoreFoundation/Conceptual/CFPropertyLists/Articles/XMLTags.html)

Theme dependencies load through an explicit Rust stack. Each file applies its
base include, workbench colors, external token table and local token rules in
that order. A resource may appear in separate branches. A reference back to
any resource on the active stack reports an include cycle, even when the cycle
combines `include` and `tokenColors`. Empty `include` values skip resource
lookup, as in the pinned theme reader. A regression loads 4,097 nested
resources and verifies workbench colors, scoped styles, and the distinct
rendered and encoded token defaults.
Pinned theme reader: `src/TextMateSharp/Themes/Theme.cs` (line 154)

### Selector operands and recovery

The pinned matcher builder
passes a complete ordered identifier run to its name predicate. `- a b` therefore excludes scopes containing `a` followed by `b`. A scope stack
containing only `a`, only `b`, or `b` followed by `a` remains eligible. The Rust
matcher applies this rule to injections, token metadata, and theme rules. The
Boolean matcher short-circuits and needs no specificity vector. The theme
cascade continues to rank matching paths by specificity.

A separate check exercised custom predicates over integer input. In the Rust
grammar regression, the injection continues to apply inside `meta.container`
until entering a nested `comment`. Both the balanced selector and its version
with a missing closing parenthesis load successfully and produce the same
tokens.

FerroUI accepts hyphenated scope names, wildcard scopes and top-level pipes as
part of its existing selector syntax. Other punctuation can also tokenize
differently from the pinned builder. The parser reports excessive nesting
beyond 128 levels. The checked recovery cases cover only part of the upstream
builder and selector fixtures.

### Theme rule parsing and specificity

A fixture compiled against the pinned TextMateSharp checkout confirmed that
`settings` and `tokenColors` contribute to the same theme, including separate
properties on a shared selector. Rules from either list can win when their
entry index is greater. FerroUI gives the modern list a stable order after the
legacy list at equal indices. The fixture also checked default formatting for
missing, empty, whitespace-only and array-valued scopes.

A whitespace-only alternative supplies the token default while named
alternatives still match. An empty alternative between adjacent commas is
skipped. FerroUI applies this rule to top-level alternatives and preserves
commas inside grouped selectors. The regressions check foreground, background
and font flags through both rendering and encoding.

The pinned matcher ranked a selector with three parents above one with two
parents when both target the same scope. With the same parent count it preferred
the longer parent selector, even when the shorter one matched a closer ancestor
in the token stack. Length is measured in UTF-16 code units. FerroUI uses the
same measure for rendered and encoded styles after checking ancestry, while
preserving include precedence and the per-field cascade. Rust regressions cover
these fixtures and the seven pinned Dark+ color cases.

The cross-implementation fixture covers public candidate precedence and selected
parsed defaults. The Rust theme API returns a resolved style. The pinned editor
renderer takes the first supplied value for each property from the candidate
sequence. FerroUI checks parent paths before applying the style and supports
explicit font resets. Further comparisons should cover those cases and
inheritance in rendered and encoded tokens. [Pinned editor
transformer](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit.TextMate/TextMateColoringTransformer.cs#L319-L367)

### Style defaults in rendering and encoding

A theme with a UI palette only produced black-on-white encoded defaults.
Unscoped root token settings overrode them. An included theme contributed scoped
colors and font flags while the root supplied the encoded defaults. An empty
scoped font style cleared the inherited flags. FerroUI matches the token
boundaries and decoded styles for those cases.

Native rendering preserves the editor's foreground and canvas when the theme
supplies only UI palette values. The sample applies those values to the
editor palette explicitly. FerroUI also supports explicit unscoped token
settings in rendering and rich copy, including inherited token settings.
The pinned editor transformer uses scoped candidates for its text runs.
The visual regression verifies the configured canvas, plain foreground and
scoped glyph color. The copy regression verifies that only the styled UTF-16
span receives formatting.

### Encoded scope inheritance

The pinned runtime was probed with five scope stacks across seven themes.
At a single scope, an eligible rule in the including theme takes precedence.
Unspecified attributes carry forward from the enclosing token scope. Dotted
prefixes inherit plain attributes within a theme layer, and redeclarations of
the same rule merge their fields. A style-only plain candidate applies at a
known scope node. For an unknown dotted suffix it needs an effective foreground.
The Rust binary path indexes dotted prefixes and caches inherited scope styles.

Two conditional rules that target the same scope compete as whole rules. A
new condition inherits plain fields within the same layer. Redefining the
same parent path at a more specific dotted target preserves the broader
condition's fields. A custom fixture added a background to
`meta.block keyword.control` and preserved the foreground and italic style
from `meta.block keyword`. Without the qualifying ancestor, it kept the
plain `keyword.control` foreground and bold style.

The pinned parser includes the target in the parent list of a multi-part
selector. FerroUI evaluates tag-value exclusions and retains the whole-stack
cascade for Boolean and wildcard target selectors. Further comparisons should
cover custom themes and how the editors compose token styles with the canvas
and typeface.
Pinned theme parser: `src/TextMateSharp/Themes/Theme.cs` (lines 353-391)

## Model cache and token wrappers

`Model/Tokenizer.cs`
uses `DecodeMap` and `TMTokenDecodeData` only to derive an equality key for
adjacent tokens. It skips the root scope, splits the remaining scopes on dots,
and compares a set of interned components. That key loses component order and
scope grouping. For example, `foo.bar` and `bar.foo` produce the same component
set and can be coalesced despite carrying different scope names.

Rust's `grammar.rs::emit` merges adjacent spans only when their complete ordered
scope vectors are equal. This preserves information needed by selectors and
themes. The implementation should not add a DecodeMap wrapper that reintroduces
the lossy equality key merely to match a class name. Its differing coalescing
semantics belong in the source mapping's limitations.

| Upstream type and members | Existing Rust destination | Mapping boundary |
| --- | --- | --- |
| `TMToken.StartIndex`, `Scopes` | `grammar.rs`: `Token::{start_index,scopes,end_index}` | The same UTF-16 start and ordered scopes are present; Rust also carries the end bound. |
| `DecodeMap.getTokenIds`, `GetToken`, `PrevToken`; `TMTokenDecodeData.Scopes`, `ScopeTokensMaps` | `grammar.rs`: `ByteToken`, `emit` | Ordered-scope equality replaces component interning and previous-token dictionaries. This is a deliberate coalescing adaptation, not a direct implementation of their helper methods. |
| `ModelLine.IsInvalid`, `State`, `Tokens`, `ResetTokenizationState` | `model.rs`: `TokenizationModel` cache/invalidation state and `LineTokens` | Invalid entries are represented by `None` or masked by the invalid suffix; state/tokens are owned by each cached line. Public independent mutation of these fields is prevented. |
| `AbstractLineList`: add/remove/get/iterate, model binding, invalidate/force, text access and disposal | `model_contract.rs`: `IModelLines`; `model.rs`: `replace_lines`, `invalidate_lines`, `force_tokenization`, owned line/cache vectors | Borrowed host text is snapshotted and edits explicitly update the cache. Managed lock ordering, one-time SetModel publication, callbacks into a mutable line list and Dispose inheritance are not reproduced. |
| `TMModel.GetGrammar`, `SetGrammar` | `model.rs`: `TokenizationModel::{grammar,set_grammar,clear_grammar}`; `model_contract.rs`: `ITMModel` | A model without a grammar keeps its document and has no scheduled token work. Clearing an active grammar discards its state and reports one whole-document change. A new grammar resumes from the first line. |
| `TMModel.TokenizerThread` | `model.rs`: `retokenize_with_options`; the editor's scheduler | The host schedules bounded batches and owns cancellation. |

The public upstream `Tokenizer.Tokenize` overload also accepts `offsetDelta`
and `maxLen`. `ITokenizationSupport::tokenize_at` accepts an output offset and
an optional UTF-16 prefix limit. Invalid scalar boundaries and overflowing
output coordinates return an error. The work budget and immutable continuation
are shared with ordinary tokenization.

## Further behavioral checks identified by the audit

The first eligible injection at the cursor wins. While searching ahead, a later
injection can replace a candidate at the same offset. Only the resulting
left-priority candidate wins a tie with the host. FerroUI also retains local
declaration order when reading XML plist grammars.

FerroUI stably sorts these entries by priority, preserving local declaration
order followed by the configured external order. Automatic discovery for hosts
without a configured list uses scope-name order. FerroUI deterministically
highlights the next declaration (`local.1`). Both selected `local.23` when
searching ahead. The large-set regression records the stable Rust behavior
explicitly.

Independent mutation of individual model lines remains an API difference.
