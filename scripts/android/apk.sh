#!/bin/bash
# Builds an Android package (APK) of a FerroUI application with the tools of the SDK alone: cargo
# with the clang of the NDK, javac and d8 for the Java layer of the Android backend, aapt2 for the
# manifest, zipalign and apksigner. No Gradle (docs/porting/android-platform.md, section 9).
#
#   scripts/android/apk.sh smoke   [options]    the smoke application of ferroui-android
#   scripts/android/apk.sh catalog [options]    the ControlCatalog (samples/ControlCatalog.Android)
#
# Options:
#   --release           build with the release profile of cargo (default: the dev profile)
#   --abi ABI           arm64-v8a (default) or x86_64
#   --out DIR           where the package and the intermediate files go
#                       (default: $FERROUI_ANDROID_BUILD_DIR, else <cargo target directory>/android)
#   --no-strip          package the library with its symbol table
#   --no-cargo          package the library that is already built
#
# Environment: ANDROID_HOME (the SDK); CARGO_TARGET_DIR is honoured; see scripts/android/env.sh for
# the pinned versions of the NDK, the build tools and the platform.
#
# The last line printed is "APK: <path>".
set -euo pipefail

usage() {
    sed -n '2,20p' "$0" | sed 's/^# \{0,1\}//'
}

app=""
profile="dev"
abi="arm64-v8a"
out=""
strip=1
run_cargo=1
while [ $# -gt 0 ]; do
    case "$1" in
        smoke|catalog) app="$1" ;;
        --release) profile="release" ;;
        --abi) abi="${2:?--abi needs a value}"; shift ;;
        --out) out="${2:?--out needs a value}"; shift ;;
        --no-strip) strip=0 ;;
        --no-cargo) run_cargo=0 ;;
        -h|--help) usage; exit 0 ;;
        *) echo "unknown argument: $1" >&2; usage >&2; exit 2 ;;
    esac
    shift
done
if [ -z "$app" ]; then
    usage >&2
    exit 2
fi

repo="$(cd "$(dirname "$0")/../.." && pwd)"
# shellcheck source=scripts/android/env.sh
. "$repo/scripts/android/env.sh"

case "$abi" in
    arm64-v8a) target="aarch64-linux-android" ;;
    x86_64) target="x86_64-linux-android" ;;
    *) echo "unknown ABI: $abi (arm64-v8a or x86_64)" >&2; exit 2 ;;
esac

# What is built and how the package names it.
case "$app" in
    smoke)
        cargo_args=(-p ferroui-android --example android_smoke)
        library="android_smoke"
        library_dir="examples"
        application_id="org.ferroui.smoke"
        label="FerroUI smoke"
        ;;
    catalog)
        cargo_args=(-p control-catalog-android --lib)
        library="control_catalog_android"
        library_dir=""
        application_id="org.ferroui.controlcatalog"
        label="ControlCatalog"
        ;;
esac
min_sdk="$FERROUI_ANDROID_API"
target_sdk="${FERROUI_ANDROID_PLATFORM#android-}"

cargo_target_dir="${CARGO_TARGET_DIR:-$repo/target}"
out="${out:-${FERROUI_ANDROID_BUILD_DIR:-$cargo_target_dir/android}}"
work="$out/$app-$abi"
if [ "$profile" = "release" ]; then
    profile_dir="release"
    profile_args=(--release)
else
    profile_dir="debug"
    profile_args=()
fi

for tool in "$FERROUI_ANDROID_BUILD_TOOLS/aapt2" "$FERROUI_ANDROID_BUILD_TOOLS/d8" "$FERROUI_ANDROID_BUILD_TOOLS/zipalign" \
    "$FERROUI_ANDROID_BUILD_TOOLS/apksigner" "$FERROUI_ANDROID_JAR" "$FERROUI_ANDROID_TOOLCHAIN/llvm-strip"; do
    if [ ! -e "$tool" ]; then
        echo "missing: $tool" >&2
        echo "  sdkmanager --sdk_root=\"$ANDROID_HOME\" \"build-tools;$FERROUI_ANDROID_BUILD_TOOLS_VERSION\" \"platforms;$FERROUI_ANDROID_PLATFORM\" \"ndk;$FERROUI_ANDROID_NDK_VERSION\"" >&2
        exit 1
    fi
done
for tool in javac keytool zip; do
    command -v "$tool" >/dev/null || { echo "missing on the PATH: $tool" >&2; exit 1; }
done

rm -rf "$work"
mkdir -p "$work/classes" "$work/stage/lib/$abi"

# 1. The native library.
if [ "$run_cargo" = 1 ]; then
    echo "== cargo build ${cargo_args[*]} --target $target (${profile})"
    (cd "$repo" && cargo build "${cargo_args[@]}" --target "$target" ${profile_args[@]+"${profile_args[@]}"})
