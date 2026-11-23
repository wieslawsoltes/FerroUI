package org.ferroui.media.camera;

import android.content.Context;
import android.media.MediaExtractor;
import android.media.MediaFormat;
import androidx.camera.video.FileOutputOptions;
import androidx.camera.video.Recorder;
import androidx.camera.video.Recording;
import androidx.camera.video.VideoRecordEvent;
import java.io.DataInputStream;
import java.io.File;
import java.io.FileInputStream;
import java.io.IOException;
import java.util.concurrent.Executor;

/** One CameraX recording and its private file. CameraX commands run on the main
 * executor. The Rust worker validates and copies the finalized file off-thread. */
public final class CameraRecordingRequest {
    private static final int MAX_BYTES = 256 * 1024 * 1024;
    private static final long MAX_MILLIS = 60_000;

    private final Executor main;
    private File file;
    private Recording recording;
    private boolean stopRequested;
    private volatile boolean cancelled;
    private volatile boolean started;
    private volatile boolean finalized;
    private volatile String error = "";
    private volatile String info = "";

    CameraRecordingRequest(Executor main) {
        this.main = main;
    }

    void start(Context context, Recorder recorder, Runnable release) throws IOException {
        file = File.createTempFile("ferroui-camera-", ".mp4", context.getCacheDir());
        FileOutputOptions output = new FileOutputOptions.Builder(file)
            .setFileSizeLimit(MAX_BYTES).setDurationLimitMillis(MAX_MILLIS).build();
        recording = recorder.prepareRecording(context, output).start(main, event -> {
            if (event instanceof VideoRecordEvent.Start) {
                started = true;
            } else if (event instanceof VideoRecordEvent.Finalize) {
                recording = null;
                int code = ((VideoRecordEvent.Finalize) event).getError();
                if (cancelled || (code != VideoRecordEvent.Finalize.ERROR_NONE
                        && code != VideoRecordEvent.Finalize.ERROR_FILE_SIZE_LIMIT_REACHED
                        && code != VideoRecordEvent.Finalize.ERROR_DURATION_LIMIT_REACHED)) {
                    fail(cancelled ? "Recording cancelled" : "CameraX recording error " + code);
                } else {
                    finalized = true;
                }
                release.run();
            }
        });
    }

    synchronized void fail(String message) {
        error = message;
        discardFile();
        finalized = true;
    }

    public boolean startPending() { return !started && !finalized && !cancelled; }
    public boolean pending() { return !finalized && !cancelled; }
    public String error() { return cancelled ? "Recording cancelled" : error; }
    public String info() { return info; }

    public void finish() { main.execute(this::stop); }

    public void cancel() {
        synchronized (this) {
            if (cancelled) return;
            cancelled = true;
        }
        main.execute(() -> {
            stop();
            synchronized (this) { if (finalized) discardFile(); }
        });
    }

    private void stop() {
        if (recording != null && !stopRequested) {
            stopRequested = true;
            recording.stop();
        }
    }

    private void discardFile() {
        if (file != null) {
            file.delete();
            file = null;
        }
    }

    public byte[] takeBytes() {
        File output;
        synchronized (this) {
            if (!finalized || cancelled || !error.isEmpty() || file == null) return null;
            output = file;
            file = null;
        }
        try {
            long size = output.length();
            if (size <= 0 || size > MAX_BYTES) throw new IOException("Recording exceeds size limit or is empty");
            inspect(output);
            if (cancelled) return null;
            byte[] bytes = new byte[(int) size];
            try (DataInputStream stream = new DataInputStream(new FileInputStream(output))) {
                stream.readFully(bytes);
            }
            return cancelled ? null : bytes;
        } catch (IOException | RuntimeException failure) {
            error = failure.toString();
            return null;
        } finally {
            output.delete();
        }
    }

    private void inspect(File output) throws IOException {
        MediaExtractor extractor = new MediaExtractor();
        try {
            extractor.setDataSource(output.getAbsolutePath());
            if (extractor.getTrackCount() != 1) throw new IOException("Expected one silent video track");
            MediaFormat format = extractor.getTrackFormat(0);
            if (!MediaFormat.MIMETYPE_VIDEO_AVC.equals(format.getString(MediaFormat.KEY_MIME))) {
                throw new IOException("Camera did not record H.264 video");
            }
            int width = format.getInteger(MediaFormat.KEY_WIDTH);
            int height = format.getInteger(MediaFormat.KEY_HEIGHT);
            long duration = format.getLong(MediaFormat.KEY_DURATION);
            if (width <= 0 || height <= 0 || duration <= 0 || duration > MAX_MILLIS * 1000) {
                throw new IOException("Invalid recording geometry or duration");
            }
            info = width + "," + height + "," + duration;
        } finally {
            extractor.release();
        }
    }
}
