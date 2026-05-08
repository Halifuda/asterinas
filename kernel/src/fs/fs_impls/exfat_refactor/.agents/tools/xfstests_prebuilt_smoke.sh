#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0

set -euo pipefail

usage() {
    cat <<'USAGE'
Usage: xfstests_prebuilt_smoke.sh [--run-id ID] [--timeout TIMEOUT] [--no-lock]
                                  [--prepare-only] [--refresh-direct-boot-kernel]
       xfstests_prebuilt_smoke.sh --prune-old-images [--keep-image-runs N]
                                  [--preserve-run-id ID] [--prune-dry-run]

Runs the exFAT refactor prebuilt-image NixOS smoke harness.

Inputs:
  kernel/src/fs/fs_impls/exfat_refactor/.agents/xfstests/images/root-base.img
  kernel/src/fs/fs_impls/exfat_refactor/.agents/xfstests/images/test-base.img
  kernel/src/fs/fs_impls/exfat_refactor/.agents/xfstests/images/scratch-base.img
  kernel/src/fs/fs_impls/exfat_refactor/.agents/xfstests/images/direct-boot/{kernel,initrd,kernel-params}

Outputs:
  kernel/src/fs/fs_impls/exfat_refactor/.agents/xfstests/logs/<run-id>/

Cleanup:
  --prune-old-images      Remove root.img/test.img/scratch.img from old run dirs.
  --keep-image-runs N     Keep images in the newest N run dirs. Default: 3.
  --preserve-run-id ID    Never prune this run id. May be passed more than once.
  --prune-dry-run         Print what would be pruned without deleting files.

Direct boot:
  --refresh-direct-boot-kernel
                           Rebuild the current linux-efi-handover64 kernel and
                           stage its bzImage into images/direct-boot/kernel.
                           Existing shell-first initrd/kernel-params are kept.
USAGE
}

RUN_ID=""
RUN_ID_GIVEN=0
TIMEOUT="60min"
USE_LOCK=1
PREPARE_ONLY=0
BOOT_MODE="${BOOT_MODE:-direct-boot}"
REFRESH_DIRECT_BOOT_KERNEL=0
PRUNE_OLD_IMAGES=0
PRUNE_DRY_RUN=0
KEEP_IMAGE_RUNS=3
PRESERVE_RUN_IDS=()
LOCK_HELD=0

while [ "$#" -gt 0 ]; do
    case "$1" in
        --run-id)
            RUN_ID="${2:?missing --run-id value}"
            RUN_ID_GIVEN=1
            shift 2
            ;;
        --timeout)
            TIMEOUT="${2:?missing --timeout value}"
            shift 2
            ;;
        --no-lock)
            USE_LOCK=0
            shift
            ;;
        --prepare-only)
            PREPARE_ONLY=1
            shift
            ;;
        --refresh-direct-boot-kernel)
            REFRESH_DIRECT_BOOT_KERNEL=1
            shift
            ;;
        --prune-old-images)
            PRUNE_OLD_IMAGES=1
            shift
            ;;
        --keep-image-runs)
            KEEP_IMAGE_RUNS="${2:?missing --keep-image-runs value}"
            shift 2
            ;;
        --preserve-run-id)
            PRESERVE_RUN_IDS+=("${2:?missing --preserve-run-id value}")
            shift 2
            ;;
        --prune-dry-run)
            PRUNE_DRY_RUN=1
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "unknown argument: $1" >&2
            usage >&2
            exit 2
            ;;
    esac
done

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
AGENTS_DIR=$(cd "${SCRIPT_DIR}/.." && pwd)
EXFAT_REFACTOR_DIR=$(cd "${AGENTS_DIR}/.." && pwd)
ASTERINAS_DIR=$(cd "${EXFAT_REFACTOR_DIR}/../../../../.." && pwd)

IMAGES_DIR="${AGENTS_DIR}/xfstests/images"
LOGS_DIR="${AGENTS_DIR}/xfstests/logs"
LOCK_SCRIPT="${AGENTS_DIR}/tools/checker_lock.sh"
CHECKER_COMPONENT="${CHECKER_COMPONENT:-xfstests_harness_20260508}"
CHECKER_PHASE="${CHECKER_PHASE:-prebuilt_image_smoke}"
DIRECT_BOOT_REFRESH_SOURCE=""

if [ -z "${RUN_ID}" ]; then
    RUN_ID=$(date +%Y%m%d-%H%M%S)-prebuilt-image-smoke
fi

