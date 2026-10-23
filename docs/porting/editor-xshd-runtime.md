# XSHD runtime regex, style and loader audit

This checkpoint follows the active AvaloniaEdit source at
[`7277e36366d754219d7ede16b6613786b32caf4c`](https://github.com/AvaloniaUI/AvaloniaEdit/tree/7277e36366d754219d7ede16b6613786b32caf4c).
It corrects executable regex semantics and rule-region matching in
`FerroUI.Edit.Core`. It also distinguishes active loader behavior from the
commented schema-validation code. It does not claim that the selected Rust
regex backend implements the complete .NET regex language.

## Source and member map

All Rust paths in this table are relative to `src/FerroUI.Edit.Core/`.

| Upstream source/member | Rust implementation | Audited contract |
| --- | --- | --- |
| [`Highlighting/Xshd/XmlHighlightingDefinition.cs`, `CreateRegex`](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/Highlighting/Xshd/XmlHighlightingDefinition.cs) | `highlighting/xshd/compiler.rs::compile_regex`, `highlighting/xshd/regex_compat.rs::explicit_capture` | Default explicit captures; inherited case setting; per-expression whitespace mode; source-positioned compilation failures |
| Same file, `VisitRule`, `VisitSpan`, `VisitKeywords` | Same compiler entry point for rules, begin/end expressions and generated keyword patterns | Capture policy applies consistently; the editable tree retains original pattern text |
| [`Highlighting/HighlightingEngine.cs`, `HighlightNonSpans`](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/Highlighting/HighlightingEngine.cs) | `highlighting/highlighting_engine.rs::highlight_non_spans` | Bounded matches, source-order ties, per-region match caching, zero-length rule failure |
| Same file, `HighlightLineInternal` | `highlighting/highlighting_engine.rs::highlight_line` | Span boundaries select each rule region; span state remains separate from non-span rule matching |
| Same file, `ScanLine` | `highlighting/highlighting_engine.rs::scan_line` | Span state transitions without rule evaluation, color-section construction or a UTF-16 offset table |
| [`Highlighting/DocumentHighlighter.cs`, `HighlightLine`, `HighlightUpTo`, `UpdateHighlightingState`](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/Highlighting/DocumentHighlighter.cs) | `highlighting/document_highlighter.rs` | Preceding and state-only lines use span scans; the requested line evaluates its ordinary rules |
| [`Highlighting/Xshd/HighlightingLoader.cs`, `GetValidatingReader`](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/Highlighting/Xshd/HighlightingLoader.cs) and [`V2Loader.cs`, `LoadDefinition`](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/Highlighting/Xshd/V2Loader.cs) | `highlighting/xshd/loader.rs` | The pinned active loader does not install an XSD validator; Rust validates supported structure and values directly |
| [`Highlighting/HighlightingColor.cs`](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/Highlighting/HighlightingColor.cs) and `XmlHighlightingDefinition.VisitColor` | `highlighting/highlighting_definition.rs::HighlightingColor`, `highlighting/xshd/compiler.rs::runtime_color` | Existing optional runtime font and brush values, style overlay and independently clearable decorations |

The wider syntax-tree, visitor, serialization, import and legacy-format mapping
is in [editor-services.md](editor-services.md). Native font, brush, rich-text and
clipboard integration is covered separately in
[editor-rich-text-clipboard.md](editor-rich-text-clipboard.md).

## Explicit captures and regex option scopes

The source compiler sets `RegexOptions.ExplicitCapture` on every expression.
The Rust backend, `fancy-regex` 0.19.2, has no equivalent global `n` option.
The compiler therefore normalizes executable group syntax before building the
regex. The original `XshdRule`, `XshdSpan` and serialized XML patterns are not
rewritten.

Ordinary parentheses are noncapturing by default. Named groups still capture.
Both scoped and unscoped `n`/`-n` options are interpreted, and enclosing groups
restore their option context. Escapes, character classes, extended-mode line
comments and `(?#...)` comments are scanned without treating their parentheses
as group delimiters. Scoped wrappers also restore backend option state around
named captures, lookarounds and atomic groups.

.NET numbers enabled unnamed captures before named captures. The backend
numbers captures in encounter order. The normalizer resolves the source
numbers and names, then rewrites backreferences and capture-existence
conditions to the corresponding backend slots. For example,
`(?<named>a)(?-n:(b))\1\2` matches `abba`: source group 1 is `b` and source
group 2 is `a`. This follows the documented
[.NET capture-numbering and explicit-capture rules](https://learn.microsoft.com/en-us/dotnet/standard/base-types/grouping-constructs-in-regular-expressions).

Conditional assertions and expression tests are adapted to the backend's
conditional syntax. Expression tests remain zero-width; an unknown name in a
condition becomes an expression test, while an undefined numeric group is an
error. These distinctions follow the
[.NET conditional-alternation contract](https://learn.microsoft.com/en-us/dotnet/standard/base-types/alternation-constructs-in-regular-expressions).
Unsupported capture-history constructs return pattern errors with XSHD line
and UTF-16 column context rather than silently binding to a different capture.

This policy applies to expressions compiled from XSHD. Hosts constructing a
`HighlightingRule` or `HighlightingSpan` with their own `fancy_regex::Regex`
continue to choose that regex's compilation options themselves.

## Matching between span boundaries

The engine first locates the next span start or current span end. It then
evaluates non-span rules in the interval before that boundary, following
`Regex.Match(input, beginning, length)` in upstream `HighlightNonSpans`.

Matching now uses the bounded text slice itself. A match that requires a suffix
beyond the boundary fails; it is not accepted and then shortened. Both region
edges affect anchors and lookarounds. Byte positions returned by the backend
are rebased to line positions and then converted to document UTF-16 offsets.
For example, a rule `a.*z` does not highlight the `a` in `a[x]z` when `[x]` is a
span, because the rule's entire required match cannot fit before the span.

Each rule's first result is cached for that region. A failed match remains a
failure until the next region. A successful future match remains cached until
another rule advances beyond its start. This is observable behavior: a failed
`^b` must not start matching after a preceding rule consumes `a`, and a future
`(?<=a)b` match must keep its original lookbehind context. Equal-position rule
matches retain source order, including compiled imports.

A zero-length rule match is an invalid definition, including a zero-length
match at the end of a nonempty region. The previous implementation silently
advanced a character and could discard highlighting decisions. An initially
empty region does not evaluate rules, matching upstream's early return.
Zero-width span transitions still use the existing cycle and operation guards;
they are not categorically forbidden.

## Span-only state updates and lazy formatting

`HighlightingEngine::scan_line` shares the span transition loop with full
highlighting, but skips ordinary rules, color overlays and UTF-16 section-offset
construction. It retains nested/multiline span transitions and their existing
cycle and operation guards.

`DocumentHighlighter::update_highlighting_state` uses this scanner. A request
to highlight a later line also scans uncached predecessors without evaluating
their ordinary rules, as upstream `HighlightLine` does through `HighlightUpTo`.
Consequently, a zero-width rule on an earlier line cannot prevent highlighting
a later line; requesting the malformed line itself still returns the rule
error.

Cache entries distinguish a computed end state from optional colored sections.
A state-only entry is upgraded when that line's formatting is requested; it is
never mistaken for a highlighted line with no colors. Re-scanning an edited
line invalidates its old formatting even if its ending span state is unchanged.
State convergence, stable live-line identities, callbacks and document/cache
generation guards remain in use. A failed formatting request leaves successfully
computed span states usable.

## Runtime style and schema findings

The style audit found the runtime fields already present: foreground,
background, font family, exact numeric weight, normal/italic/oblique slant,
font size, underline and strikethrough. Explicit weight/slant take precedence
over compatibility bold/italic flags. Overlay colors replace only specified
properties; clearing one decoration does not clear the other. Existing
programmatic-tree and runtime-color regressions cover these contracts. This
checkpoint changes no rendering API or style representation.

`HighlightingLoader.GetValidatingReader` has its schema assignment and
`ValidationType.Schema` code commented out. `V2Loader` likewise has its schema
set construction commented out and does not use `skipValidation` to install a
validator. Consequently, the presence of `ModeV2.xsd` in the upstream tree does
not mean that the pinned loader enforces it.

The Rust loader continues namespace-aware parsing and its direct
structure/value checks. It does not reject unknown attributes by consulting
the XSD. Fractional font sizes are accepted, although the archived schema
describes that attribute as an integer. The regression for these inputs records
active loader behavior; a new strict-schema mode is not introduced or counted
as required upstream behavior. DTD/external-resource handling and the archived
v1 compatibility extension remain as documented in the broader service report.

## Validation

Thirteen focused regressions were added across the regex normalizer, the XSHD
compiler pipeline and the highlighting engine. They cover default capture
policy, named/numbered references, scoped option restoration, conditionals,
escapes/classes/comments, unsupported capture diagnostics, preserved editable
patterns, active loader validation, bounded suffixes, both lookaround edges,
UTF-16 offsets, cached failures/future matches and zero-length rule errors.

The initial highlighting filter passed **90 tests**, including
compilation/execution and syntax-tree round trips of all **21 bundled
definitions**, and the unfiltered Core command passed **318 tests**.
The span-only follow-up adds three focused tests for scanner state, skipped
malformed rules, lazy formatting and stale-section invalidation. It also checks
the recursive-span guard through the scanner. The follow-up passed **93
highlighting tests** and **321 unfiltered Core tests**. The command used for the
full Core regression gate is:

```sh
cargo test -p ferroui-edit-core --lib --offline
```

These are executable Rust regressions checked against audited source
contracts. They are not an exhaustive differential run against the .NET regex
engine or a native rendering test.

## Remaining differences

- The backend consumes Unicode scalar text; .NET regexes operate on UTF-16 code
  units. Supplementary-character matching, isolated surrogates, Unicode
  character classes and culture-invariant case behavior are not claimed to be
  equivalent for every pattern.
- Balancing groups, duplicate capture names and explicitly numbered capture
  declarations are rejected. .NET capture histories and repeated-capture
  collections have no replacement public model here. Legacy numeric escapes
  that fall back to octal are not translated.
- The public backend regex's capture-index inspection follows backend encounter
  order. Matching backreferences and conditional capture tests are remapped;
  the backend's `Captures` collection is not replaced with a .NET group model.
- Complete .NET regex syntax, anchor edge cases, all option combinations,
  backend-specific extensions and adversarial-pattern performance still need
  wider differential coverage. Backtracking, compilation and line-operation
  budgets deliberately produce recoverable Rust errors.

The fixed capture defaults and bounded-region behavior are specific parity
improvements. They do not close the remaining regex-engine or native visual
validation work.
