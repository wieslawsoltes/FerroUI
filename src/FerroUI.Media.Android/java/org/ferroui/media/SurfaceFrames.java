package org.ferroui.media;

import android.graphics.SurfaceTexture;
import android.opengl.EGL14;
import android.opengl.EGLContext;
import android.opengl.GLES11Ext;
import android.opengl.GLES30;
import android.os.Handler;
import android.os.Looper;
import android.view.Surface;
import java.util.concurrent.atomic.AtomicBoolean;

/** One bounded native SurfaceTexture source; immutable RGBA outputs belong to Rust.
 * All GL methods run on FerroUI's render thread. A conversion fence must retire
 * before updateTexImage is called again. No readPixels or preview pixel arrays. */
public final class SurfaceFrames {
    private final EGLContext context;
    private final Thread thread;
    private final AtomicBoolean pending = new AtomicBoolean();
    private final float[] transform = new float[16];
    private SurfaceTexture source;
    private Surface surface;
    private int external, program, vao, framebuffer;
    private long sequence, timestamp;
    private volatile boolean closed;
    private String error = "";
    private int surfaceUsers;
    private boolean orphaned;
    private boolean counted;
    private static final java.util.concurrent.atomic.AtomicInteger LIVE=new java.util.concurrent.atomic.AtomicInteger();
    public static int liveSources() {return LIVE.get();}

