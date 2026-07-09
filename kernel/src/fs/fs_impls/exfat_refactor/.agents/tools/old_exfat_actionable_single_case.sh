#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0

set -euo pipefail

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
agents_dir=$(CDPATH= cd -- "$script_dir/.." && pwd)
run_tool="$script_dir/xfstests_run.sh"
default_list="$script_dir/old_exfat_actionable_51.list"

phase_prefix="old_exfat_single_case_20260707"
component="old_exfat_comparison_20260707"
disk_size="8G"
mem_size="12G"
container="codex-asterinas-dev"
repo_dir="/root/asterinas"
test_dev="/dev/vdc"
scratch_dev="/dev/vdd"
list_path="$default_list"
out_root="$agents_dir/tmp"
summary_file=""
case_name=""
from_case=""
run_all=0
list_only=0
dry_run=0

declare -a all_cases=()

usage() {
    cat <<'EOF'
Usage:
  old_exfat_actionable_single_case.sh CASE [OPTIONS]
  old_exfat_actionable_single_case.sh --all [OPTIONS]
  old_exfat_actionable_single_case.sh --list

Runs the old-exfat actionable comparison surface strictly one xfstests case at a
time by delegating to `xfstests_run.sh case`.

Positional arguments:
  CASE                         One testcase from `old_exfat_actionable_51.list`.

Options:
  --all                        Run the entire list sequentially, but still one
                               xfstests case per invocation.
  --from CASE                  With `--all`, start from this case.
  --list                       Print the authoritative 51-case list and exit.
  --list-file PATH             Default: .agents/tools/old_exfat_actionable_51.list
  --out-root PATH              Default: .agents/tmp
  --summary-file PATH          Default: <out-root>/old_exfat_actionable_single_case_results.tsv
  --phase-prefix PREFIX        Default: old_exfat_single_case_20260707
  --component ID               Default: old_exfat_comparison_20260707
  --disk SIZE                  Default: 8G
  --mem SIZE                   Default: 12G
  --container NAME             Default: codex-asterinas-dev
  --repo-dir PATH              Default: /root/asterinas
  --test-dev DEV               Default: /dev/vdc
  --scratch-dev DEV            Default: /dev/vdd
  --dry-run                    Forwarded to `xfstests_run.sh`.
  -h, --help                   Show this help text.

Outputs:
  - one receipt directory per executed case under `--out-root`
  - an appended TSV summary file for manual tracking
  - one TSV line on stdout per executed case:
      case result receipt_exit launcher_exit scratch_inconsistent test_inconsistent scratch_taint receipt
EOF
}

timestamp() {
    date '+%Y%m%d-%H%M%S'
}

sanitize_name() {
    printf '%s' "$1" | tr -c 'A-Za-z0-9_.-' '_' | cut -c 1-180
}

load_case_list() {
    local line

    [ -f "$list_path" ] || {
        printf 'missing case list: %s\n' "$list_path" >&2
        exit 1
    }

    all_cases=()
    while IFS= read -r line || [ -n "$line" ]; do
        line=${line%$'\r'}
        if [[ "$line" =~ ^[[:space:]]*$ ]]; then
            continue
        fi
        if [[ "$line" =~ ^[[:space:]]*# ]]; then
            continue
        fi
        all_cases+=("$line")
    done < "$list_path"

    [ "${#all_cases[@]}" -gt 0 ] || {
        printf 'case list is empty: %s\n' "$list_path" >&2
        exit 1
    }
}

case_exists_in_list() {
    local candidate=$1
    local listed_case

    for listed_case in "${all_cases[@]}"; do
        if [ "$listed_case" = "$candidate" ]; then
            return 0
        fi
    done

    return 1
}

parse_args() {
    while [ $# -gt 0 ]; do
        case "$1" in
            -h|--help)
                usage
                exit 0
                ;;
            --all)
                run_all=1
                shift
                ;;
            --from)
                from_case=$2
                shift 2
                ;;
            --list)
                list_only=1
                shift
                ;;
            --list-file)
                list_path=$2
                shift 2
                ;;
            --out-root)
                out_root=$2
                shift 2
                ;;
            --summary-file)
                summary_file=$2
                shift 2
                ;;
            --phase-prefix)
                phase_prefix=$2
                shift 2
                ;;
            --component)
                component=$2
                shift 2
                ;;
            --disk)
                disk_size=$2
                shift 2
                ;;
            --mem)
                mem_size=$2
                shift 2
                ;;
            --container)
                container=$2
                shift 2
                ;;
            --repo-dir)
                repo_dir=$2
                shift 2
                ;;
            --test-dev)
                test_dev=$2
                shift 2
                ;;
            --scratch-dev)
                scratch_dev=$2
                shift 2
                ;;
            --dry-run)
                dry_run=1
                shift
                ;;
            -*)
                usage >&2
                exit 64
                ;;
            *)
                if [ -n "$case_name" ]; then
                    usage >&2
                    exit 64
                fi
                case_name=$1
                shift
                ;;
        esac
    done

    if [ -z "$summary_file" ]; then
        summary_file="$out_root/old_exfat_actionable_single_case_results.tsv"
    fi
}

ensure_summary_header() {
    mkdir -p "$out_root"
    if [ ! -f "$summary_file" ]; then
        printf 'case\tresult\treceipt_exit\tlauncher_exit\tscratch_inconsistent\ttest_inconsistent\tscratch_taint\treceipt\n' > "$summary_file"
    fi
}

receipt_has() {
    local receipt_dir=$1
    local pattern=$2
    local log_path

    for log_path in \
        "$receipt_dir/qemu.log" \
        "$receipt_dir/stdout-stderr.log" \
        "$receipt_dir/execution-proof.txt"
    do
        if [ -f "$log_path" ] && grep -Eq "$pattern" "$log_path"; then
            return 0
        fi
    done

    return 1
}

