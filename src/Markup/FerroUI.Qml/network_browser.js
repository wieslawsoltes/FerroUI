// Read the response as a bounded stream, including replies without Content-Length.
export async function fetchComponent(address, limit) {
    const url = new URL(address);
    if (url.origin !== self.location.origin || !["https:", "http:"].includes(url.protocol) ||
        url.username || url.password) {
        throw "QML browser transport requires a same-origin HTTP(S) URL";
    }
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 15000);
    try {
        const response = await fetch(url, {signal: controller.signal, redirect: "error", mode: "same-origin"});
        if (response.status === 404) return undefined;
        if (!response.ok) throw new Error("HTTP " + response.status);
        if (Number(response.headers.get("content-length")) > limit) throw new Error("component exceeds size limit");
        if (!response.body) return new Uint8Array();
        const reader = response.body.getReader();
        const chunks = [];
        let size = 0;
        for (;;) {
            const {done, value} = await reader.read();
            if (done) break;
            size += value.byteLength;
            if (size > limit) throw new Error("component exceeds size limit");
            chunks.push(value);
        }
        const bytes = new Uint8Array(size);
        let at = 0;
        for (const chunk of chunks) { bytes.set(chunk, at); at += chunk.byteLength; }
        return bytes;
    } catch (error) {
        throw String(error);
    } finally {
        clearTimeout(timeout);
        controller.abort();
    }
}
