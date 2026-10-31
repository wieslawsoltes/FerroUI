package org.ferroui.media.video;

import android.content.Context;
import android.media.MediaFormat;
import android.net.Uri;
import android.os.Looper;
import android.os.Handler;
import android.os.HandlerThread;
import android.view.Surface;
import androidx.media3.common.AudioAttributes;
import androidx.media3.common.C;
import androidx.media3.common.Format;
import androidx.media3.common.MediaItem;
import androidx.media3.common.PlaybackException;
import androidx.media3.common.Player;
import androidx.media3.common.VideoSize;
import androidx.media3.exoplayer.ExoPlayer;
import androidx.media3.exoplayer.SeekParameters;
import androidx.media3.exoplayer.DefaultLoadControl;
import java.io.File;
import org.ferroui.media.SurfaceFrames;

/** Native MediaCodec/AudioTrack playback. A bounded frame-clock table associates
 * SurfaceTexture release timestamps with actual media presentation timestamps. */
public final class VideoPlayer {
    private static java.lang.ref.WeakReference<VideoPlayer> last=new java.lang.ref.WeakReference<>(null);
    public static String diagnostics() {VideoPlayer value=last.get();return value==null ? "no player" : value.clockDiagnostics();}
    private long lastSurfaceTimestamp,clockMisses,clockMatches;
    private String clockDiagnostics() {synchronized(clock){StringBuilder value=new StringBuilder("state="+snapshotState+" position="+snapshotPosition+" seek="+appliedRequest+"/"+completedRequest+" surface="+lastSurfaceTimestamp+" mapped="+clockMatches+" misses="+clockMisses+" epoch="+epoch+" count="+count);
        for(int i=1;i<=Math.min(4,count);i++){int at=(head-i+releases.length)%releases.length;value.append(" [").append(releases[at]).append(',').append(times[at]).append(',').append(epochs[at]).append(']');}return value.toString();}}
    private ExoPlayer player;
    private volatile boolean prepared,closed;
    private volatile int snapshotState;
    private volatile long snapshotPosition,snapshotDuration=-1,completedRequest;
    private volatile int snapshotWidth,snapshotHeight;
    private volatile boolean snapshotSeekable;
    private final SurfaceFrames surfaceFrames;
    private final HandlerThread thread=new HandlerThread("FerroUI.Media.Video");
    private final Handler worker;
    private final Object commands=new Object();
    private boolean drainQueued,wantedPlaying,wantedMuted;
    private float wantedVolume=1;
    private long wantedSeek,appliedRequest;
    private volatile String error="";
    private long request;
    private final Object clock=new Object();
    private final long[] releases=new long[32],times=new long[32],epochs=new long[32];
    private final int[] widths=new int[32],heights=new int[32],colors=new int[32];
    private int head,count;
    private long epoch;
    private volatile int frameWidth,frameHeight;
    private static final java.util.concurrent.atomic.AtomicInteger LIVE=new java.util.concurrent.atomic.AtomicInteger();
    public static int livePlayers() {return LIVE.get();}
    public VideoPlayer(Context context,SurfaceFrames surface,String source) {
        requireUi();
        last=new java.lang.ref.WeakReference<>(this);
        surfaceFrames=surface;surfaceFrames.retainSurface();
        thread.start();LIVE.incrementAndGet();worker=new Handler(thread.getLooper());
        worker.post(()->initialize(context,source));
    }
    private void initialize(Context context,String source) {
        if(closed) return;
        try {
            player=new ExoPlayer.Builder(context).setLooper(thread.getLooper())
                .setLoadControl(new DefaultLoadControl.Builder().setBufferDurationsMs(2500,5000,500,1000)
                    .setTargetBufferBytes(16*1024*1024).setPrioritizeTimeOverSizeThresholds(false).build()).build();
            player.setAudioAttributes(new AudioAttributes.Builder().setUsage(C.USAGE_MEDIA)
                .setContentType(C.AUDIO_CONTENT_TYPE_MOVIE).build(),true);
            player.setSeekParameters(SeekParameters.EXACT);
            player.setVideoSurface(surfaceFrames.surface());
            player.addListener(new Player.Listener(){
                @Override public void onPlayerError(PlaybackException failure) {error=failure.toString();}
                @Override public void onPlaybackStateChanged(int state) {
                    if(state==Player.STATE_READY) {prepared=true;updateSnapshot();completedRequest=appliedRequest;}
                }
                @Override public void onEvents(Player player,Player.Events events) {updateSnapshot();}
            });
            player.setVideoFrameMetadataListener((presentationUs,releaseNs,format,mediaFormat)->{
                int color=frameColor(format,mediaFormat);
                if(color==0) {error="HDR/wide-gamut video has no admitted SDR texture route";return;}
                synchronized(clock) {
                    int index=head;head=(head+1)%releases.length;count=Math.min(count+1,releases.length);
                    releases[index]=releaseNs;times[index]=presentationUs;epochs[index]=epoch;
                    widths[index]=format.width;heights[index]=format.height;colors[index]=color;
                    frameWidth=format.width;frameHeight=format.height;
                }
            });
            Uri uri=source.startsWith("/") ? Uri.fromFile(new File(source)) : Uri.parse(source);
            player.setMediaItem(MediaItem.fromUri(uri));player.prepare();
            updateSnapshot();
        } catch(RuntimeException failure) {error=failure.toString();}
    }
    private static void requireUi() {
        if(Looper.myLooper()!=Looper.getMainLooper()) throw new IllegalStateException("Player requires Android UI thread");
    }
    private static int frameColor(Format format,MediaFormat nativeFormat) {
        int transfer=format.colorInfo==null ? -1 : format.colorInfo.colorTransfer;
        int space=format.colorInfo==null ? -1 : format.colorInfo.colorSpace;
        if(nativeFormat!=null) {
            if(nativeFormat.containsKey(MediaFormat.KEY_COLOR_TRANSFER)) transfer=nativeFormat.getInteger(MediaFormat.KEY_COLOR_TRANSFER);
            if(nativeFormat.containsKey(MediaFormat.KEY_COLOR_STANDARD)) space=nativeFormat.getInteger(MediaFormat.KEY_COLOR_STANDARD);
        }
        if(transfer==C.COLOR_TRANSFER_ST2084 || transfer==C.COLOR_TRANSFER_HLG || space==C.COLOR_SPACE_BT2020) return 0;
        return space==C.COLOR_SPACE_BT709 ? 2 : 1;
    }
    /** Called only on the render thread; never calls the UI-owned player. An
     * unmapped frame is dropped rather than publishing an inferred timestamp. */
    public String frameInfo(long surfaceTimestampNs) {
        synchronized(clock) {
            lastSurfaceTimestamp=surfaceTimestampNs;
            for(int offset=1;offset<=count;offset++) {
                int index=(head-offset+releases.length)%releases.length;
                if(epochs[index]!=epoch || times[index]<0) continue;
                if(releases[index]==surfaceTimestampNs || releases[index]/1000==surfaceTimestampNs/1000 || times[index]*1000==surfaceTimestampNs) {
                    clockMatches++;
                    return times[index]+","+widths[index]+","+heights[index]+","+colors[index];
                }
            }
            clockMisses++;return "";
        }
    }
    public String error() {return error;}
    public int state() {
        if(!error.isEmpty()) return 5;
        return snapshotState;
    }
    public long position() {return snapshotPosition;}
    public long duration() {return snapshotDuration;}
    public int width() {return Math.max(0,frameWidth);}
    public int height() {return Math.max(0,frameHeight);}
    public int naturalWidth() {return snapshotWidth;}
    public int naturalHeight() {return snapshotHeight;}
    public boolean seekable() {return snapshotSeekable;}
    private boolean tickQueued;
    private void updateSnapshot() {
        if(closed || player==null) return;
        snapshotPosition=player.getCurrentPosition();snapshotDuration=player.getDuration()==C.TIME_UNSET ? -1 : player.getDuration();
        VideoSize size=player.getVideoSize();snapshotWidth=size.width;snapshotHeight=size.height;
        snapshotSeekable=player.isCurrentMediaItemSeekable();
        snapshotState=!prepared ? 0 : player.getPlaybackState()==Player.STATE_ENDED ? 4 : player.getPlaybackState()==Player.STATE_BUFFERING ? 3 : player.isPlaying() ? 2 : 1;
        if(!tickQueued && (snapshotState==0 || snapshotState==2 || snapshotState==3)) {
            tickQueued=true;worker.postDelayed(()->{tickQueued=false;updateSnapshot();},30);
        }
    }
    private void scheduleDrain() {
        synchronized(commands) {if(closed || drainQueued) return;drainQueued=true;}
        worker.post(()->{
            boolean playing,muted;float volume;long seek,number;
            synchronized(commands) {drainQueued=false;playing=wantedPlaying;muted=wantedMuted;volume=wantedVolume;seek=wantedSeek;number=request;}
            if(closed || player==null) return;
            try {
                // Stop/paused seeks must pause the native clock before the
                // seek, so a coalesced command cannot advance past frame zero.
                if(!playing) player.pause();
                if(number>appliedRequest) {
                    synchronized(clock) {epoch++;head=0;count=0;}
                    appliedRequest=number;player.seekTo(Math.max(0,seek));
                }
                player.setVolume(muted ? 0 : volume);
                if(playing) player.play();
                updateSnapshot();
                if(player.getPlaybackState()==Player.STATE_READY) completedRequest=appliedRequest;
            } catch(RuntimeException failure) {error=failure.toString();}
        });
    }
    public void play() {requireUi();synchronized(commands){wantedPlaying=true;}scheduleDrain();}
    public void pause() {requireUi();synchronized(commands){wantedPlaying=false;}scheduleDrain();}
    public long seek(long position) {
        requireUi();if(!prepared || closed) return 0;
        synchronized(clock) {epoch++;head=0;count=0;}
        long number;synchronized(commands){request++;wantedSeek=position;number=request;}scheduleDrain();return number;
    }
    public long completedSeek() {return completedRequest;}
    public void audio(float volume,boolean muted) {requireUi();synchronized(commands){wantedVolume=volume;wantedMuted=muted;}scheduleDrain();}
    public void close() {
        requireUi();if(closed) return;closed=true;
        synchronized(clock) {epoch++;head=0;count=0;}
        worker.post(()->{
            try {if(player!=null) {player.clearVideoSurface();player.release();player=null;}}
            finally {surfaceFrames.releaseSurface();LIVE.decrementAndGet();thread.quitSafely();}
        });
    }
}
