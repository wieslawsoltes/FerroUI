package org.ferroui.media.camera;

import android.Manifest;
import android.app.Activity;
import android.app.Application;
import android.app.Fragment;
import android.content.Context;
import android.content.pm.PackageManager;
import android.graphics.ImageFormat;
import android.graphics.SurfaceTexture;
import android.hardware.camera2.CameraCharacteristics;
import android.hardware.camera2.CameraManager;
import android.hardware.camera2.CameraCaptureSession;
import android.hardware.camera2.CaptureRequest;
import android.hardware.camera2.TotalCaptureResult;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.util.Size;
import android.view.Surface;
import androidx.camera.camera2.interop.Camera2CameraInfo;
import androidx.camera.camera2.interop.Camera2Interop;
import androidx.camera.core.Camera;
import androidx.camera.core.CameraSelector;
import androidx.camera.core.ImageCapture;
import androidx.camera.core.ImageAnalysis;
import androidx.camera.core.ImageCaptureException;
import androidx.camera.core.ImageProxy;
import androidx.camera.core.Preview;
import androidx.camera.core.resolutionselector.ResolutionSelector;
import androidx.camera.core.resolutionselector.ResolutionStrategy;
import androidx.camera.lifecycle.ProcessCameraProvider;
import androidx.lifecycle.Lifecycle;
import androidx.lifecycle.LifecycleOwner;
import androidx.lifecycle.LifecycleRegistry;
import com.google.common.util.concurrent.ListenableFuture;
import java.nio.ByteBuffer;
import java.util.ArrayList;
import java.util.concurrent.Executor;
import java.util.concurrent.atomic.AtomicBoolean;
import org.json.JSONArray;
import org.json.JSONObject;
import org.ferroui.media.SurfaceFrames;

/** CameraX owns acquisition, native JPEG capture and camera lifecycle. Preview
 * uses the common GPU SurfaceTexture; QR is a separate optional integration. */
@SuppressWarnings("deprecation")
public final class CameraController implements LifecycleOwner,Application.ActivityLifecycleCallbacks,android.content.ComponentCallbacks2 {
    private final Activity activity;
    private final SurfaceFrames frames;
    private final Executor main;
    private final LifecycleRegistry lifecycle=new LifecycleRegistry(this);
    private ProcessCameraProvider provider;
    private Preview preview;
    private ImageCapture still;
    private ImageAnalysis analysis;
    private volatile FrameAnalyzer analyzer;
    private final java.util.concurrent.ExecutorService analysisExecutor=java.util.concurrent.Executors.newSingleThreadExecutor();
    private final Object geometryLock=new Object();
    private android.graphics.Matrix previewSensorToBuffer;
    private long geometryEpoch;
    private CameraSelector selectedCamera;
    private Camera camera;
    private volatile boolean closed;
    private volatile boolean ready;
    private final Object clockLock=new Object();
    private final java.util.ArrayDeque<long[]> frameClock=new java.util.ArrayDeque<>();
    private CameraCaptureSession clockSession;
    private long clockStart;
    private int clockWidth,clockHeight;
    private int width,height,rotation;
    private boolean mirrored;
    private int desiredRotation;
    private boolean desiredMirror;
    private volatile double fps;
    private int actualFacing;
    private String deviceId="";
    private volatile String error="";
    private volatile int errorKind;
    private volatile boolean capturing;
    private java.lang.ref.WeakReference<CapturedPhoto> activePhoto=new java.lang.ref.WeakReference<>(null);
    private final java.util.concurrent.ExecutorService photoExecutor=java.util.concurrent.Executors.newSingleThreadExecutor();
    private static final AtomicBoolean PHOTO_BUSY=new AtomicBoolean();
    private final String selectedId;
    private final int facing,preferredWidth,preferredHeight;
    private final boolean exact;

