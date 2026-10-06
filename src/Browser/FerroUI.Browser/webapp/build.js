require("esbuild").build({
    entryPoints: [
        "./modules/ferroui.ts",
        "./modules/storage.ts",
        "./modules/ferroui-sw.ts"
    ],
    outdir: "../dist",
    bundle: true,
    minify: true,
    format: "esm",
    target: "es2020",
    platform: "browser",
    sourcemap: "linked",
    loader: { ".ts": "ts" }
})
    .then(() => console.log("Done"))
    .catch(() => process.exit(1));
