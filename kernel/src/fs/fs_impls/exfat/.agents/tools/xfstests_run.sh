#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0

set -euo pipefail

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
agents_dir=$(CDPATH= cd -- "$script_dir/.." && pwd)
exfat_refactor_dir=$(CDPATH= cd -- "$agents_dir/.." && pwd)
asterinas_dir=$(CDPATH= cd -- "$exfat_refactor_dir/../../../../.." && pwd)
lock_tool="$script_dir/checker_lock.sh"

container="codex-asterinas-dev"
repo_dir="/root/asterinas"
component="xfstests_linux_baseline_20260624"
phase=""
tag=""
disk_size="8G"
mem_size="12G"
timeout=""
release="1"
fstyp="exfat_refactor"
mkfs_prog="mkfs.exfat"
mkfs_options=""
fsck_prog="fsck.exfat"
mount_options=""
test_dev="/dev/vdd"
scratch_dev="/dev/vde"
out_dir=""
runlist_name=""
runlist_path=""
direct_command=""
direct_command_file=""
direct_workdir="/opt/xfstests/test"
wait_budget_seconds=0
retry_seconds=60
use_lock=1
backend="upstream"
reuse_images_from=""
prepare_only=0
keep_going=0
dry_run=0
declare -a tests=()

usage() {
    cat <<'EOF'
Usage:
  xfstests_run.sh case TEST --phase PHASE [OPTIONS]
  xfstests_run.sh batch --tests TEST[,TEST...] --phase PHASE [OPTIONS]
  xfstests_run.sh direct (--cmd TEXT | --cmd-file PATH) --phase PHASE [OPTIONS]
  xfstests_run.sh prebuilt --phase PHASE [OPTIONS]

Short Checker-owned xfstests harness for exfat_refactor.

Subcommands:
  case      Run one xfstests case, e.g. generic/694.
  batch     Run a comma-separated or repeated list of xfstests cases.
  direct    Run the upstream direct diagnostic command mode.
  prebuilt  Delegate to the historical prebuilt-image diagnostic runner.

Common options:
  --phase PHASE               Receipt / lock phase. Required.
  --component ID              Receipt grouping. Default: xfstests_linux_baseline_20260624.
  --tag TAG                   Human label for receipt and generated runlist names.
  --disk SIZE                 XFSTESTS_DISK_SIZE. Default: 2G.
  --mem SIZE                  MEM. Default: 12G.
  --timeout TIMEOUT           Prefix the run with timeout(1), e.g. 90min.
  --container NAME            Docker container. Default: codex-asterinas-dev.
  --repo-dir PATH             Repository path inside container. Default: /root/asterinas.
  --out-dir PATH              Host receipt dir. Default: .agents/tmp/<timestamp>-<phase>_xfstests.
  --wait-budget-seconds N     Checker lock wait budget. Default: 0.
  --retry-seconds N           Checker lock retry interval. Default: 60.
  --no-lock                   Do not acquire checker_lock.sh.
  --keep-going                Continue batch cases after failure.
  --dry-run                   Write receipt/reproduce files without running QEMU.

Filesystem defaults:
  --fstyp NAME                Default: exfat_refactor.
  --mkfs PROG                Default: mkfs.exfat.
  --mkfs-options OPTIONS     Default: empty.
  --fsck PROG                Default: fsck.exfat.
  --mount-options OPTIONS    Default: empty.
  --test-dev DEV             Default: /dev/vdc.
  --scratch-dev DEV          Default: /dev/vdd.

Direct mode:
  --cmd TEXT                  Diagnostic shell command.
  --cmd-file PATH             Read diagnostic shell command from file.
  --direct-workdir PATH       Default: /opt/xfstests/test.

Prebuilt mode:
  --reuse-images-from RUN_ID  Pass through to xfstests_prebuilt_runner.sh.
  --prepare-only             Prepare prebuilt run directory without executing.

Outputs:
  - checker lock acquire/release receipts
  - generated runlist content
  - exact reproduce command
  - qemu.log and qemu-serial.log copied from the container when present
  - execution-proof.txt and summary.tsv
EOF
}

timestamp() {
    date '+%Y%m%d-%H%M%S'
}

sanitize_name() {
    printf '%s' "$1" | tr -c 'A-Za-z0-9_.-' '_' | cut -c 1-180
}

shell_quote() {
    printf '%q' "$1"
}

append_test_list() {
    local raw_list=$1
    local old_ifs=$IFS
    local test_name

    IFS=,
    for test_name in $raw_list; do
        [ -n "$test_name" ] || continue
        tests+=("$test_name")
    done
    IFS=$old_ifs
}

