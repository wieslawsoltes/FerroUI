# Integrated native source group

All 26 formerly staged Rust source bodies are now integrated into the normal
Cargo modules under `src/Dock/FerroDock.Ferro`. The duplicate staged files were removed.
The integration includes actual native controls, hosts, state/adorner/preview
helpers, factory services and peers; document/tool tabs are also compiled.

See [the current continuation record](../../../../docs/porting/dock/CONTINUATION.md) for validation,
remaining source scenarios, theme integration and runnable-sample work.
Compilation is not a claim of full source behavior or API parity.
