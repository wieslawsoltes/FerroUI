#!/usr/bin/env bash
# Builds an iOS application of the workspace with cargo and puts it into an application bundle,
# without an Xcode project (docs/porting/ios-platform.md, section 5).
#
#   scripts/ios/bundle.sh [options]
#
#   --package <name>       the package that has the executable        (default: ferroui-ios)
#   --example <name>       an example of the package                  (default: ios_view)
#   --bin <name>           a binary of the package, instead of an example
#   --features <list>      features of the package                    (default: example, for the default example)
#   --target <triple>      aarch64-apple-ios-sim (default), x86_64-apple-ios or aarch64-apple-ios
#   --profile <name>       the cargo profile                          (default: dev)
#   --bundle-id <id>       CFBundleIdentifier                         (default: org.ferroui.<executable with dashes>)
#   --name <name>          the display name                           (default: the executable)
#   --out <directory>      where the bundle is put                    (default: <target dir>/ios-bundles)
#   --no-build             bundle the executable that is already built
#
# The build directory is CARGO_TARGET_DIR when it is set, else `target` of the workspace.
# The last line of the output is the path of the bundle.
#
# A bundle for the simulator is signed ad hoc, which is all the simulator asks for. A bundle for a
# device is left unsigned: installing it needs a development certificate and a provisioning
# profile, which this script does not handle.
set -euo pipefail

package=ferroui-ios
kind=example
executable=ios_view
features=
features_set=0
target=aarch64-apple-ios-sim
profile=dev
bundle_id=
display_name=
out=
build=1

while [ $# -gt 0 ]; do
  case "$1" in
    --package) package="$2"; shift 2 ;;
    --example) kind=example; executable="$2"; shift 2 ;;
    --bin) kind=bin; executable="$2"; shift 2 ;;
    --features) features="$2"; features_set=1; shift 2 ;;
    --target) target="$2"; shift 2 ;;
    --profile) profile="$2"; shift 2 ;;
    --bundle-id) bundle_id="$2"; shift 2 ;;
    --name) display_name="$2"; shift 2 ;;
    --out) out="$2"; shift 2 ;;
    --no-build) build=0; shift ;;
    -h|--help) sed -n '2,23p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "bundle.sh: unknown argument: $1" >&2; exit 2 ;;
  esac
done

case "$target" in
  aarch64-apple-ios-sim|x86_64-apple-ios) platform=iPhoneSimulator; sdk=iphonesimulator; simulator=1 ;;
  aarch64-apple-ios) platform=iPhoneOS; sdk=iphoneos; simulator=0 ;;
  *) echo "bundle.sh: not an iOS target: $target" >&2; exit 2 ;;
esac

if [ "$features_set" = 0 ] && [ "$package" = ferroui-ios ] && [ "$kind" = example ]; then
  features=example
fi

root="$(cd "$(dirname "$0")/../.." && pwd)"
target_dir="${CARGO_TARGET_DIR:-$root/target}"
case "$profile" in
  dev) profile_dir=debug ;;
  *) profile_dir="$profile" ;;
esac

if [ "$build" = 1 ]; then
  args=(build --locked -p "$package" --target "$target" --profile "$profile")
  if [ "$kind" = example ]; then args+=(--example "$executable"); else args+=(--bin "$executable"); fi
  if [ -n "$features" ]; then args+=(--features "$features"); fi
  (cd "$root" && cargo "${args[@]}") >&2
fi

if [ "$kind" = example ]; then
  binary="$target_dir/$target/$profile_dir/examples/$executable"
else
  binary="$target_dir/$target/$profile_dir/$executable"
fi
if [ ! -f "$binary" ]; then
  echo "bundle.sh: the executable was not built: $binary" >&2
  exit 1
fi

if [ -z "$bundle_id" ]; then bundle_id="org.ferroui.$(printf '%s' "$executable" | tr '_' '-')"; fi
if [ -z "$display_name" ]; then display_name="$executable"; fi
if [ -z "$out" ]; then out="$target_dir/ios-bundles"; fi

