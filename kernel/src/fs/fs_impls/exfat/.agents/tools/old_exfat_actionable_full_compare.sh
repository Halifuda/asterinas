#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0

set -euo pipefail

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
agents_dir=$(CDPATH= cd -- "$script_dir/.." && pwd)
run_tool="$script_dir/xfstests_run.sh"
lock_tool="$script_dir/checker_lock.sh"

phase="old_exfat_actionable_full_compare_51"
component="old_exfat_comparison_20260707"
disk_size="8G"
mem_size="12G"
timeout=""
container="codex-asterinas-dev"
repo_dir="/root/asterinas"
wait_budget_seconds=0
retry_seconds=60
dry_run=0
out_dir=""

declare -a all_tests=(
    generic/001
    generic/006
    generic/007
    generic/011
    generic/013
    generic/028
    generic/030
    generic/035
    generic/100
    generic/124
    generic/132
    generic/133
    generic/135
    generic/141
    generic/192
    generic/221
    generic/246
    generic/247
    generic/248
    generic/249
    generic/257
    generic/308
    generic/309
    generic/313
    generic/339
    generic/340
    generic/344
    generic/345
    generic/346
    generic/354
    generic/393
    generic/406
    generic/412
    generic/428
    generic/437
    generic/438
    generic/443
    generic/452
    generic/532
    generic/609
    generic/615
    generic/634
    generic/637
    generic/638
    generic/639
    generic/642
    generic/676
    generic/694
    generic/701
    generic/707
    generic/708
)

sanitize_name() {
    printf '%s' "$1" | tr -c 'A-Za-z0-9_.-' '_' | cut -c 1-180
}

timestamp() {
    date '+%Y%m%d-%H%M%S'
}

usage() {
    cat <<'EOF'
Usage:
  old_exfat_actionable_full_compare.sh [OPTIONS]

Runs the 51-case actionable old-exfat comparison set only.
This wrapper hardcodes `FSTYP=exfat` and verifies that every receipt proves it.

Options:
  --phase PHASE               Default: old_exfat_actionable_full_compare_51
  --component ID              Default: old_exfat_comparison_20260707
  --disk SIZE                 Default: 8G
  --mem SIZE                  Default: 12G
  --timeout TIMEOUT           Pass through to xfstests_run.sh
  --container NAME            Default: codex-asterinas-dev
  --repo-dir PATH             Default: /root/asterinas
  --out-dir PATH              Default: .agents/tmp/<timestamp>-<phase>_old_exfat
  --wait-budget-seconds N     Default: 0
  --retry-seconds N           Default: 60
  --dry-run                   Do not execute QEMU
EOF
}

parse_args() {
    while [ $# -gt 0 ]; do
        case "$1" in
            -h|--help|help)
                usage
                exit 0
                ;;
            --phase)
                phase=$2
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
            --timeout)
                timeout=$2
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
            --out-dir)
                out_dir=$2
                shift 2
                ;;
            --wait-budget-seconds)
                wait_budget_seconds=$2
                shift 2
                ;;
            --retry-seconds)
                retry_seconds=$2
                shift 2
                ;;
            --dry-run)
                dry_run=1
                shift
                ;;
            *)
                usage >&2
                exit 64
                ;;
        esac
    done

    if [ -z "$out_dir" ]; then
        out_dir="$agents_dir/tmp/$(timestamp)-$(sanitize_name "$phase")_old_exfat"
    fi
}

join_csv() {
    local first=1
    local item

    for item in "$@"; do
        if [ "$first" -eq 1 ]; then
            printf '%s' "$item"
            first=0
        else
            printf ',%s' "$item"
        fi
    done
}

find_qemu_log() {
    local receipt_dir=$1

    if [ -f "$receipt_dir/qemu.log" ]; then
        printf '%s\n' "$receipt_dir/qemu.log"
        return 0
    fi

    if [ -f "$receipt_dir/postrun/qemu.log" ]; then
        printf '%s\n' "$receipt_dir/postrun/qemu.log"
        return 0
    fi

    return 1
}

