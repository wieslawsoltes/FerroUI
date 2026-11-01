// One writer and one reader per direction. A bounded slot streams large JSON
// frames in chunks. Publishing the state after the copy also publishes its bytes.
(buffer => {
    const words = new Int32Array(buffer, 0, 2);
    const data = new Uint8Array(buffer, 8);
    const encoder = new TextEncoder(), decoder = new TextDecoder("utf-8", {fatal:true});
    const limit = 32 * 1024 * 1024;
    function waitFor(empty, until) {
        for (;;) {
            const state = Atomics.load(words, 0);
            if ((state === 0) === empty) return state;
            const left = until - performance.now();
            if (left <= 0 || Atomics.wait(words, 0, state, left) === "timed-out")
                throw new Error("QML browser worker deadline exceeded");
        }
    }
    function publish(state) {
        Atomics.store(words, 0, state);
        Atomics.notify(words, 0, 1);
    }
    return {
        write(text, until) {
            const bytes = encoder.encode(text);
            if (bytes.length > limit) throw new Error("QML browser frame exceeds 32 MiB");
            let offset = 0;
            do {
                waitFor(true, until);
                const count = Math.min(data.length, bytes.length - offset);
                data.set(bytes.subarray(offset, offset + count));
                Atomics.store(words, 1, count);
                offset += count;
                publish(offset === bytes.length ? 2 : 1);
            } while (offset < bytes.length);
        },
        read(until) {
            let total = 0;
            const chunks = [];
            for (;;) {
                const state = waitFor(false, until), length = Atomics.load(words, 1);
                if ((state !== 1 && state !== 2) || length < 0 || length > data.length || (total += length) > limit)
                    throw new Error("Invalid QML browser frame");
                const chunk = data.slice(0, length);
                publish(0);
                if (state === 2 && !chunks.length) return decoder.decode(chunk);
                chunks.push(chunk);
                if (state === 2) break;
            }
            const result = new Uint8Array(total);
            let at = 0;
            for (const chunk of chunks) { result.set(chunk, at); at += chunk.length; }
            return decoder.decode(result);
        }
    };
})
