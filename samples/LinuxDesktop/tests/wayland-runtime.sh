# Both headless fixtures start a private session bus in a temporary runtime.
# Its activated Documents portal may still be releasing its FUSE mount after
# dbus-run-session returns. Never recurse into a live document portal.
remove_wayland_runtime() {
    local dir=${1:?expected private test directory} docs=$1/runtime/doc attempt
    for ((attempt=0; attempt<30; attempt++)); do
        if mountpoint -q -- "$docs"; then
            fusermount3 -uz -- "$docs" 2>/dev/null || :
        elif rm -rf --one-file-system -- "$dir" 2>/dev/null; then
            return 0
        fi
        sleep .1
    done
    echo "Could not clean up private Wayland test directory $dir" >&2
    return 1
}