find_stdout_log() {
    local receipt_dir=$1

    if [ -f "$receipt_dir/stdout-stderr.log" ]; then
        printf '%s\n' "$receipt_dir/stdout-stderr.log"
        return 0
    fi

    if [ -f "$receipt_dir/stdout-stderr.txt" ]; then
        printf '%s\n' "$receipt_dir/stdout-stderr.txt"
        return 0
    fi

    return 1
}

for_each_runtime_log() {
    local receipt_dir=$1
    local log_path

    for log_path in \
        "$receipt_dir/qemu.log" \
        "$receipt_dir/postrun/qemu.log" \
        "$receipt_dir/stdout-stderr.log" \
        "$receipt_dir/stdout-stderr.txt"
    do
        if [ -f "$log_path" ] && [ -s "$log_path" ]; then
            printf '%s\n' "$log_path"
        fi
    done
}

find_runtime_log_matching() {
    local receipt_dir=$1
    local pattern=$2
    local log_path

    while IFS= read -r log_path; do
        if grep -Eq "$pattern" "$log_path"; then
            printf '%s\n' "$log_path"
            return 0
        fi
    done < <(for_each_runtime_log "$receipt_dir")

    return 1
}

find_runtime_log() {
    local receipt_dir=$1
    local log_path

    while IFS= read -r log_path; do
        printf '%s\n' "$log_path"
        return 0
    done < <(for_each_runtime_log "$receipt_dir")

    if [ -f "$receipt_dir/qemu.log" ]; then
        printf '%s\n' "$receipt_dir/qemu.log"
        return 0
    fi
    if [ -f "$receipt_dir/postrun/qemu.log" ]; then
        printf '%s\n' "$receipt_dir/postrun/qemu.log"
        return 0
    fi
    if [ -f "$receipt_dir/stdout-stderr.log" ]; then
        printf '%s\n' "$receipt_dir/stdout-stderr.log"
        return 0
    fi
    if [ -f "$receipt_dir/stdout-stderr.txt" ]; then
        printf '%s\n' "$receipt_dir/stdout-stderr.txt"
        return 0
    fi

    return 1
}

verify_old_exfat_receipt() {
    local receipt_dir=$1
    local runtime_log
    local saw_guest_fstyp_line=0

    runtime_log=$(find_runtime_log "$receipt_dir") || {
        printf 'missing runtime log under %s\n' "$receipt_dir" >&2
        return 1
    }

    if ! find_runtime_log_matching "$receipt_dir" 'xfstests FSTYP=exfat '; then
        printf 'receipt %s does not prove xfstests FSTYP=exfat\n' "$receipt_dir" >&2
        return 1
    fi

    while IFS= read -r runtime_log; do
        if grep -Eq '^FSTYP[[:space:]]*--[[:space:]]*exfat[[:space:]]*$' "$runtime_log"; then
            saw_guest_fstyp_line=1
            break
        fi
    done < <(for_each_runtime_log "$receipt_dir")

    if [ "$saw_guest_fstyp_line" -ne 1 ]; then
        printf 'receipt %s does not prove guest FSTYP -- exfat\n' "$receipt_dir" >&2
        return 1
    fi

    return 0
}

