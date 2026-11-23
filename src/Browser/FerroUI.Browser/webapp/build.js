require("esbuild").build({
    entryPoints: [
        "./modules/ferroui.ts",
        "./modules/storage.ts",
        "./modules/ferroui-sw.ts",
        "./modules/qr-worker.ts"
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
    .then(() => {
        require("node:fs").copyFileSync("node_modules/jsqr/LICENSE", "../dist/qr-worker.LICENSE.txt");
        console.log("Done");
    })
    .catch(() => process.exit(1));
