#!/usr/bin/env bash

set -euo pipefail

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
agents_dir=$(CDPATH= cd -- "$script_dir/.." && pwd)
lock_tool="$script_dir/checker_lock.sh"

container="codex-asterinas-dev"
repo_dir="/root/asterinas"
out_dir=""
component=""
phase=""
retry_seconds=60
wait_budget_seconds=0
keep_going=0
tests=()

usage() {
    cat <<'EOF'
Usage:
  checker_run.sh cargo-check --component ID --phase PHASE [OPTIONS]
  checker_run.sh make-kernel --component ID --phase PHASE [OPTIONS]
  checker_run.sh ktest --component ID --phase PHASE --test FULL_NAME [--test FULL_NAME ...] [OPTIONS]
  checker_run.sh pass --component ID --phase PHASE --test FULL_NAME [--test FULL_NAME ...] [OPTIONS]

Subcommands:
  cargo-check   Run `cargo check -p aster-kernel --target x86_64-unknown-none` in `/root/asterinas/kernel`.
  make-kernel   Run `make kernel` in `/root/asterinas`.
  ktest         Run one or more exact-name `cargo osdk test` commands in `/root/asterinas/kernel`.
  pass          Run `cargo check`, then `make kernel`, then the requested exact-name ktests.

Options:
  --component ID              Parent meso-component identifier for checker lock metadata and receipt grouping.
  --phase PHASE               Checker phase label for checker lock metadata.
  --test FULL_NAME            Exact ktest full name. May be repeated.
  --container NAME            Docker container name. Default: codex-asterinas-dev.
  --repo-dir PATH             Repository path inside the container. Default: /root/asterinas.
  --out-dir PATH              Host output directory. Default: .agents/checker-runs/<component>/<timestamp>-<component>-<phase>.
  --retry-seconds SECONDS     Lock retry interval. Must be >= 60. Default: 60.
  --wait-budget-seconds SECONDS
                              Checker lock wait budget. Default: 0 (fail if busy).
  --keep-going                Continue running later ktests after a ktest failure.

Outputs:
  - stdout/stderr logs for each command.
  - qemu-serial.log copied after each ktest before the next test can overwrite it.
  - summary.tsv with command status and archived log paths.

Exit codes:
  0   All requested commands passed.
  1   At least one build/test command failed.
  64  Usage error.
EOF
}

shell_quote() {
    printf '%q' "$1"
}

timestamp() {
    date '+%Y%m%d-%H%M%S'
}

sanitize_name() {
    printf '%s' "$1" | tr -c 'A-Za-z0-9_.-' '_' | cut -c 1-180
}

parse_args() {
    [ $# -ge 1 ] || {
        usage >&2
        exit 64
    }

    subcommand=$1
    shift

    case "$subcommand" in
        -h|--help|help)
            usage
            exit 0
            ;;
        cargo-check|make-kernel|ktest|pass)
            ;;
        *)
            usage >&2
            exit 64
            ;;
    esac

    while [ $# -gt 0 ]; do
        case "$1" in
            --component)
                [ $# -ge 2 ] || { usage >&2; exit 64; }
                component=$2
                shift 2
                ;;
            --phase)
                [ $# -ge 2 ] || { usage >&2; exit 64; }
                phase=$2
                shift 2
                ;;
            --test)
                [ $# -ge 2 ] || { usage >&2; exit 64; }
                tests+=("$2")
                shift 2
                ;;
            --container)
                [ $# -ge 2 ] || { usage >&2; exit 64; }
                container=$2
                shift 2
                ;;
            --repo-dir)
                [ $# -ge 2 ] || { usage >&2; exit 64; }
                repo_dir=$2
                shift 2
                ;;
            --out-dir)
                [ $# -ge 2 ] || { usage >&2; exit 64; }
                out_dir=$2
                shift 2
                ;;
            --retry-seconds)
                [ $# -ge 2 ] || { usage >&2; exit 64; }
                retry_seconds=$2
                shift 2
                ;;
            --wait-budget-seconds)
                [ $# -ge 2 ] || { usage >&2; exit 64; }
                wait_budget_seconds=$2
                shift 2
                ;;
            --keep-going)
                keep_going=1
                shift
                ;;
            *)
                usage >&2
                exit 64
                ;;
        esac
    done

    [ -n "$component" ] || { usage >&2; exit 64; }
    [ -n "$phase" ] || { usage >&2; exit 64; }

    case "$subcommand" in
        ktest|pass)
            [ "${#tests[@]}" -gt 0 ] || { usage >&2; exit 64; }
            ;;
        cargo-check|make-kernel)
            [ "${#tests[@]}" -eq 0 ] || { usage >&2; exit 64; }
            ;;
    esac

    if [ -z "$out_dir" ]; then
        out_dir="$agents_dir/checker-runs/$(sanitize_name "$component")/$(timestamp)-$(sanitize_name "$component")-$(sanitize_name "$phase")"
    fi
}