parse_batch_results() {
    local receipt_dir=$1
    local output_file=$2
    local runtime_log

    runtime_log=$(find_runtime_log_matching "$receipt_dir" '^(generic/[0-9]+)[[:space:]]+QA output created by [0-9]+|^Ran: |^Failures: |^Not run: |^Passed all |^Failed [0-9]+ of [0-9]+ tests|^All conformance tests passed\.|Error: xfstests failed') || \
        runtime_log=$(find_runtime_log "$receipt_dir") || {
        printf 'missing runtime log under %s\n' "$receipt_dir" >&2
        return 1
    }

    awk '
        function sanitize_note(raw) {
            gsub(/\t/, " ", raw)
            gsub(/\r/, "", raw)
            return raw
        }
        function mark_current(new_status, raw_note) {
            if (current == "") {
                return
            }
            if (status[current] == "") {
                status[current] = new_status
            }
            if (note[current] == "") {
                note[current] = sanitize_note(raw_note)
            }
        }
        function close_previous() {
            if (current == "") {
                return
            }
            if (status[current] == "") {
                status[current] = "pass"
                note[current] = "next_banner_seen"
            }
        }
        function add_footer_cases(raw_list, map_name,    count, idx, items) {
            gsub(/^[^:]*:[[:space:]]*/, "", raw_list)
            if (raw_list == "") {
                return
            }
            count = split(raw_list, items, /[[:space:]]+/)
            for (idx = 1; idx <= count; idx++) {
                if (items[idx] == "") {
                    continue
                }
                if (map_name == "ran") {
                    footer_ran[items[idx]] = 1
                } else if (map_name == "fail") {
                    footer_fail[items[idx]] = 1
                } else if (map_name == "notrun") {
                    footer_notrun[items[idx]] = 1
                }
            }
        }
        {
            line = $0
            if (match(line, /^(generic\/[0-9]+)[[:space:]]+QA output created by [0-9]+/, match_arr)) {
                close_previous()
                current = match_arr[1]
                order[++order_count] = current
                next
            }

            if (line ~ /^Ran: /) {
                add_footer_cases(line, "ran")
                saw_footer = 1
            } else if (line ~ /^Failures: /) {
                add_footer_cases(line, "fail")
                saw_footer = 1
            } else if (line ~ /^Not run: /) {
                add_footer_cases(line, "notrun")
                saw_footer = 1
            } else if (line ~ /^Passed all / || line ~ /^Failed [0-9]+ of [0-9]+ tests/ || line ~ /^All conformance tests passed\./) {
                saw_footer = 1
            }

            if (current == "") {
                next
            }

            if (line ~ /\[not run\]/) {
                mark_current("notrun", line)
                next
            }

            if (line ~ /\[failed, exit status / || line ~ /Error: xfstests failed/) {
                mark_current("fail", line)
                next
            }

            if (line ~ /Uncaught panic/ || line ~ /panicked/ || line ~ /deadlock/ || line ~ /This function might break atomic mode/) {
                mark_current("fail", line)
                next
            }
        }
        END {
            if (current != "") {
                if (status[current] == "") {
                    if (footer_fail[current]) {
                        status[current] = "fail"
                        note[current] = "footer_fail_list"
                    } else if (footer_notrun[current]) {
                        status[current] = "notrun"
                        note[current] = "footer_not_run_list"
                    } else if (saw_footer || footer_ran[current]) {
                        status[current] = "pass"
                        note[current] = "footer_seen"
                    } else {
                        status[current] = "fail"
                        note[current] = "log_ended_without_footer"
                    }
                }
            }

            for (idx = 1; idx <= order_count; idx++) {
                case_name = order[idx]
                if (status[case_name] == "") {
                    if (footer_fail[case_name]) {
                        status[case_name] = "fail"
                        note[case_name] = "footer_fail_list"
                    } else if (footer_notrun[case_name]) {
                        status[case_name] = "notrun"
                        note[case_name] = "footer_not_run_list"
                    } else if (saw_footer || footer_ran[case_name]) {
                        status[case_name] = "pass"
                        note[case_name] = "footer_seen"
                    } else {
                        status[case_name] = "fail"
                        note[case_name] = "log_ended_without_footer"
                    }
                }

                print case_name "\t" status[case_name] "\t" note[case_name]
            }
        }
    ' "$runtime_log" > "$output_file"
}

exfat_refactor_status() {
    case "$1" in
        generic/642) printf 'deferred\n' ;;
        *) printf 'pass\n' ;;
    esac
}

exfat_refactor_note() {
    case "$1" in
        generic/192) printf 'experimental_runtime_green\n' ;;
        generic/452) printf 'shared_layer_vfs_assisted_closure\n' ;;
        generic/634) printf 'mixed_shared_and_exfat_local_closure\n' ;;
        *) printf '\n' ;;
    esac
}