sdk_version="$(xcrun --sdk "$sdk" --show-sdk-version)"
sdk_build="$(xcrun --sdk "$sdk" --show-sdk-build-version)"
xcode_version="$(xcodebuild -version | awk 'NR==1 {print $2}')"
xcode_build="$(xcodebuild -version | awk 'NR==2 {print $3}')"
# The version the executable was linked for (LC_BUILD_VERSION), so that the property list and the
# executable agree.
minimum_os="$(xcrun vtool -show-build "$binary" | awk '$1 == "minos" {print $2; exit}')"
if [ -z "$minimum_os" ]; then minimum_os=13.0; fi

app="$out/$executable.app"
rm -rf "$app"
mkdir -p "$app"
cp "$binary" "$app/$executable"

# The property list. `UIApplicationSceneManifest` without configurations makes UIKit ask the
# application delegate for the configuration of each scene, which is where the platform names its
# scene delegate. `UILaunchScreen` (an empty dictionary) stands for a launch storyboard: without
# one of the two the system runs the application in a letterboxed compatibility size.
plist="$app/Info.plist"
cat > "$plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleDevelopmentRegion</key>
	<string>en</string>
	<key>CFBundleDisplayName</key>
	<string>$display_name</string>
	<key>CFBundleExecutable</key>
	<string>$executable</string>
	<key>CFBundleIdentifier</key>
	<string>$bundle_id</string>
	<key>CFBundleInfoDictionaryVersion</key>
	<string>6.0</string>
	<key>CFBundleName</key>
	<string>$display_name</string>
	<key>CFBundlePackageType</key>
	<string>APPL</string>
	<key>CFBundleShortVersionString</key>
	<string>1.0</string>
	<key>CFBundleSupportedPlatforms</key>
	<array>
		<string>$platform</string>
	</array>
	<key>CFBundleVersion</key>
	<string>1</string>
	<key>DTPlatformName</key>
	<string>$sdk</string>
	<key>DTPlatformVersion</key>
	<string>$sdk_version</string>
	<key>DTSDKBuild</key>
	<string>$sdk_build</string>
	<key>DTSDKName</key>
	<string>$sdk$sdk_version</string>
	<key>DTXcode</key>
	<string>$xcode_version</string>
	<key>DTXcodeBuild</key>
	<string>$xcode_build</string>
	<key>LSRequiresIPhoneOS</key>
	<true/>
	<key>MinimumOSVersion</key>
	<string>$minimum_os</string>
	<key>UIApplicationSceneManifest</key>
	<dict>
		<key>UIApplicationSupportsMultipleScenes</key>
		<false/>
	</dict>
	<key>UIDeviceFamily</key>
	<array>
		<integer>1</integer>
		<integer>2</integer>
	</array>
	<key>UILaunchScreen</key>
	<dict/>
	<key>UIRequiredDeviceCapabilities</key>
	<array>
		<string>arm64</string>
	</array>
	<key>UISupportedInterfaceOrientations</key>
	<array>
		<string>UIInterfaceOrientationPortrait</string>
		<string>UIInterfaceOrientationPortraitUpsideDown</string>
		<string>UIInterfaceOrientationLandscapeLeft</string>
		<string>UIInterfaceOrientationLandscapeRight</string>
	</array>
	<key>UISupportedInterfaceOrientations~ipad</key>
	<array>
		<string>UIInterfaceOrientationPortrait</string>
		<string>UIInterfaceOrientationPortraitUpsideDown</string>
		<string>UIInterfaceOrientationLandscapeLeft</string>
		<string>UIInterfaceOrientationLandscapeRight</string>
	</array>
</dict>
</plist>
EOF
if [ "$target" = x86_64-apple-ios ]; then
  /usr/libexec/PlistBuddy -c "Delete :UIRequiredDeviceCapabilities" "$plist" >/dev/null
fi
plutil -lint "$plist" >&2
plutil -convert binary1 "$plist"

printf 'APPL????' > "$app/PkgInfo"

if [ "$simulator" = 1 ]; then
  codesign --force --sign - --timestamp=none "$app" >&2
else
  echo "bundle.sh: the bundle for a device is not signed" >&2
fi

echo "$app"
