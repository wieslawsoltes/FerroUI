import init, {application_model, qmlApplicationModule, probe, probe_fetch} from "./pkg/qml_browser_tests.js";
if (new URL(self.location.href).searchParams.has("application")) {
    await init();
    const {serveQmlApplication} = await import(qmlApplicationModule());
    serveQmlApplication(async () => {
        if (new URL(self.location.href).searchParams.has("fail")) throw new Error("fixture initialization failed");
        const model = await application_model();
        const data = JSON.parse(model.dispatch("get","","rows",""));
        if (data.t !== "Array" || data.v[0].t !== "Object")
            throw new Error("ListModel snapshot exposed a worker identity");
        return model;
    });
} else {
    try {
        await init();
        await probe_fetch(new URL("./Fetched.qml", self.location.href).href);
        postMessage(await probe());
    } catch (e) {
        postMessage(String(e?.stack || e));
    }
}