write_root_manifest() {
    {
        echo "phase=$phase"
        echo "component=$component"
        echo "created_at=$(date -Iseconds)"
        echo "out_dir=$out_dir"
        echo "fstyp=exfat"
        echo "disk_size=$disk_size"
        echo "mem_size=$mem_size"
        echo "timeout=${timeout:-}"
        echo "total_cases=${#all_tests[@]}"
    } > "$out_dir/manifest.txt"

    printf '%s\n' "${all_tests[@]}" > "$out_dir/intended-tests.list"
}

append_case_result() {
    local case_name=$1
    local old_exfat_status=$2
    local note=$3
    local receipt_dir=$4
    local batch_name=$5

    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
        "$case_name" \
        "pass" \
        "$old_exfat_status" \
        "$note" \
        "$(exfat_refactor_status "$case_name")" \
        "$(exfat_refactor_note "$case_name")" \
        "$batch_name|$receipt_dir" \
        >> "$out_dir/three_way_matrix.tsv"
}

write_summary() {
    local pass_count notrun_count fail_count deferred_count

    pass_count=$(awk -F '\t' 'NR > 1 && $3 == "pass" { count++ } END { print count + 0 }' "$out_dir/three_way_matrix.tsv")
    notrun_count=$(awk -F '\t' 'NR > 1 && $3 == "notrun" { count++ } END { print count + 0 }' "$out_dir/three_way_matrix.tsv")
    fail_count=$(awk -F '\t' 'NR > 1 && $3 == "fail" { count++ } END { print count + 0 }' "$out_dir/three_way_matrix.tsv")
    deferred_count=$(awk -F '\t' 'NR > 1 && $5 == "deferred" { count++ } END { print count + 0 }' "$out_dir/three_way_matrix.tsv")

    {
        echo "old_exfat_pass=$pass_count"
        echo "old_exfat_notrun=$notrun_count"
        echo "old_exfat_fail=$fail_count"
        echo "exfat_refactor_deferred=$deferred_count"
    } > "$out_dir/summary.txt"
}

acquire_lock() {
    "$lock_tool" acquire \
        --component "$component" \
        --phase "$phase" \
        --command "$0 --phase $phase" \
        --retry-seconds "$retry_seconds" \
        --wait-budget-seconds "$wait_budget_seconds" \
        > "$out_dir/checker-lock-acquire.txt"
}

release_lock() {
    "$lock_tool" release > "$out_dir/checker-lock-release.txt"
}

