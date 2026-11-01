package org.ferroui.media.video;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

/** Bounded encoded-container metadata inspection, never decoded preview pixels.
 * Reads the bytes already consumed by Media3, including ranged network reads.
 * Media3 1.8 supplies rotation but intentionally discards track reflections. */
public final class Mp4Geometry {
    public static final int MAX_METADATA=8*1024*1024;
    public static final class Info {
        public final int rotation,width,height,x,y,cropWidth,cropHeight;
        public final boolean mirror;
        Info(int rotation,boolean mirror,int width,int height,int x,int y,int cropWidth,int cropHeight){
            this.rotation=rotation;this.mirror=mirror;this.width=width;this.height=height;
            this.x=x;this.y=y;this.cropWidth=cropWidth;this.cropHeight=cropHeight;
        }
    }
    private final byte[] header=new byte[16];
    private int headerCount,headerLength=8,received;
    private long nextHeader,moovStart;
    private byte[] moov;
    private boolean iso,disabled,parsed,closed;
    private String error;
    private Map<String,Info> tracks=new HashMap<>();
    private static int type(String value){return (value.charAt(0)<<24)|(value.charAt(1)<<16)|(value.charAt(2)<<8)|value.charAt(3);}
    private static int integer(byte[] b,int at){return ((b[at]&255)<<24)|((b[at+1]&255)<<16)|((b[at+2]&255)<<8)|(b[at+3]&255);}
    private static long unsigned(byte[] b,int at){return integer(b,at)&0xffffffffL;}
    private static long wide(byte[] b,int at){long high=unsigned(b,at);if(high>0x7fffffffL)throw invalid();return (high<<32)|unsigned(b,at+4);}
    private static IllegalArgumentException invalid(){return new IllegalArgumentException("Unsupported or malformed MP4 geometry");}
    public synchronized void close(){closed=true;moov=null;tracks.clear();}
    public synchronized int retainedMetadataBytes(){return moov==null?0:moov.length;}
    public synchronized void accept(long position,byte[] data,int offset,int length){
        if(closed || disabled || parsed || error!=null || length<=0)return;
        if(position<0 || offset<0 || length>data.length-offset || position>Long.MAX_VALUE-length){error=invalid().getMessage();return;}
        long end=position+length;
        try {
            // Top-level headers only: mdat is skipped by absolute size, never
            // buffered or scanned for accidental box-name byte patterns.
            for(int step=0;step<4096;step++){
                long wanted=nextHeader+headerCount;
                if(wanted<nextHeader || wanted<position || wanted>=end)break;
                int count=(int)Math.min(headerLength-headerCount,end-wanted);
                System.arraycopy(data,offset+(int)(wanted-position),header,headerCount,count);headerCount+=count;
                if(headerCount<headerLength)break;
                long size=unsigned(header,0);int kind=integer(header,4);
                if(size==1 && headerLength==8){headerLength=16;continue;}
                if(size==1)size=wide(header,8);
                if(kind==type("ftyp") || kind==type("moov"))iso=true;
                if(!iso && nextHeader==0 && kind!=type("wide") && kind!=type("free") && kind!=type("skip") && kind!=type("mdat")){
                    disabled=true;break; // Another container remains Media3-owned.
                }
                if(size==0){disabled=!iso;nextHeader=Long.MAX_VALUE;break;}
                if(size<headerLength || size>Long.MAX_VALUE-nextHeader)throw invalid();
                if(kind==type("moov")){
                    if(size>MAX_METADATA || moov!=null)throw invalid();
                    moov=new byte[(int)size];moovStart=nextHeader;received=headerLength;
                    System.arraycopy(header,0,moov,0,headerLength);
                }
                nextHeader+=size;headerCount=0;headerLength=8;
            }
            if(moov!=null){
                long wanted=moovStart+received;
                if(position<=wanted && end>wanted){
                    int count=(int)Math.min(moov.length-received,end-wanted);
                    System.arraycopy(data,offset+(int)(wanted-position),moov,received,count);received+=count;
                }
                if(received==moov.length){tracks=parse(moov);moov=null;parsed=true;}
            }
        }catch(IllegalArgumentException failure){moov=null;error=failure.getMessage();}
    }
    public synchronized Info info(String id){
        if(closed)throw new IllegalArgumentException("MP4 geometry source is closed");
        if(error!=null)throw new IllegalArgumentException(error);
        if(!iso)return null;
        if(!parsed)throw new IllegalArgumentException("MP4 geometry metadata is incomplete");
        Info info=tracks.get(id);
        if(info==null && tracks.size()==1)info=tracks.values().iterator().next();
        if(info==null)throw new IllegalArgumentException("MP4 video track geometry is ambiguous");
        return info;
    }
    private static final class Box {
        final int start,payload,end,kind;
        Box(int start,int payload,int end,int kind){this.start=start;this.payload=payload;this.end=end;this.kind=kind;}
    }
    private static List<Box> boxes(byte[] b,int start,int end,int[] budget){
        List<Box> result=new ArrayList<>();
        while(start<end){
            if(--budget[0]<0 || end-start<8)throw invalid();
            long size=unsigned(b,start);int header=8,kind=integer(b,start+4);
            if(size==1){if(end-start<16)throw invalid();size=wide(b,start+8);header=16;}
            if(size==0)size=end-start;
            if(size<header || size>end-start)throw invalid();
            result.add(new Box(start,start+header,start+(int)size,kind));start+=(int)size;
        }
        return result;
    }
    private static Box child(byte[] b,Box parent,String name,int[] budget){
        Box found=null;for(Box v:boxes(b,parent.payload,parent.end,budget))if(v.kind==type(name)){if(found!=null)throw invalid();found=v;}
        if(found==null)throw invalid();return found;
    }
    private static int integral(double value){
        if(!Double.isFinite(value) || value<0 || value>8192 || Math.abs(value-Math.rint(value))>1e-6)throw invalid();return (int)Math.rint(value);
    }
    private static double ratio(byte[] b,int at,boolean signed){long denominator=unsigned(b,at+4);if(denominator==0)throw invalid();return (signed?integer(b,at):unsigned(b,at))/(double)denominator;}
    private static Map<String,Info> parse(byte[] b){
        int[] budget={4096};List<Box> roots=boxes(b,0,b.length,budget);
        if(roots.size()!=1 || roots.get(0).kind!=type("moov"))throw invalid();
        Map<String,Info> result=new HashMap<>();
        for(Box track:boxes(b,roots.get(0).payload,b.length,budget)){
            if(track.kind!=type("trak"))continue;
            Box media=child(b,track,"mdia",budget),handler=child(b,media,"hdlr",budget);
            if(handler.end-handler.payload<12)throw invalid();
            if(integer(b,handler.payload+8)!=type("vide"))continue;
            if(result.size()>=16)throw invalid();
            Box tkhd=child(b,track,"tkhd",budget);if(tkhd.end-tkhd.payload<4)throw invalid();int version=b[tkhd.payload]&255;
            int matrix=tkhd.payload+(version==0?40:version==1?52:-1);
            int idAt=tkhd.payload+(version==0?12:20);
            if(version>1 || matrix<tkhd.payload || tkhd.end-matrix<44)throw invalid();
            int a=integer(b,matrix),c=integer(b,matrix+12),bb=integer(b,matrix+4),d=integer(b,matrix+16);
            if(integer(b,matrix+8)!=0 || integer(b,matrix+20)!=0 || integer(b,matrix+32)!=0x40000000)throw invalid();
            int[][] matrices={{65536,0,0,65536},{0,65536,-65536,0},{-65536,0,0,-65536},{0,-65536,65536,0},
                {-65536,0,0,65536},{0,65536,65536,0},{65536,0,0,-65536},{0,-65536,-65536,0}};
            int orientation=-1;for(int i=0;i<8;i++)if(a==matrices[i][0] && bb==matrices[i][1] && c==matrices[i][2] && d==matrices[i][3])orientation=i;
            if(orientation<0)throw invalid();
            Box minf=child(b,media,"minf",budget),stbl=child(b,minf,"stbl",budget),stsd=child(b,stbl,"stsd",budget);
            if(stsd.end-stsd.payload<8 || unsigned(b,stsd.payload+4)!=1)throw invalid();
            List<Box> entries=boxes(b,stsd.payload+8,stsd.end,budget);if(entries.size()!=1)throw invalid();Box entry=entries.get(0);
            if(entry.end-entry.payload<78)throw invalid();
            int width=((b[entry.payload+24]&255)<<8)|(b[entry.payload+25]&255),height=((b[entry.payload+26]&255)<<8)|(b[entry.payload+27]&255);
            if(width==0 || height==0 || width>8192 || height>8192)throw invalid();
            int x=0,y=0,w=width,h=height;boolean clean=false,aspect=false;
            for(Box field:boxes(b,entry.payload+78,entry.end,budget)){
                if(field.kind==type("clap")){
                    if(clean || field.end-field.payload!=32)throw invalid();clean=true;
                    double cw=ratio(b,field.payload,false),ch=ratio(b,field.payload+8,false);
                    x=integral((width-cw)/2+ratio(b,field.payload+16,true));y=integral((height-ch)/2+ratio(b,field.payload+24,true));
                    w=integral(cw);h=integral(ch);if(w==0 || h==0 || x+w>width || y+h>height)throw invalid();
                }else if(field.kind==type("pasp")){
                    if(aspect || field.end-field.payload!=8 || unsigned(b,field.payload)==0 || unsigned(b,field.payload)!=unsigned(b,field.payload+4))throw invalid();aspect=true;
                }
            }
            String id=Long.toString(unsigned(b,idAt));if(result.containsKey(id))throw invalid();
            result.put(id,new Info((orientation%4)*90,orientation>=4,width,height,x,y,w,h));
        }
        return result;
    }
}
