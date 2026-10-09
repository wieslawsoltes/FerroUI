// Rendering from a worker thread against rendering on one thread: runs the measurement scripts of this
// directory over the configurations of a browser site and prints the tables of
// docs/porting/browser-platform.md, "Rendering from a worker: measurements (B3)".
//
//   scripts/build-browser.sh themed_view --both && scripts/build-browser.sh control-catalog-browser --both
//   node scripts/browser/measure-render-thread.mjs [options]
//
//   --target <directory>    the directory of the built sites (default target): browser/<application>,
//                           browser-threads/<application> and browser-both/<application> below it
//   --runs <n>              runs of every measurement in every configuration (default 5)
//   --angles <a,b>          what WebGL runs on, each in turn (default swiftshader,metal; see harness.mjs)
//   --only <a,b>            the measurements to run (default all): first-frame, scroll, latency, animation,
//                           software, navigate, start, memory (the sizes of the modules are those of
//                           module-sizes.mjs)
//   --max-load <n>          wait until the load average of the last minute is below this before each run
//                           (default 8); after --load-wait seconds (default 600) the run is made anyway
//   --out <file>            write every result, with the load of the machine at each run, as JSON
//   --report <file>         measure nothing: print the tables of the results --out wrote earlier
//
// The configurations: the site built without threads; the site built with threads, drawn by its render
// thread; the same module kept on one thread (`?RenderThread=false`), which separates the cost of the
// threaded build from the gain of the render thread; and, for the first frame, the site with both modules,
// once served with the headers of cross-origin isolation and once without them (the loader registers the
// service worker and reloads the page once).
//
// The runs of the configurations alternate (run 1 of each, then run 2 of each), so that a change of the
// load of the machine falls on all of them, and each run is a fresh browser with a fresh profile. A value
// of a table is the median of the runs with the smallest and the largest in brackets. The load average of
// the machine is read before every run and printed with the tables: the figures of a busy machine are
// not worth much.
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { execFile } from "node:child_process";
import { sleep } from "./harness.mjs";

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, "..", "..");
const args = process.argv.slice(2);
const option = (name, fallback) => { const i = args.indexOf(name); return i >= 0 ? args[i + 1] : fallback; };
const target = path.resolve(option("--target", path.join(root, "target")));
const runs = Number(option("--runs", "5"));
const angles = option("--angles", "swiftshader,metal").split(",");
const only = option("--only")?.split(",");
const maxLoad = Number(option("--max-load", "8"));
const loadWait = Number(option("--load-wait", "600"));
const outFile = option("--out");
const reportFile = option("--report");
const stored = reportFile ? JSON.parse(fs.readFileSync(reportFile, "utf8")) : null;
const wanted = (name) => !only || only.includes(name);

const siteOf = (kind, application) => path.join(target, kind, application);
const CONFIGURATIONS = [
    { key: "single", name: "without threads", kind: "browser", isolated: false, query: "" },
    { key: "render-thread", name: "with threads, render thread", kind: "browser-threads", isolated: true, query: "" },
    { key: "one-thread", name: "with threads, `?RenderThread=false`", kind: "browser-threads", isolated: true, query: "RenderThread=false" }
];
const LOADER = [
    { key: "both-headers", name: "both modules, isolated by headers", kind: "browser-both", isolated: true, query: "" },
    { key: "both-service-worker", name: "both modules, service worker and reload", kind: "browser-both", isolated: false, query: "" }
];

const results = stored ?? { machine: `${os.cpus()[0].model}, ${os.cpus().length} cores, ${Math.round(os.totalmem() / 2 ** 30)} GB, ${os.type()} ${os.release()}`, started: new Date().toISOString(), runs, measurements: [] };
const save = () => { if (outFile) { fs.writeFileSync(outFile, JSON.stringify(results, null, 1)); } };

function script(name, parameters) {
    return new Promise((resolve) => {
        execFile(process.execPath, [path.join(here, name), ...parameters, "--json"], { maxBuffer: 256 * 1024 * 1024, timeout: 600_000 }, (error, stdout, stderr) => {
            if (error) { resolve({ failed: `${error.message}\n${stdout}\n${stderr}`.slice(0, 2000) }); return; }
            try { resolve(JSON.parse(stdout)); } catch { resolve({ failed: stdout.slice(0, 2000) }); }
        });
    });
}

