#!/usr/bin/env bash

set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
agents_dir=$(CDPATH= cd -- "$script_dir/.." && pwd)
locks_dir="$agents_dir/locks"
lock_dir="$locks_dir/checker-execution.lock"
owner_file="$lock_dir/owner.toml"

usage() {
    cat <<'EOF'
Usage:
  checker_lock.sh status
  checker_lock.sh release
  checker_lock.sh acquire --component ID --phase PHASE --command COMMAND \
      [--retry-seconds SECONDS] [--wait-budget-seconds SECONDS]

Subcommands:
  status    Print the current lock owner metadata when present.
  release   Remove the current checker execution lock.
  acquire   Atomically acquire the checker execution lock and write owner metadata.

Exit codes:
  0  Success.
  2  Lock busy and no wait budget left.
  3  Wait budget exceeded before acquisition.
  64 Usage error.
EOF
}

toml_escape() {
    printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'
}

write_owner_file() {
    local component=$1
    local phase=$2
    local command=$3
    local start_time

    start_time=$(date '+%Y-%m-%d %H:%M:%S %Z')

    printf 'component = "%s"\n' "$(toml_escape "$component")" > "$owner_file"
    printf 'phase = "%s"\n' "$(toml_escape "$phase")" >> "$owner_file"
    printf 'command = "%s"\n' "$(toml_escape "$command")" >> "$owner_file"
    printf 'pid = %s\n' "$$" >> "$owner_file"
    printf 'start_time = "%s"\n' "$(toml_escape "$start_time")" >> "$owner_file"
}

print_lock_owner() {
    if [ -f "$owner_file" ]; then
        cat "$owner_file"
        return
    fi

    if [ -d "$lock_dir" ]; then
        printf 'lock_dir = "%s"\n' "$lock_dir"
        printf 'status = "held-without-owner-file"\n'
        return
    fi

    printf 'status = "unlocked"\n'
}

acquire_lock() {
    local component=""
    local phase=""
    local command=""
    local retry_seconds=60
    local wait_budget_seconds=0
    local waited_seconds=0

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
            --command)
                [ $# -ge 2 ] || { usage >&2; exit 64; }
                command=$2
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
            *)
                usage >&2
                exit 64
                ;;
        esac
    done

    [ -n "$component" ] || { usage >&2; exit 64; }
    [ -n "$phase" ] || { usage >&2; exit 64; }
    [ -n "$command" ] || { usage >&2; exit 64; }
    [ "$retry_seconds" -ge 60 ] || {
        printf 'retry interval must be at least 60 seconds\n' >&2
        exit 64
    }
    [ "$wait_budget_seconds" -ge 0 ] || {
        printf 'wait budget must be non-negative\n' >&2
        exit 64
    }

    mkdir -p "$locks_dir"

    while ! mkdir "$lock_dir" 2>/dev/null; do
        if [ "$wait_budget_seconds" -eq 0 ]; then
            print_lock_owner >&2
            exit 2
        fi

        if [ "$waited_seconds" -ge "$wait_budget_seconds" ]; then
            print_lock_owner >&2
            exit 3
        fi

        sleep "$retry_seconds"
        waited_seconds=$((waited_seconds + retry_seconds))
    done

    write_owner_file "$component" "$phase" "$command"
    print_lock_owner
}

release_lock() {
    if [ ! -d "$lock_dir" ]; then
        printf 'status = "unlocked"\n'
        return
    fi

    if [ -f "$owner_file" ]; then
        rm "$owner_file"
    fi

    rmdir "$lock_dir"
    printf 'status = "unlocked"\n'
}

main() {
    [ $# -ge 1 ] || {
        usage >&2
        exit 64
    }

    case "$1" in
        status)
            shift
            [ $# -eq 0 ] || { usage >&2; exit 64; }
            print_lock_owner
            ;;
        release)
            shift
            [ $# -eq 0 ] || { usage >&2; exit 64; }
            release_lock
            ;;
        acquire)
            shift
            acquire_lock "$@"
            ;;
        *)
            usage >&2
            exit 64
            ;;
    esac
}

main "$@"
