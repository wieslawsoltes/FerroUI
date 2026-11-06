let consent=0; // NotDetermined until a non-prompting permission query or acquisition answers.
let permissionRequestPending=false;
let photoInFlight=false;
async function jpeg(image,width,height) {
    if(!width||!height||width>4096||height>4096){throw new DOMException("Photo dimensions exceed the encoding bound","NotSupportedError");}
    const canvas=document.createElement("canvas");canvas.width=width;canvas.height=height;
    const context=canvas.getContext("2d");if(!context){throw new Error("Photo encoder canvas is unavailable");}
    context.drawImage(image,0,0);
    try {return await new Promise((resolve,reject)=>canvas.toBlob(b=>b?resolve(b):reject(new Error("JPEG encoding failed")),"image/jpeg",0.9));}
    finally {canvas.width=0;canvas.height=0;}
}
export function permission() {return consent;}
async function refreshPermission() {
    try {
        const result=await navigator.permissions.query({name:"camera"});
        const update=() => {consent=result.state==="granted"?1:result.state==="denied"?2:0;};
        update();
        // The permission object owns its handler; no native device is acquired.
        result.onchange=update;
    } catch { /* Unsupported permission query is not evidence of a grant. */ }
}
export async function devices() {
    if (!navigator.mediaDevices?.enumerateDevices) {throw new DOMException("Camera discovery is unavailable","NotSupportedError");}
    await refreshPermission();
    const devices=await navigator.mediaDevices.enumerateDevices();
    return JSON.stringify(devices.filter(d=>d.kind==="videoinput").map(d=>({id:d.deviceId,label:d.label||null})));
}
export function start(options) {return new CameraRequest(JSON.parse(options));}
export class CameraRequest {
    constructor(options) {
        this.closed=false;this.camera=undefined;
        this.promise=this.acquire(options);
        this.promise.catch(()=>{});
    }
    async acquire(options) {
        if (!globalThis.isSecureContext || !navigator.mediaDevices?.getUserMedia) {throw new DOMException("Camera acquisition requires a secure context","NotSupportedError");}
        if(!HTMLVideoElement.prototype.requestVideoFrameCallback){throw new DOMException("Accurate native video-frame callbacks are unavailable","NotSupportedError");}
        if (permissionRequestPending) {throw new DOMException("A prior camera permission request is still pending","InvalidStateError");}
        const constraint=value=>options.exact?{exact:value}:{ideal:value};
        const video={};
        if (options.device) {video.deviceId=constraint(options.device);}
        if (options.facing) {video.facingMode=constraint(options.facing);}
        if (options.width) {video.width=constraint(options.width);video.height=constraint(options.height);video.frameRate=constraint(options.fps);}
        permissionRequestPending=true;
        let stream;
        try {
            stream=await navigator.mediaDevices.getUserMedia({audio:false,video});
            consent=1;
        } catch(error) {
            if (error.name==="NotAllowedError") {consent=2;}
            throw error;
        } finally {permissionRequestPending=false;}
        if (this.closed) {stream.getTracks().forEach(t=>t.stop());throw new DOMException("Camera start was canceled","AbortError");}
        const camera=new BrowserCamera(stream);this.camera=camera;
        try {
            await camera.prepare();
            if (this.closed) {throw new DOMException("Camera start was canceled","AbortError");}
            return camera;
        } catch(error) {camera.close();throw error;}
    }
    commit() {this.camera=undefined;}
    close() {this.closed=true;this.camera?.close();this.camera=undefined;}
}
export class BrowserCamera {
    constructor(stream) {
        this.stream=stream;this.track=stream.getVideoTracks()[0];this.closed=false;
        this.video=document.createElement("video");this.video.playsInline=true;this.video.muted=true;
        this.video.srcObject=stream;this.sequence=0;this.timestamp=0;this.width=0;this.height=0;
        this.events=new AbortController();this.disconnected=false;this.nativeStill=false;
        this.track.addEventListener("ended",()=>{this.disconnected=true;},{signal:this.events.signal});
        const next=(_now,metadata)=>{
            if(this.closed){return;}
            this.sequence++;this.timestamp=metadata.mediaTime;this.width=metadata.width;this.height=metadata.height;
            this.callback=this.video.requestVideoFrameCallback(next);
        };
        if(this.video.requestVideoFrameCallback){this.callback=this.video.requestVideoFrameCallback(next);}
    }
    async prepare(){
        await this.video.play();
        if(typeof ImageCapture!=="undefined"&&typeof ImageCapture.prototype.takePhoto==="function"){
            try {this.photoCapabilities=await new ImageCapture(this.track).getPhotoCapabilities();this.nativeStill=true;}
            catch { /* This device has no admitted native still path. */ }
        }
    }
    get active(){return !this.closed&&!this.disconnected&&this.track.readyState==="live";}
    settings() {
        const s=this.track.getSettings();
        return JSON.stringify({device:s.deviceId||"",label:this.track.label||null,facing:s.facingMode||"unknown",width:s.width||0,height:s.height||0,fps:s.frameRate||0,nativeStill:this.nativeStill});
    }
    sample() {
        if(!this.active||this.video.readyState<2){return null;}
        if(!this.sequence||!this.width||!this.height){return null;}
        return {image:this.video,sequence:this.sequence,timestamp:this.timestamp,width:this.width,height:this.height};
    }
    async photo() {
        if(!this.active){throw new DOMException("Camera is disconnected","NotReadableError");}
        if(photoInFlight){throw new DOMException("A photo is already pending","InvalidStateError");}
        photoInFlight=true;
        try {
            let blob;let origin;
            if(this.nativeStill){
                const capture=new ImageCapture(this.track);
                const capabilities=this.photoCapabilities;
                const options={};
                if(capabilities.imageWidth?.min>4096||capabilities.imageHeight?.min>4096){throw new DOMException("Native still dimensions exceed the photo bound","NotSupportedError");}
                if(capabilities.imageWidth?.max){options.imageWidth=Math.min(4096,capabilities.imageWidth.max);}
                if(capabilities.imageHeight?.max){options.imageHeight=Math.min(4096,capabilities.imageHeight.max);}
                const native=await capture.takePhoto(options);origin="native";
                if(!this.active){throw new DOMException("Camera stopped during capture","AbortError");}
                if(native.size>64*1024*1024){throw new DOMException("Native photo exceeds 64 MiB","NotSupportedError");}
                // Native still formats vary (including PNG). Decode and encode
                // only this owned photo, flattening EXIF orientation into JPEG.
                // No preview frame is read back or staged through this canvas.
                const image=await createImageBitmap(native);
                try {blob=await jpeg(image,image.width,image.height);} finally {image.close();}
            } else {
                // Explicit still-output snapshot provenance. This canvas is
                // never a preview staging/readback route and has no pixel API.
                const width=this.video.videoWidth,height=this.video.videoHeight;
                if(!width||!height||width>4096||height>4096){throw new DOMException("Snapshot dimensions exceed the capture bound","NotSupportedError");}
                blob=await jpeg(this.video,width,height);
                origin="preview";
            }
            if(!this.active){throw new DOMException("Camera stopped during capture","AbortError");}
            if(blob.type!=="image/jpeg"){throw new DOMException("Native still output is not JPEG","NotSupportedError");}
            if(blob.size>64*1024*1024){throw new DOMException("Encoded photo exceeds 64 MiB","NotSupportedError");}
            const bitmap=await createImageBitmap(blob);
            const width=bitmap.width,height=bitmap.height;bitmap.close();
            if(width>8192||height>8192){throw new DOMException("Photo dimensions exceed the capture bound","NotSupportedError");}
            const bytes=new Uint8Array(await blob.arrayBuffer());
            if(!this.active){throw new DOMException("Camera stopped during capture","AbortError");}
            return {bytes,length:bytes.length,width,height,timestamp:this.timestamp,origin};
        } finally {photoInFlight=false;}
    }
    close() {
        if(this.closed){return;}
        this.closed=true;this.events.abort();
        if(this.callback!==undefined){this.video.cancelVideoFrameCallback(this.callback);}
        this.video.pause();this.video.srcObject=null;
        this.stream.getTracks().forEach(t=>t.stop());
    }
}
