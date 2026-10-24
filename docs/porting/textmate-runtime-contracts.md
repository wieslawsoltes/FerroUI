# TextMate runtime contracts and helper audit

This audit uses TextMateSharp revision
[`e0db45be30278c0f3f80302c31f498c8f8f1c4f2`](https://github.com/danipen/TextMateSharp/tree/e0db45be30278c0f3f80302c31f498c8f8f1c4f2/src/TextMateSharp).
It distinguishes an upstream responsibility implemented by an existing Rust
contract from an upstream public helper API that Rust does not expose. Adding
a source alias to an adapted contract does not establish exact API or behavioral
parity. The generated [source map](editor-source-map.md) remains authoritative
for accepted mappings; the tables below provide member-level evidence for
reviewing its remaining runtime entries.

## Standard token types: a behavioral correction

The pinned
[`BasicScopeAttributesProvider.ToStandardTokenType`](https://github.com/danipen/TextMateSharp/blob/e0db45be30278c0f3f80302c31f498c8f8f1c4f2/src/TextMateSharp/Internal/Grammars/BasicScopeAttributesProvider.cs)
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
| `Grammar.Injection`: priority, rule ID, grammar and `Match(scopes)` | `grammar.rs`: `Program::compile`, its `(ScopeSelector, RuleId)` injection entries; `selector.rs`: `ScopeSelector::matches`, `SelectorPriority`; `Grammar::scan` | The upstream descriptor is internal. Ownership is represented by `Node::owner` and the program's roots; no separate managed descriptor is needed. Same-priority injection ordering still needs an explicit differential test. |
| Internal `Grammar.GrammarRepository`: `Lookup`, `Injections` | `registry.rs`: `IGrammarRepository`, `Registry::lookup`, `Registry::injections`, `Grammar::from_repository`; `grammar.rs`: `Program::roots` | Repository lookup and dependency collection happen before returning a compiled grammar. The nested forwarding object is not part of the Rust API. |
| `IGrammarRegistry.GetExternalGrammar` | `registry.rs`: `Registry::load_dependencies`, `Grammar::from_repository`; `grammar.rs`: `Program::collect` | External rules and `$base` are resolved through the compiled arena and lexical repositories. Custom implementations of the managed compiler interface are not accepted. |
| `IRuleRegistry.GetRule`, `RegisterRule`; `IRuleFactoryHelper` | `grammar.rs`: `Program::reserve`, `Program::fill`, indexed `Program::nodes` | Two-phase reservation and filling supports recursive references. Rust accepts owned `RawGrammar`/`RawRule` values rather than externally subclassed compiled Rule objects or a rule-factory callback. |
| `MatchResult.CaptureIndexes`, `MatchedRuleId` | `grammar.rs`: `Match`; `regex_source.rs`: `Captures`; `Grammar::consider` | The private Rust match owns checked optional byte ranges and an arena ID. Public token bounds are converted to UTF-16 separately. |
| `MatchInjectionsResult.IsPriorityMatch` | `grammar.rs`: `Match::precedence`, `Grammar::scan` | Left-priority injections receive the winning tie rank; a second match-result class is unnecessary. |
| `LineTokenizer.WhileStack`, `WhileCheckResult`: stack, rule, cursor, anchor and first-line flag | `grammar.rs`: `Frame`, `Grammar::tokenize_line_with_options`, `Grammar::scan` | The local state, cursor and anchor carry the helper members. They are not independently constructible objects. |
| `RegExpSourceListAnchorCache.A0_G0`, `A0_G1`, `A1_G0`, `A1_G1` | `regex_source.rs`: `resolve_anchors`, `RegexCache::search` | The cache key is the resolved expression, incorporating both anchor choices. Patterns are cached individually; the upstream multi-pattern scanner's performance is not reproduced. |
| `MetadataConsts` masks and offsets | `metadata.rs`: `EncodedTokenAttributes` constructors/accessors | The packed layout is implemented directly. No constants-only wrapper is required; its boundary values have round-trip tests. |
| `OptionalStandardTokenType.NotSet` and concrete types | `metadata.rs`: `scope_token_type -> Option<StandardTokenType>` | `None` is absence/inheritance; `Some(Other)` is the explicit reset. The managed numeric sentinel 8 is not placed in the two-bit token field. |
| `IStateStack.Depth`, `RuleId`, `EndRule` | `grammar.rs`: `StateStack`, private `Frame` | Continuations, equality and cloning are present. Public rule/end inspection and mutable stack helpers are absent. Rust `depth()` counts active nested frames, excluding the root; upstream Depth includes the root. This is a partial API adaptation, not identical depth semantics. |

The compiled Rust `Grammar` exposes its scope name and tokenization methods.
Upstream `IGrammar.GetName()` and `GetFileTypes()` are available on the raw source
through `Registry::lookup` and `RawGrammar` fields, but are not retained as
accessors on the compiled handle. `IsCompiling` also has no exposed counterpart:
program construction is synchronous and regex patterns are cached lazily during
tokenization. These member distinctions should remain explicit if the
`IGrammar` source alias is broadened.

## Raw values, selectors and theme helpers

| Upstream type and members | Existing Rust destination | Mapping boundary |
| --- | --- | --- |
| `IRawCaptures`: capture-key enumeration and `GetCapture` | `raw.rs`: `RawRule::{captures,begin_captures,end_captures,while_captures}`; `Program::fill` | Owned `BTreeMap<String, RawRule>` values provide enumeration and lookup. The compiler converts numeric keys to ordered capture entries; no polymorphic captures interface is needed. |
| `IRawRepository`: `GetProp`, `Merge`, `$self`/`$base` getters/setters | `raw.rs`: `RawRule::repository`, `RawGrammar::repository`; `Program::fill`, `Program::collect` | Lexical maps are cloned/extended and shared through `Rc`; owner and base rule IDs replace synthetic raw `$self` and `$base` entries. The mutable managed repository facade is not exposed. |
| `IMatchesName.Match` and `MatcherBuilder.Tokenizer.Next` | `selector.rs`: `ScopeSelector::matches`, `Expr::rank`, `matches_name`, `lex`, `Parser` | Ancestor matching and parsing exist as concrete Rust logic. A caller-supplied generic name predicate is not supported. Error handling and permissive parsing of malformed selectors are not proven equivalent. |
| `PListObject`: array/map accumulation, parent and pending key | `raw.rs`: `read_plist` and its recursive XML-to-`serde_json::Value` conversion | The intermediate managed builder object is unnecessary. Supported plist value kinds and parser errors are the actual compatibility boundary. |
| `SimpleJSON` nodes, enumerators and primitive wrappers | `serde_json::Value` through `raw.rs::read_json` | This is dependency reuse, not a port of SimpleJSON's operators, lazy mutation and iterator APIs. A local wrapper for each external JSON type would add no runtime functionality. |
| `IRawTheme`: name, include, settings/tokenColors and GUI colors | `theme.rs`: `Theme::apply`, `Theme::{name,colors}`; JSON/plist input from `read_json_or_plist` | Input fields are consumed, includes resolved and colors parsed. Arbitrary raw object values and a mutable typed raw-theme API are not preserved for round trips. |
| `IRawThemeSetting`: name, scope and setting | `theme.rs`: `Theme::apply`, private `Rule`, `ScopeSelector` | String/array scopes and style fields are consumed. Rule display names are not retained; the raw setting interface cannot be implemented by callers. |
| `IThemeSetting`: font style, background and foreground | `theme.rs`: `TokenStyle`, `FontStyle`, `Color`, private `Rule` | Optional rule fields preserve inheritance and explicit font reset. Resolved styles are typed; unrecognized raw style data is discarded. |
| Internal `ParsedTheme`: defaults, parsed rule order and lookup cache | `theme.rs`: `Theme::{defaults,match_scopes}`, private `Rule`, scope cache | An ordered selector cascade replaces the trie and its mutable API. Existing corpus checks are finite; the complete upstream trie suite is not translated. |
| `StringUtils`: indexed slicing, hex validation and string/array comparison | `line_text.rs`: `LineText::slice`; `theme.rs`: `Color::parse`; Rust comparisons in selector/theme processing | Checked UTF-16 slicing rejects a split surrogate pair. Color parsing requires a complete supported hex form; upstream validation accepts some longer strings with valid prefixes. Rust ordering does not reproduce culture-dependent `string.CompareTo`. These are explicit adaptations. |
| `GrammarNames.SupportedGrammars` | `build.rs` generated resource inventory; `catalog.rs`: `BundledCatalog` | The 51 actual package resources drive enumeration. The internal managed spelling/casing array is not a separate Rust API. |

## Model cache and token wrappers

[`Model/Tokenizer.cs`](https://github.com/danipen/TextMateSharp/blob/e0db45be30278c0f3f80302c31f498c8f8f1c4f2/src/TextMateSharp/Model/Tokenizer.cs)
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
| `TMModel.TokenizerThread` | `model.rs`: `retokenize_with_options`; the editor's scheduler | Work budgets, cancellation between searches, convergence and event ranges are implemented. Thread creation, joining and the managed nullable-grammar lifecycle remain host/API differences. |

The public upstream `Tokenizer.Tokenize` overload also accepts `offsetDelta`
and `maxLen`. Rust callers can explicitly slice `LineText` and translate token
coordinates, but `ITokenizationSupport` has no corresponding offset/length
overload. A future convenience contract should preserve checked UTF-16 bounds
and cancellation rather than exposing mutable legacy state objects.

## Further behavioral checks identified by the audit

Configured external injection names are currently filtered through the
alphabetically ordered grammar map. Their supplied list order is not used to
break equal-position, equal-priority matches. The pinned compiler appends
injections in the repository's order before sorting by priority. Since .NET's
sort is not generally stable, this needs a targeted upstream differential case
before choosing or claiming identical tie behavior.

The raw theme interfaces, compiled grammar metadata, public state inspection,
mutable model-line facade and tokenizer slice overload should not be recorded
as exact public API parity solely because their underlying runtime behavior
has a Rust destination. These are reviewable API decisions or remaining work,
separate from the corrected token-type classification bug.