parse_common_option() {
    case "${1:-}" in
        --phase)
            [ $# -ge 2 ] || { usage >&2; exit 64; }
            phase=$2
            return 2
            ;;
        --component)
            [ $# -ge 2 ] || { usage >&2; exit 64; }
            component=$2
            return 2
            ;;
        --tag)
            [ $# -ge 2 ] || { usage >&2; exit 64; }
            tag=$2
            return 2
            ;;
        --disk)
            [ $# -ge 2 ] || { usage >&2; exit 64; }
            disk_size=$2
            return 2
            ;;
        --mem)
            [ $# -ge 2 ] || { usage >&2; exit 64; }
            mem_size=$2
            return 2
            ;;
        --timeout)
            [ $# -ge 2 ] || { usage >&2; exit 64; }
            timeout=$2
            return 2
            ;;
        --container)
            [ $# -ge 2 ] || { usage >&2; exit 64; }
            container=$2
            return 2
            ;;
        --repo-dir)
            [ $# -ge 2 ] || { usage >&2; exit 64; }
            repo_dir=$2
            return 2
            ;;
        --out-dir)
            [ $# -ge 2 ] || { usage >&2; exit 64; }
            out_dir=$2
            return 2
            ;;
        --wait-budget-seconds)
            [ $# -ge 2 ] || { usage >&2; exit 64; }
            wait_budget_seconds=$2
            return 2
            ;;
        --retry-seconds)
            [ $# -ge 2 ] || { usage >&2; exit 64; }
            retry_seconds=$2
            return 2
            ;;
        --no-lock)
            use_lock=0
            return 1
            ;;
        --keep-going)
            keep_going=1
            return 1
            ;;
        --dry-run)
            dry_run=1
            return 1
            ;;
        --fstyp)
            [ $# -ge 2 ] || { usage >&2; exit 64; }
            fstyp=$2
            return 2
            ;;
        --mkfs)
            [ $# -ge 2 ] || { usage >&2; exit 64; }
            mkfs_prog=$2
            return 2
            ;;
        --mkfs-options)
            [ $# -ge 2 ] || { usage >&2; exit 64; }
            mkfs_options=$2
            return 2
            ;;
        --fsck)
            [ $# -ge 2 ] || { usage >&2; exit 64; }
            fsck_prog=$2
            return 2
            ;;
        --mount-options)
            [ $# -ge 2 ] || { usage >&2; exit 64; }
            mount_options=$2
            return 2
            ;;
        --test-dev)
            [ $# -ge 2 ] || { usage >&2; exit 64; }
            test_dev=$2
            return 2
            ;;
        --scratch-dev)
            [ $# -ge 2 ] || { usage >&2; exit 64; }
            scratch_dev=$2
            return 2
            ;;
        *)
            return 0
            ;;
    esac
}

parse_args() {
    [ $# -ge 1 ] || { usage >&2; exit 64; }

    case "$1" in
        -h|--help|help)
            usage
            exit 0
            ;;
        case|batch|direct|prebuilt)
            subcommand=$1
            shift
            ;;
        *)
            usage >&2
            exit 64
            ;;
    esac

    if [ "$subcommand" = "case" ]; then
        [ $# -ge 1 ] || { usage >&2; exit 64; }
        tests+=("$1")
        shift
    fi

    while [ $# -gt 0 ]; do
        parse_common_option "$@" || parsed_count=$?
        parsed_count=${parsed_count:-0}
        if [ "$parsed_count" -gt 0 ]; then
            shift "$parsed_count"
            unset parsed_count
            continue
        fi
        unset parsed_count

        case "$1" in
            --tests)
                [ $# -ge 2 ] || { usage >&2; exit 64; }
                append_test_list "$2"
                shift 2
                ;;
            --cmd)
                [ $# -ge 2 ] || { usage >&2; exit 64; }
                direct_command=$2
                shift 2
                ;;
            --cmd-file)
                [ $# -ge 2 ] || { usage >&2; exit 64; }
                direct_command_file=$2
                shift 2
                ;;
            --direct-workdir)
                [ $# -ge 2 ] || { usage >&2; exit 64; }
                direct_workdir=$2
                shift 2
                ;;
            --reuse-images-from)
                [ $# -ge 2 ] || { usage >&2; exit 64; }
                reuse_images_from=$2
                shift 2
                ;;
            --prepare-only)
                prepare_only=1
                shift
                ;;
            *)
                usage >&2
                exit 64
                ;;
        esac
    done

    [ -n "$phase" ] || { usage >&2; exit 64; }

    case "$subcommand" in
        case|batch)
            [ "${#tests[@]}" -gt 0 ] || { usage >&2; exit 64; }
            ;;
        direct)
            [ -n "$direct_command" ] || [ -n "$direct_command_file" ] || { usage >&2; exit 64; }
            ;;
        prebuilt)
            backend="prebuilt"
            ;;
    esac

    if [ -z "$tag" ]; then
        tag=$phase
    fi

    if [ -z "$out_dir" ]; then
        out_dir="$agents_dir/tmp/$(timestamp)-$(sanitize_name "$phase")_xfstests"
    fi
}

