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
    /** Map logical raw-image crop into the already-oriented immutable output.
     * Sampling scales caused by decoder padding are excluded: width/height
     * describe Media3's logical decoded image, rather than padded GL storage. */
    public static int[] outputCrop(float[] matrix,int width,int height,int x,int y,int w,int h){
        if(width<=0 || height<=0 || x<0 || y<0 || w<=0 || h<=0 || (long)x+w>width || (long)y+h>height)return null;
        int[] sampling=remaining(matrix,0,false);if(sampling==null)return null;
        boolean swap=Math.abs(matrix[1])>Math.abs(matrix[0]) && Math.abs(matrix[4])>Math.abs(matrix[5]);
        int ow=swap?height:width,oh=swap?width:height;
        double minX=1,minY=1,maxX=0,maxY=0;
        for(int i=0;i<4;i++){
            double u=(x+((i&1)==0?0:w))/(double)width,v=(y+((i&2)==0?0:h))/(double)height;
            // Invert the normalized output-to-raw sampling orientation.
            if(sampling[1]!=0)u=1-u;
            double a=u,b=v;
            switch(sampling[0]){case 90:u=b;v=1-a;break;case 180:u=1-a;v=1-b;break;case 270:u=1-b;v=a;break;default:break;}
            minX=Math.min(minX,u);minY=Math.min(minY,v);maxX=Math.max(maxX,u);maxY=Math.max(maxY,v);
        }
        double[] edges={minX*ow,minY*oh,maxX*ow,maxY*oh};int[] rounded=new int[4];
        for(int i=0;i<4;i++){if(!Double.isFinite(edges[i]) || Math.abs(edges[i]-Math.rint(edges[i]))>1e-3)return null;rounded[i]=(int)Math.rint(edges[i]);}
        return new int[]{rounded[0],rounded[1],rounded[2]-rounded[0],rounded[3]-rounded[1]};
    }
    private static void rotate(double[] point,int clockwise,boolean mirror){
        double x=point[0],y=point[1];
        switch(clockwise){case 0:break;case 90:x=-point[1];y=point[0];break;case 180:x=-x;y=-y;break;case 270:x=point[1];y=-point[0];break;default:throw new IllegalArgumentException();}
        point[0]=mirror?-x:x;point[1]=y;
    }
}
