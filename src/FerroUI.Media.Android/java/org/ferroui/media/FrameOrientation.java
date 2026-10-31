package org.ferroui.media;
/** Decomposes desired raw-image orientation composed with the transform already
 * applied while sampling SurfaceTexture into an immutable GL framebuffer. */
public final class FrameOrientation {
    private FrameOrientation(){}
    public static int[] remaining(float[] matrix,int clockwise,boolean mirror){
        if(matrix.length!=16 || clockwise<0 || clockwise>=360 || clockwise%90!=0)return null;
        // A logical top-left output point samples GL at (u,1-v).
        double[] x={matrix[0],matrix[1]},y={-matrix[4],-matrix[5]};
        rotate(x,clockwise,mirror);rotate(y,clockwise,mirror);
        double lx=Math.hypot(x[0],x[1]),ly=Math.hypot(y[0],y[1]);
        if(!Double.isFinite(lx) || !Double.isFinite(ly) || lx<1e-8 || ly<1e-8)return null;
        for(int i=0;i<2;i++){x[i]/=lx;y[i]/=ly;}
        for(int rotation=0;rotation<360;rotation+=90)for(int reflected=0;reflected<2;reflected++){
            double[] a={1,0},b={0,1};rotate(a,rotation,reflected!=0);rotate(b,rotation,reflected!=0);
            if(Math.abs(a[0]-x[0])+Math.abs(a[1]-x[1])+Math.abs(b[0]-y[0])+Math.abs(b[1]-y[1])<1e-3)return new int[]{rotation,reflected};
        }
        return null;
    }
    private static void rotate(double[] point,int clockwise,boolean mirror){
        double x=point[0],y=point[1];
        switch(clockwise){case 0:break;case 90:x=-point[1];y=point[0];break;case 180:x=-x;y=-y;break;case 270:x=point[1];y=-point[0];break;default:throw new IllegalArgumentException();}
        point[0]=mirror?-x:x;point[1]=y;
    }
}