docker_bash() {
    local command=$1

    if command -v docker >/dev/null 2>&1; then
        docker exec "$container" bash -lc "$command"
        return
    fi

    bash -lc "$command"
}

copy_container_file_if_present() {
    local container_path=$1
    local host_path=$2
    local quoted_path

    quoted_path=$(shell_quote "$container_path")
    if docker_bash "[ -f $quoted_path ]"; then
        if command -v docker >/dev/null 2>&1; then
            docker cp "$container:$container_path" "$host_path"
        else
            cp "$container_path" "$host_path"
        fi
        return 0
    fi

    printf 'missing container file: %s\n' "$container_path" > "$host_path.missing"
    return 1
}

acquire_lock() {
    [ "$use_lock" -eq 1 ] || return 0

    "$lock_tool" acquire \
        --component "$component" \
        --phase "$phase" \
        --command "$0 $subcommand --phase $phase --tag $tag" \
        --retry-seconds "$retry_seconds" \
        --wait-budget-seconds "$wait_budget_seconds" \
        > "$out_dir/checker-lock-acquire.txt"
}

release_lock() {
    [ "$use_lock" -eq 1 ] || return 0

    "$lock_tool" release > "$out_dir/checker-lock-release.txt"
}

write_runlist() {
    local runlist_dir="$asterinas_dir/test/initramfs/src/conformance/xfstests"
    local safe_tag
    local test_name

    safe_tag=$(sanitize_name "$tag")
    runlist_name="agent_${safe_tag}.list"
    if [ "$dry_run" -eq 1 ]; then
        runlist_path="$out_dir/$runlist_name"
    else
        runlist_path="$runlist_dir/$runlist_name"
    fi

    : > "$runlist_path"
    for test_name in "${tests[@]}"; do
        printf '%s\n' "$test_name" >> "$runlist_path"
    done

    cp "$runlist_path" "$out_dir/runlist.list"
    cp "$runlist_path" "$out_dir/runlist-content.txt"
}

