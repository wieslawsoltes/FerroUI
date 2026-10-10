# Sourced by the scripts of this directory: the Android SDK and NDK, and the environment that makes
# cargo link and compile C and C++ for an Android target with the clang of the NDK. No Gradle and no
# cargo-ndk: cargo's own variables are enough (docs/porting/android-platform.md, section 9).
#
#   ANDROID_HOME          the SDK (required; ANDROID_SDK_ROOT is taken when it is not set)
#   FERROUI_ANDROID_NDK   the NDK directory; default: the pinned version under $ANDROID_HOME/ndk
#   FERROUI_ANDROID_API   the API level the native code is built for; default 26
#
# ANDROID_NDK is deliberately not exported: the build script of the Skia bindings reads it only when
# it builds Skia from source, which this project never does (a published binary is used); without
# the variable that fall-back stops at once instead of starting an hour of compilation.

FERROUI_ANDROID_NDK_VERSION="28.2.13676358"
FERROUI_ANDROID_BUILD_TOOLS_VERSION="36.0.0"
FERROUI_ANDROID_PLATFORM="android-36"
: "${FERROUI_ANDROID_API:=26}"

if [ -z "${ANDROID_HOME:-}" ] && [ -n "${ANDROID_SDK_ROOT:-}" ]; then
    ANDROID_HOME="$ANDROID_SDK_ROOT"
fi
if [ -z "${ANDROID_HOME:-}" ]; then
    echo "ANDROID_HOME is not set (the Android SDK directory)" >&2
    return 1 2>/dev/null || exit 1
fi
export ANDROID_HOME
: "${FERROUI_ANDROID_NDK:=$ANDROID_HOME/ndk/$FERROUI_ANDROID_NDK_VERSION}"
if [ ! -d "$FERROUI_ANDROID_NDK" ]; then
    echo "the NDK $FERROUI_ANDROID_NDK_VERSION is not installed: $FERROUI_ANDROID_NDK" >&2
    echo "  sdkmanager --sdk_root=\"$ANDROID_HOME\" \"ndk;$FERROUI_ANDROID_NDK_VERSION\"" >&2
    return 1 2>/dev/null || exit 1
fi

case "$(uname -s)" in
    Darwin) ferroui_ndk_host="darwin-x86_64" ;;
    Linux) ferroui_ndk_host="linux-x86_64" ;;
    *) echo "unsupported host: $(uname -s)" >&2; return 1 2>/dev/null || exit 1 ;;
esac
FERROUI_ANDROID_TOOLCHAIN="$FERROUI_ANDROID_NDK/toolchains/llvm/prebuilt/$ferroui_ndk_host/bin"
FERROUI_ANDROID_BUILD_TOOLS="$ANDROID_HOME/build-tools/$FERROUI_ANDROID_BUILD_TOOLS_VERSION"
FERROUI_ANDROID_JAR="$ANDROID_HOME/platforms/$FERROUI_ANDROID_PLATFORM/android.jar"

export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$FERROUI_ANDROID_TOOLCHAIN/aarch64-linux-android$FERROUI_ANDROID_API-clang"
export CC_aarch64_linux_android="$FERROUI_ANDROID_TOOLCHAIN/aarch64-linux-android$FERROUI_ANDROID_API-clang"
export CXX_aarch64_linux_android="$FERROUI_ANDROID_TOOLCHAIN/aarch64-linux-android$FERROUI_ANDROID_API-clang++"
export AR_aarch64_linux_android="$FERROUI_ANDROID_TOOLCHAIN/llvm-ar"

export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="$FERROUI_ANDROID_TOOLCHAIN/x86_64-linux-android$FERROUI_ANDROID_API-clang"
export CC_x86_64_linux_android="$FERROUI_ANDROID_TOOLCHAIN/x86_64-linux-android$FERROUI_ANDROID_API-clang"
export CXX_x86_64_linux_android="$FERROUI_ANDROID_TOOLCHAIN/x86_64-linux-android$FERROUI_ANDROID_API-clang++"
export AR_x86_64_linux_android="$FERROUI_ANDROID_TOOLCHAIN/llvm-ar"
