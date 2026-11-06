// Browser-owned decode/audio/timing. The video is never used as a DOM overlay.
// Frames reach FerroUI through the active WebGL context's TexImageSource path.
export class BrowserVideo {
    constructor(url, ownedUrl) {
        if(!HTMLVideoElement.prototype.requestVideoFrameCallback){throw new DOMException("Accurate native video-frame callbacks are unavailable","NotSupportedError");}
        this.video = document.createElement("video");
        this.video.crossOrigin = "anonymous";
        this.video.playsInline = true;
        this.video.preload = "auto";
        this.video.autoplay = false;
        this.video.loop = false;
        this.video.volume = 1;
        this.video.muted = false;
        this.ownedUrl = ownedUrl;
        this.closed = false;
        this.waiting = false;
        this.sequence = 0;
        this.frameValid = false;
        this.timestamp = 0;
        this.width = 0;
        this.height = 0;
        this.callback = undefined;
        this.seekRequest = undefined;
        this.events = new AbortController();
        const signal = this.events.signal;
        this.video.addEventListener("waiting", () => { this.waiting = true; }, {signal});
        this.video.addEventListener("playing", () => { this.waiting = false; }, {signal});
        this.video.addEventListener("canplay", () => { this.waiting = false; }, {signal});
        this.ready = new Promise((resolve,reject) => {
            this.rejectReady = reject;
            this.video.addEventListener("loadeddata", resolve, {once:true,signal});
            this.video.addEventListener("error", () => reject(new Error(this.error)), {once:true,signal});
        });
        // A canceled source can be retired before Rust registers its promise.
        // Keep a rejection handler without altering the promise Rust observes.
        this.ready.catch(() => {});
        const next = (_now,metadata) => {
            if (this.closed) { return; }
            this.sequence++;
            this.frameValid = true;
            this.timestamp = metadata.mediaTime;
            this.width = metadata.width;
            this.height = metadata.height;
            this.callback = this.video.requestVideoFrameCallback(next);
        };
        if (this.video.requestVideoFrameCallback) {
            this.callback = this.video.requestVideoFrameCallback(next);
        }
        this.video.src = url;
        this.video.load();
    }
    get error() { return this.video.error?.message || "Native browser playback failed"; }
    get state() {
        if (this.closed || this.video.error) { return 5; }
        if (this.video.readyState < 2) { return 0; }
        if (this.video.ended) { return 4; }
        if (this.video.paused) { return 1; }
        return this.waiting ? 3 : 2;
    }
    get position() { return this.video.currentTime; }
    get duration() { return this.video.duration; }
    get naturalWidth() { return this.video.videoWidth; }
    get naturalHeight() { return this.video.videoHeight; }
    get seekable() {
        for(let i=0;i<this.video.seekable.length;i++) {
            if(this.video.seekable.end(i)>this.video.seekable.start(i)){return true;}
        }
        return false;
    }
    prepare() { return this.ready; }
    play() {
        if (this.closed) { return Promise.reject(new DOMException("Playback is closed","AbortError")); }
        return this.video.play();
    }
    pause() { this.video.pause(); }
    audio(volume,muted) { this.video.volume=volume; this.video.muted=muted; }
    seek(position) {
        this.seekRequest?.close();
        this.frameValid=false;
        const request = new BrowserSeek(this.video,position,() => this.closed);
        this.seekRequest = request;
        return request;
    }
    sample() {
        if (this.closed || this.video.readyState < 2 || this.video.seeking) { return null; }
        if (!this.frameValid || !this.sequence || !this.width || !this.height) { return null; }
        return {image:this.video,sequence:this.sequence,timestamp:this.timestamp,width:this.width,height:this.height};
    }
    close() {
        if (this.closed) { return; }
        this.closed = true;
        this.seekRequest?.close();this.seekRequest=undefined;
        this.rejectReady?.(new DOMException("Playback was canceled","AbortError"));this.rejectReady=undefined;
        this.events.abort();
        if (this.callback!==undefined) { this.video.cancelVideoFrameCallback(this.callback); }
        this.video.pause();this.video.removeAttribute("src");this.video.load();
        if (this.ownedUrl) { URL.revokeObjectURL(this.ownedUrl);this.ownedUrl=undefined; }
    }
}
export class BrowserSeek {
    constructor(video,position,closed) {
        this.done=false;
        this.events=new AbortController();
        this.promise=new Promise((resolve,reject) => {
            this.reject=reject;
            const finish=() => {
                if (this.done || video.seeking) {return;}
                if (closed()) {this.close();return;}
                this.done=true;this.events.abort();this.reject=undefined;resolve(video.currentTime);
            };
            video.addEventListener("seeked",finish,{signal:this.events.signal});
            video.addEventListener("error",() => {this.close();},{signal:this.events.signal});
            try {
                if (closed()) {throw new DOMException("Playback is closed","AbortError");}
                video.currentTime=position;
                // Assigning the current position may not dispatch seeked.
                queueMicrotask(finish);
            } catch (error) {this.done=true;this.events.abort();reject(error);}
        });
        this.promise.catch(() => {});
    }
    close() {
        if (this.done) {return;}
        this.done=true;this.events.abort();
        this.reject?.(new DOMException("Seek was canceled","AbortError"));this.reject=undefined;
    }
}
export function openUrl(text) {
    const url=new URL(text,document.baseURI);
    if (!["http:","https:","blob:"].includes(url.protocol)) {throw new TypeError("Unsupported browser media URL scheme");}
    return new BrowserVideo(url.href,undefined);
}
export function openResource(encoded,contentType) {
    const url=URL.createObjectURL(new Blob([encoded],{type:contentType}));
    try {return new BrowserVideo(url,url);} catch(error) {URL.revokeObjectURL(url);throw error;}
}