    public SurfaceFrames() {
        thread = Thread.currentThread();
        context = EGL14.eglGetCurrentContext();
        if (context.equals(EGL14.EGL_NO_CONTEXT)) throw new IllegalStateException("No current EGL context");
        int active = integer(GLES30.GL_ACTIVE_TEXTURE);
        GLES30.glActiveTexture(GLES30.GL_TEXTURE0);
        int binding = integer(GLES11Ext.GL_TEXTURE_BINDING_EXTERNAL_OES);
        int[] id = new int[1];
        int vertex = 0, fragment = 0;
        try {
            GLES30.glGenTextures(1, id, 0); external = id[0];
            GLES30.glBindTexture(GLES11Ext.GL_TEXTURE_EXTERNAL_OES, external);
            GLES30.glTexParameteri(GLES11Ext.GL_TEXTURE_EXTERNAL_OES, GLES30.GL_TEXTURE_MIN_FILTER, GLES30.GL_LINEAR);
            GLES30.glTexParameteri(GLES11Ext.GL_TEXTURE_EXTERNAL_OES, GLES30.GL_TEXTURE_MAG_FILTER, GLES30.GL_LINEAR);
            GLES30.glTexParameteri(GLES11Ext.GL_TEXTURE_EXTERNAL_OES, GLES30.GL_TEXTURE_WRAP_S, GLES30.GL_CLAMP_TO_EDGE);
            GLES30.glTexParameteri(GLES11Ext.GL_TEXTURE_EXTERNAL_OES, GLES30.GL_TEXTURE_WRAP_T, GLES30.GL_CLAMP_TO_EDGE);
            vertex = shader(GLES30.GL_VERTEX_SHADER,
                "#version 300 es\nuniform mat4 imageTransform; out vec2 uv; void main(){"
                + "vec2 p=vec2(float((gl_VertexID<<1)&2),float(gl_VertexID&2));"
                + "uv=(imageTransform*vec4(p,0,1)).xy; gl_Position=vec4(p*2.0-1.0,0,1);}");
            fragment = shader(GLES30.GL_FRAGMENT_SHADER,
                "#version 300 es\n#extension GL_OES_EGL_image_external_essl3 : require\n"
                + "precision highp float; uniform samplerExternalOES image; in vec2 uv; out vec4 color;"
                + "void main(){color=vec4(texture(image,uv).rgb,1.0);}");
            program = GLES30.glCreateProgram();
            GLES30.glAttachShader(program, vertex); GLES30.glAttachShader(program, fragment);
            GLES30.glLinkProgram(program);
            GLES30.glGetProgramiv(program, GLES30.GL_LINK_STATUS, id, 0);
            if (id[0] == 0) throw new IllegalStateException(GLES30.glGetProgramInfoLog(program));
            GLES30.glGenVertexArrays(1, id, 0); vao = id[0];
            GLES30.glGenFramebuffers(1, id, 0); framebuffer = id[0];
            source = new SurfaceTexture(external);
            source.setOnFrameAvailableListener(unused -> pending.set(true), new Handler(Looper.getMainLooper()));
            surface = new Surface(source);
            counted=true;LIVE.incrementAndGet();
        } catch (RuntimeException failure) {
            release(true);
            error = failure.toString();
        } finally {
            if (vertex != 0) GLES30.glDeleteShader(vertex);
            if (fragment != 0) GLES30.glDeleteShader(fragment);
            GLES30.glBindTexture(GLES11Ext.GL_TEXTURE_EXTERNAL_OES, binding);
            GLES30.glActiveTexture(active);
        }
    }
    public synchronized Surface surface() { return surface; }
    public String error() { return error; }
    public boolean pending() { return pending.get() && !closed; }
    public long sequence() { return sequence; }
    public long timestamp() { return timestamp; }
    public boolean swapsAxes() { return Math.abs(transform[1])>Math.abs(transform[0]) && Math.abs(transform[4])>Math.abs(transform[5]); }
    public void setSize(int width, int height) { source.setDefaultBufferSize(width, height); }
    private void requireContext() {
        if (closed || thread != Thread.currentThread() || !context.equals(EGL14.eglGetCurrentContext()))
            throw new IllegalStateException("Surface frame context changed");
    }
    public String acquire() {
        requireContext();
        int active=integer(GLES30.GL_ACTIVE_TEXTURE);
        GLES30.glActiveTexture(GLES30.GL_TEXTURE0);
        int binding=integer(GLES11Ext.GL_TEXTURE_BINDING_EXTERNAL_OES);
        try {
            if(!pending.getAndSet(false)) return "";
            source.updateTexImage(); source.getTransformMatrix(transform);
            timestamp=source.getTimestamp(); sequence++;
            return "";
        } catch(RuntimeException failure) { return failure.toString(); }
        finally { GLES30.glBindTexture(GLES11Ext.GL_TEXTURE_EXTERNAL_OES,binding); GLES30.glActiveTexture(active); }
    }
    /** Returns an error rather than allowing JNI exceptions to escape. */
    public String draw(int output, int width, int height) {
        requireContext();
        int active = integer(GLES30.GL_ACTIVE_TEXTURE);
        GLES30.glActiveTexture(GLES30.GL_TEXTURE0);
        int binding = integer(GLES11Ext.GL_TEXTURE_BINDING_EXTERNAL_OES);
        int sampler = integer(GLES30.GL_SAMPLER_BINDING);
        int draw = integer(GLES30.GL_DRAW_FRAMEBUFFER_BINDING), read = integer(GLES30.GL_READ_FRAMEBUFFER_BINDING);
        int oldProgram = integer(GLES30.GL_CURRENT_PROGRAM), oldVao = integer(GLES30.GL_VERTEX_ARRAY_BINDING);
        int[] viewport = new int[4]; GLES30.glGetIntegerv(GLES30.GL_VIEWPORT, viewport, 0);
        boolean[] mask = new boolean[4]; GLES30.glGetBooleanv(GLES30.GL_COLOR_WRITEMASK, mask, 0);
        int[] capabilities = {GLES30.GL_BLEND,GLES30.GL_DEPTH_TEST,GLES30.GL_STENCIL_TEST,
            GLES30.GL_SCISSOR_TEST,GLES30.GL_CULL_FACE,GLES30.GL_RASTERIZER_DISCARD,GLES30.GL_DITHER,
            GLES30.GL_SAMPLE_ALPHA_TO_COVERAGE,GLES30.GL_SAMPLE_COVERAGE};
        boolean[] enabled = new boolean[capabilities.length];
        for (int i=0;i<capabilities.length;i++) { enabled[i]=GLES30.glIsEnabled(capabilities[i]); GLES30.glDisable(capabilities[i]); }
        try {
            if (sequence == 0) return "No decoded source frame";
            GLES30.glBindFramebuffer(GLES30.GL_DRAW_FRAMEBUFFER, framebuffer);
            GLES30.glFramebufferTexture2D(GLES30.GL_DRAW_FRAMEBUFFER, GLES30.GL_COLOR_ATTACHMENT0, GLES30.GL_TEXTURE_2D, output, 0);
            if (GLES30.glCheckFramebufferStatus(GLES30.GL_DRAW_FRAMEBUFFER) != GLES30.GL_FRAMEBUFFER_COMPLETE)
                return "RGBA output framebuffer is incomplete";
            GLES30.glViewport(0,0,width,height); GLES30.glColorMask(true,true,true,true);
            GLES30.glUseProgram(program); GLES30.glBindVertexArray(vao);
            GLES30.glBindSampler(0,0); GLES30.glBindTexture(GLES11Ext.GL_TEXTURE_EXTERNAL_OES,external);
            GLES30.glUniform1i(GLES30.glGetUniformLocation(program,"image"),0);
            GLES30.glUniformMatrix4fv(GLES30.glGetUniformLocation(program,"imageTransform"),1,false,transform,0);
            GLES30.glDrawArrays(GLES30.GL_TRIANGLES,0,3);
            int error = GLES30.glGetError();
            return error == GLES30.GL_NO_ERROR ? "" : "Surface conversion GL error " + error;
        } catch (RuntimeException failure) { return failure.toString(); }
        finally {
            GLES30.glBindFramebuffer(GLES30.GL_DRAW_FRAMEBUFFER,framebuffer);
            GLES30.glFramebufferTexture2D(GLES30.GL_DRAW_FRAMEBUFFER,GLES30.GL_COLOR_ATTACHMENT0,GLES30.GL_TEXTURE_2D,0,0);
            GLES30.glBindFramebuffer(GLES30.GL_DRAW_FRAMEBUFFER,draw); GLES30.glBindFramebuffer(GLES30.GL_READ_FRAMEBUFFER,read);
            GLES30.glViewport(viewport[0],viewport[1],viewport[2],viewport[3]);
            GLES30.glColorMask(mask[0],mask[1],mask[2],mask[3]);
            for(int i=0;i<capabilities.length;i++) { if(enabled[i]) GLES30.glEnable(capabilities[i]); else GLES30.glDisable(capabilities[i]); }
            GLES30.glUseProgram(oldProgram); GLES30.glBindVertexArray(oldVao);
            GLES30.glBindTexture(GLES11Ext.GL_TEXTURE_EXTERNAL_OES,binding); GLES30.glBindSampler(0,sampler);
            GLES30.glActiveTexture(active);
        }
    }
    public synchronized void retainSurface() { if(closed) throw new IllegalStateException("Surface closed"); surfaceUsers++; }
    public synchronized void releaseSurface() {
        if(surfaceUsers<=0) throw new IllegalStateException("Unbalanced Surface ownership");
        surfaceUsers--;
        if(surfaceUsers==0 && orphaned) release(false);
    }
    public synchronized boolean release(boolean validContext) {
        if (closed) return true;
        if(surfaceUsers>0) { if(!validContext) orphaned=true; return !validContext; }
        closed=true; pending.set(false);
        if (surface!=null) { surface.release(); surface=null; }
        if (source!=null) { source.setOnFrameAvailableListener(null); source.release(); source=null; }
        if(counted) {counted=false;LIVE.decrementAndGet();}
        if (validContext && context.equals(EGL14.eglGetCurrentContext())) {
            if(program!=0) GLES30.glDeleteProgram(program);
            if(external!=0) GLES30.glDeleteTextures(1,new int[]{external},0);
            if(vao!=0) GLES30.glDeleteVertexArrays(1,new int[]{vao},0);
            if(framebuffer!=0) GLES30.glDeleteFramebuffers(1,new int[]{framebuffer},0);
        }
        return true;
    }
    private static int integer(int name) { int[] value=new int[1]; GLES30.glGetIntegerv(name,value,0); return value[0]; }
    /** Independent GPU producer, without a media decoder or camera. */
    public static String quadrants(int output,int width,int height) {
        int oldFramebuffer=integer(GLES30.GL_DRAW_FRAMEBUFFER_BINDING);
        int[] box=new int[4];GLES30.glGetIntegerv(GLES30.GL_SCISSOR_BOX,box,0);
        float[] color=new float[4];GLES30.glGetFloatv(GLES30.GL_COLOR_CLEAR_VALUE,color,0);
        boolean[] mask=new boolean[4];GLES30.glGetBooleanv(GLES30.GL_COLOR_WRITEMASK,mask,0);
        boolean scissor=GLES30.glIsEnabled(GLES30.GL_SCISSOR_TEST),dither=GLES30.glIsEnabled(GLES30.GL_DITHER);
        int[] name=new int[1];GLES30.glGenFramebuffers(1,name,0);
        try {
            GLES30.glBindFramebuffer(GLES30.GL_DRAW_FRAMEBUFFER,name[0]);
            GLES30.glFramebufferTexture2D(GLES30.GL_DRAW_FRAMEBUFFER,GLES30.GL_COLOR_ATTACHMENT0,GLES30.GL_TEXTURE_2D,output,0);
            if(GLES30.glCheckFramebufferStatus(GLES30.GL_DRAW_FRAMEBUFFER)!=GLES30.GL_FRAMEBUFFER_COMPLETE) return "Independent GPU framebuffer incomplete";
            GLES30.glEnable(GLES30.GL_SCISSOR_TEST);GLES30.glDisable(GLES30.GL_DITHER);GLES30.glColorMask(true,true,true,true);
            for(int i=0;i<4;i++) {
                int x=(i&1)==0 ? 0 : width/2,y=(i&2)==0 ? 0 : height/2;
                GLES30.glScissor(x,y,(i&1)==0 ? width/2 : width-width/2,(i&2)==0 ? height/2 : height-height/2);
                GLES30.glClearColor(i==0 || i==3 ? 1 : 0,i==1 || i==3 ? 1 : 0,i>=2 ? 1 : 0,1);
                GLES30.glClear(GLES30.GL_COLOR_BUFFER_BIT);
            }
            int error=GLES30.glGetError();return error==0 ? "" : "Independent producer GL error "+error;
        } finally {
            GLES30.glBindFramebuffer(GLES30.GL_DRAW_FRAMEBUFFER,oldFramebuffer);GLES30.glDeleteFramebuffers(1,name,0);
            GLES30.glScissor(box[0],box[1],box[2],box[3]);GLES30.glClearColor(color[0],color[1],color[2],color[3]);
            GLES30.glColorMask(mask[0],mask[1],mask[2],mask[3]);
            if(scissor) GLES30.glEnable(GLES30.GL_SCISSOR_TEST);else GLES30.glDisable(GLES30.GL_SCISSOR_TEST);
            if(dither) GLES30.glEnable(GLES30.GL_DITHER);else GLES30.glDisable(GLES30.GL_DITHER);
        }
    }
    private static int shader(int type,String text) {
        int shader=GLES30.glCreateShader(type); GLES30.glShaderSource(shader,text); GLES30.glCompileShader(shader);
        int[] status=new int[1]; GLES30.glGetShaderiv(shader,GLES30.GL_COMPILE_STATUS,status,0);
        if(status[0]!=0) return shader;
        String error=GLES30.glGetShaderInfoLog(shader); GLES30.glDeleteShader(shader); throw new IllegalStateException(error);
    }
}
