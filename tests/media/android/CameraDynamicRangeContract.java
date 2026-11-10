package org.ferroui.media.camera;

import androidx.camera.core.DynamicRange;

/** Producer admission, not physical camera or GPU pixel evidence. */
public final class CameraDynamicRangeContract {
    public static void main(String[] args) {
        if(!CameraPreviewProfile.supports(DynamicRange.SDR))
            throw new AssertionError("Camera SDR profile rejected");
        if(!CameraPreviewProfile.supports(new DynamicRange(DynamicRange.ENCODING_SDR,8)))
            throw new AssertionError("Resolved SDR value rejected");
        for(DynamicRange range:new DynamicRange[]{null,DynamicRange.UNSPECIFIED,
                DynamicRange.HLG_10_BIT,DynamicRange.HDR10_10_BIT,
                DynamicRange.HDR10_PLUS_10_BIT,DynamicRange.HDR_UNSPECIFIED_10_BIT}) {
            if(CameraPreviewProfile.supports(range))
                throw new AssertionError("Unsupported/unresolved camera range admitted: "+range);
        }
        System.out.println("PASS camera SDR producer admission and unresolved/HDR rejection");
    }
}
