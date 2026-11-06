# FerroBoard component

Keep keyboard implementation, resources, tests and notices inside this directory.
Use Rust-native types and ownership; no CLR/BCL runtime emulation. Keep Core free of controls.
Keep the C# reference unchanged. Its .NET code is not built by ordinary Rust tests.
Use actual selected FerroUI APIs at the control boundary. No private SDK vendoring.
Format only the ferroboard-core, ferroboard-core-ferro, ferroboard-controls and ferroboard-sample packages, and the separate native consumer. Never mutate the parent SDK with cargo fmt --all.
Native platform certification is separate from Headless tests. A-001 remains deferred.