run_batch() {
    local batch_index=$1
    shift
    local batch_name=$1
    shift
    local -a batch_tests=("$@")
    local -a extra_args=()
    local restore_errexit=0
    local tests_csv batch_dir batch_tag status_file parsed_file last_executed first_unexecuted_index idx case_name note batch_status

    batch_dir="$out_dir/$batch_name"
    batch_tag="old_exfat_51_b$(printf '%02d' "$batch_index")_$(sanitize_name "$batch_tests")"
    batch_tag=${batch_tag:0:70}
    tests_csv=$(join_csv "${batch_tests[@]}")

    mkdir -p "$batch_dir"
    printf '%s\n' "${batch_tests[@]}" > "$batch_dir/intended-suffix.list"
    rm -f "$batch_dir"/qemu.log "$batch_dir"/qemu-serial.log
    rm -rf "$batch_dir"/postrun

    if [ -n "$timeout" ]; then
        extra_args+=(--timeout "$timeout")
    fi
    if [ "$dry_run" -eq 1 ]; then
        extra_args+=(--dry-run)
    fi

    case $- in
        *e*) restore_errexit=1 ;;
        *) restore_errexit=0 ;;
    esac

    set +e
    "$run_tool" batch \
        --tests "$tests_csv" \
        --phase "$phase" \
        --component "$component" \
        --tag "$batch_tag" \
        --no-lock \
        --fstyp exfat \
        --disk "$disk_size" \
        --mem "$mem_size" \
        --container "$container" \
        --repo-dir "$repo_dir" \
        --wait-budget-seconds "$wait_budget_seconds" \
        --retry-seconds "$retry_seconds" \
        --out-dir "$batch_dir" \
        "${extra_args[@]}"
    batch_status=$?
    if [ "$restore_errexit" -eq 1 ]; then
        set -e
    else
        set +e
    fi

    printf '%s\n' "$batch_status" > "$batch_dir/wrapper-batch-exit-status.txt"

    if [ "$dry_run" -eq 1 ]; then
        printf 'dry-run mode does not parse runtime results\n' > "$batch_dir/dry-run-note.txt"
        : > "$batch_dir/remaining-suffix.list"
        return 0
    fi

    verify_old_exfat_receipt "$batch_dir"

    parsed_file="$batch_dir/parsed-results.tsv"
    parse_batch_results "$batch_dir" "$parsed_file"

    if [ ! -s "$parsed_file" ]; then
        printf 'batch %s produced no parsed case results\n' "$batch_name" >&2
        return 1
    fi

    last_executed=""
    while IFS=$'\t' read -r case_name status_file note; do
        last_executed=$case_name
        append_case_result "$case_name" "$status_file" "$note" "$batch_dir" "$batch_name"
    done < "$parsed_file"

    if [ -z "$last_executed" ]; then
        printf 'batch %s did not reach any testcase banner\n' "$batch_name" >&2
        return 1
    fi

    first_unexecuted_index=-1
    for idx in "${!batch_tests[@]}"; do
        if [ "${batch_tests[$idx]}" = "$last_executed" ]; then
            first_unexecuted_index=$((idx + 1))
            break
        fi
    done

    if [ "$first_unexecuted_index" -lt 0 ]; then
        printf 'could not locate last executed case %s inside batch %s\n' "$last_executed" "$batch_name" >&2
        return 1
    fi

    if [ "$first_unexecuted_index" -ge "${#batch_tests[@]}" ]; then
        : > "$batch_dir/remaining-suffix.list"
        return 0
    fi

    printf '%s\n' "${batch_tests[@]:$first_unexecuted_index}" > "$batch_dir/remaining-suffix.list"
    return 10
}

main() {
    local batch_index=1
    local batch_name
    local -a remaining_tests
    local -a next_remaining
    local run_status

    parse_args "$@"
    mkdir -p "$out_dir"
    write_root_manifest
    acquire_lock
    trap 'release_lock' EXIT

    {
        printf 'case\tlinux_baseline\told_exfat_status\told_exfat_note\texfat_refactor_status\texfat_refactor_note\told_exfat_receipt\n'
    } > "$out_dir/three_way_matrix.tsv"
    printf 'batch\tfirst_case\n' > "$out_dir/batch-sequence.tsv"

    remaining_tests=("${all_tests[@]}")
    while [ "${#remaining_tests[@]}" -gt 0 ]; do
        if [ "$batch_index" -eq 1 ]; then
            batch_name="batch_01_seed"
        else
            batch_name=$(printf 'batch_%02d_from_%s' "$batch_index" "$(sanitize_name "${remaining_tests[0]}")")
        fi

        printf '%s\t%s\n' "$batch_name" "${remaining_tests[0]}" >> "$out_dir/batch-sequence.tsv"

        set +e
        run_batch "$batch_index" "$batch_name" "${remaining_tests[@]}"
        run_status=$?
        set -e

        if [ "$run_status" -eq 0 ]; then
            remaining_tests=()
            break
        fi

        if [ "$run_status" -ne 10 ]; then
            printf 'batch %s failed before suffix continuation could be determined\n' "$batch_name" >&2
            exit 1
        fi

        mapfile -t next_remaining < "$out_dir/$batch_name/remaining-suffix.list"
        if [ "${#next_remaining[@]}" -eq 0 ]; then
            remaining_tests=()
            break
        fi

        remaining_tests=("${next_remaining[@]}")
        batch_index=$((batch_index + 1))
    done

    write_summary
    release_lock
    trap - EXIT
}

main "$@"