build_make_command() {
    local quoted_repo
    local command_body
    local direct_b64=""
    local timeout_shell_command=""

    quoted_repo=$(shell_quote "$repo_dir")

    if [ "$subcommand" = "direct" ]; then
        if [ -n "$direct_command_file" ]; then
            direct_command=$(cat "$direct_command_file")
        fi
        printf '%s' "$direct_command" > "$out_dir/direct-command.txt"
        direct_b64=$(printf '%s' "$direct_command" | base64 -w0)
    fi

    command_body=$(printf 'cd %s && : > qemu.log && : > qemu-serial.log && make run_kernel AUTO_TEST=conformance CONFORMANCE_TEST_SUITE=xfstests RELEASE=%s MEM=%s XFSTESTS_DISK_SIZE=%s XFSTESTS_FSTYP=%s XFSTESTS_MKFS=%s XFSTESTS_MKFS_OPTIONS=%s XFSTESTS_FSCK=%s XFSTESTS_MOUNT_OPTIONS=%s XFSTESTS_TEST_DEV=%s XFSTESTS_SCRATCH_DEV=%s' \
        "$quoted_repo" \
        "$(shell_quote "$release")" \
        "$(shell_quote "$mem_size")" \
        "$(shell_quote "$disk_size")" \
        "$(shell_quote "$fstyp")" \
        "$(shell_quote "$mkfs_prog")" \
        "$(shell_quote "$mkfs_options")" \
        "$(shell_quote "$fsck_prog")" \
        "$(shell_quote "$mount_options")" \
        "$(shell_quote "$test_dev")" \
        "$(shell_quote "$scratch_dev")")

    if [ "$subcommand" = "direct" ]; then
        command_body+=$(printf ' XFSTESTS_DIRECT_COMMAND_B64=%s XFSTESTS_DIRECT_WORKDIR=%s' \
            "$(shell_quote "$direct_b64")" \
            "$(shell_quote "$direct_workdir")")
    else
        command_body+=$(printf ' XFSTESTS_RUNLIST=/opt/xfstests/%s' "$(shell_quote "$runlist_name")")
    fi

    if [ -n "$timeout" ]; then
        # When timeout terminates the wrapper shell, kill the whole foreground
        # process group so `make run_kernel` cannot leave orphaned QEMU/image
        # lock holders behind for the next suffix-continuation batch.
        timeout_shell_command=$(printf 'trap '\''kill 0'\'' TERM INT; %s' "$command_body")
        printf 'timeout --kill-after=30s %s bash -lc %s' \
            "$(shell_quote "$timeout")" \
            "$(shell_quote "$timeout_shell_command")"
    else
        printf '%s' "$command_body"
    fi
}

archive_execution_files() {
    local qemu_log="$repo_dir/qemu.log"
    local serial_log="$repo_dir/qemu-serial.log"

    copy_container_file_if_present "$qemu_log" "$out_dir/qemu.log" || true
    copy_container_file_if_present "$serial_log" "$out_dir/qemu-serial.log" || true

    {
        echo "xfstests receipt: $phase"
        echo "subcommand=$subcommand"
        echo "fstyp=$fstyp"
        echo "disk_size=$disk_size"
        echo "mem_size=$mem_size"
        echo "test_dev=$test_dev"
        echo "scratch_dev=$scratch_dev"
        if [ -f "$out_dir/runlist-content.txt" ]; then
            echo
            echo "[runlist]"
            cat "$out_dir/runlist-content.txt"
        fi
        if [ -f "$out_dir/qemu.log" ]; then
            echo
            echo "[qemu selected lines]"
            grep -E 'xfstests|FSTYP|Ran:|Passed all|Failures:|Not run:|QA output|Silence is golden|Uncaught panic|panicked|TCG|deadlock|All conformance tests passed|Error: xfstests failed' "$out_dir/qemu.log" || true
        fi
    } > "$out_dir/execution-proof.txt"
}

run_upstream() {
    local command
    local status

    if [ "$subcommand" != "direct" ]; then
        write_runlist
    fi

    command=$(build_make_command)
    if command -v docker >/dev/null 2>&1; then
        printf 'docker exec %s bash -lc %s\n' "$container" "$(shell_quote "$command")" > "$out_dir/reproduce-command.txt"
    else
        printf 'bash -lc %s\n' "$(shell_quote "$command")" > "$out_dir/reproduce-command.txt"
    fi

    if [ "$dry_run" -eq 1 ]; then
        {
            echo "dry_run=1"
            echo "No QEMU execution was attempted."
        } > "$out_dir/execution-proof.txt"
        printf 'batch\texit_status\nmain\tdry-run\n' > "$out_dir/summary.tsv"
        return 0
    fi

    set +e
    docker_bash "$command" > "$out_dir/stdout-stderr.log" 2>&1
    status=$?
    set -e

    printf '%s\n' "$status" > "$out_dir/command-exit-status.txt"
    printf 'batch\texit_status\nmain\t%s\n' "$status" > "$out_dir/summary.tsv"
    archive_execution_files

    return "$status"
}

run_prebuilt() {
    local args=(--run-id "$(basename "$out_dir")" --timeout "${timeout:-60min}")

    if [ "$use_lock" -eq 0 ]; then
        args+=(--no-lock)
    fi
    if [ "$prepare_only" -eq 1 ]; then
        args+=(--prepare-only)
    fi
    if [ -n "$reuse_images_from" ]; then
        args+=(--reuse-images-from "$reuse_images_from")
    fi

    printf '%s %s\n' "$script_dir/xfstests_prebuilt_runner.sh" "${args[*]}" > "$out_dir/reproduce-command.txt"
    "$script_dir/xfstests_prebuilt_runner.sh" "${args[@]}" > "$out_dir/stdout-stderr.log" 2>&1
}

main() {
    parse_args "$@"
    mkdir -p "$out_dir"

    {
        echo "phase=$phase"
        echo "component=$component"
        echo "backend=$backend"
        echo "created_at=$(date -Iseconds)"
        echo "out_dir=$out_dir"
    } > "$out_dir/manifest.txt"

    if [ "$backend" = "prebuilt" ]; then
        run_prebuilt
        exit $?
    fi

    acquire_lock
    trap 'release_lock' EXIT

    if [ "$keep_going" -eq 0 ] || [ "$subcommand" = "direct" ] || [ "${#tests[@]}" -le 1 ]; then
        run_upstream
        status=$?
    else
        status=0
        all_tests=("${tests[@]}")
        tests=()
        for test_name in "${all_tests[@]}"; do
            tests=("$test_name")
            tag="${phase}_$(sanitize_name "$test_name")"
            if ! run_upstream; then
                status=1
            fi
        done
    fi

    release_lock
    trap - EXIT
    exit "$status"
}

main "$@"