docker_bash() {
    local command=$1

    docker exec "$container" bash -lc "$command"
}

container_path_exists() {
    local path=$1
    local quoted_path

    quoted_path=$(shell_quote "$path")
    docker_bash "[ -e $quoted_path ]"
}

copy_container_file_if_present() {
    local container_path=$1
    local host_path=$2

    if container_path_exists "$container_path"; then
        docker cp "$container:$container_path" "$host_path"
        return 0
    fi

    printf 'missing container file: %s\n' "$container_path" > "$host_path.missing"
    return 1
}

append_summary() {
    local step=$1
    local status=$2
    local command=$3
    local stdout_log=$4
    local serial_log=$5

    printf '%s\t%s\t%s\t%s\t%s\n' "$step" "$status" "$command" "$stdout_log" "$serial_log" >> "$out_dir/summary.tsv"
}

run_logged_command() {
    local step=$1
    local command=$2
    local stdout_log=$3

    printf '==> %s\n' "$command" | tee "$stdout_log"
    set +e
    docker_bash "$command" >> "$stdout_log" 2>&1
    local status=$?
    set -e
    return "$status"
}

run_make_kernel() {
    local quoted_repo
    local command
    local log_path

    quoted_repo=$(shell_quote "$repo_dir")
    command="cd $quoted_repo && make kernel"
    log_path="$out_dir/00-make-kernel.log"

    if run_logged_command "make-kernel" "$command" "$log_path"; then
        append_summary "make-kernel" "0" "$command" "$log_path" ""
        return 0
    fi

    append_summary "make-kernel" "1" "$command" "$log_path" ""
    return 1
}

run_cargo_check() {
    local quoted_repo
    local command
    local log_path

    quoted_repo=$(shell_quote "$repo_dir")
    command="cd $quoted_repo/kernel && cargo check -p aster-kernel --target x86_64-unknown-none"
    log_path="$out_dir/00-cargo-check.log"

    if run_logged_command "cargo-check" "$command" "$log_path"; then
        append_summary "cargo-check" "0" "$command" "$log_path" ""
        return 0
    fi

    append_summary "cargo-check" "1" "$command" "$log_path" ""
    return 1
}

run_one_ktest() {
    local index=$1
    local test_name=$2
    local quoted_repo
    local quoted_test
    local command
    local safe_test_name
    local stdout_log
    local serial_log
    local serial_container_path
    local status

    quoted_repo=$(shell_quote "$repo_dir")
    quoted_test=$(shell_quote "$test_name")
    command="cd $quoted_repo/kernel && cargo osdk test $quoted_test"
    safe_test_name=$(sanitize_name "$test_name")
    stdout_log="$out_dir/$(printf '%02d' "$index")-ktest-$safe_test_name.log"
    serial_log="$out_dir/$(printf '%02d' "$index")-qemu-serial-$safe_test_name.log"
    serial_container_path="$repo_dir/qemu-serial.log"

    set +e
    run_logged_command "ktest-$index" "$command" "$stdout_log"
    status=$?
    set -e

    copy_container_file_if_present "$serial_container_path" "$serial_log" || true
    append_summary "ktest-$index" "$status" "$command" "$stdout_log" "$serial_log"
    return "$status"
}

run_ktests() {
    local failed=0
    local index=1
    local test_name

    for test_name in "${tests[@]}"; do
        if ! run_one_ktest "$index" "$test_name"; then
            failed=1
            if [ "$keep_going" -eq 0 ]; then
                return 1
            fi
        fi
        index=$((index + 1))
    done

    return "$failed"
}

main() {
    parse_args "$@"

    mkdir -p "$out_dir"
    printf 'step\tstatus\tcommand\tstdout_log\tserial_log\n' > "$out_dir/summary.tsv"

    "$lock_tool" acquire \
        --component "$component" \
        --phase "$phase" \
        --command "checker_run.sh $subcommand" \
        --retry-seconds "$retry_seconds" \
        --wait-budget-seconds "$wait_budget_seconds" \
        > "$out_dir/checker-lock-acquire.toml"

    local overall_status=0
    trap '"$lock_tool" release > "$out_dir/checker-lock-release.toml"' EXIT

    case "$subcommand" in
        cargo-check)
            run_cargo_check || overall_status=1
            ;;
        make-kernel)
            run_make_kernel || overall_status=1
            ;;
        ktest)
            run_ktests || overall_status=1
            ;;
        pass)
            if ! run_cargo_check; then
                overall_status=1
            elif ! run_make_kernel; then
                overall_status=1
            elif ! run_ktests; then
                overall_status=1
            fi
            ;;
    esac

    printf 'output_dir = "%s"\n' "$out_dir"
    printf 'summary = "%s"\n' "$out_dir/summary.tsv"
    return "$overall_status"
}

main "$@"
