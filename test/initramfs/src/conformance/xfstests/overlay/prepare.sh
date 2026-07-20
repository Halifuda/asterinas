# SPDX-License-Identifier: MPL-2.0

mkdir -p "$TEST_DIR" "$SCRATCH_MNT"

# common/overlay mounts the base filesystems and creates the overlay layer
# directories after this script returns.
for dev in "$TEST_DEV" "$SCRATCH_DEV"; do
    if [ ! -b "$dev" ]; then
        echo "Expected $dev to be a block device for overlay xfstests" >&2
        exit 1
    fi
done
