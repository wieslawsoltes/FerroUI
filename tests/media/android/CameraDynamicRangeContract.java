package org.ferroui.media.camera;

import androidx.camera.core.DynamicRange;

/** Producer admission, not physical camera or GPU pixel evidence. Runs on the
 * host with the classes of androidx.camera:camera-core on the class path. */
public final class CameraDynamicRangeContract {
    public static void main(String[] args) {
        if(!CameraPreviewProfile.supports(DynamicRange.SDR))
            throw new AssertionError("Camera SDR profile rejected");
        if(!CameraPreviewProfile.supports(new DynamicRange(DynamicRange.ENCODING_SDR,DynamicRange.BIT_DEPTH_8_BIT)))
            throw new AssertionError("Resolved SDR value rejected");
        for(DynamicRange range:new DynamicRange[]{null,DynamicRange.UNSPECIFIED,
                DynamicRange.HLG_10_BIT,DynamicRange.HDR10_10_BIT,
                DynamicRange.HDR10_PLUS_10_BIT,DynamicRange.HDR_UNSPECIFIED_10_BIT,
                // Eight bits are not SDR: Dolby Vision has an 8-bit profile.
                DynamicRange.DOLBY_VISION_8_BIT,DynamicRange.DOLBY_VISION_10_BIT,
                new DynamicRange(DynamicRange.ENCODING_SDR,DynamicRange.BIT_DEPTH_10_BIT),
                new DynamicRange(DynamicRange.ENCODING_UNSPECIFIED,DynamicRange.BIT_DEPTH_8_BIT)}) {
            if(CameraPreviewProfile.supports(range))
                throw new AssertionError("Unsupported/unresolved camera range admitted: "+range);
        }
        System.out.println("PASS camera SDR producer admission and unresolved/HDR rejection");
    }
}
