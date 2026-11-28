# XSHD runtime regex, style and loader audit

The runtime follows the active AvaloniaEdit source at
[`7277e36366d754219d7ede16b6613786b32caf4c`](https://github.com/AvaloniaUI/AvaloniaEdit/tree/7277e36366d754219d7ede16b6613786b32caf4c).
It corrects executable regex semantics and rule-region matching in
`FerroUI.Edit.Core`. It also distinguishes active loader behavior from the
commented schema-validation code. It does not claim that the selected Rust
regex backend implements the complete .NET regex language.

## Source and member map

All Rust paths in this table are relative to `src/FerroUI.Edit.Core/`.

| Upstream source/member | Rust implementation | Audited contract |
| --- | --- | --- |
| [`Highlighting/Xshd/XmlHighlightingDefinition.cs`, `CreateRegex`](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/Highlighting/Xshd/XmlHighlightingDefinition.cs) | `highlighting/xshd/compiler.rs::compile_regex`, `regex_syntax.rs::normalize` | Default explicit captures, inherited case setting, per-expression whitespace mode and source-positioned compilation failures |
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

## Compiled definition resolvers

The pinned loader accepts a host-provided compiled definition for an external
name. FerroUI exposes this contract through `HighlightingDefinitionResolver`.
The resolver can return a shared typed definition, resolve multiple aliases,
or report a lookup failure. Compilation caches each successful lookup by name.
An external color lookup reads the named color. Importing a rule set maps its
spans and programmatic import indices into the new definition. Named lookup
exposes the owning definition's names, so transitive helpers stay private.

The Rust fixture compares the effective foreground and background at every
offset. It also checks lookup of an external definition with the same name as
the input syntax and source lines for missing definitions, rule sets and colors.
A separate regression checks that invalid imported indices produce a reference
error. [Pinned resolver
contract](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/Highlighting/IHighlightingDefinitionReferenceResolver.cs)

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

Capture conditions recognize nonspacing marks, connector punctuation, and
join controls in names. A condition beginning with an ASCII digit requires
a complete numeric group reference. The pinned search strategy verified 24
match outcomes, six named replacements, and the expression fallback for an
unknown Unicode name. The pinned XSHD compiler applied the expected two
highlight ranges after a supplementary character and left the third line
uncolored. FerroUI also verified those ranges after serializing and loading the
XSHD. [Reference capture parser](https://github.com/dotnet/runtime/blob/v10.0.0/src/libraries/System.Text.RegularExpressions/src/System/Text/RegularExpressions/RegexParser.cs)

Unsupported capture-history constructs return pattern errors with XSHD line
and UTF-16 column context rather than silently binding to a different capture.

### Extended whitespace and repetition

The search and XSHD compilers share extended-mode token handling. Form feeds
act as ignored spacing outside classes when extended mode is active. Inside
a class or when escaped, they remain literal. Inline comments end at their
first closing parenthesis. Repetition counts require a minimum and adjacent
digits, commas and braces. In extended mode, `a{1, 3}` matches literal
`a{1,3}`, while `a{1,3}` matches one to three letters. The native compiler
also retains supported backend braced hexadecimal escapes.

Find and Replace checks a named backreference, UTF-16 offsets after an emoji,
substitution, and one-step undo. XSHD checks the highlighted range before and
after XML serialization. [Reference regex
parser](https://github.com/dotnet/runtime/blob/v10.0.0/src/libraries/System.Text.RegularExpressions/src/System/Text/RegularExpressions/RegexParser.cs)

### Numeric escapes and character classes

The shared normalizer resolves `\1` through `\9` as backreferences. A longer
bare number resolves to a declared capture when one exists. Otherwise, the
normalizer consumes up to three leading octal digits and leaves any remaining
digits in the pattern. An escape starting with zero and numeric escapes inside
character classes use octal directly. The compiler also accepts ASCII control
escapes such as `\cA` and treats `\b` as backspace inside a class.

XSHD and regex search support class subtraction, including nested subtraction
and subtraction from a negated base. The normalizer preserves the base class
as a group before applying the subtraction and treats literal brackets and
punctuation as class members. POSIX classes and bracketed Rust set operands
remain available. An unbracketed `&&` or `~~` is literal punctuation in an
editor character class. Nested classes are bounded to 128 levels.

Hyphens can also serve as range endpoints. For example, `[--a]` includes
characters from `-` through `a`, and `[!--b]` includes the range from `!`
through `-` plus `b`. A hyphen after a shorthand class or Unicode property
is a literal when followed by an ordinary member. Reversed ranges such as
`[a--b]` produce a compile diagnostic. The shared search compiler matched
the reference on 1,872 generated classes and 12 targeted cases covering
range endpoints, escaping, properties, and subtraction.

The pinned AvaloniaEdit search factory and XSHD highlighter also agreed with the
Rust tests on a numbered replacement and three styled ranges following a
supplementary character. See the reference syntax for [character
escapes](https://learn.microsoft.com/en-us/dotnet/standard/base-types/character-escapes-in-regular-expressions)
and [character
classes](https://learn.microsoft.com/en-us/dotnet/standard/base-types/character-classes-in-regular-expressions).

### Escaped supplementary characters

Search and XSHD compile a high-surrogate and low-surrogate `\uXXXX` escape
pair outside a character class as one Rust scalar. Ignored comments and
extended-mode whitespace may appear between the two escapes. Case-insensitive
patterns still compare these escaped supplementary characters exactly, as the
pinned reference does. XSHD serialization retains the source escapes.

A quantifier applied directly to the second escape targets the low code unit in
the reference and produces a diagnostic in FerroUI. Group the two escapes to
quantify the complete character.

### Named Unicode blocks

Find, Replace and XSHD lower named blocks to ranges in the Rust engine,
including negation and subtraction inside a character class. Both
`\p{IsGreek}` and `\p{IsGreekandCoptic}` match U+0370–U+03FF,
including U+037E GREEK QUESTION MARK. The separate Greek Extended block and
the native regex engine's Greek script property have their own ranges.
The source pattern remains intact in Find and in serialized XSHD.

The table records 108 reference spellings for 105 blocks from Unicode 17.0.0.
The Find engine agreed with the .NET 10 reference on 1,042 positive and
negative outcomes at block edges and interior code points, across 210
compiled patterns. The three surrogate-only blocks return a diagnostic
because the Rust backend matches Unicode scalar values. Case-insensitive
matching follows the backend's Unicode case folding.
[Reference block names](https://learn.microsoft.com/en-us/dotnet/standard/base-types/character-classes-in-regular-expressions#supported-named-blocks) ·
[Unicode block data](https://www.unicode.org/Public/17.0.0/ucd/Blocks.txt)

Hosts constructing a `HighlightingRule` or `HighlightingSpan` directly supply
a compiled `fancy_regex::Regex` with their chosen syntax and options.

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

Only the innermost span can match its end expression. A multiline child can
carry a line-limited parent past a line break. After the child closes, the
parent can match its own end expression again. The bundled JSON definition
uses this behavior when an expression inside a property value stays open.

An application creating a compiled `HighlightingSpan` can set
`span_color_includes_start` and `span_color_includes_end` independently.
`HighlightingSpan::new` defaults to applying the span style to the interior.
Each delimiter inherits its enclosing spans and can supply its own color.
The XSHD compiler enables both flags, so parsed spans layer their own style
underneath explicit delimiter colors. Importing an application-provided
compiled definition preserves its flags.

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
background, font family, exact numeric weight, normal/italic/oblique slant, font
size, underline and strikethrough. Explicit weight/slant take precedence over
compatibility bold/italic flags. Overlay colors replace only specified
properties; clearing one decoration does not clear the other. Existing
programmatic-tree and runtime-color regressions cover these contracts.

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

## Tests and regex contracts

```sh
cargo test -p ferroui-edit-core --lib
```

The [bundled corpus](../../src/FerroUI.Edit.Core/highlighting/xshd/bundled-reference.tsv)
records 134 lines across 20 definitions from the pinned reference's
`DocumentHighlighter`. It stores the source line, named UTF-16 paint ranges
and end-of-line color stack. The
`bundled_reference_corpus_matches_named_ranges_and_span_stacks` test requests
the final line first to exercise lazy state-only scans. The pinned reference reports `Could not find main RuleSet` for
its bundled TeX definition. FerroUI exercises TeX in the separate all-definitions
smoke test through its legacy XSHD loader.

### Unicode word and digit classes

The shared Find/Replace and XSHD compiler lowers `\w` to the reference letter,
nonspacing mark, decimal digit and connector punctuation categories. `\d`
uses decimal digits. Both sets are restricted to the Basic Multilingual Plane
to reflect the reference's UTF-16 code-unit classification. The complements
and class subtraction use the same sets. `\b` and `\B` classify U+200C and
U+200D as boundary word characters. The compiler retains the backspace escape
inside a class and passes through backend boundary specifiers such as
`\b{start}`. [Reference character classes](https://learn.microsoft.com/en-us/dotnet/standard/base-types/character-classes-in-regular-expressions)
[Reference boundary implementation](https://github.com/dotnet/runtime/blob/v10.0.0/src/libraries/System.Text.RegularExpressions/src/System/Text/RegularExpressions/RegexCharClass.cs)

The Rust tests compare UTF-16 match and paint ranges recorded by the pinned
AvaloniaEdit search and `DocumentHighlighter`. The fixtures include join
controls, vowel signs, nonspacing and enclosing marks, Roman numerals, Arabic
digits, connector punctuation and supplementary letters and digits. 

The backend's candidate scan skips irrelevant positions for whole-word literals
in large inputs. For Find/Replace, a native boundary variant also preserves fast
scanning when both word sets classify every scalar in the document the same
way. When the expanded boundary scan reaches its backtracking cap, and the
pattern requires `\b` or `\B` at the start of every match, Find/Replace resumes
at the eligible positions in the original document. A linear scan finds maximal
runs of reference word characters. The edges of the runs give `\b` positions,
and the other scalar positions give `\B` positions. Each candidate still goes
through the bounded regex engine on the full input. Lookaround, named captures,
replacement and the order of empty matches use the same match path. Searches
that depend on `\G` keep the original iteration. Regression tests cover absent
and late hits in mixed-script documents, one million Devanagari spacing marks
before a word, alternating word and nonword scalars before `\B`, and a sparse
keyword on a 720,007-unit XSHD line.

### End anchors and match iteration

`\Z` and `$` with multiline mode disabled accept the end of the input or the
position before its single final LF. `\z` accepts the end only. On CRLF the
optional position is between CR and LF. The compiler tracks scoped `m` switches
and keeps an LF line-end assertion so the native candidate scan can skip most
positions in long inputs. XSHD rules still match within the current region of
the logical line. A span beginning later on that line supplies the region end.
[Reference anchor behavior](https://learn.microsoft.com/en-us/dotnet/standard/base-types/anchors-in-regular-expressions)

Find/Replace reports empty matches at the end of a consuming match. After an
empty match it advances one Rust scalar and suppresses `\G` for the next scan,
as the new starting position is beyond the previous match. A consuming match
restores `\G` at its end. The iteration keeps the full input available when a
caller filters results to a range. Replace All applies adjacent consuming and
empty matches in one undo step.

## Remaining differences

- The backend consumes Unicode scalar text. The reference regex engine consumes
  UTF-16 code units. The compiler cannot match either half of a surrogate pair
  or assert a boundary between halves. Unicode category versions and case
  behavior can also differ on other patterns.
- Distinct explicitly numbered captures are supported, including sparse group
  numbers. The compiler rejects balancing groups, duplicate names and duplicate
  numbered declarations. .NET capture histories and repeated-capture
  collections have no replacement public model here.
- The public backend regex's capture-index inspection follows backend encounter
  order. Matching backreferences and conditional capture tests are remapped;
  the backend's `Captures` collection is not replaced with a .NET group model.
- Complete .NET regex syntax, anchor edge cases, all option combinations,
  backend-specific extensions and adversarial-pattern performance still need
  wider differential coverage. Large sparse searches with expanded boundaries
  can reach the matching budget when no single boundary kind is required at the
  match start. Backtracking, compilation and line-operation budgets produce
  recoverable Rust errors.

The fixed capture defaults and bounded-region behavior are specific parity
improvements. They do not close the remaining regex-engine or native visual
validation work.