receipt_has_logs() {
    local receipt_dir=$1
    local log_path

    for log_path in \
        "$receipt_dir/qemu.log" \
        "$receipt_dir/stdout-stderr.log" \
        "$receipt_dir/execution-proof.txt"
    do
        if [ -s "$log_path" ]; then
            return 0
        fi
    done

    return 1
}

receipt_flag() {
    local receipt_dir=$1
    local pattern=$2

    if ! receipt_has_logs "$receipt_dir"; then
        printf 'unknown\n'
        return 0
    fi

    if receipt_has "$receipt_dir" "$pattern"; then
        printf 'yes\n'
    else
        printf 'no\n'
    fi
}

read_receipt_exit_status() {
    local receipt_dir=$1

    if [ -f "$receipt_dir/command-exit-status.txt" ]; then
        tr -d '\r\n' < "$receipt_dir/command-exit-status.txt"
    else
        printf 'missing'
    fi
}

classify_result() {
    local receipt_dir=$1
    local target_case=$2

    if [ "$dry_run" -eq 1 ]; then
        printf 'dry-run\n'
        return 0
    fi

    if receipt_has "$receipt_dir" "^Passed all 1 tests$|^All conformance tests passed\\.$"; then
        printf 'pass\n'
        return 0
    fi

    if receipt_has "$receipt_dir" "^Not run: ${target_case}$|\\[not run\\]"; then
        printf 'notrun\n'
        return 0
    fi

    if receipt_has "$receipt_dir" "^Failures: ${target_case}$|^Failed [0-9]+ of [0-9]+ tests$|\\[failed, exit status |Error: xfstests failed"; then
        printf 'fail\n'
        return 0
    fi

    printf 'unknown\n'
}

print_stdout_header() {
    printf 'case\tresult\treceipt_exit\tlauncher_exit\tscratch_inconsistent\ttest_inconsistent\tscratch_taint\treceipt\n'
}

append_summary_line() {
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
        "$1" "$2" "$3" "$4" "$5" "$6" "$7" "$8" \
        | tee -a "$summary_file"
}

run_one_case() {
    local target_case=$1
    local safe_case receipt_dir phase tag launcher_exit receipt_exit result scratch_inconsistent test_inconsistent scratch_taint
    local -a cmd=()

    safe_case=$(sanitize_name "$target_case")
    receipt_dir="$out_root/$(timestamp)-old_exfat-single-case-${safe_case}"
    phase="${phase_prefix}_${safe_case}"
    tag="old_exfat_single_case_${safe_case}"

    cmd=(
        "$run_tool" case "$target_case"
        --phase "$phase"
        --component "$component"
        --tag "$tag"
        --fstyp exfat
        --mkfs mkfs.exfat
        --fsck fsck.exfat
        --disk "$disk_size"
        --mem "$mem_size"
        --container "$container"
        --repo-dir "$repo_dir"
        --test-dev "$test_dev"
        --scratch-dev "$scratch_dev"
        --out-dir "$receipt_dir"
    )

    if [ "$dry_run" -eq 1 ]; then
        cmd+=(--dry-run)
    fi

    set +e
    "${cmd[@]}"
    launcher_exit=$?
    set -e

    receipt_exit=$(read_receipt_exit_status "$receipt_dir")
    result=$(classify_result "$receipt_dir" "$target_case")
    scratch_inconsistent=$(receipt_flag "$receipt_dir" '_check_generic_filesystem: filesystem on /dev/vdd is inconsistent')
    test_inconsistent=$(receipt_flag "$receipt_dir" '_check_generic_filesystem: filesystem on /dev/vdc is inconsistent')
    scratch_taint=$(receipt_flag "$receipt_dir" '/opt/xfstests/scratch/testfile: File exists')

    append_summary_line \
        "$target_case" \
        "$result" \
        "$receipt_exit" \
        "$launcher_exit" \
        "$scratch_inconsistent" \
        "$test_inconsistent" \
        "$scratch_taint" \
        "$receipt_dir"

    return "$launcher_exit"
}

run_all_cases() {
    local listed_case
    local started=0
    local overall_exit=0

    for listed_case in "${all_cases[@]}"; do
        if [ -n "$from_case" ] && [ "$started" -eq 0 ]; then
            if [ "$listed_case" = "$from_case" ]; then
                started=1
            else
                continue
            fi
        else
            started=1
        fi

        if ! run_one_case "$listed_case"; then
            overall_exit=1
        fi
    done

    return "$overall_exit"
}

main() {
    parse_args "$@"
    load_case_list

    if [ "$list_only" -eq 1 ]; then
        printf '%s\n' "${all_cases[@]}"
        exit 0
    fi

    if [ "$run_all" -eq 1 ] && [ -n "$case_name" ]; then
        usage >&2
        exit 64
    fi

    if [ "$run_all" -eq 0 ] && [ -z "$case_name" ]; then
        usage >&2
        exit 64
    fi

    if [ -n "$from_case" ] && ! case_exists_in_list "$from_case"; then
        printf 'unknown --from case: %s\n' "$from_case" >&2
        exit 64
    fi

    if [ -n "$case_name" ] && ! case_exists_in_list "$case_name"; then
        printf 'case is not in %s: %s\n' "$list_path" "$case_name" >&2
        exit 64
    fi

    ensure_summary_header
    print_stdout_header

    if [ "$run_all" -eq 1 ]; then
        run_all_cases
        exit $?
    fi

    run_one_case "$case_name"
}

main "$@"
