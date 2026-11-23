package org.ferroui.media.camera;

import androidx.camera.core.DynamicRange;

/** The GPU camera pipeline supports SDR input. Request it explicitly and verify
 * the resolved producer contract before providing a Surface. This is independent
 * of per-buffer color tags, which continue to take priority in SurfaceFrames.
 * https://developer.android.com/reference/androidx/camera/core/Preview.Builder#setDynamicRange(androidx.camera.core.DynamicRange)
 */
final class CameraPreviewProfile {
    private CameraPreviewProfile() {}
    static boolean supports(DynamicRange resolved) {
        return DynamicRange.SDR.equals(resolved);
    }
}
