import init, {application_model, subscription_model, qmlApplicationModule, probe, probe_fetch} from "./pkg/qml_browser_tests.js";
if (new URL(self.location.href).searchParams.has("application")) {
    await init();
    const {serveQmlApplication} = await import(qmlApplicationModule());
    serveQmlApplication(async () => {
        if (new URL(self.location.href).searchParams.has("fail")) throw new Error("fixture initialization failed");
        const model = await application_model();
        if (new URL(self.location.href).searchParams.has("trap")) {
            return {
                pump() { throw new WebAssembly.RuntimeError("fixture Wasm trap"); },
                free() { model.free(); throw new Error("fixture cleanup error"); }
            };
        }
        const data = JSON.parse(model.dispatch("get","","rows",""));
        if (data.t !== "Array" || data.v[0].t !== "Object")
            throw new Error("ListModel snapshot exposed a worker identity");
        return model;
    });
} else {
    try {
        await init();
        if (new URL(self.location.href).searchParams.has("benchmark-subscriptions")) {
            postMessage(JSON.stringify(benchmarkSubscriptions()));
        } else {
            checkSubscriptions();
            await probe_fetch(new URL("./Fetched.qml", self.location.href).href);
            postMessage(await probe());
        }
    } catch (e) {
        postMessage(String(e?.stack || e));
    }
}

function checkSubscriptions() {
    const m = subscription_model();
    try {
        m.watch(8, "", "rows");
        m.watch(9, "", "record");
        m.changes();
        const arg = n => JSON.stringify([{t:"Number",v:String(n)}]);
        for (let i = 1; i <= 10; ++i) {
            m.dispatch("call","","editOther",arg(i));
            if (m.changes() !== "[]") throw Error("unrelated model published a snapshot");
        }
        m.dispatch("call","","editChild",arg(7));
        const [child] = JSON.parse(m.changes());
        if (child?.tokens[0] !== 8 || child.update.value.v[0].v.children.v[0].v.n.v !== "7")
            throw Error("nested model subscription");
        m.dispatch("call","","editRecord",arg(12));
        const [record] = JSON.parse(m.changes());
        if (record?.tokens[0] !== 9 || record.update.value.v.n.v !== "12") throw Error("record subscription");
        try { m.dispatch("call","","failRecord","[]"); } catch {}
        if (m.changes() !== "[]") throw Error("rolled-back record change escaped");
    } finally { m.free(); }
}

// Timed inside the dedicated worker, excluding Wasm startup, model creation and
// command dispatch. Each sample measures only draining the committed changes.
function benchmarkSubscriptions() {
    const m = subscription_model();
    try {
        m.watch(1,"","rows");
        m.changes();
        function median(method) {
            const samples = [];
            let n = 1000;
            for (let sample = 0; sample < 10; ++sample) {
                let elapsed = 0;
                for (let i = 0; i < 20; ++i) {
                    m.dispatch("call","",method,JSON.stringify([{t:"Number",v:String(++n)}]));
                    const start = performance.now();
                    const packet = m.changes();
                    elapsed += performance.now() - start;
                    if ((packet === "[]") !== (method === "editOther")) throw Error("unexpected change packet");
                }
                if (sample) samples.push(elapsed / 20);
            }
            return samples.sort((a,b)=>a-b)[4];
        }
        return {rows:256, unrelatedEditMs:median("editOther"), nestedEditMs:median("editChild")};
    } finally { m.free(); }
}
