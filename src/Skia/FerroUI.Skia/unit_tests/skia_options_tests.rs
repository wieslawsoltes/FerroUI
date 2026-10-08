//! Port of upstream's `SkiaOptionsTests.cs` of the Skia unit tests.

use crate::SkiaOptions;

// Stencil buffers let Skia choose multisample-based path rendering, which quantizes edge
// coverage and visibly degraded vector geometry anti-aliasing on GPU backends in 12.1.0.
// They are a performance opt-in, so anything other than an explicit `true` must avoid them.
#[test]
fn stencil_buffers_are_avoided_unless_explicitly_enabled() {
    for (use_stencil_buffers, expected) in [(None, true), (Some(false), true), (Some(true), false)] {
        assert_eq!(
            expected,
            SkiaOptions::should_avoid_stencil_buffers(use_stencil_buffers),
            "{use_stencil_buffers:?}"
        );
    }
}
