#!/usr/bin/env bash
# publish.sh --dry-run [--no-verify] [--allow-dirty]
# publish.sh --publish [--no-verify]
#
# The one list of what a release publishes, for the release workflow and for a maintainer
# (docs/release.md). The crates are the members of the workspace that are not `publish = false`, in
# the order `scripts/release/crates.py order` derives from `cargo metadata`.
#
# --dry-run   Publishes nothing. Checks the manifests (crates.py check), packages every published
#             crate and builds each from its package, against the packages of the other crates of
#             the workspace instead of the registry (`cargo package --workspace`, Cargo 1.90 or
#             later), then lists the size of every package against the limit of crates.io.
#             With --no-verify the packages are assembled and not built: a check of a minute.
#             --allow-dirty packages a working tree with changes that are not committed.
#
# --publish   Publishes, one crate at a time in dependency order, with `cargo publish`, which
#             builds the crate from its package first and waits until the registry has it. A
#             version crates.io already has is skipped, so a run that stopped half way is run
#             again. It needs CARGO_REGISTRY_TOKEN in the environment, a clean working tree and
#             the commit of the release checked out. A published version cannot be removed:
#             read docs/release.md before the first run.
#
# One of the two has to be stated; there is no default.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
crates="$here/crates.py"
cd "$root"

mode=""
verify=1
allow_dirty=0
for argument in "$@"; do
  case "$argument" in
    --dry-run) mode="dry-run" ;;
    --publish) mode="publish" ;;
    --no-verify) verify=0 ;;
    --allow-dirty) allow_dirty=1 ;;
    -h|--help) sed -n '2,23p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "error: unknown option $argument (see --help)" >&2; exit 2 ;;
  esac
done
if [ -z "$mode" ]; then
  echo "error: state --dry-run or --publish (see --help)" >&2
  exit 2
fi
if [ "$mode" = "publish" ] && [ "$allow_dirty" = 1 ]; then
  echo "error: a release is published from a clean working tree; --allow-dirty is for --dry-run" >&2
  exit 2
fi

python3 "$crates" check
python3 "$crates" readme --check

version="$(python3 "$crates" version)"
order=()
while IFS= read -r name; do order+=("$name"); done < <(python3 "$crates" order)
unpublished=()
while IFS= read -r name; do unpublished+=("$name"); done < <(python3 "$crates" unpublished)
echo "version $version: ${#order[@]} crates to publish, ${#unpublished[@]} never published"

# 10 MiB, the limit of crates.io for a package.
limit=10485760

if [ "$mode" = "dry-run" ]; then
  # Packaging a crate whose dependencies are not in the registry yet needs the packages of the
  # workspace to stand in for it, which Cargo does since 1.90 when it packages several crates.
  minor="$(cargo --version | sed -E 's/^cargo 1\.([0-9]+).*/\1/')"
  if ! [ "$minor" -ge 90 ] 2>/dev/null; then
    echo "error: the dry run needs Cargo 1.90 or later ($(cargo --version))" >&2
    exit 1
  fi

  arguments=(package --workspace --locked)
  # (Written so that an empty list is not an unbound variable for the bash of macOS.)
  for name in ${unpublished[@]+"${unpublished[@]}"}; do arguments+=(--exclude "$name"); done
  [ "$verify" = 1 ] || arguments+=(--no-verify)
  [ "$allow_dirty" = 0 ] || arguments+=(--allow-dirty)
  cargo "${arguments[@]}"

  target="$(cargo metadata --format-version 1 --no-deps | python3 -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])')"
  echo
  printf '%-34s %12s  %s\n' "crate (in publish order)" "package" "on crates.io"
  too_large=0
  for name in "${order[@]}"; do
    file="$target/package/$name-$version.crate"
    size="$(wc -c < "$file" | tr -d ' ')"
    status=0
    python3 "$crates" published "$name" "$version" || status=$?
    case "$status" in
      0) state="$version is published" ;;
      1) state="not published" ;;
      *) state="unknown (the index did not answer)" ;;
    esac
    printf '%-34s %9s KiB  %s\n' "$name" "$((size / 1024))" "$state"
    if [ "$size" -gt "$limit" ]; then
      echo "error: the package of $name is above the 10 MiB limit of crates.io" >&2
      too_large=1
    fi
  done
  [ "$too_large" = 0 ] || exit 1
  echo
  if [ "$verify" = 1 ]; then
    echo "dry run: ${#order[@]} crates packaged and built from their packages; nothing was published"
  else
    echo "dry run: ${#order[@]} crates packaged (not built); nothing was published"
  fi
  exit 0
fi

# --publish
if [ -z "${CARGO_REGISTRY_TOKEN:-}" ]; then
  echo "error: CARGO_REGISTRY_TOKEN is not set" >&2
  exit 1
fi

# crates.io lets an account publish a few new crates at once and then one every ten minutes (new
# versions of crates that exist: far more). A refusal for that reason is waited out; the first
# release of the whole set therefore takes hours (docs/release.md, "Rate limits").
rate_limit_wait=660
rate_limit_attempts=40

log="$(mktemp)"
trap 'rm -f "$log"' EXIT

wait_until_published() {
  # `cargo publish` waits for the index itself; this covers the case that its wait ran out.
  local name="$1" attempt status
  for attempt in $(seq 1 60); do
    status=0
    python3 "$crates" published "$name" "$version" || status=$?
    [ "$status" = 0 ] && return 0
    sleep 10
  done
  echo "error: crates.io does not list $name $version ten minutes after it was uploaded" >&2
  return 1
}

published=0
skipped=0
for name in "${order[@]}"; do
  status=0
  python3 "$crates" published "$name" "$version" || status=$?
  if [ "$status" = 0 ]; then
    echo "skipped: $name $version is on crates.io already"
    skipped=$((skipped + 1))
    continue
  fi
  if [ "$status" != 1 ]; then
    echo "error: cannot tell whether $name $version is on crates.io (the index did not answer); run again" >&2
    exit 1
  fi

  arguments=(publish --locked -p "$name")
  [ "$verify" = 1 ] || arguments+=(--no-verify)
  attempt=1
  while :; do
    echo "publishing $name $version"
    if cargo "${arguments[@]}" 2>&1 | tee "$log"; then
      break
    fi
    if grep -q -i -E '429|too many requests|published too many' "$log" && [ "$attempt" -lt "$rate_limit_attempts" ]; then
      echo "crates.io limits the rate of new crates: waiting $rate_limit_wait seconds before $name is tried again"
      sleep "$rate_limit_wait"
      attempt=$((attempt + 1))
      continue
    fi
    # The upload may have succeeded and a later step failed (the wait for the index).
    status=0
    python3 "$crates" published "$name" "$version" || status=$?
    if [ "$status" = 0 ]; then
      break
    fi
    echo "error: $name $version was not published; the crates before it in the order are, and a second run continues here" >&2
    exit 1
  done
  wait_until_published "$name"
  published=$((published + 1))
done

echo "published $published crates at $version; $skipped were on crates.io already"