RUN_DIR="${LOGS_DIR}/${RUN_ID}"
RUN_DIRECT_BOOT_DIR="${RUN_DIR}/direct-boot"
ROOT_IMAGE="${RUN_DIR}/root.img"
TEST_IMAGE="${RUN_DIR}/test.img"
SCRATCH_IMAGE="${RUN_DIR}/scratch.img"
QEMU_SCRIPT="${RUN_DIR}/run-qemu.sh"
MANIFEST="${RUN_DIR}/manifest.txt"
STDOUT_LOG="${RUN_DIR}/test-stdout-stderr.log"

validate_prune_args() {
    case "${KEEP_IMAGE_RUNS}" in
        ''|*[!0-9]*)
            echo "--keep-image-runs must be a non-negative integer" >&2
            exit 2
            ;;
    esac
}

is_preserved_run() {
    local run_id=$1
    local preserved_run_id

    if [ "${run_id}" = "${RUN_ID}" ]; then
        return 0
    fi

    for preserved_run_id in "${PRESERVE_RUN_IDS[@]}"; do
        if [ "${run_id}" = "${preserved_run_id}" ]; then
            return 0
        fi
    done

    return 1
}

run_has_images() {
    local run_dir=$1

    [ -e "${run_dir}/root.img" ] || \
        [ -e "${run_dir}/test.img" ] || \
        [ -e "${run_dir}/scratch.img" ]
}

prune_old_run_images() {
    local kept=0
    local timestamp
    local run_dir
    local run_id
    local image

    validate_prune_args
    mkdir -p "${LOGS_DIR}"

    while read -r timestamp run_dir; do
        [ -n "${timestamp}" ] || continue
        run_id=$(basename "${run_dir}")

        if ! run_has_images "${run_dir}"; then
            continue
        fi

        if is_preserved_run "${run_id}"; then
            echo "preserve images: ${run_id}"
            continue
        fi

        if [ "${kept}" -lt "${KEEP_IMAGE_RUNS}" ]; then
            kept=$((kept + 1))
            echo "keep images: ${run_id}"
            continue
        fi

        for image in root.img test.img scratch.img; do
            if [ -e "${run_dir}/${image}" ]; then
                if [ "${PRUNE_DRY_RUN}" -eq 1 ]; then
                    echo "would prune: ${run_dir}/${image}"
                else
                    rm -f -- "${run_dir}/${image}"
                    echo "pruned: ${run_dir}/${image}"
                fi
            fi
        done

        if [ "${PRUNE_DRY_RUN}" -eq 0 ] && [ -f "${run_dir}/manifest.txt" ]; then
            {
                echo
                echo "[prune]"
                echo "pruned_at=$(date -Iseconds)"
                echo "pruned_files=root.img test.img scratch.img"
            } >> "${run_dir}/manifest.txt"
        fi
    done < <(find "${LOGS_DIR}" -mindepth 1 -maxdepth 1 -type d -printf '%T@ %p\n' | sort -rn)
}

copy_image() {
    local source=$1
    local dest=$2

    if [ ! -f "${source}" ]; then
        echo "missing required image: ${source}" >&2
        exit 1
    fi
    cp --reflink=auto --sparse=always "${source}" "${dest}"
}

acquire_checker_lock() {
    if [ "${USE_LOCK}" -eq 1 ] && [ "${LOCK_HELD}" -eq 0 ]; then
        "${LOCK_SCRIPT}" acquire \
            --component "${CHECKER_COMPONENT}" \
            --phase "${CHECKER_PHASE}" \
            --command "${0} --run-id ${RUN_ID} --timeout ${TIMEOUT}" \
            --wait-budget-seconds 60
        LOCK_HELD=1
        trap 'release_checker_lock' EXIT
    fi
}

release_checker_lock() {
    if [ "${LOCK_HELD}" -eq 1 ]; then
        "${LOCK_SCRIPT}" release
        LOCK_HELD=0
        trap - EXIT
    fi
}

refresh_direct_boot_kernel() {
    local candidate="${ASTERINAS_DIR}/target/osdk/iso_root/boot/aster-kernel-osdk-bin"
    local staged_kernel="${IMAGES_DIR}/direct-boot/kernel"
    local file_output

    if [ "${BOOT_MODE}" != "direct-boot" ]; then
        echo "--refresh-direct-boot-kernel requires BOOT_MODE=direct-boot" >&2
        exit 2
    fi

    make -C "${ASTERINAS_DIR}" kernel BOOT_PROTOCOL=linux-efi-handover64

    if [ ! -f "${candidate}" ]; then
        echo "missing direct-boot kernel candidate: ${candidate}" >&2
        exit 1
    fi

    file_output=$(file "${candidate}")
    case "${file_output}" in
        *"Linux kernel x86 boot executable bzImage"*) ;;
        *)
            echo "direct-boot kernel candidate is not a bzImage: ${file_output}" >&2
            exit 1
            ;;
    esac

    mkdir -p "${IMAGES_DIR}/direct-boot"
    cp -- "${candidate}" "${staged_kernel}"
    chmod 0555 "${staged_kernel}"
    DIRECT_BOOT_REFRESH_SOURCE="${candidate}"
    echo "refreshed direct-boot kernel: ${staged_kernel}"
    sha256sum "${staged_kernel}"
}

