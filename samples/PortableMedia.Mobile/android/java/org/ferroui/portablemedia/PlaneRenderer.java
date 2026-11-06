package org.ferroui.portablemedia;
import android.opengl.GLES30;
/** Client scene: one transparent perspective-projected textured plane. */
public final class PlaneRenderer {
    private int program,vao,framebuffer;
    public PlaneRenderer(){
        int vertex=0,fragment=0;int[] name=new int[1];
        try{
            vertex=shader(GLES30.GL_VERTEX_SHADER,"#version 300 es\nuniform mat4 projection;uniform vec2 uv0,uvx,uvy;out vec2 uv;void main(){vec2 p=vec2(float(gl_VertexID&1),float((gl_VertexID>>1)&1));gl_Position=projection*vec4(p,0,1);uv=uv0+p.x*uvx+p.y*uvy;}");
            fragment=shader(GLES30.GL_FRAGMENT_SHADER,"#version 300 es\nprecision highp float;uniform sampler2D image;uniform bool textured,rec709;in vec2 uv;out vec4 color;void main(){vec3 rgb=textured?texture(image,uv).rgb:vec3(uv,.5);if(rec709){vec3 light=mix(pow((rgb+.099)/1.099,vec3(1./.45)),rgb/4.5,lessThan(rgb,vec3(.081)));rgb=mix(1.055*pow(max(light,vec3(0)),vec3(1./2.4))-.055,12.92*light,lessThan(light,vec3(.0031308)));}color=vec4(rgb*.85,.85);}");
            program=GLES30.glCreateProgram();GLES30.glAttachShader(program,vertex);GLES30.glAttachShader(program,fragment);GLES30.glLinkProgram(program);GLES30.glGetProgramiv(program,GLES30.GL_LINK_STATUS,name,0);
            if(name[0]==0)throw new IllegalStateException(GLES30.glGetProgramInfoLog(program));
            GLES30.glGenVertexArrays(1,name,0);vao=name[0];GLES30.glGenFramebuffers(1,name,0);framebuffer=name[0];
        }catch(RuntimeException failure){release(true);throw failure;}
        finally{if(vertex!=0)GLES30.glDeleteShader(vertex);if(fragment!=0)GLES30.glDeleteShader(fragment);}
    }
    public String draw(int output,int input,int width,int height,String values,boolean convert){
        String[] fields=values.split(",");if(fields.length!=28)return "Invalid scene parameters";
        float[] parameters=new float[28];for(int i=0;i<28;i++)parameters[i]=Float.parseFloat(fields[i]);
        int active=integer(GLES30.GL_ACTIVE_TEXTURE);GLES30.glActiveTexture(GLES30.GL_TEXTURE0);
        int texture=integer(GLES30.GL_TEXTURE_BINDING_2D),sampler=integer(GLES30.GL_SAMPLER_BINDING);
        int draw=integer(GLES30.GL_DRAW_FRAMEBUFFER_BINDING),read=integer(GLES30.GL_READ_FRAMEBUFFER_BINDING);
        int oldProgram=integer(GLES30.GL_CURRENT_PROGRAM),oldVao=integer(GLES30.GL_VERTEX_ARRAY_BINDING);
        int[] viewport=new int[4];GLES30.glGetIntegerv(GLES30.GL_VIEWPORT,viewport,0);
        float[] clear=new float[4];GLES30.glGetFloatv(GLES30.GL_COLOR_CLEAR_VALUE,clear,0);
        boolean[] mask=new boolean[4];GLES30.glGetBooleanv(GLES30.GL_COLOR_WRITEMASK,mask,0);
        int[] capabilities={GLES30.GL_BLEND,GLES30.GL_DEPTH_TEST,GLES30.GL_STENCIL_TEST,GLES30.GL_SCISSOR_TEST,GLES30.GL_CULL_FACE,GLES30.GL_RASTERIZER_DISCARD,GLES30.GL_DITHER,GLES30.GL_SAMPLE_ALPHA_TO_COVERAGE,GLES30.GL_SAMPLE_COVERAGE};
        boolean[] enabled=new boolean[capabilities.length];for(int i=0;i<enabled.length;i++){enabled[i]=GLES30.glIsEnabled(capabilities[i]);GLES30.glDisable(capabilities[i]);}
        try{
            GLES30.glBindFramebuffer(GLES30.GL_DRAW_FRAMEBUFFER,framebuffer);GLES30.glFramebufferTexture2D(GLES30.GL_DRAW_FRAMEBUFFER,GLES30.GL_COLOR_ATTACHMENT0,GLES30.GL_TEXTURE_2D,output,0);
            if(GLES30.glCheckFramebufferStatus(GLES30.GL_DRAW_FRAMEBUFFER)!=GLES30.GL_FRAMEBUFFER_COMPLETE)return "Scene framebuffer incomplete";
            GLES30.glViewport(0,0,width,height);GLES30.glColorMask(true,true,true,true);GLES30.glClearColor(0,0,0,0);GLES30.glClear(GLES30.GL_COLOR_BUFFER_BIT);
            GLES30.glUseProgram(program);GLES30.glBindVertexArray(vao);GLES30.glBindSampler(0,0);GLES30.glBindTexture(GLES30.GL_TEXTURE_2D,input);
            GLES30.glUniform1i(GLES30.glGetUniformLocation(program,"image"),0);GLES30.glUniform1i(GLES30.glGetUniformLocation(program,"textured"),input==0?0:1);GLES30.glUniform1i(GLES30.glGetUniformLocation(program,"rec709"),convert?1:0);
            GLES30.glUniformMatrix4fv(GLES30.glGetUniformLocation(program,"projection"),1,false,parameters,0);
            GLES30.glUniform2fv(GLES30.glGetUniformLocation(program,"uv0"),1,parameters,16);GLES30.glUniform2fv(GLES30.glGetUniformLocation(program,"uvx"),1,parameters,20);GLES30.glUniform2fv(GLES30.glGetUniformLocation(program,"uvy"),1,parameters,24);
            GLES30.glDrawArrays(GLES30.GL_TRIANGLE_STRIP,0,4);int error=GLES30.glGetError();return error==0?"":"Scene GL error "+error;
        }finally{
            GLES30.glBindFramebuffer(GLES30.GL_DRAW_FRAMEBUFFER,framebuffer);GLES30.glFramebufferTexture2D(GLES30.GL_DRAW_FRAMEBUFFER,GLES30.GL_COLOR_ATTACHMENT0,GLES30.GL_TEXTURE_2D,0,0);
            GLES30.glBindFramebuffer(GLES30.GL_DRAW_FRAMEBUFFER,draw);GLES30.glBindFramebuffer(GLES30.GL_READ_FRAMEBUFFER,read);
            GLES30.glViewport(viewport[0],viewport[1],viewport[2],viewport[3]);GLES30.glColorMask(mask[0],mask[1],mask[2],mask[3]);GLES30.glClearColor(clear[0],clear[1],clear[2],clear[3]);
            for(int i=0;i<enabled.length;i++){if(enabled[i])GLES30.glEnable(capabilities[i]);else GLES30.glDisable(capabilities[i]);}
            GLES30.glUseProgram(oldProgram);GLES30.glBindVertexArray(oldVao);GLES30.glBindTexture(GLES30.GL_TEXTURE_2D,texture);GLES30.glBindSampler(0,sampler);GLES30.glActiveTexture(active);
        }
    }
    // Explicit diagnostic only; renderer output is GPU-complete before call.
    public static String probe(int texture,int width,int height){
        int old=integer(GLES30.GL_READ_FRAMEBUFFER_BINDING),pack=integer(GLES30.GL_PIXEL_PACK_BUFFER_BINDING),alignment=integer(GLES30.GL_PACK_ALIGNMENT);
        int[] name=new int[1];GLES30.glGenFramebuffers(1,name,0);
        try{
            GLES30.glBindFramebuffer(GLES30.GL_READ_FRAMEBUFFER,name[0]);GLES30.glFramebufferTexture2D(GLES30.GL_READ_FRAMEBUFFER,GLES30.GL_COLOR_ATTACHMENT0,GLES30.GL_TEXTURE_2D,texture,0);
            if(GLES30.glCheckFramebufferStatus(GLES30.GL_READ_FRAMEBUFFER)!=GLES30.GL_FRAMEBUFFER_COMPLETE)return "";
            GLES30.glBindBuffer(GLES30.GL_PIXEL_PACK_BUFFER,0);GLES30.glPixelStorei(GLES30.GL_PACK_ALIGNMENT,1);
            java.nio.ByteBuffer pixel=java.nio.ByteBuffer.allocateDirect(4);int[] x={2,98,98,2,50,78,78,18,18},y={2,2,98,98,50,30,65,70,20};
            StringBuilder result=new StringBuilder();
            for(int i=0;i<9;i++){pixel.clear();GLES30.glReadPixels(width*x[i]/100,height-1-height*y[i]/100,1,1,GLES30.GL_RGBA,GLES30.GL_UNSIGNED_BYTE,pixel);if(i!=0)result.append(',');result.append(pixel.get(3)&255);}
            int[] qx={3833,6683,3385,6136},qy={3389,3606,6302,6136};
            for(int i=0;i<4;i++){
                pixel.clear();GLES30.glReadPixels(width*qx[i]/10000,height-1-height*qy[i]/10000,1,1,GLES30.GL_RGBA,GLES30.GL_UNSIGNED_BYTE,pixel);
                for(int channel=0;channel<4;channel++)result.append(',').append(pixel.get(channel)&255);
            }
            return GLES30.glGetError()==0?result.toString():"";
        }finally{
            GLES30.glBindFramebuffer(GLES30.GL_READ_FRAMEBUFFER,old);GLES30.glBindBuffer(GLES30.GL_PIXEL_PACK_BUFFER,pack);GLES30.glPixelStorei(GLES30.GL_PACK_ALIGNMENT,alignment);GLES30.glDeleteFramebuffers(1,name,0);
        }
    }
    public void release(boolean valid){if(valid){if(program!=0)GLES30.glDeleteProgram(program);if(vao!=0)GLES30.glDeleteVertexArrays(1,new int[]{vao},0);if(framebuffer!=0)GLES30.glDeleteFramebuffers(1,new int[]{framebuffer},0);}program=vao=framebuffer=0;}
    private static int integer(int name){int[] value=new int[1];GLES30.glGetIntegerv(name,value,0);return value[0];}
    private static int shader(int kind,String source){int shader=GLES30.glCreateShader(kind);GLES30.glShaderSource(shader,source);GLES30.glCompileShader(shader);int[] ok=new int[1];GLES30.glGetShaderiv(shader,GLES30.GL_COMPILE_STATUS,ok,0);if(ok[0]!=0)return shader;String error=GLES30.glGetShaderInfoLog(shader);GLES30.glDeleteShader(shader);throw new IllegalStateException(error);}
}
