// Reproducible build-tool measurements. These are repeat-JavaScript-validation and exact
// compression-report workloads, not Rust compilation or complete CI speedups.
//
//   node scripts/build-perf/browser.mjs [--output target/build-perf/browser.json]
//     [--samples 3] [--site path/to/an/existing/browser/site]
//
// Every full validation installs locked dependencies, type checks, lints and bundles. Its paired
// reuse verifies the successful receipt against all inputs and outputs. The compression workload
// defaults to the six actual emitted bundle files (source maps are excluded by module-sizes.mjs,
// just as in production); --site measures a real WebAssembly site instead.
import assert from "node:assert/strict";
import crypto from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { performance } from "node:perf_hooks";
import { fileURLToPath } from "node:url";

const repository = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const bundleFiles = ["ferroui.js", "ferroui.js.map", "storage.js", "storage.js.map", "ferroui-sw.js", "ferroui-sw.js.map"];
let output = path.join(repository, "target/build-perf/browser.json");
let samples = 3;
let requestedSite;
for (let i = 2; i < process.argv.length; i++) {
    const option = process.argv[i];
    const value = process.argv[++i];
    if (!value || value.startsWith("--")) throw new Error(`missing value for ${option}`);
    if (option === "--output") output = path.resolve(value);
    else if (option === "--samples") samples = Number(value);
    else if (option === "--site") requestedSite = path.resolve(value);
    else throw new Error(`unknown option: ${option}`);
}
if (!Number.isSafeInteger(samples) || samples < 3 || samples > 20) throw new Error("--samples must be an integer between 3 and 20");
// Capture before creating a possibly untracked results directory in the checkout.
const initialStatus = spawnSync("git", ["status", "--porcelain"], { cwd: repository, encoding: "utf8" });
const logs = path.join(path.dirname(output), `${path.parse(output).name}-logs`);
fs.mkdirSync(logs, { recursive: true });
const report = {
    schema_version: 1,
    status: "running",
    observed_at: new Date().toISOString(),
    provenance: { node: process.version, zlib: process.versions.zlib, brotli: process.versions.brotli, platform: process.platform, arch: process.arch, cpus: os.availableParallelism(), runner: process.env.RUNNER_NAME ?? null, github_sha: process.env.GITHUB_SHA ?? null },
    methodology: {
        samples,
        javascript: "Repeated pairs: full npm ci/typecheck/lint/bundle with receipt creation, followed by verified reuse of the exact validated inputs and six outputs. npm transfer time is included; the dependency download cache is not reset.",
        compression: "Repeated pairs: exact uncached report, separately recorded cache population, then reuse of that populated cache. Same file bytes, gzip9, Brotli11 and Node implementation; report stdout must be byte-identical.",
        limits: "These measure avoided repeated build-tool work, not Rust compilation, cold full builds, runtime rendering, or whole CI. Speedup is median baseline seconds divided by median reuse seconds.",
    },
    javascript_pairs: [],
    compression_pairs: [],
};

function digest(content) { return crypto.createHash("sha256").update(content).digest("hex"); }
function fileIdentity(file, label) {
    const content = fs.readFileSync(file);
    return { path: label, bytes: content.length, sha256: digest(content) };
}
function bundleIdentity() {
    const dist = path.join(repository, "src/Browser/FerroUI.Browser/dist");
    return bundleFiles.map(name => fileIdentity(path.join(dist, name), name));
}
function run(label, executable, args) {
    const stdoutFile = path.join(logs, `${label}.stdout`);
    const stderrFile = path.join(logs, `${label}.stderr`);
    const stdout = fs.openSync(stdoutFile, "w");
    const stderr = fs.openSync(stderrFile, "w");
    const start = performance.now();
    let result;
    try { result = spawnSync(executable, args, { cwd: repository, stdio: ["ignore", stdout, stderr] }); }
    finally { fs.closeSync(stdout); fs.closeSync(stderr); }
    const seconds = (performance.now() - start) / 1000;
    if (result.error) throw result.error;
    if (result.status !== 0) throw new Error(`${label} failed (${result.signal ?? result.status}); see ${stderrFile}`);
    return { seconds, command: [executable, ...args], stdout_file: path.relative(path.dirname(output), stdoutFile), stderr_file: path.relative(path.dirname(output), stderrFile), stdout_sha256: digest(fs.readFileSync(stdoutFile)) };
}
function save() { fs.writeFileSync(output, `${JSON.stringify(report, null, 2)}\n`); }
function median(values) { const sorted = values.toSorted((left, right) => left - right); const index = Math.floor(sorted.length / 2); return sorted.length % 2 ? sorted[index] : (sorted[index - 1] + sorted[index]) / 2; }
function summary(pairs) {
    const baseline = median(pairs.map(pair => pair.full.seconds));
    const reused = median(pairs.map(pair => pair.reused.seconds));
    return { median_full_seconds: baseline, median_reused_seconds: reused, ratio_of_medians: baseline / reused, individual_ratios: pairs.map(pair => pair.full.seconds / pair.reused.seconds) };
}

