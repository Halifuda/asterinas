#!/bin/sh

# SPDX-License-Identifier: MPL-2.0

set -eu

# RUNTIME_PATH is substituted by the Nix build.
export PATH=__RUNTIME_PATH__

XFSTESTS_DIR=/opt/xfstests
cd "$XFSTESTS_DIR"

TEST_DEV=${XFSTESTS_TEST_DEV:-/dev/vdc}
SCRATCH_DEV=${XFSTESTS_SCRATCH_DEV:-/dev/vdd}
FSTYP=${XFSTESTS_FSTYP:-ext2}
MKFS=${XFSTESTS_MKFS:-mkfs.ext2}
MKFS_OPTIONS=${XFSTESTS_MKFS_OPTIONS--F}
FSCK=${XFSTESTS_FSCK:-fsck.ext2}
MOUNT_OPTIONS=${XFSTESTS_MOUNT_OPTIONS-"-o noacl"}
DIRECT_COMMAND=${XFSTESTS_DIRECT_COMMAND-}
DIRECT_COMMAND_B64=${XFSTESTS_DIRECT_COMMAND_B64-}
DIRECT_WORKDIR=${XFSTESTS_DIRECT_WORKDIR:-$XFSTESTS_DIR/test}
export TEST_DEV SCRATCH_DEV
export FSTYP MKFS_OPTIONS

WRAPPER_DIR="$XFSTESTS_DIR/bin"
mkdir -p "$WRAPPER_DIR"
export PATH="$WRAPPER_DIR:$PATH"

cat > "$WRAPPER_DIR/mkfs.$FSTYP" <<EOF
#!/bin/sh
if [ "\${1:-}" = "-t" ] && [ "\${2:-}" = "$FSTYP" ]; then
    shift 2
    if [ "\${1:-}" = "--" ]; then
        shift
    fi
fi
exec $MKFS "\$@"
EOF
chmod +x "$WRAPPER_DIR/mkfs.$FSTYP"

cat > "$WRAPPER_DIR/fsck.$FSTYP" <<EOF
#!/bin/sh
exec $FSCK "\$@"
EOF
chmod +x "$WRAPPER_DIR/fsck.$FSTYP"

# Check xfstests images with explicit error checking before handing them to
# upstream ./check. xfstests owns the actual TEST/SCRATCH mount lifecycle.
for entry in "$TEST_DEV:$XFSTESTS_DIR/test:test" "$SCRATCH_DEV:$XFSTESTS_DIR/scratch:scratch"; do
    dev="${entry%%:*}"; rest="${entry#*:}"; mnt="${rest%%:*}"; role="${rest##*:}"
    if [ ! -b "$dev" ]; then
        echo "Expected $dev to be a block device for xfstests $role" >&2
        exit 1
    fi
    if [ ! -d "$mnt" ]; then
        echo "Expected $mnt to be a directory for xfstests $role" >&2
        exit 1
    fi
done

cat > "$XFSTESTS_DIR/local.config" <<EOF
export FSTYP=$FSTYP
export TEST_DEV=$TEST_DEV
export SCRATCH_DEV=$SCRATCH_DEV
export TEST_DIR=$XFSTESTS_DIR/test
export SCRATCH_MNT=$XFSTESTS_DIR/scratch
export MOUNT_OPTIONS="$MOUNT_OPTIONS"
export EXT_MOUNT_OPTIONS="$MOUNT_OPTIONS"
export FSTYP_HAS_NON_DEFAULT_OPTS=0
export SELINUX_MOUNT_OPTIONS=" "
export MKFS_OPTIONS="$MKFS_OPTIONS"
export MKFS_PROG="$WRAPPER_DIR/mkfs.$FSTYP"
EOF

# Asterinas' initramfs lane can re-enter common/rc in a per-test shell with
# CONFIG_INCLUDED still set but TEST_DIR empty. Reload the generated config
# before init_rc's unquoted mount checks can shift FSTYP into TEST_DIR's slot.
sed '/^init_rc()/,/^{$/s|^{$|{\
\tif [ -z "$TEST_DIR" ] \&\& [ -f "$here/local.config" ]; then\
\t\t. "$here/local.config"\
\tfi|' common/rc > common/rc.runtime
mv common/rc.runtime common/rc

