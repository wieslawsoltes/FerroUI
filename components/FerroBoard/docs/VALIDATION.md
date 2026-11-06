# Branch validation — 2026-11-03

Base: main `df8b92d9438a7f1f797f9f86c2c4c602e3a4bf75`.
Branch: `feature/ferroboard-component`.
[Exact commands, log hashes, source hashes and limits](branch-validation.json).

- 746 standalone workspace tests and85 independent compiled/runtime consumer tests pass.
- 4 ControlCatalog tests pass: compiled/runtime XAML, class construction and real SDK
 keyboard-device routing across message, email and protected-password fields.
- Strict component/consumer Clippy and owned formatting pass.
- Release Core/Core.Ferro/Controls checks and native desktop catalog linking pass.
- The unchanged351-file C# reference,206 embedded resources,66 vector paths/68 logical
 names and12 themes pass integrity/generation checks.204 frozen fixture files retain
 their exact bytes in Git; the component attributes prevent newline normalization.
- The SDK idioms inventory reports no new .NET-shaped infrastructure declarations.
 The pointer-focus builder is regenerated and compiler checked separately.

The actual macOS catalog opens Text → FerroBoard. Focusing Message opens the keyboard.
Invoking Q inserts Q; native injected a produces Qa; Backspace returns Q. Message
keeps focus throughout. The private validation window was closed and its process
verified absent. This is AX/native key injection evidence, not physical input or
complete platform/accessibility certification.

The SDK retains inherited warnings/formatting differences; it was not globally
reformatted. Five native prerequisites and the five-line generated builder follow-up
are separate framework commits. No keyboard engine logic is placed there.

Physical devices, broad Windows/Linux/mobile/browser native runs, full accessibility,
IME/privacy/fonts/performance and signed/notarized distribution remain unverified.
Clipboard monitoring uses a500ms poll and default reveal is native minimal reveal.
Nine historical local-time offsets retain documented native differences. A-001
remains deferred. This is ready for branch testing, not a full-port completion claim.