// Waits until the machine is quiet enough and returns the load average of the last minute.
async function quiet() {
    const end = Date.now() + loadWait * 1000;
    while (os.loadavg()[0] >= maxLoad && Date.now() < end) { await sleep(5000); }
    return Number(os.loadavg()[0].toFixed(1));
}

// Runs `parameters(configuration)` of a script `runs` times for each configuration, alternating.
async function measure(title, name, configurations, parameters, angle) {
    if (stored) { return stored.measurements.find((m) => m.title === title && m.angle === angle) ?? null; }
    const measurement = { title, script: name, angle, configurations: configurations.map((c) => ({ key: c.key, name: c.name, runs: [] })) };
    results.measurements.push(measurement);
    for (let run = 0; run < runs; run++) {
        for (const [i, configuration] of configurations.entries()) {
            const load = await quiet();
            const result = await script(name, [...parameters(configuration), ...(angle ? ["--angle", angle] : [])]);
            measurement.configurations[i].runs.push({ load, result });
            process.stderr.write(`${title} [${angle ?? "-"}] ${configuration.name} run ${run + 1}: load ${load}${result.failed ? ` FAILED ${result.failed.split("\n")[0]}` : ""}\n`);
            save();
        }
    }
    return measurement;
}

const number = (value) => (value == null ? "-" : Number.isInteger(value) ? String(value) : Math.abs(value) >= 100 ? String(Math.round(value)) : Math.abs(value) >= 10 ? value.toFixed(1) : value.toFixed(2));
// The median of the runs with the smallest and the largest: "12.3 (11.0 to 14.1)".
function summary(runsOf, pick) {
    const values = runsOf.map((run) => (run.result.failed ? null : pick(run.result))).filter((value) => value != null && !Number.isNaN(value)).sort((a, b) => a - b);
    if (values.length === 0) { return "-"; }
    const median = values.length % 2 ? values[(values.length - 1) / 2] : (values[values.length / 2 - 1] + values[values.length / 2]) / 2;
    return `${number(median)} (${number(values[0])} to ${number(values[values.length - 1])})`;
}
function table(measurement, columns) {
    if (!measurement?.configurations) { return; }
    const runs = Math.max(...measurement.configurations.map((c) => c.runs.length));
    const loads = measurement.configurations.flatMap((c) => c.runs.map((run) => run.load));
    const failed = measurement.configurations.flatMap((c) => c.runs.filter((run) => run.result.failed)).length;
    console.log(`\n### ${measurement.title}${measurement.angle ? `, WebGL on ${measurement.angle}` : ""}\n`);
    console.log(`${runs} runs each, median (smallest to largest); load average of the machine before the runs ${Math.min(...loads)} to ${Math.max(...loads)}${failed ? `; ${failed} runs failed` : ""}.\n`);
    console.log(`| Configuration | ${columns.map((column) => column[0]).join(" | ")} |`);
    console.log(`|---|${columns.map(() => "---:").join("|")}|`);
    for (const configuration of measurement.configurations) {
        console.log(`| ${configuration.name} | ${columns.map((column) => summary(configuration.runs, column[1])).join(" | ")} |`);
    }
}

const queryOf = (configuration, first = "?") => (configuration.query ? `${first}${configuration.query}` : "");
const isolatedOf = (configuration) => (configuration.isolated ? ["--isolated"] : []);

