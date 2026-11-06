# Source after choosing an SSD work directory outside the extracted source.
: "${FERROBOARD_WORK_DIR:?Set FERROBOARD_WORK_DIR to an absolute SSD work directory}"
case "$FERROBOARD_WORK_DIR" in
  /*) ;;
  *) echo 'FERROBOARD_WORK_DIR must be absolute' >&2; return 1 ;;
esac
mkdir -p "$FERROBOARD_WORK_DIR/tmp" "$FERROBOARD_WORK_DIR/target"
export TMPDIR="$FERROBOARD_WORK_DIR/tmp"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$FERROBOARD_WORK_DIR/target}"
export CARGO_HOME="${CARGO_HOME:-$FERROBOARD_WORK_DIR/cargo-home}"
export CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_PROFILE_TEST_DEBUG=0
