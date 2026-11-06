package org.ferroui.media;
import android.graphics.ColorSpace;
import android.hardware.DataSpace;
/** Native SDR RGB interpretation after the external sampler's YUV conversion.
 * A source tag is required; no untagged/wide-gamut/HDR image is called sRGB. */
final class CameraColor {
    final float[] matrix;
    final boolean transfer709;
    final boolean conversion;
    private CameraColor(float[] matrix,boolean transfer709,boolean conversion){this.matrix=matrix;this.transfer709=transfer709;this.conversion=conversion;}
    static CameraColor from(int dataSpace,int untagged){
        if(dataSpace==0 && (untagged==1 || untagged==2))return new CameraColor(new float[]{1,0,0,0,1,0,0,0,1},untagged==2,untagged==2);
        if(android.os.Build.VERSION.SDK_INT<33)throw new UnsupportedOperationException("Camera color metadata requires Android 33+ or an explicitly declared source profile");
        return Api33.from(dataSpace);
    }
    private static final class Api33 {
    static CameraColor from(int dataSpace){
        int standard=DataSpace.getStandard(dataSpace),transfer=DataSpace.getTransfer(dataSpace),range=DataSpace.getRange(dataSpace);
        if(range!=DataSpace.RANGE_FULL && range!=DataSpace.RANGE_LIMITED)throw new UnsupportedOperationException("Unsupported/unknown camera sample range: "+dataSpace);
        if(transfer!=DataSpace.TRANSFER_SRGB && transfer!=DataSpace.TRANSFER_SMPTE_170M)throw new UnsupportedOperationException("Unsupported/unknown camera transfer: "+dataSpace);
        float[] primaries;
        if(standard==DataSpace.STANDARD_BT709)primaries=null;
        else if(standard==DataSpace.STANDARD_BT601_525 || standard==DataSpace.STANDARD_BT601_525_UNADJUSTED)primaries=new float[]{.630f,.340f,.310f,.595f,.155f,.070f};
        else if(standard==DataSpace.STANDARD_BT601_625 || standard==DataSpace.STANDARD_BT601_625_UNADJUSTED)primaries=new float[]{.640f,.330f,.290f,.600f,.150f,.060f};
        else throw new UnsupportedOperationException("Unsupported/unknown camera primaries: "+dataSpace);
        float[] matrix={1,0,0,0,1,0,0,0,1};
        if(primaries!=null){
            ColorSpace.Rgb source=new ColorSpace.Rgb("Camera SDR",primaries,new float[]{.3127f,.3290f},1.0);
            float[] xyz=source.getTransform(),rgb=((ColorSpace.Rgb)ColorSpace.get(ColorSpace.Named.LINEAR_SRGB)).getInverseTransform();
            matrix=new float[9];for(int column=0;column<3;column++)for(int row=0;row<3;row++)for(int k=0;k<3;k++)matrix[column*3+row]+=rgb[k*3+row]*xyz[column*3+k];
        }
        return new CameraColor(matrix,transfer==DataSpace.TRANSFER_SMPTE_170M,primaries!=null || transfer!=DataSpace.TRANSFER_SRGB);
    }
    }
}
