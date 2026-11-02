#!/usr/bin/env python3
"""Replay native Metal media acceptance; retain every failure outside Git."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import plistlib
import re
import shutil
import subprocess
import sys
import tempfile
import time

REPO = Path(__file__).resolve().parents[2]


def digest(path):
    h = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            h.update(block)
    return h.hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, help="New evidence directory, outside the repository")
    parser.add_argument("--target-dir", type=Path, default=Path(os.environ.get("CARGO_TARGET_DIR", REPO / "target")))
    parser.add_argument("--cycles", type=int, default=100)
    parser.add_argument("--skip-contracts", action="store_true")
    args = parser.parse_args()
    if sys.platform != "darwin" or not 12 <= args.cycles <= 1000:
        parser.error("Requires macOS; cycles must be 12..1000")
    out = args.output.resolve() if args.output else Path(tempfile.mkdtemp(prefix="ferroui-macos-verification-"))
    if out == REPO or REPO in out.parents:
        parser.error("Evidence must be outside the repository")
    if args.output:
        out.mkdir(parents=True, exist_ok=False)
    target = args.target_dir.resolve()
    env = dict(os.environ, CARGO_TARGET_DIR=str(target), CARGO_PROFILE_DEV_DEBUG="0", CARGO_PROFILE_TEST_DEBUG="0", CARGO_INCREMENTAL="0")
    if env.get("SKIA_LIBRARY_SEARCH_PATH"):
        parser.error("Native acceptance must build the patched source, not import a caller-managed Skia archive")
    report = {"revision": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=REPO, text=True).strip(),
              "os": platform.platform(), "architecture": platform.machine(),
              "hardware": subprocess.check_output(["sysctl", "-n", "machdep.cpu.brand_string"], text=True).strip(),
              "scope": "Metal/Graphite SDR; synthetic AVC/AAC 160x90; 220x220 projection; own-window captures; 10 warmup cycles; two resets/cycle",
              "limits": {"footprint_growth_bytes": 64 * 1024**2, "metal_growth_bytes": 32 * 1024**2, "port_growth": 64},
              "not_accepted": ["physical camera/privacy/hotplug/photo", "physical audio and AV sync", "VoiceOver end-to-end", "older OS runtime", "Intel runtime", "Developer ID/notarization", "full performance workload"],
              "steps": []}
    report["skia"] = {"bindings": "workspace-patched 0.153.3", "source_override": env.get("SKIA_SOURCE_DIR"), "ninja_override": env.get("SKIA_NINJA_COMMAND")}
    if env.get("SKIA_SOURCE_DIR"):
        source = Path(env["SKIA_SOURCE_DIR"]) / "src/gpu/graphite/mtl/MtlGraphicsPipeline.mm"
        if source.is_file():
            report["skia"]["pipeline_source_sha256"] = digest(source)
    files = subprocess.check_output(["git", "ls-files", "--modified", "--others", "--exclude-standard"], cwd=REPO, text=True).splitlines()
    report["changed_source_sha256"] = {name: digest(REPO / name) for name in files if (REPO / name).is_file()}

    def save():
        (out / "report.json").write_text(json.dumps(report, indent=2) + "\n")
        rows = ["# macOS media verification", "", f"Revision: `{report['revision']}`", "", "| Stage | Outcome | Seconds |", "| --- | --- | --- |"]
        rows += [f"| {s['name']} | {'PASS' if s['passed'] else 'FAIL'}{' (expected negative)' if s['negative'] else ''} | {s['seconds']} |" for s in report["steps"]]
        rows += ["", "Open acceptance: " + ", ".join(report["not_accepted"]), ""]
        (out / "report.md").write_text("\n".join(rows))

    def run(name, command, required=(), negative=False, timeout=240, extra_env=None):
        log = out / f"{name}.log"
        print(f"START {name}", flush=True)
        start = time.monotonic()
        code = 124
        with log.open("w") as stream:
            try:
                result = subprocess.run([str(x) for x in command], cwd=REPO, env=env | (extra_env or {}), stdout=stream, stderr=subprocess.STDOUT, timeout=timeout)
                code = result.returncode
            except subprocess.TimeoutExpired:
                stream.write("\nVERIFICATION_TIMEOUT\n")
        text = log.read_text(errors="replace")
        missing = [marker for marker in required if marker not in text]
        passed = (code != 0 and code != 124 if negative else code == 0) and not missing
        report["steps"].append({"name": name, "command": [str(x) for x in command], "exit_code": code, "negative": negative, "passed": passed, "missing_markers": missing, "seconds": round(time.monotonic() - start, 3), "log": str(log)})
        save()
        print(f"{'PASS' if passed else 'FAIL'} {name} ({code})", flush=True)
        if not passed:
            print("\n".join(text.splitlines()[-35:]), flush=True)
            raise RuntimeError(f"{name} failed; retained evidence: {log}")
        return text

    cargo = ["cargo"]
    try:
        run("audit-build", cargo + ["build", "--locked", "-j", "2", "-p", "portable-media", "--features", "macos-verification"], timeout=900)
        binary = out / "portable-media-audit"
        shutil.copy2(target / "debug/portable-media", binary)
        report["audit_sha256"] = digest(binary)
        save()
        fixtures = out / "geometry-fixtures"
        run("geometry-fixtures", ["node", "tests/media/make-windows-geometry-fixtures.mjs", fixtures])
        for crop in [False, True]:
            for orientation in range(8):
                name = f"pixels-{orientation}{'-crop' if crop else ''}"
                image_dir = out / name
                image_dir.mkdir()
                source = fixtures / f"orientation-{orientation}{'-crop' if crop else ''}.mp4"
                command = [binary, source, "--resource", "--renderer", "--video-pixel-smoke", "--video-lifecycle-pixels", f"--orientation={orientation}"]
                if crop:
                    command.append("--crop-smoke")
                run(name, command, ["VIDEO_PIXEL_SMOKE_PASS", "VIDEO_LIFECYCLE_PIXELS_PASS phase=seek", "VIDEO_LIFECYCLE_PIXELS_PASS phase=recovered", "VIDEO_PRESENTATION_READY phase=recovered", "retained=0"], extra_env={"FERROUI_MEDIA_EVIDENCE": str(image_dir)})
        run("wrong-orientation-negative", [binary, fixtures / "orientation-1.mp4", "--resource", "--renderer", "--video-pixel-smoke", "--orientation=0"], ["VIDEO_PIXEL_FAILED"], negative=True)
        source = REPO / "tests/media/fixtures/avc-aac.mp4"
        run("transport", [binary, source, "--resource", "--macos-transport-smoke"], ["MACOS_TRANSPORT_AX_PASS", "MACOS_TRANSPORT_KEYBOARD_PASS"])
        run("native-memory-soak", [binary, source, "--resource", "--renderer", f"--macos-lifecycle-cycles={args.cycles}", "--quiet-diagnostics"], [f"MACOS_LIFECYCLE_PASS cycles={args.cycles}", "native_owners=0"], timeout=max(240, args.cycles * 5))
        run("retained-memory-negative", [binary, source, "--resource", "--renderer", "--macos-lifecycle-cycles=20", "--macos-leak-control", "--quiet-diagnostics"], ["Native memory growth exceeds frozen limit"], negative=True)
        if not args.skip_contracts:
            run("media-contracts", cargo + ["test", "--locked", "-j", "2", "--lib", "-p", "ferroui-media", "-p", "ferroui-media-video", "-p", "ferroui-media-camera", "-p", "ferroui-media-qr", "-p", "ferroui-media-controls", "-p", "ferroui-media-video-controls", "-p", "ferroui-media-camera-controls"], ["test result: ok."], timeout=900)
            for name, command in [
                ("native-metal", ["-p", "ferroui-skia", "--features", "native-frame-tests", "native_shared_metal_frame"]),
                ("camera-lifetime", ["-p", "ferroui-media-camera-apple", "--features", "native-lifetime-tests"]),
                ("native-vision", ["-p", "ferroui-media-qr-apple", "--features", "test-fixtures"]),
            ]:
                run(name, cargo + ["test", "--locked", "-j", "2", "--lib", *command], ["test result: ok. 1 passed"], timeout=900)
        run("default-build", cargo + ["build", "--locked", "-j", "2", "-p", "portable-media"], timeout=900)
        default = target / "debug/portable-media"
        run("default-recovery", [default, source, "--resource", "--smoke", "--renderer", "--device-recovery-smoke"], ["DEVICE_RECOVERY_PASS", "RENDERER_PIXELS_PASS", "NATIVE_MEDIA_SMOKE_PASS"])
        run("default-ended-restart", [default, source, "--resource", "--smoke", "--renderer", "--ended-restart-smoke"], ["ENDED_RESTART_COMPLETED", "NATIVE_MEDIA_SMOKE_PASS"])
        bundle = out / "FerroUI Portable Media.app"
        macos = bundle / "Contents/MacOS"
        macos.mkdir(parents=True)
        executable = macos / "PortableMedia"
        shutil.copy2(default, executable)
        imports = run("default-native-imports", ["otool", "-L", executable])
        if "ScreenCaptureKit.framework" in imports:
            raise RuntimeError("Development screenshot dependency leaked into default packaging")
        info = plistlib.loads((REPO / "samples/PortableMedia/Info.plist").read_bytes())
        info.update(CFBundleExecutable="PortableMedia", CFBundlePackageType="APPL", CFBundleVersion="1", CFBundleShortVersionString="0.1")
        (bundle / "Contents/Info.plist").write_bytes(plistlib.dumps(info))
        entitlements = out / "development-entitlements.plist"
        entitlements.write_bytes(plistlib.dumps({"com.apple.security.device.camera": True, "com.apple.security.network.client": True}))
        run("bundle-sign", ["codesign", "--force", "--sign", "-", "--timestamp=none", "--options", "runtime", "--entitlements", entitlements, bundle])
        run("bundle-verify", ["codesign", "--verify", "--strict", "--verbose=2", bundle])
        run("bundle-entitlements", ["codesign", "--display", "--entitlements", ":-", bundle], ["com.apple.security.device.camera"])
        run("bundle-playback", [executable, source, "--resource", "--smoke", "--renderer"], ["RENDERER_PIXELS_PASS", "NATIVE_MEDIA_SMOKE_PASS"])
        run("bundle-camera-discovery", [executable, "--camera-discovery"], ["CAMERA_DISCOVERY_PASS", "acquired=false"])
        run("deployment-metadata", ["vtool", "-show-build", executable], ["minos"])
        report["passed"] = True
        save()
        print(f"MACOS_VERIFICATION_PASS stages={len(report['steps'])} evidence={out}", flush=True)
    except (RuntimeError, OSError) as error:
        report["passed"] = False
        report["error"] = str(error)
        save()
        print(str(error), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