write_qemu_script() {
    local kernel_params
    local boot_drive_args
    local boot_device_args
    local boot_loader_args

    boot_drive_args="-drive \"if=none,format=raw,id=root0,file=${ROOT_IMAGE}\""
    boot_device_args="-device virtio-blk-pci,bus=pcie.0,addr=0x5,drive=root0,disable-legacy=on,disable-modern=off"

    case "${BOOT_MODE}" in
        direct-boot)
            kernel_params=$(cat "${RUN_DIRECT_BOOT_DIR}/kernel-params")
            boot_loader_args="\
    -kernel \"${RUN_DIRECT_BOOT_DIR}/kernel\" \\
    -initrd \"${RUN_DIRECT_BOOT_DIR}/initrd\" \\
    -append \"${kernel_params}\" \\"
            ;;
        root-disk)
            boot_loader_args="    -boot c \\"
            ;;
        *)
            echo "unsupported BOOT_MODE: ${BOOT_MODE}" >&2
            exit 2
            ;;
    esac

    cat > "${QEMU_SCRIPT}" <<EOF_QEMU
#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0

set -euo pipefail
cd "${RUN_DIR}"

ACCEL_ARGS=""
if [ "\${ENABLE_KVM:-0}" = "1" ]; then
    ACCEL_ARGS="-accel kvm"
fi

qemu-system-x86_64 \\
    -bios /root/ovmf/release/OVMF.fd \\
${boot_loader_args}
    -cpu Icelake-Server,+x2apic \\
    -machine q35,kernel-irqchip=split \\
    -smp "\${SMP:-1}" \\
    -m "\${MEM:-8G}" \\
    --no-reboot \\
    -nographic \\
    -display "vnc=0.0.0.0:\${VNC_PORT:-42}" \\
    -monitor chardev:mux \\
    -chardev stdio,id=mux,mux=on,signal=off,logfile=qemu.log \\
    -netdev user,id=net01 \\
    -device isa-debug-exit,iobase=0xf4,iosize=0x04 \\
    ${boot_drive_args} \\
    ${boot_device_args} \\
    -drive "if=none,format=raw,id=xfstest0,file=${TEST_IMAGE}" \\
    -device virtio-blk-pci,bus=pcie.0,addr=0x6,drive=xfstest0,serial=xfstest-test,disable-legacy=on,disable-modern=off,queue-size=64,num-queues=1,request-merging=off,backend_defaults=off,discard=off,write-zeroes=off,event_idx=off,indirect_desc=off,queue_reset=off,logical_block_size=512,physical_block_size=512 \\
    -drive "if=none,format=raw,id=xfstest1,file=${SCRATCH_IMAGE}" \\
    -device virtio-blk-pci,bus=pcie.0,addr=0x7,drive=xfstest1,serial=xfstest-scratch,disable-legacy=on,disable-modern=off,queue-size=64,num-queues=1,request-merging=off,backend_defaults=off,discard=off,write-zeroes=off,event_idx=off,indirect_desc=off,queue_reset=off,logical_block_size=512,physical_block_size=512 \\
    -object rng-random,id=rng0,filename=/dev/urandom \\
    -device virtio-rng-pci,bus=pcie.0,addr=0x8,disable-legacy=on,disable-modern=off,rng=rng0,event_idx=off,indirect_desc=off,queue_reset=off \\
    -device virtio-net-pci,netdev=net01,disable-legacy=on,disable-modern=off,mrg_rxbuf=off,ctrl_rx=off,ctrl_rx_extra=off,ctrl_vlan=off,ctrl_vq=off,ctrl_guest_offloads=off,ctrl_mac_addr=off,event_idx=off,queue_reset=off,guest_announce=off,indirect_desc=off \\
    -device virtio-serial-pci,disable-legacy=on,disable-modern=off \\
    -device virtconsole,chardev=mux \\
    -serial file:qemu-serial.log \\
    \${ACCEL_ARGS} || exit_code=\$?

