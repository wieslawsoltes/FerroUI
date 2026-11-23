// The web engine of the browser: an iframe.
//
// The frame is sandboxed without `allow-same-origin`, so a page in it can never reach the
// application, also after navigating itself to the origin of the application. A document given
// as text carries the bridge; a page from an address has none, so scripts and messages are
// refused for it, and the browser does not report its navigations inside the frame.

const SANDBOX = "allow-scripts allow-forms allow-popups";
const PROBE_MILLISECONDS = 1000;

export class WebFrame {
    constructor(bridgeScript, onLoad, onMessage, onResult, onTitle) {
        this.bridgeScript = bridgeScript;
        this.onResult = onResult;
        this.hasBridge = false;
        this.pending = new Set();
        this.probe = 0;
        this.isProbeAnswered = false;
        this.frame = document.createElement("iframe");
        this.frame.setAttribute("sandbox", SANDBOX);
        this.frame.style.border = "0";
        this.frame.style.width = "100%";
        this.frame.style.height = "100%";

        this.loadListener = () => {
            this.probeDocument();
            onLoad();
        };
        this.messageListener = (event) => {
            if (event.source !== this.frame.contentWindow || !event.data || event.data.ferrouiWebView !== true) {
                return;
            }
            const data = event.data;
            if (data.type === "ready") {
                this.hasBridge = true;
                onTitle(String(data.title));
            } else if (data.type === "pong" && data.probe === this.probe) {
                this.isProbeAnswered = true;
                this.hasBridge = true;
            } else if (data.type === "message") {
                onMessage(String(data.text));
            } else if (data.type === "result" && this.pending.delete(data.id)) {
                onResult(data.id, String(data.text));
            } else if (data.type === "title") {
                onTitle(String(data.title));
            }
        };
        this.frame.addEventListener("load", this.loadListener);
        window.addEventListener("message", this.messageListener);
    }

    get element() {
        return this.frame;
    }

    failPending(message) {
        const pending = [...this.pending];
        this.pending.clear();
        for (const id of pending) {
            this.onResult(id, "e" + message);
        }
    }

    // The page can navigate itself to a document without a bridge, which no event reports:
    // after each load the bridge has to answer, or what waits for it is told.
    probeDocument() {
        const probe = ++this.probe;
        this.isProbeAnswered = false;
        if (this.frame.contentWindow) {
            this.frame.contentWindow.postMessage({ ferrouiWebView: true, type: "ping", probe }, "*");
        }
        clearTimeout(this.probeTimer);
        this.probeTimer = setTimeout(() => {
            if (probe === this.probe && !this.isProbeAnswered) {
                this.hasBridge = false;
                this.failPending("the page has no script bridge");
            }
        }, PROBE_MILLISECONDS);
    }

    beginDocument() {
        this.probe++;
        clearTimeout(this.probeTimer);
        this.hasBridge = false;
        this.failPending("the document was replaced");
    }

    navigate(uri) {
        this.beginDocument();
        this.frame.removeAttribute("srcdoc");
        this.frame.src = uri;
    }

    navigateToString(html) {
        this.beginDocument();
        this.frame.removeAttribute("src");
        this.frame.srcdoc = html;
    }

    reload() {
        this.beginDocument();
        if (this.frame.hasAttribute("srcdoc")) {
            this.frame.srcdoc = this.frame.getAttribute("srcdoc");
        } else if (this.frame.hasAttribute("src")) {
            this.frame.src = this.frame.getAttribute("src");
        }
    }

    executeScript(id, script) {
        if (!this.hasBridge) {
            this.onResult(id, "ethe page has no script bridge");
            return;
        }
        this.pending.add(id);
        this.frame.contentWindow.postMessage({ ferrouiWebView: true, type: "script", id, script }, "*");
    }

    postMessage(json) {
        if (this.hasBridge) {
            this.frame.contentWindow.postMessage({ ferrouiWebView: true, type: "deliver", json }, "*");
        }
    }

    close() {
        this.probe++;
        clearTimeout(this.probeTimer);
        this.frame.removeEventListener("load", this.loadListener);
        window.removeEventListener("message", this.messageListener);
        this.failPending("the web view was closed");
        this.frame.remove();
    }
}

// The part of the bridge that talks to the application through `postMessage`. It is added to
// the script of `document_script`, which defines `window.chrome.webview`.
export function frameBridge() {
    return "(function(){" +
        "var send=function(m){m.ferrouiWebView=true;parent.postMessage(m,'*');};" +
        "window.addEventListener('message',function(e){" +
        "if(e.source!==parent||!e.data||e.data.ferrouiWebView!==true)return;" +
        "if(e.data.type==='ping'){send({type:'pong',probe:e.data.probe});}" +
        "else if(e.data.type==='deliver'){window.chrome.webview.__ferrouiDeliver(e.data.json);}" +
        "else if(e.data.type==='script'){var text;" +
        "try{text=(0,eval)(e.data.script);}catch(error){text='e'+String(error);}" +
        "send({type:'result',id:e.data.id,text:text});}" +
        "});" +
        "var title=function(){send({type:'title',title:document.title});};" +
        "document.addEventListener('DOMContentLoaded',function(){" +
        "title();var t=document.querySelector('title');" +
        "if(t)new MutationObserver(title).observe(t,{childList:true,characterData:true,subtree:true});" +
        "});" +
        "send({type:'ready',title:document.title});" +
        "})();";
}