    public CameraController(Activity activity,SurfaceFrames frames,String id,int facing,int width,int height,boolean exact,int colorProfile) {
        this.activity=activity;this.frames=frames;frames.setCameraColorMode(colorProfile);this.selectedId=id;this.facing=facing;
        preferredWidth=width;preferredHeight=height;this.exact=exact;
        Handler handler=new Handler(Looper.getMainLooper());main=command->handler.post(command);
        activity.getApplication().registerActivityLifecycleCallbacks(this);
        activity.getApplication().registerComponentCallbacks(this);
        lifecycle.handleLifecycleEvent(Lifecycle.Event.ON_CREATE);
        lifecycle.handleLifecycleEvent(Lifecycle.Event.ON_START);
        lifecycle.handleLifecycleEvent(Lifecycle.Event.ON_RESUME);
        if(activity.checkSelfPermission(Manifest.permission.CAMERA)==PackageManager.PERMISSION_GRANTED) bind();
        else PermissionRequest.request(activity,granted->{
            if(closed) return;
            if(granted) bind(); else fail(4,"Camera permission denied");
        });
    }
    @Override public Lifecycle getLifecycle() {return lifecycle;}
    private void fail(int kind,String message) {errorKind=kind;error=message;ready=false;}
    public String error() {return error;}
    public int errorKind() {return errorKind;}
    public boolean ready() {return ready && fps>0 && !closed;}
    public boolean closed() {return closed;}
    private void bind() {
        if(closed) return;
        try {
            ListenableFuture<ProcessCameraProvider> future=ProcessCameraProvider.getInstance(activity);
            future.addListener(()->{
                if(closed) return;
                try {
                    provider=future.get();
                    CameraSelector.Builder selector=new CameraSelector.Builder();
                    if(!selectedId.isEmpty()) selector.addCameraFilter(infos->{
                        ArrayList<androidx.camera.core.CameraInfo> selected=new ArrayList<>();
                        for(androidx.camera.core.CameraInfo info:infos)
                            if(Camera2CameraInfo.from(info).getCameraId().equals(selectedId)) selected.add(info);
                        return selected;
                    });
                    else if(facing==1 || facing==2) selector.requireLensFacing(facing==1 ? CameraSelector.LENS_FACING_FRONT : CameraSelector.LENS_FACING_BACK);
                    Preview.Builder builder=new Preview.Builder().setTargetRotation(displayRotation());
                    if(preferredWidth>0 && preferredHeight>0) builder.setResolutionSelector(new ResolutionSelector.Builder()
                        .setResolutionStrategy(new ResolutionStrategy(new Size(preferredWidth,preferredHeight),
                            ResolutionStrategy.FALLBACK_RULE_CLOSEST_HIGHER_THEN_LOWER)).build());
                    new Camera2Interop.Extender<>(builder).setSessionCaptureCallback(new CameraCaptureSession.CaptureCallback(){
                        private long first,last,count;
                        @Override public void onCaptureCompleted(CameraCaptureSession session,CaptureRequest request,TotalCaptureResult result) {
                            Long timestamp=result.get(TotalCaptureResult.SENSOR_TIMESTAMP);
                            if(timestamp==null) return;
                            recordFrame(session,timestamp);
                            Long interval=result.get(TotalCaptureResult.SENSOR_FRAME_DURATION);
                            if(interval!=null && interval>0) {fps=1e9/interval;return;}
                            if(first==0 || timestamp-last>1000000000L) {first=timestamp;count=0;}
                            if(timestamp>last) {last=timestamp;count++;}
                            if(last>first && count>1) fps=(count-1)*1e9/(last-first);
                        }
                    });
                    preview=builder.build();
                    still=new ImageCapture.Builder().setTargetRotation(displayRotation()).setCaptureMode(ImageCapture.CAPTURE_MODE_MINIMIZE_LATENCY).build();
                    preview.setSurfaceProvider(main,request->{
                        if(closed) {request.willNotProvideSurface();return;}
                        Size size=request.getResolution();synchronized(geometryLock){width=size.getWidth();height=size.getHeight();}
                        if(exact && preferredWidth>0 && (width!=preferredWidth || height!=preferredHeight)) {
                            request.willNotProvideSurface();fail(7,"Exact camera size unavailable");return;
                        }
                        frames.setSize(width,height);
                        request.setTransformationInfoListener(main,info->{
                            // SurfaceFrames applies getTransformMatrix, which
                            // already contains the HAL camera transform.
                            synchronized(geometryLock) {
                                desiredRotation=info.getRotationDegrees();
                                // A GL-processed surface already includes mirroring;
                                // direct camera surfaces encode it in their sampling matrix.
                                desiredMirror=info.hasCameraTransform() && info.isMirroring();
                                previewSensorToBuffer=new android.graphics.Matrix(info.getSensorToBufferTransform());geometryEpoch++;
                            }
                            if(!info.getCropRect().equals(new android.graphics.Rect(0,0,width,height)))
                                fail(7,"Non-full CameraX viewport crop is not admitted");
                        });
                        frames.retainSurface();
                        request.provideSurface(frames.surface(),main,result->frames.releaseSurface());
                        ready=true;
                    });
                    selectedCamera=selector.build();
                    bindUseCases();
                    deviceId=Camera2CameraInfo.from(camera.getCameraInfo()).getCameraId();
                    Integer lens=Camera2CameraInfo.from(camera.getCameraInfo()).getCameraCharacteristic(CameraCharacteristics.LENS_FACING);
                    actualFacing=lens==null ? 0 : lens==CameraCharacteristics.LENS_FACING_FRONT ? 1 : lens==CameraCharacteristics.LENS_FACING_BACK ? 2 : 3;
                    camera.getCameraInfo().getCameraState().observe(this,state->{
                        if(state.getError()!=null && !closed) fail(6,"CameraX error "+state.getError().getCode());
                    });
                } catch(Exception failure) {fail(8,failure.toString());}
            },main);
        } catch(RuntimeException failure) {fail(8,failure.toString());}
    }
    private void recordFrame(CameraCaptureSession session,long timestamp) {
        synchronized(clockLock) {
            if(closed || timestamp<=0)return;
            if(session!=clockSession) {
                // A closed earlier session may deliver its last callback after
                // rebinding. Sensor timestamps precede the new session start.
                if(timestamp<=clockStart)return;
                synchronized(geometryLock) {
                    if(width<=0 || height<=0)return;
                    clockWidth=width;clockHeight=height;
                }
                clockSession=session;clockStart=timestamp;frameClock.clear();
            }
            if(timestamp<clockStart)return;
            if(frameClock.size()==32)frameClock.removeFirst();
            frameClock.addLast(new long[]{timestamp,clockWidth,clockHeight});
        }
    }
    public String frameInfo(long timestamp) {
        synchronized(clockLock) {
            if(closed)return "";
            for(long[] frame:frameClock)if(frame[0]==timestamp){
                synchronized(geometryLock){
                    if(previewSensorToBuffer==null)return "";
                    int[] remaining=frames.remainingOrientation(desiredRotation,desiredMirror);
                    if(remaining==null){fail(7,"Non-orthogonal camera sampling transform is not admitted");return "";}
                    rotation=remaining[0];mirrored=remaining[1]!=0;
                    return frame[0]+","+frame[1]+","+frame[2]+",1,"+rotation+","+(mirrored?1:0);
                }
            }
            return "";
        }
    }
    private void bindUseCases() {
        ready=false;
        synchronized(geometryLock){previewSensorToBuffer=null;geometryEpoch++;}
        if(analysis!=null) {analysis.clearAnalyzer();provider.unbind(preview,still,analysis);analysis=null;}
        else provider.unbind(preview,still);
        if(analyzer!=null) {
            analysis=new ImageAnalysis.Builder().setBackpressureStrategy(ImageAnalysis.STRATEGY_KEEP_ONLY_LATEST)
                .setTargetRotation(Surface.ROTATION_0).setOutputImageRotationEnabled(false)
                .setResolutionSelector(new ResolutionSelector.Builder().setResolutionStrategy(new ResolutionStrategy(
                    new Size(1280,720),ResolutionStrategy.FALLBACK_RULE_CLOSEST_LOWER)).build()).build();
            analysis.setAnalyzer(analysisExecutor,image->{
                FrameAnalyzer current=analyzer;
                if(closed || current==null) image.close();
                else {try {current.analyze(image,this);}catch(RuntimeException failure){image.close();}}
            });
            camera=provider.bindToLifecycle(this,selectedCamera,preview,still,analysis);
        } else camera=provider.bindToLifecycle(this,selectedCamera,preview,still);
    }
    /** UI-thread opt-in; Camera remains independent of any recognition engine. */
    public String setAnalyzer(FrameAnalyzer value) {
        if(closed) return "Camera is closed";
        FrameAnalyzer previous=analyzer;analyzer=value;
        if(previous!=null && previous!=value) previous.close();
        try {if(provider!=null && preview!=null) bindUseCases();return "";}
        catch(RuntimeException failure) {
            analyzer=null;if(value!=null)value.close();
            // Optional analysis failure must restore camera preview/stills.
            try {if(provider!=null && preview!=null) bindUseCases();}
            catch(RuntimeException restore) {fail(8,restore.toString());}
            return failure.toString();
        }
    }
    public void removeAnalyzer(FrameAnalyzer expected) {
        if(analyzer==expected && !closed) setAnalyzer(null);
    }
    public long geometryEpoch() {synchronized(geometryLock){return (geometryEpoch<<32)^frames.geometryEpoch();}}
    public int[] analysisFrameGeometry(long expectedEpoch){
        synchronized(geometryLock){
            if(closed || previewSensorToBuffer==null || expectedEpoch!=geometryEpoch())return null;
            return frames.frameGeometry(width,height,desiredRotation,desiredMirror);
        }
    }
    public float[] mapAnalysisPoints(ImageProxy image,float[] points,long expectedEpoch) {
        android.graphics.Matrix rawToSensor=new android.graphics.Matrix();
        if(!image.getImageInfo().getSensorToBufferTransformMatrix().invert(rawToSensor)) return null;
        float[] mapped=points.clone();rawToSensor.mapPoints(mapped);
        synchronized(geometryLock) {
            if(previewSensorToBuffer==null || expectedEpoch!=geometryEpoch() || width<=0 || height<=0) return null;
            previewSensorToBuffer.mapPoints(mapped);
            for(int i=0;i<mapped.length;i+=2) {mapped[i]/=width;mapped[i+1]/=height;}
        }
        return frames.mapRawPoints(mapped);
    }
    public String settings() {
        try {
            JSONObject value=new JSONObject().put("id",deviceId).put("dataSpace",frames.dataSpace()).put("declaredUntaggedColor",frames.declaredUntaggedColor()).put("width",width).put("height",height)
                .put("fps",fps).put("rotation",rotation).put("mirrored",mirrored).put("facing",actualFacing).put("torch",camera!=null && camera.getCameraInfo().hasFlashUnit());
            if(camera!=null && camera.getCameraInfo().getZoomState().getValue()!=null) {
                value.put("minZoom",camera.getCameraInfo().getZoomState().getValue().getMinZoomRatio());
                value.put("maxZoom",camera.getCameraInfo().getZoomState().getValue().getMaxZoomRatio());
            }
            return value.toString();
        } catch(Exception failure) {fail(8,failure.toString());return "{}";}
    }
    public String control(int kind,float value) {
        if(closed || camera==null) return "Camera is not active";
        ListenableFuture<Void> pending;
        if(kind==0) {
            if(!camera.getCameraInfo().hasFlashUnit()) return "Torch unavailable";
            pending=camera.getCameraControl().enableTorch(value!=0);
        } else if(kind==1) {
            androidx.camera.core.ZoomState zoom=camera.getCameraInfo().getZoomState().getValue();
            if(zoom==null || value<zoom.getMinZoomRatio() || value>zoom.getMaxZoomRatio()) return "Zoom outside supported range";
            pending=camera.getCameraControl().setZoomRatio(value);
        } else return "Camera control unsupported";
        pending.addListener(()->{try {pending.get();}catch(Exception failure){if(!closed) fail(8,failure.toString());}},main);
        return "";
    }
    public CapturedPhoto capture() {
        CapturedPhoto photo=new CapturedPhoto();
        if(!ready() || still==null) {photo.fail("Camera is not ready");return photo;}
        if(!PHOTO_BUSY.compareAndSet(false,true)) {photo.fail("Still capture busy");return photo;}
        capturing=true;activePhoto=new java.lang.ref.WeakReference<>(photo);
        try {
            still.takePicture(photoExecutor,new ImageCapture.OnImageCapturedCallback(){
                @Override public void onCaptureSuccess(ImageProxy image) {
                    try {
                        if(closed || photo.abandoned()) return;
                        if(image.getFormat()!=ImageFormat.JPEG) {photo.fail("Native JPEG output unavailable");return;}
                        ByteBuffer bytes=image.getPlanes()[0].getBuffer();
                        if(bytes.remaining()>64*1024*1024) {photo.fail("Encoded photo exceeds limit");return;}
                        byte[] encoded=new byte[bytes.remaining()];bytes.get(encoded);
                        if(!closed) photo.complete(encoded,image.getWidth(),image.getHeight(),image.getImageInfo().getRotationDegrees(),image.getImageInfo().getTimestamp());
                    } finally {image.close();captureFinished();}
                }
                @Override public void onError(ImageCaptureException failure) {
                    photo.fail(failure.toString());captureFinished();
                }
            });
        } catch(RuntimeException failure) {photo.fail(failure.toString());captureFinished();}
        return photo;
    }
    private void captureFinished() {capturing=false;PHOTO_BUSY.set(false);if(closed) photoExecutor.shutdown();}
    public void close() {
        if(closed) return;
        closed=true;ready=false;synchronized(clockLock){frameClock.clear();clockSession=null;}CapturedPhoto photo=activePhoto.get();if(photo!=null) photo.abandon();
        if(!capturing) photoExecutor.shutdown();
        FrameAnalyzer old=analyzer;analyzer=null;if(old!=null) old.close();
        if(analysis!=null) analysis.clearAnalyzer();analysisExecutor.shutdown();
        lifecycle.handleLifecycleEvent(Lifecycle.Event.ON_DESTROY);
        if(provider!=null && preview!=null) {if(analysis!=null) provider.unbind(preview,still,analysis);else provider.unbind(preview,still);}
        activity.getApplication().unregisterActivityLifecycleCallbacks(this);
        activity.getApplication().unregisterComponentCallbacks(this);
    }
    private int displayRotation() {return activity.getWindowManager().getDefaultDisplay().getRotation();}
    @Override public void onConfigurationChanged(android.content.res.Configuration configuration) {
        if(closed) return;int target=displayRotation();if(preview!=null) preview.setTargetRotation(target);if(still!=null) still.setTargetRotation(target);
    }
    @Override public void onLowMemory() {}
    @Override public void onTrimMemory(int level) {}
    public static String devices(Context context) {
        JSONArray devices=new JSONArray();
        try {
            CameraManager manager=(CameraManager)context.getSystemService(Context.CAMERA_SERVICE);
            for(String id:manager.getCameraIdList()) {
                CameraCharacteristics info=manager.getCameraCharacteristics(id);
                Integer lens=info.get(CameraCharacteristics.LENS_FACING);
                int facing=lens==null ? 0 : lens==CameraCharacteristics.LENS_FACING_FRONT ? 1 : lens==CameraCharacteristics.LENS_FACING_BACK ? 2 : 3;
                JSONArray formats=new JSONArray();
                android.hardware.camera2.params.StreamConfigurationMap map=info.get(CameraCharacteristics.SCALER_STREAM_CONFIGURATION_MAP);
                if(map!=null && map.getOutputSizes(SurfaceTexture.class)!=null) for(Size size:map.getOutputSizes(SurfaceTexture.class)) {
                    long interval=map.getOutputMinFrameDuration(SurfaceTexture.class,size);
                    if(interval>0) formats.put(new JSONObject().put("width",size.getWidth()).put("height",size.getHeight()).put("fps",1e9/interval));
                }
                devices.put(new JSONObject().put("id",id).put("facing",facing).put("formats",formats));
            }
            return devices.toString();
        } catch(Exception failure) {return "{\"error\":\"Camera discovery unavailable\"}";}
    }
    public static int permission(Context context) {
        if(context.checkSelfPermission(Manifest.permission.CAMERA)==PackageManager.PERMISSION_GRANTED) return 1;
        return context.getSharedPreferences("ferroui.media.camera",Context.MODE_PRIVATE).getBoolean("requested",false) ? 2 : 0;
    }
    @Override public void onActivityResumed(Activity a){if(a==activity && !closed) {lifecycle.handleLifecycleEvent(Lifecycle.Event.ON_START);lifecycle.handleLifecycleEvent(Lifecycle.Event.ON_RESUME);}}
    @Override public void onActivityPaused(Activity a){if(a==activity && !closed) {lifecycle.handleLifecycleEvent(Lifecycle.Event.ON_PAUSE);lifecycle.handleLifecycleEvent(Lifecycle.Event.ON_STOP);}}
    @Override public void onActivityDestroyed(Activity a){if(a==activity) close();}
    @Override public void onActivityCreated(Activity a,Bundle b){}
    @Override public void onActivityStarted(Activity a){}
    @Override public void onActivityStopped(Activity a){}
    @Override public void onActivitySaveInstanceState(Activity a,Bundle b){}

