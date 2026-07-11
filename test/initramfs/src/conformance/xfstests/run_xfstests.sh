#!/bin/sh

# SPDX-License-Identifier: MPL-2.0

set -eu

# RUNTIME_PATH is substituted by the Nix build.
export PATH=__RUNTIME_PATH__

XFSTESTS_DIR=/opt/xfstests
cd "$XFSTESTS_DIR"

TEST_DEV=${XFSTESTS_TEST_DEV:-/dev/vdd}
SCRATCH_DEV=${XFSTESTS_SCRATCH_DEV:-/dev/vde}
FSTYP=${XFSTESTS_FSTYP:-ext2}
MKFS=${XFSTESTS_MKFS:-mkfs.ext2}
MKFS_OPTIONS=${XFSTESTS_MKFS_OPTIONS--F}
FSCK=${XFSTESTS_FSCK:-fsck.ext2}
MOUNT_OPTIONS=${XFSTESTS_MOUNT_OPTIONS-"-onoacl"}
export TEST_DEV SCRATCH_DEV
export FSTYP MKFS_OPTIONS

REAL_MKFS=$(command -v "$MKFS" 2>/dev/null || true)
[ -n "$REAL_MKFS" ] || REAL_MKFS=$MKFS
REAL_FSCK=$(command -v "$FSCK" 2>/dev/null || true)
[ -n "$REAL_FSCK" ] || REAL_FSCK=$FSCK
REAL_SYSTEM_FSCK=$(command -v fsck 2>/dev/null || true)
[ -n "$REAL_SYSTEM_FSCK" ] || REAL_SYSTEM_FSCK=$REAL_FSCK

if [ "$TEST_DEV" = "$SCRATCH_DEV" ]; then
    echo "TEST_DEV and SCRATCH_DEV must be distinct for xfstests: $TEST_DEV" >&2
    exit 1
fi

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
exec "$REAL_MKFS" "\$@"
EOF
chmod +x "$WRAPPER_DIR/mkfs.$FSTYP"

cat > "$WRAPPER_DIR/fsck.$FSTYP" <<EOF
#!/bin/sh
exec "$REAL_FSCK" "\$@"
EOF
chmod +x "$WRAPPER_DIR/fsck.$FSTYP"

cat > "$WRAPPER_DIR/fsck" <<EOF
#!/bin/sh
if [ "\${1:-}" = "-t" ] && [ "\${2:-}" = "$FSTYP" ]; then
    shift 2
    exec "$REAL_FSCK" "\$@"
fi
exec "$REAL_SYSTEM_FSCK" "\$@"
EOF
chmod +x "$WRAPPER_DIR/fsck"

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
export XFSTESTS_FSCK_WRAPPER="$WRAPPER_DIR/fsck.$FSTYP"
export XFSTESTS_SYSTEM_FSCK="$REAL_SYSTEM_FSCK"
EOF

append_common_rc_asterinas_compat()
{
    cat >> common/rc <<'EOF'

# Asterinas-specific compatibility overrides for upstream xfstests.
_df_device()
{
    if [ $# -ne 1 ]
    then
        echo "Usage: _df_device device" 1>&2
        exit 1
    fi

    local df_line mount_point

    df_line=$($DF_PROG 2>/dev/null | $AWK_PROG -v what=$1 '
        ($1==what) && (NF==1) {
            v=$1
            getline
            print v, $0
            exit
        }
        ($1==what) {
            print
            exit
        }
    ')
    if [ -n "$df_line" ]; then
        printf '%s\n' "$df_line"
        return 0
    fi

    mount_point=$(findmnt -rncv -S "$1" -o TARGET 2>/dev/null | head -n 1)
    if [ -n "$mount_point" ]; then
        $DF_PROG "$mount_point" 2>/dev/null | $AWK_PROG -v what="$mount_point" '
            NR == 2 && NF==1 {
                v=$1
                getline
                print v, $0
                exit 0
            }
            NR == 2 {
                print
                exit 0
            }
        '
    fi
}

_fs_type()
{
    if [ $# -ne 1 ]
    then
        echo "Usage: _fs_type device" 1>&2
        exit 1
    fi

    local df_type

    df_type=$(_df_device "$1" | $AWK_PROG '{ print $2 }' | \
        sed -e 's/nfs4/nfs/' -e 's/fuse.glusterfs/glusterfs/' \
            -e 's/fuse.ceph-fuse/ceph-fuse/')
    if [ -n "$df_type" ]; then
        printf '%s\n' "$df_type"
        return 0
    fi

    findmnt -rncv -S "$1" -o FSTYPE 2>/dev/null | head -n 1 | \
        sed -e 's/nfs4/nfs/' -e 's/fuse.glusterfs/glusterfs/' \
            -e 's/fuse.ceph-fuse/ceph-fuse/'
}

_check_if_dev_already_mounted()
{
    local dev=$1
    local mnt=$2
    local mount_rec

    mount_rec=$(findmnt -rncv -S "$dev" -o SOURCE,TARGET 2>/dev/null | \
        $AWK_PROG '!seen[$0]++ { print }' | head -n 1)
    [ -n "$mount_rec" ] || return 1

    if [ "$mount_rec" != "$dev $mnt" ]; then
        echo "$devname=$dev is mounted but not on $mntname=$mnt - aborting"
        echo "Already mounted result:"
        echo "$mount_rec"
        return 2
    fi
}

fsck()
{
    if [ "${1:-}" = "-t" ] && [ "${2:-}" = "$FSTYP" ]; then
        shift 2
        "$XFSTESTS_FSCK_WRAPPER" "$@"
        return $?
    fi

    "$XFSTESTS_SYSTEM_FSCK" "$@"
}
EOF
}

# Asterinas' initramfs lane can re-enter common/rc in a per-test shell with
# CONFIG_INCLUDED still set but TEST_DIR empty. Reload the generated config
# before init_rc's unquoted mount checks can shift FSTYP into TEST_DIR's slot.
sed '/^init_rc()/,/^{$/s|^{$|{\
\tif [ -z "$TEST_DIR" ] \&\& [ -f "$here/local.config" ]; then\
\t\t. "$here/local.config"\
\tfi|' common/rc > common/rc.runtime
mv common/rc.runtime common/rc
append_common_rc_asterinas_compat

echo "xfstests FSTYP=$FSTYP TEST_DEV=$TEST_DEV SCRATCH_DEV=$SCRATCH_DEV"
echo "xfstests TEST_DIR=$XFSTESTS_DIR/test SCRATCH_MNT=$XFSTESTS_DIR/scratch MOUNT_OPTIONS=$MOUNT_OPTIONS"
echo "xfstests mkfs wrapper: $(command -v "mkfs.$FSTYP") -> $MKFS"
echo "xfstests fsck wrapper: $(command -v "fsck.$FSTYP") -> $FSCK"

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
