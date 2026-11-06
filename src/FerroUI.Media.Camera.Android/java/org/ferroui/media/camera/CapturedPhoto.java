package org.ferroui.media.camera;

/** One independently owned encoded result. Canceling an earlier request cannot
 * clear another request's bytes, including after the native capture slot frees. */
public final class CapturedPhoto {
    private int state;
    private byte[] bytes;
    private String error="";
    private int width,height,rotation;
    private long timestamp;
    public synchronized void complete(byte[] value,int width,int height,int rotation,long timestamp) {
        if(state!=0) return;
        bytes=value;this.width=width;this.height=height;this.rotation=rotation;this.timestamp=timestamp;state=1;
    }
    public synchronized void fail(String error) {if(state==0) {this.error=error;state=2;}}
    public synchronized void abandon() {state=3;bytes=null;}
    public synchronized boolean abandoned() {return state==3;}
    public synchronized boolean pending() {return state==0;}
    public synchronized String error() {return error;}
    public synchronized int length() {return bytes==null ? 0 : bytes.length;}
    public synchronized String info() {return width+","+height+","+rotation+","+timestamp;}
    public synchronized byte[] takeBytes() {byte[] result=bytes;bytes=null;return result;}
}
