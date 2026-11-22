# TextMateSharp grammar regression fixtures

The six grammar files are byte-for-byte copies of the inputs in
`src/TextMateSharp.Tests/Resources` at TextMateSharp revision
`e0db45be30278c0f3f80302c31f498c8f8f1c4f2`. The token ranges and scopes in
`expected.json` were captured from that revision using the registry in
`src/TextMateSharp.Tests/Internal/Grammars/GrammarTests.cs`. The TextMateSharp
MIT notice is in [the crate notice](../../../NOTICE.md).

The grammar files identify these component revisions. Their license texts are
retained under `licenses/`.

| Grammar | Component | Revision | License |
| --- | --- | --- | --- |
| Batch File | [mmims/language-batchfile](https://github.com/mmims/language-batchfile) | `6154ae25a24e01ac9329e7bcf958e093cd8733a9` | [MIT](licenses/batchfile.md) |
| C# | [dotnet/csharp-tmLanguage](https://github.com/dotnet/csharp-tmLanguage) | `572697a2c2267430010b3534281f73337144e94f` | [MIT](licenses/csharp.md) |
| CSS | [atom/language-css](https://github.com/atom/language-css) | `339e9600ce4e024fa97a711d6330e3b5344fcbdc` | [MIT](licenses/css.md) |
| TypeScript | [Microsoft/TypeScript-TmLanguage](https://github.com/Microsoft/TypeScript-TmLanguage) | `fea3b25eb632f689a02057a1715faab29343dda2` | [MIT](licenses/typescript.md) |

The two Angular injection fixtures are covered by the TextMateSharp notice.
