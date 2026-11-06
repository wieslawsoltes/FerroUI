package org.ferroui.media.qr;

import android.graphics.Point;
import android.os.SystemClock;
import androidx.camera.core.ImageProxy;
import com.google.mlkit.vision.barcode.BarcodeScanner;
import com.google.mlkit.vision.barcode.BarcodeScannerOptions;
import com.google.mlkit.vision.barcode.BarcodeScanning;
import com.google.mlkit.vision.barcode.common.Barcode;
import com.google.mlkit.vision.common.InputImage;
import org.ferroui.media.camera.CameraController;
import org.ferroui.media.camera.FrameAnalyzer;
import org.json.JSONArray;
import org.json.JSONObject;
import java.util.concurrent.atomic.AtomicBoolean;

/** Bundled offline QR-only engine. The ImageProxy remains borrowed until the
 * actual native recognition task completes, including after cancellation. */
public final class QrAnalyzer implements FrameAnalyzer {
    private final BarcodeScanner scanner=BarcodeScanning.getClient(new BarcodeScannerOptions.Builder()
        .setBarcodeFormats(Barcode.FORMAT_QR_CODE).build());
    private final AtomicBoolean busy=new AtomicBoolean();
    private final AtomicBoolean released=new AtomicBoolean();
    private static final AtomicBoolean NATIVE_BUSY=new AtomicBoolean();
    private final long intervalMs,maxAgeMs;
    private final int maxWidth,maxHeight;
    private volatile boolean closed;
    private long lastStart=-1,resultAt,resultEpoch;
    private java.lang.ref.WeakReference<CameraController> resultController=new java.lang.ref.WeakReference<>(null);
    private String latest="",error="";
    private long analyzed,dropped;
    private static final java.util.concurrent.Executor DIRECT=Runnable::run;
    public QrAnalyzer(int width,int height,long intervalMs,long maxAgeMs) {
        maxWidth=width;maxHeight=height;this.intervalMs=Math.max(100,intervalMs);this.maxAgeMs=Math.min(250,maxAgeMs);
    }
    @Override public void analyze(ImageProxy image,CameraController controller) {
        long now=SystemClock.elapsedRealtime();
        long epoch=controller.geometryEpoch();
        int[] frameGeometry=controller.analysisFrameGeometry(epoch);
        if(frameGeometry==null){synchronized(this){dropped++;}image.close();return;}
        synchronized(this) {
            if(closed || !image.getCropRect().equals(new android.graphics.Rect(0,0,image.getWidth(),image.getHeight())) || image.getWidth()>maxWidth || image.getHeight()>maxHeight ||
                (lastStart>=0 && now-lastStart<intervalMs) || !NATIVE_BUSY.compareAndSet(false,true)) {
                dropped++;image.close();return;
            }
            busy.set(true);lastStart=now;analyzed++;
        }
        try {
            if(image.getImage()==null) {finish(image);return;}
            // Rotation 0 preserves the raw analysis-buffer coordinate system.
            // CameraController maps sensor -> preview -> GPU output separately.
            scanner.process(InputImage.fromMediaImage(image.getImage(),0))
                .addOnSuccessListener(DIRECT,barcodes->{
                    try {
                        if(closed || controller.closed() || controller.geometryEpoch()!=epoch ||
                            SystemClock.elapsedRealtime()-now>maxAgeMs) return;
                        JSONArray output=new JSONArray();
                        for(Barcode barcode:barcodes) {
                            if(output.length()>=64) break;
                            Point[] corners=barcode.getCornerPoints();
                            if(corners==null || corners.length!=4) continue;
                            float[] points=new float[8];for(int i=0;i<4;i++){points[2*i]=corners[i].x;points[2*i+1]=corners[i].y;}
                            float[] mapped=controller.mapAnalysisPoints(image,points,epoch);
                            if(mapped==null) continue;
                            JSONArray geometry=new JSONArray();boolean valid=true;
                            for(int i=0;i<4;i++) {
                                if(!Float.isFinite(mapped[i*2]) || !Float.isFinite(mapped[i*2+1])) {valid=false;break;}
                                geometry.put(new JSONArray().put(mapped[i*2]).put(mapped[i*2+1]));
                            }
                            if(!valid) continue;
                            JSONObject candidate=new JSONObject().put("corners",geometry);
                            String text=barcode.getRawValue();if(text!=null && text.length()<=65536) candidate.put("text",text);
                            byte[] bytes=barcode.getRawBytes();
                            if(bytes!=null && bytes.length<=65536) candidate.put("bytes",android.util.Base64.encodeToString(bytes,android.util.Base64.NO_WRAP));
                            output.put(candidate);
                        }
                        JSONArray snapshot=new JSONArray();for(int field:frameGeometry)snapshot.put(field);
                        String value=new JSONObject().put("timestamp",image.getImageInfo().getTimestamp()).put("frameGeometry",snapshot).put("symbols",output).toString();
                        if(value.length()<=1024*1024) synchronized(this) {
                            if(!closed && !controller.closed() && controller.geometryEpoch()==epoch) {
                                latest=value;resultAt=now;resultEpoch=epoch;resultController=new java.lang.ref.WeakReference<>(controller);
                            }
                        }
                    } catch(Exception failure) {fail(failure);}
                }).addOnFailureListener(DIRECT,this::fail).addOnCompleteListener(DIRECT,task->finish(image));
        } catch(RuntimeException failure) {fail(failure);finish(image);}
    }
    private synchronized void fail(Exception failure) {if(!closed) error=failure.toString();}
    private void finish(ImageProxy image) {image.close();busy.set(false);NATIVE_BUSY.set(false);if(closed) releaseScanner();}
    private void releaseScanner() {if(released.compareAndSet(false,true)) scanner.close();}
    public synchronized String take() {
        CameraController controller=resultController.get();
        if(closed || controller==null || controller.closed() || controller.geometryEpoch()!=resultEpoch || SystemClock.elapsedRealtime()-resultAt>maxAgeMs) {latest="";return "";}
        String value=latest;latest="";return value;
    }
    public synchronized String error() {return error;}
    public synchronized String statistics() {return analyzed+","+dropped;}
    @Override public synchronized void close() {if(closed)return;closed=true;latest="";if(!busy.get())releaseScanner();}
}