for (const angle of angles) {
    if (wanted("first-frame")) {
        for (const application of ["themed_view", "control-catalog-browser"]) {
            const measurement = await measure(`First frame of ${application}`, "first-frame.mjs", [...CONFIGURATIONS, ...LOADER],
                (c) => [siteOf(c.kind, application), "--runs", "1", "--phases", "--trace", ...(c.query ? ["--query", queryOf(c)] : []), ...isolatedOf(c)], angle);
            const phase = (name) => (r) => r.loads[0].phases[name];
            // The first frame: of a view the page draws, the animation frame after its first draw call;
            // of a view a render thread draws, the first frame the module reports.
            if (!measurement) { continue; }
            table({ ...measurement, title: `${measurement.title}: phases, ms from the start of the first navigation` }, [
                ["page loads", (r) => r.pageLoads],
                ["start of the load that ran the module", (r) => r.loads[0].phases["navigation of the load that ran the module"] ?? 0],
                ["module downloaded", (r) => r.loads[0].wasm],
                ["module instantiated", phase("instantiated")],
                ...(application === "themed_view" ? [] : [["runtime created, threads started", phase("module instantiated")], ["`runMain` returned", phase("runMain end")]]),
                ["splash closed", (r) => r.loads[0].splash],
                ["**first frame**", (r) => (r.renderThread ? r.loads[0].rendered : r.loads[0].frame)],
                ["first frame reported by the module", (r) => r.loads[0].rendered]
            ]);
            table({ ...measurement, title: `${measurement.title}: the threads up to the first frame, ms` }, [
                ["long tasks of the page (50 ms and more), sum", (r) => r.loads[0].longTasks.sum],
                ["longest task of the page", (r) => r.loads[0].threads.mainLongestTaskMs],
                ["page thread, processor", (r) => r.loads[0].threads.mainCpuMs],
                ["workers, processor", (r) => r.loads[0].threads.workerCpuMs],
                ["longest task of a worker", (r) => r.loads[0].threads.workerLongestTaskMs],
                ["GPU process main thread, processor", (r) => r.loads[0].threads.gpuMainCpuMs]
            ]);
        }
    }
    const catalog = (c) => siteOf(c.kind, "control-catalog-browser");
    if (wanted("scroll")) {
        const measurement = await measure("Scrolling the TableView page: processor time per thread", "scroll-profile.mjs", CONFIGURATIONS,
            (c) => [catalog(c), "--trace", ...(c.query ? ["--query", queryOf(c, "&")] : []), ...isolatedOf(c)], angle);
        table(measurement, [
            ["page thread, ms per wheel event (processor)", (r) => r.mainCpuPerEventMs],
            ["page thread busy per event, median ms", (r) => r.mainBusyPerEvent.median],
            ["p95 ms", (r) => r.mainBusyPerEvent.p95],
            ["longest task ms", (r) => r.mainTasks.max],
            ["tasks over 16.7 ms", (r) => r.mainTasksOver16],
            ["page thread total ms", (r) => r.mainCpuMs],
            ["workers total ms", (r) => r.workerCpuMs],
            ["both ms", (r) => r.totalCpuMs],
            ["GPU process main thread ms", (r) => r.gpuMainCpuMs],
            ["frames drawn", (r) => r.frames]
        ]);
    }
    if (wanted("latency")) {
        const measurement = await measure("Scrolling the TableView page: from a wheel event to the frame that shows it", "scroll-profile.mjs", CONFIGURATIONS,
            (c) => [catalog(c), "--latency", ...(c.query ? ["--query", queryOf(c, "&")] : []), ...isolatedOf(c)], angle);
        table(measurement, [
            ["median ms", (r) => r.frameDelay.median], ["p95 ms", (r) => r.frameDelay.p95], ["max ms", (r) => r.frameDelay.max],
            ["frames not drawn in 5 s", (r) => r.framesNotDrawnIn5s], ["frames drawn", (r) => r.frames]
        ]);
    }
    const responsiveness = [
        ["frames drawn per second", (r) => r.framesDrawnPerSecond],
        ["page thread in tasks, % of the time", (r) => r.trace.mainBusyShare * 100],
        ["page thread task p99 ms", (r) => r.trace.mainTasks.p99],
        ["longest task ms", (r) => r.trace.mainTasks.max],
        ["tasks over 8 ms", (r) => r.trace.mainTasksOver8],
        ["tasks over 50 ms", (r) => r.trace.mainTasksOver50],
        ["4 ms timer: p99 gap ms", (r) => r.heartbeat.gaps.p99],
        ["longest gap ms", (r) => r.heartbeat.gaps.max],
        ["blocked ms", (r) => r.heartbeat.blockedMs],
        ["workers processor ms", (r) => r.trace.workerCpuMs]
    ];
    const gaps = [
        ["animation frames of the thread that renders", (r) => r.renderingFrames.gaps.count + 1],
        ["gap median ms", (r) => r.renderingFrames.gaps.median],
        ["p95 ms", (r) => r.renderingFrames.gaps.p95],
        ["p99 ms", (r) => r.renderingFrames.gaps.p99],
        ["longest ms", (r) => r.renderingFrames.gaps.max],
        ["gaps over 33 ms", (r) => r.renderingFrames.gapsOver33],
        ["over 100 ms", (r) => r.renderingFrames.gapsOver100],
        ["over 250 ms", (r) => r.renderingFrames.gapsOver250],
        ["overdue ms", (r) => r.renderingFrames.overdueMs],
        ["callback median ms", (r) => r.renderingFrames.callbacks.median]
    ];
    const frameTimes = (c, more) => [catalog(c), "--frame-rate-limit", "--trace", "--top", "0", ...more, ...(c.query ? ["--query", queryOf(c, "&")] : []), ...isolatedOf(c)];
    for (const [key, title, more] of [
        ["animation", "The Home page animating for 8 s (WebGL2)", ["--page", "Home", "--scenario", "idle", "--seconds", "8"]],
        ["software", "The Home page animating for 8 s (Software2D)", ["--page", "Home", "--scenario", "idle", "--seconds", "8", "--mode", "Software2D"]],
        ["navigate", "Page transitions for 10 s: Buttons and Slider in turn, one every 1.2 s (WebGL2)", ["--page", "Buttons", "--scenario", "navigate", "--other", "Slider", "--seconds", "10"]]
    ]) {
        if (!wanted(key)) { continue; }
        const measurement = await measure(title, "frame-times.mjs", CONFIGURATIONS, (c) => frameTimes(c, more), angle);
        table(measurement && { ...measurement, title: `${title}: the thread of the page` }, responsiveness);
        table(measurement && { ...measurement, title: `${title}: the animation frames of the thread that renders` }, gaps);
    }
    if (wanted("start")) {
        const measurement = await measure("The first 6 s of the catalog", "frame-times.mjs", CONFIGURATIONS,
            (c) => [catalog(c), "--scenario", "start", "--seconds", "6", "--top", "0", ...(c.query ? ["--query", queryOf(c, "&")] : []), ...isolatedOf(c)], angle);
        const started = (of) => [
            ["animation frames", (r) => of(r).animationFrames],
            ["gap median ms", (r) => of(r).gaps.median],
            ["gaps over 33 ms", (r) => of(r).gapsOver33],
            ["over 100 ms", (r) => of(r).gapsOver100],
            ["over 250 ms", (r) => of(r).gapsOver250],
            ["longest gap ms", (r) => of(r).gaps.max],
            ["gaps over 100 ms, sum ms", (r) => of(r).longMs],
            ["of it the thread was busy ms", (r) => of(r).longBusyMs],
            ["of it the thread waited ms", (r) => of(r).longWaitedMs],
            ["longest wait ms", (r) => of(r).longestWaitedMs],
            ["frames drawn", (r) => r.framesDrawn]
        ];
        table(measurement && { ...measurement, title: `${measurement.title}: the animation frames of the thread that renders` }, started((r) => r.renderingFrames));
        table(measurement && { ...measurement, title: `${measurement.title}: the animation frames of the thread of the page` }, started((r) => r.pageFrames));
    }
    if (wanted("memory")) {
        const measurement = await measure("Memory of the module after the tour of the catalog", "catalog-memory.mjs", CONFIGURATIONS,
            (c) => [catalog(c), ...(c.query ? ["--query", queryOf(c, "&")] : []), ...isolatedOf(c)], angle);
        table(measurement, [["at the start MB", (r) => r.startMb], ["at the end MB", (r) => r.endMb], ["peak MB", (r) => r.peakMb], ["size of the memory MB", (r) => r.sizeMb]]);
    }
}
if (!stored) { results.ended = new Date().toISOString(); save(); }
console.log(`\n${results.machine}; ${results.started} to ${results.ended}.`);