fi
built="$cargo_target_dir/$target/$profile_dir/${library_dir:+$library_dir/}lib$library.so"
if [ ! -f "$built" ]; then
    echo "the library was not built: $built" >&2
    exit 1
fi
if [ "$strip" = 1 ]; then
    "$FERROUI_ANDROID_TOOLCHAIN/llvm-strip" --strip-unneeded -o "$work/stage/lib/$abi/lib$library.so" "$built"
else
    cp "$built" "$work/stage/lib/$abi/lib$library.so"
fi
# The library exports the one symbol the virtual machine looks for.
if ! "$FERROUI_ANDROID_TOOLCHAIN/llvm-nm" -D --defined-only "$work/stage/lib/$abi/lib$library.so" | grep -q ' JNI_OnLoad$'; then
    echo "lib$library.so does not export JNI_OnLoad (the application lacks ferroui_android::android_application!)" >&2
    exit 1
fi

# 2. The Java layer: classes, then one dex file.
echo "== javac, d8"
java_sources=()
while IFS= read -r file; do
    java_sources+=("$file")
done < <(find "$repo/src/Android/FerroUI.Android/java" -name '*.java' | sort)
javac -source 11 -target 11 -Xlint:all -Xlint:-options -Werror -classpath "$FERROUI_ANDROID_JAR" -d "$work/classes" "${java_sources[@]}"
class_files=()
while IFS= read -r file; do
    class_files+=("$file")
done < <(find "$work/classes" -name '*.class' | sort)
if [ "$profile" = "release" ]; then d8_mode="--release"; else d8_mode="--debug"; fi
"$FERROUI_ANDROID_BUILD_TOOLS/d8" "$d8_mode" --min-api "$min_sdk" --lib "$FERROUI_ANDROID_JAR" --output "$work/stage" "${class_files[@]}"

# 3. The manifest, and the package that holds it.
cat > "$work/AndroidManifest.xml" <<EOF
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="$application_id"
    android:versionCode="1"
    android:versionName="1.0">
    <uses-sdk android:minSdkVersion="$min_sdk" android:targetSdkVersion="$target_sdk" />
    <application
        android:name="org.ferroui.android.FerroApplication"
        android:label="$label"
        android:hasCode="true"
        android:extractNativeLibs="true">
        <meta-data android:name="org.ferroui.android.library" android:value="$library" />
        <activity
            android:name="org.ferroui.android.FerroMainActivity"
            android:exported="true"
            android:configChanges="orientation|screenSize|uiMode"
            android:theme="@android:style/Theme.DeviceDefault.NoActionBar">
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>
</manifest>
EOF
echo "== aapt2 link"
aapt_args=(link -o "$work/unaligned.apk" --manifest "$work/AndroidManifest.xml" -I "$FERROUI_ANDROID_JAR"
    --min-sdk-version "$min_sdk" --target-sdk-version "$target_sdk" --version-code 1 --version-name 1.0)
if [ "$profile" != "release" ]; then
    # Debuggable: the emulator scripts read and write the files of the application with run-as.
    aapt_args+=(--debug-mode)
fi
"$FERROUI_ANDROID_BUILD_TOOLS/aapt2" "${aapt_args[@]}"

# 4. The code, aligned and signed.
(cd "$work/stage" && zip -q -r "$work/unaligned.apk" classes.dex lib)
apk="$out/$app-$abi-$profile_dir.apk"
rm -f "$apk"
"$FERROUI_ANDROID_BUILD_TOOLS/zipalign" -f 4 "$work/unaligned.apk" "$apk"

keystore="$out/debug.keystore"
if [ ! -f "$keystore" ]; then
    keytool -genkeypair -keystore "$keystore" -storepass android -keypass android -alias androiddebugkey \
        -keyalg RSA -keysize 2048 -validity 10000 -dname "CN=Android Debug,O=Android,C=US" >/dev/null 2>&1
fi
"$FERROUI_ANDROID_BUILD_TOOLS/apksigner" sign --ks "$keystore" --ks-pass pass:android --key-pass pass:android \
    --ks-key-alias androiddebugkey "$apk"
"$FERROUI_ANDROID_BUILD_TOOLS/apksigner" verify "$apk"

echo "application id: $application_id"
echo "activity:       $application_id/org.ferroui.android.FerroMainActivity"
echo "library:        lib$library.so ($(du -h "$work/stage/lib/$abi/lib$library.so" | cut -f1 | tr -d ' ')), ABI $abi, API $min_sdk to $target_sdk"
echo "size:           $(du -h "$apk" | cut -f1 | tr -d ' ')"
echo "APK: $apk"
