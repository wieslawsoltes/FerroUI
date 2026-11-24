#!/bin/sh
# markup-regenerate.sh: writes again every checked-in file the markup compiler generates, with the
# ignored tests that write them. Run it after a change of the compiler, on the commit that is
# landing, and commit its output on its own.
#
#   sh scripts/markup-regenerate.sh
#
# CARGO names the command to build with (default `cargo`). SKIP_SKIA=1 leaves out the packages
# whose tests build Skia, and says which they are.
set -eu
CARGO=${CARGO:-cargo}

# A package and the tests that write its files.
run() {
    package=$1
    shift
    "$CARGO" test --locked -p "$package" --lib -- --ignored --exact "$@"
}

run ferroui-themes-fluent tests::compiled_xaml_tests::regenerate_compiled_xaml
run ferroui-themes-macos tests::compiled_xaml_tests::regenerate_compiled_xaml
run ferroui-themes-winui tests::compiled_xaml_tests::regenerate_compiled_xaml
if [ "${SKIP_SKIA:-}" = 1 ]; then
    echo "skipped, as its tests build Skia: ferroui-themes-simple" >&2
else
    run ferroui-themes-simple tests::compiled_xaml_tests::regenerate_compiled_xaml
fi
run ferroui-markup-xaml-winui compiled_tests::regenerate_winui_binding_fixture
run ferroui-markup-xaml-tests \
    emitter::differential_tests::regenerate_emitter_output \
    emitter::event_handlers::regenerate_class_documents \
    markup_extensions::x_bind_extension_tests::regenerate_x_bind_fixture \
    markup_extensions::x_bind_extension_tests::regenerate_x_bind_class_fixture