exit_code=\${exit_code:-0}
if [ "\${exit_code}" -eq 0 ] || [ "\${exit_code}" -eq 33 ]; then
    exit 0
fi
exit "\${exit_code}"
EOF_QEMU
    chmod +x "${QEMU_SCRIPT}"
}

write_manifest() {
    {
        echo "run_id=${RUN_ID}"
        echo "created_at=$(date -Iseconds)"
        echo "boot_mode=${BOOT_MODE}"
        echo "vnc_port=${VNC_PORT:-42}"
        echo "asterinas_dir=${ASTERINAS_DIR}"
        echo "images_dir=${IMAGES_DIR}"
        echo "run_dir=${RUN_DIR}"
        echo "root_image=${ROOT_IMAGE}"
        echo "test_image=${TEST_IMAGE}"
        echo "scratch_image=${SCRATCH_IMAGE}"
        echo "kernel=${RUN_DIRECT_BOOT_DIR}/kernel"
        echo "initrd=${RUN_DIRECT_BOOT_DIR}/initrd"
        echo "kernel_params=${RUN_DIRECT_BOOT_DIR}/kernel-params"
        echo "refresh_direct_boot_kernel=${REFRESH_DIRECT_BOOT_KERNEL}"
        if [ -n "${DIRECT_BOOT_REFRESH_SOURCE}" ]; then
            echo "direct_boot_refresh_source=${DIRECT_BOOT_REFRESH_SOURCE}"
        fi
        echo "timeout=${TIMEOUT}"
        echo "test_case=exfat_refactor_mount_smoke"
        echo "test_dev=/dev/vdb"
        echo "scratch_dev=/dev/vdc"
        echo
        echo "[sha256]"
        sha256sum \
            "${ROOT_IMAGE}" \
            "${TEST_IMAGE}" \
            "${SCRATCH_IMAGE}" \
            "${RUN_DIRECT_BOOT_DIR}/kernel" \
            "${RUN_DIRECT_BOOT_DIR}/initrd" \
            "${RUN_DIRECT_BOOT_DIR}/kernel-params"
    } > "${MANIFEST}"
}

run_smoke() {
    cd "${ASTERINAS_DIR}/test/nixos/tests/xfstests"
    NIXOS_TEST_TIMEOUT="${TIMEOUT}" \
    XFSTESTS_TEST_DEV="/dev/vdb" \
    XFSTESTS_SCRATCH_DEV="/dev/vdc" \
    cargo run -- \
        --qemu-cmd "${QEMU_SCRIPT}" \
        --test exfat_refactor_mount_smoke
}

main() {
    if [ "${PRUNE_OLD_IMAGES}" -eq 1 ]; then
        prune_old_run_images
        if [ "${PREPARE_ONLY}" -eq 0 ] && [ "${RUN_ID_GIVEN}" -eq 0 ]; then
            exit 0
        fi
    fi

    mkdir -p "${RUN_DIRECT_BOOT_DIR}"
    if [ "${REFRESH_DIRECT_BOOT_KERNEL}" -eq 1 ]; then
        acquire_checker_lock
        refresh_direct_boot_kernel
    elif [ "${PREPARE_ONLY}" -eq 0 ]; then
        acquire_checker_lock
    fi

    copy_image "${IMAGES_DIR}/root-base.img" "${ROOT_IMAGE}"
    copy_image "${IMAGES_DIR}/test-base.img" "${TEST_IMAGE}"
    copy_image "${IMAGES_DIR}/scratch-base.img" "${SCRATCH_IMAGE}"
    copy_image "${IMAGES_DIR}/direct-boot/kernel" "${RUN_DIRECT_BOOT_DIR}/kernel"
    copy_image "${IMAGES_DIR}/direct-boot/initrd" "${RUN_DIRECT_BOOT_DIR}/initrd"
    copy_image "${IMAGES_DIR}/direct-boot/kernel-params" "${RUN_DIRECT_BOOT_DIR}/kernel-params"
    write_qemu_script
    write_manifest

    echo "Run directory: ${RUN_DIR}"
    echo "Reproduce: ${0} --run-id ${RUN_ID} --timeout ${TIMEOUT}"

    if [ "${PREPARE_ONLY}" -eq 1 ]; then
        echo "prepare_only=1" >> "${MANIFEST}"
        echo "Prepared run directory only; QEMU/test execution was skipped."
        release_checker_lock
        exit 0
    fi

    set +e
    run_smoke 2>&1 | tee "${STDOUT_LOG}"
    status=${PIPESTATUS[0]}
    set -e

    release_checker_lock

    echo "exit_status=${status}" >> "${MANIFEST}"
    exit "${status}"
}

main "$@"
