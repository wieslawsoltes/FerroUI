import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Arrays;
import java.util.Random;
import org.ferroui.media.video.Mp4Geometry;
public final class Mp4GeometryContract {
    private static Mp4Geometry read(byte[] bytes,int step){
        Mp4Geometry parser=new Mp4Geometry();for(int i=0;i<bytes.length;i+=step)parser.accept(i,bytes,i,Math.min(step,bytes.length-i));return parser;
    }
    private static void fails(Mp4Geometry parser){try{parser.info("1");throw new AssertionError("Invalid MP4 admitted");}catch(IllegalArgumentException expected){}}
    private static void check(Mp4Geometry.Info info,int rotation,boolean mirror,int x,int y,int w,int h){
        if(info==null || info.rotation!=rotation || info.mirror!=mirror || info.width!=160 || info.height!=90 || info.x!=x || info.y!=y || info.cropWidth!=w || info.cropHeight!=h)throw new AssertionError("Fixture geometry differs");
    }
    private static int integer(byte[] bytes,int at){return ((bytes[at]&255)<<24)|((bytes[at+1]&255)<<16)|((bytes[at+2]&255)<<8)|(bytes[at+3]&255);}
    public static void main(String[] args)throws Exception{
        Path root=Path.of(args[0]);
        for(int step:new int[]{1,3,8,17,4096}){
            check(read(Files.readAllBytes(root.resolve("avc-aac.mp4")),step).info("1"),0,false,0,0,160,90);
            check(read(Files.readAllBytes(root.resolve("avc-aac-rotate90-mirror.mp4")),step).info("1"),90,true,0,0,160,90);
            check(read(Files.readAllBytes(root.resolve("avc-aac-geometry-crop.mp4")),step).info("1"),90,true,10,6,144,80);
        }
        byte[] source=Files.readAllBytes(root.resolve("avc-aac-geometry-crop.mp4"));int moov=0;
        while(integer(source,moov+4)!=0x6d6f6f76){int length=integer(source,moov);moov+=length==1?(int)java.nio.ByteBuffer.wrap(source,moov+8,8).getLong():length;}
        int size=integer(source,moov);byte[] extended=new byte[size+24];
        java.nio.ByteBuffer out=java.nio.ByteBuffer.wrap(extended);out.putInt(16).putInt(0x66747970).putLong(0);
        out.putInt(1).putInt(0x6d6f6f76).putLong(size+8);out.put(source,moov+8,size-8);
        check(read(extended,3).info("1"),90,true,10,6,144,80);
        // A large skipped mdat must not allocate/copy encoded media payload.
        Mp4Geometry sparse=new Mp4Geometry();byte[] prefix=java.nio.ByteBuffer.allocate(24).putInt(16).putInt(0x66747970).putLong(0).putInt(1_000_000).putInt(0x6d646174).array();
        sparse.accept(0,prefix,0,prefix.length);sparse.accept(1_000_016,source,moov,size);
        check(sparse.info("1"),90,true,10,6,144,80);
        if(sparse.retainedMetadataBytes()!=0)throw new AssertionError("Parsed bytes retained");
        Mp4Geometry canceled=new Mp4Geometry();canceled.accept(0,source,0,moov+16);if(canceled.retainedMetadataBytes()==0)throw new AssertionError("Partial metadata missing");canceled.close();
        if(canceled.retainedMetadataBytes()!=0)throw new AssertionError("Canceled metadata retained");fails(canceled);
        byte[] oversized=java.nio.ByteBuffer.allocate(24).putInt(16).putInt(0x66747970).putLong(0).putInt(Mp4Geometry.MAX_METADATA+1).putInt(0x6d6f6f76).array();
        Mp4Geometry rejected=read(oversized,8);fails(rejected);if(rejected.retainedMetadataBytes()!=0)throw new AssertionError("Oversized metadata allocated");
        byte[] broken=source.clone();for(int i=0;i<broken.length-4;i++)if(integer(broken,i)==0x746b6864){Arrays.fill(broken,i+44,i+48,(byte)0x7f);break;}fails(read(broken,7));
        // Mutated bounded metadata must reject or return valid bounded geometry,
        // never escape with indexing errors or unbounded allocations.
        Random random=new Random(1);for(int i=0;i<1000;i++){
            byte[] mutation=extended.clone();int at=16+random.nextInt(mutation.length-16);mutation[at]^=(byte)(1+random.nextInt(255));
            Mp4Geometry parser=read(mutation,31);try{Mp4Geometry.Info info=parser.info("1");if(info!=null && (info.width>8192 || info.height>8192))throw new AssertionError();}catch(IllegalArgumentException expected){}
        }
        System.out.println("PASS fragmented/ranged MP4 matrix, clean aperture, bounded malformed input and cancellation");
    }
}