try {
    const commit = run("commit", "git", ["rev-parse", "HEAD"]);
    report.provenance.checkout_sha = fs.readFileSync(path.join(path.dirname(output), commit.stdout_file), "utf8").trim();
    if (initialStatus.error) throw initialStatus.error;
    if (initialStatus.status !== 0) throw new Error(`git status failed: ${initialStatus.stderr}`);
    fs.writeFileSync(path.join(logs, "worktree-status.stdout"), initialStatus.stdout);
    fs.writeFileSync(path.join(logs, "worktree-status.stderr"), initialStatus.stderr);
    report.provenance.worktree_dirty = initialStatus.stdout.trim().length > 0;
    report.measurement_sources = ["scripts/build-perf/browser.mjs", "scripts/browser/build-webapp.mjs", "scripts/browser/module-sizes.mjs", "scripts/browser/compression-cache.mjs"].map(name => fileIdentity(path.join(repository, name), name));
    let firstBundle;
    for (let index = 0; index < samples; index++) {
        const full = run(`javascript-${index}-full`, process.execPath, ["scripts/browser/build-webapp.mjs", "build"]);
        const built = bundleIdentity();
        if (firstBundle) assert.deepEqual(built, firstBundle, "full validations emitted different bundle bytes");
        else firstBundle = built;
        const reused = run(`javascript-${index}-reused`, process.execPath, ["scripts/browser/build-webapp.mjs", "verify"]);
        assert.deepEqual(bundleIdentity(), built, "receipt verification changed emitted files");
        report.javascript_pairs.push({ index, full, reused, identical_output: true });
        save();
        console.log(`JavaScript pair ${index + 1}/${samples}: full ${full.seconds.toFixed(3)}s, verified reuse ${reused.seconds.toFixed(3)}s`);
    }
    report.bundle_files = firstBundle;

    let site = requestedSite;
    if (!site) {
        site = fs.mkdtempSync(path.join(logs, "bundle-site-"));
        for (const name of bundleFiles) fs.copyFileSync(path.join(repository, "src/Browser/FerroUI.Browser/dist", name), path.join(site, name));
    }
    const siteIdentity = () => fs.readdirSync(site, { recursive: true }).map(name => name.split(path.sep).join("/"))
        .filter(name => fs.statSync(path.join(site, name)).isFile() && !name.endsWith(".map")).sort()
        .map(name => fileIdentity(path.join(site, name), name));
    report.compression_inputs = siteIdentity();
    report.compression_input_kind = requestedSite ? "explicit site" : "actual emitted JavaScript bundles";
    for (let index = 0; index < samples; index++) {
        const cache = fs.mkdtempSync(path.join(logs, `compression-${index}-`));
        const common = ["scripts/browser/module-sizes.mjs", site];
        const full = run(`compression-${index}-full`, process.execPath, [...common, "--no-cache"]);
        const populate = run(`compression-${index}-populate`, process.execPath, [...common, "--cache-dir", cache]);
        const reused = run(`compression-${index}-reused`, process.execPath, [...common, "--cache-dir", cache]);
        assert.equal(populate.stdout_sha256, full.stdout_sha256, "cache population changed report bytes");
        assert.equal(reused.stdout_sha256, full.stdout_sha256, "cache reuse changed report bytes");
        assert.deepEqual(siteIdentity(), report.compression_inputs, "site files changed during measurement");
        report.compression_pairs.push({ index, full, populate, reused, identical_report: true });
        save();
        console.log(`Compression pair ${index + 1}/${samples}: full ${full.seconds.toFixed(3)}s, exact reuse ${reused.seconds.toFixed(3)}s`);
    }
    report.summary = { javascript: summary(report.javascript_pairs), compression: summary(report.compression_pairs) };
    report.status = "passed";
    save();
    const rows = Object.entries(report.summary).map(([name, value]) => `| ${name} | ${value.median_full_seconds.toFixed(3)} | ${value.median_reused_seconds.toFixed(3)} | ${value.ratio_of_medians.toFixed(2)}x |`);
    const markdown = ["## Browser build-tool measurements", "", "Same input/output bytes; repeated full/reuse pairs. These ratios apply to repeated JavaScript validation and compression reporting, not Rust compilation or whole CI.", "", "| Workload | Full median (s) | Reuse median (s) | Ratio |", "|---|---:|---:|---:|", ...rows, "", `All ${samples} JavaScript pairs preserved the six emitted files; all ${samples} compression pairs preserved the complete report bytes.`, ""].join("\n");
    fs.writeFileSync(`${output}.md`, markdown);
    if (process.env.GITHUB_STEP_SUMMARY) fs.appendFileSync(process.env.GITHUB_STEP_SUMMARY, markdown);
    console.log(markdown);
} catch (error) {
    report.status = "failed";
    report.error = error.message;
    save();
    console.error(error.stack);
    process.exitCode = 1;
}