    /** Explicit-start-only permission request. A late result after cancellation
     * is delivered to a closed controller and cannot open the camera. */
    public static final class PermissionRequest extends Fragment {
        private static final String TAG="ferroui.media.camera.permission";
        private java.util.function.Consumer<Boolean> result;
        static void request(Activity activity,java.util.function.Consumer<Boolean> result) {
            if(activity.getFragmentManager().findFragmentByTag(TAG)!=null) {result.accept(false);return;}
            PermissionRequest request=new PermissionRequest();request.result=result;
            activity.getSharedPreferences("ferroui.media.camera",Context.MODE_PRIVATE).edit().putBoolean("requested",true).apply();
            activity.getFragmentManager().beginTransaction().add(request,TAG).commitNow();
            request.requestPermissions(new String[]{Manifest.permission.CAMERA},4713);
        }
        @Override public void onRequestPermissionsResult(int code,String[] names,int[] grants) {
            super.onRequestPermissionsResult(code,names,grants);
            if(code!=4713) return;
            java.util.function.Consumer<Boolean> callback=result;result=null;
            if(callback!=null) callback.accept(grants.length>0 && grants[0]==PackageManager.PERMISSION_GRANTED);
            if(getFragmentManager()!=null) getFragmentManager().beginTransaction().remove(this).commitAllowingStateLoss();
        }
    }
}
