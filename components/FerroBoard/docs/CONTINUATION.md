# Continuation

Current integration work is on feature/ferroboard-component, from main
df8b92d9. The component is kept in this directory; do not spread its implementation
into framework controls or other ports. Preserve the Apache license and immutable
reference/resource data. No CLR/BCL emulation. A-001 remains deferred.

The framework prerequisites are five separate native commits already on main,
but absent from the base of the branch. Current build/test results belong
in VALIDATION.md and the branch validation JSON. Full platform parity remains unverified.

The branch now passes746 workspace/85 consumer/4 catalog tests, strict owned lint
and formatting, release libraries, native desktop linking and focused macOS
AX/key-input smoke. See branch-validation.json. Source/resource/fixture bytes
are pinned in Git. Broader device/platform certification is not complete.