echo "xfstests FSTYP=$FSTYP TEST_DEV=$TEST_DEV SCRATCH_DEV=$SCRATCH_DEV"
echo "xfstests TEST_DIR=$XFSTESTS_DIR/test SCRATCH_MNT=$XFSTESTS_DIR/scratch MOUNT_OPTIONS=$MOUNT_OPTIONS"
echo "xfstests mkfs wrapper: $(command -v "mkfs.$FSTYP") -> $MKFS"
echo "xfstests fsck wrapper: $(command -v "fsck.$FSTYP") -> $FSCK"

mount_xfstests_device()
{
    device="$1"
    mount_point="$2"

    if [ -n "$MOUNT_OPTIONS" ]; then
        # Word-splitting is intentional: mount options follow the same format
        # that upstream xfstests passes to mount(8).
        # shellcheck disable=SC2086
        mount -t "$FSTYP" $MOUNT_OPTIONS "$device" "$mount_point"
    else
        mount -t "$FSTYP" "$device" "$mount_point"
    fi
}

unmount_if_mounted()
{
    mount_point="$1"

    if grep -q " $mount_point " /proc/mounts; then
        umount "$mount_point"
    fi
}

run_direct_command()
{
    if [ -n "$DIRECT_COMMAND_B64" ]; then
        DIRECT_COMMAND=$(printf '%s' "$DIRECT_COMMAND_B64" | base64 -d)
    fi

    echo "xfstests direct command mode"
    echo "xfstests direct workdir: $DIRECT_WORKDIR"
    echo "xfstests direct command: $DIRECT_COMMAND"

    mount_xfstests_device "$TEST_DEV" "$XFSTESTS_DIR/test"
    mount_xfstests_device "$SCRATCH_DEV" "$XFSTESTS_DIR/scratch"
    trap 'unmount_if_mounted "$XFSTESTS_DIR/scratch"; unmount_if_mounted "$XFSTESTS_DIR/test"' EXIT

    export TEST_DIR=$XFSTESTS_DIR/test
    export SCRATCH_MNT=$XFSTESTS_DIR/scratch

    cd "$DIRECT_WORKDIR"
    set +e
    /bin/sh -c "$DIRECT_COMMAND"
    direct_status=$?
    set -e
    echo "xfstests direct command status: $direct_status"
    sync
    exit "$direct_status"
}

if [ -n "$DIRECT_COMMAND" ] || [ -n "$DIRECT_COMMAND_B64" ]; then
    run_direct_command
fi

RUNLIST_FILE=""
TEST_ARGS=""

# Parse -R flag and collect direct test names.
# Test names are simple identifiers (e.g. "generic/001") so accumulating
# them in a space-separated string is safe.
while [ $# -gt 0 ]; do
  case "$1" in
    -R|--runlist)
      if [ $# -lt 2 ]; then
        echo "Error: -R|--runlist requires a filename argument." >&2
        exit 2
      fi
      RUNLIST_FILE="$2"
      shift 2
      ;;
    --)
      shift
      TEST_ARGS="$TEST_ARGS $*"
      break
      ;;
    *)
      TEST_ARGS="$TEST_ARGS $1"
      shift
      ;;
  esac
done

if [ -n "$RUNLIST_FILE" ]; then
  if [ ! -f "$RUNLIST_FILE" ]; then
    echo "Run list file not found: $RUNLIST_FILE" >&2
    exit 2
  fi
  while IFS= read -r test; do
    case "$test" in
      ""|\#*) continue ;;
    esac
    TEST_ARGS="$TEST_ARGS $test"
  done < "$RUNLIST_FILE"
fi

# Prepend block-list exclusion so blocked tests are skipped.
if [ -f "$XFSTESTS_DIR/block.list" ]; then
    TEST_ARGS="-E $XFSTESTS_DIR/block.list $TEST_ARGS"
fi

# Word-splitting is intentional here: TEST_ARGS contains only test names
# and the -E flag, none of which contain whitespace or shell metacharacters.
# shellcheck disable=SC2086
if ! ./check -d $TEST_ARGS; then
    echo "xfstests result failure files:"
    find "$XFSTESTS_DIR/results" -type f \( -name "*.out.bad" -o -name "*.full" \) -print |
    while IFS= read -r result_file; do
        echo "----- $result_file -----"
        cat "$result_file"
    done
    exit 1
fi
