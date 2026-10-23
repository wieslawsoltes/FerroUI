// The checked-in compiled WinUI theme reference is stored in compiled_xaml/parts.txt
// and its ordered .rs.part files. Concatenation preserves the full emitted Rust source
// byte for byte while keeping each reference file below 4 MiB.
//
// This pointer is not compiled. Production uses $OUT_DIR/xaml/compiled_xaml.rs.
// Both the build-time and runtime emitters are compared with the complete reference
// in tests/compiled_xaml_tests.rs. Regenerate it explicitly with:
// cargo test -p ferroui-themes-winui --lib tests::compiled_xaml_tests::regenerate_compiled_xaml -- --ignored --exact
