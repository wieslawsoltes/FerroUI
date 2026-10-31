import java.util.Arrays;
import org.ferroui.media.FrameOrientation;
public final class FrameOrientationContract {
    private static float[] sample(float a,float b,float c,float d){float[] matrix=new float[16];matrix[0]=a;matrix[4]=b;matrix[1]=c;matrix[5]=d;matrix[15]=1;return matrix;}
    private static void check(float[] matrix,int desired,boolean mirror,int rotation,int remainingMirror){
        int[] result=FrameOrientation.remaining(matrix,desired,mirror);
        if(!Arrays.equals(result,new int[]{rotation,remainingMirror}))throw new AssertionError(Arrays.toString(result));
    }
    public static void main(String[] args){
        check(sample(1,0,0,-1),0,false,0,0);
        check(sample(1,0,0,-1),90,true,90,1);
        // HAL has already rotated raw pixels by 90 degrees.
        check(sample(0,-1,-1,0),90,false,0,0);
        // HAL has rotated and mirrored: mirroring must not be applied twice.
        check(sample(0,-1,1,0),90,true,0,0);
        // Same camera in a display rotated 90 degrees.
        check(sample(0,-1,-1,0),0,false,270,0);
        // Positive crop scaling changes no orientation.
        check(sample(0,-.9f,-.8f,0),90,false,0,0);
        if(FrameOrientation.remaining(sample(1,.3f,0,-1),0,false)!=null)throw new AssertionError("Shear was admitted as orientation");
        System.out.println("PASS camera HAL orientation, mirroring, display rotation, crop and incompatible shear");
    }
}
