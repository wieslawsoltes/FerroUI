package org.ferroui.media.camera;
import androidx.camera.core.ImageProxy;
/** Optional native analysis; no QR dependency. A received image belongs to the
 * analyzer until it closes it, including asynchronous failure/cancellation. */
public interface FrameAnalyzer {
    void analyze(ImageProxy image, CameraController controller);
    void close();
}
