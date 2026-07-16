#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0

set -euo pipefail

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
agents_dir=$(CDPATH= cd -- "$script_dir/.." && pwd)
exfat_refactor_dir=$(CDPATH= cd -- "$agents_dir/.." && pwd)
asterinas_dir=$(CDPATH= cd -- "$exfat_refactor_dir/../../../../.." && pwd)

mode=""
dest=""
src=""
archive_name=""
dry_run=0
timestamp=$(date '+%Y%m%d-%H%M%S')
staging_dir=""
declare -a requested_paths=()
declare -a list_files=()
declare -a archive_paths=()

usage() {
    cat <<'EOF'
Usage:
  agent_state_pack.sh create --dest PATH (--path PATH | --from-list FILE)...
  agent_state_pack.sh push --dest REMOTE (--path PATH | --from-list FILE)... [--archive-name NAME]
  agent_state_pack.sh pull --src REMOTE_ARCHIVE --dest LOCAL_PATH

Packages only the files explicitly named by the caller. There is no default
payload; this enforces minimum-package migration.

Modes:
  create   Create a local tar archive.
  push     Create a temporary tar archive from the explicit path list and copy
           it to REMOTE with scp.
  pull     Copy a remote archive to LOCAL_PATH with scp.

Path selection for create/push:
  --path PATH        Include one file or directory. May be repeated.
  --from-list FILE   Read newline-separated paths. Blank lines and lines
                     starting with # are ignored. May be repeated.

Path resolution:
  - Repository-relative paths are accepted directly.
  - exfat_refactor-relative paths such as .agents/main-agent are accepted and
    stored under kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent.

Archive formats:
  PATH or NAME ending in .tar creates an uncompressed tar archive.
  PATH or NAME ending in .tar.gz or .tgz creates a gzip archive.
  Other names are suffixed with .tar.gz.

Examples:
  agent_state_pack.sh create --dest /tmp/state.tar.gz \
      --path kernel/src/fs/fs_impls/exfat_refactor/.agents/main-agent \
      --path kernel/src/fs/fs_impls/exfat_refactor/.agents/components/xfstests_linux_baseline_20260624/pass_72_generic_694_allocator_cursor_locality_checker.md

  agent_state_pack.sh push --dest debian:/tmp/ --archive-name xfstests-state.tar.gz \
      --from-list /tmp/xfstests-state-files.txt

  agent_state_pack.sh pull --src debian:/tmp/xfstests-state.tar.gz \
      --dest /tmp/xfstests-state.tar.gz
EOF
}

parse_args() {
    [ $# -ge 1 ] || { usage >&2; exit 64; }

    case "$1" in
        create|push|pull)
            mode=$1
            shift
            ;;
        -h|--help|help)
            usage
            exit 0
            ;;
        *)
            usage >&2
            exit 64
            ;;
    esac

    while [ $# -gt 0 ]; do
        case "$1" in
            --dest)
                [ $# -ge 2 ] || { usage >&2; exit 64; }
                dest=$2
                shift 2
                ;;
            --src)
                [ $# -ge 2 ] || { usage >&2; exit 64; }
                src=$2
                shift 2
                ;;
            --path)
                [ $# -ge 2 ] || { usage >&2; exit 64; }
                requested_paths+=("$2")
                shift 2
                ;;
            --from-list)
                [ $# -ge 2 ] || { usage >&2; exit 64; }
                list_files+=("$2")
                shift 2
                ;;
            --archive-name)
                [ $# -ge 2 ] || { usage >&2; exit 64; }
                archive_name=$2
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

    case "$mode" in
        create)
            [ -n "$dest" ] || { usage >&2; exit 64; }
            require_path_selection
            ;;
        push)
            [ -n "$dest" ] || { usage >&2; exit 64; }
            require_path_selection
            ;;
        pull)
            [ -n "$src" ] || { usage >&2; exit 64; }
            [ -n "$dest" ] || { usage >&2; exit 64; }
            [ "${#requested_paths[@]}" -eq 0 ] || { usage >&2; exit 64; }
            [ "${#list_files[@]}" -eq 0 ] || { usage >&2; exit 64; }
            ;;
    esac
}

require_path_selection() {
    if [ "${#requested_paths[@]}" -eq 0 ] && [ "${#list_files[@]}" -eq 0 ]; then
        echo "create/push require at least one --path or --from-list entry" >&2
        exit 64
    fi
}

make_staging_dir() {
    staging_dir=$(mktemp -d "${TMPDIR:-/tmp}/exfat-agent-state.${timestamp}.XXXXXX")
}

cleanup() {
    if [ -n "${staging_dir:-}" ] && [ -d "$staging_dir" ]; then
        rm -rf "$staging_dir"
    fi
}

normalize_archive_name() {
    local name=$1

    case "$name" in
        *.tar|*.tar.gz|*.tgz)
            printf '%s\n' "$name"
            ;;
        *)
            printf '%s.tar.gz\n' "$name"
            ;;
    esac
}

read_list_files() {
    local list_file
    local line

    for list_file in "${list_files[@]}"; do
        [ -f "$list_file" ] || {
            echo "missing list file: $list_file" >&2
            exit 64
        }
        while IFS= read -r line || [ -n "$line" ]; do
            case "$line" in
                ""|\#*) continue ;;
            esac
            requested_paths+=("$line")
        done < "$list_file"
    done
}

resolve_one_path() {
    local input=$1
    local repo_relative
    local exfat_relative

    case "$input" in
        /*)
            case "$input" in
                "$asterinas_dir"/*)
                    repo_relative=${input#"$asterinas_dir"/}
                    ;;
                *)
                    echo "path is outside repository: $input" >&2
                    exit 64
                    ;;
            esac
            ;;
        *)
            if [ -e "$asterinas_dir/$input" ]; then
                repo_relative=$input
            elif [ -e "$exfat_refactor_dir/$input" ]; then
                exfat_relative=${exfat_refactor_dir#"$asterinas_dir"/}
                repo_relative="$exfat_relative/$input"
            else
                echo "missing requested path: $input" >&2
                exit 64
            fi
            ;;
    esac

    case "$repo_relative" in
        ./*) repo_relative=${repo_relative#./} ;;
    esac

    archive_paths+=("$repo_relative")
}

resolve_paths() {
    local path

    read_list_files
    for path in "${requested_paths[@]}"; do
        resolve_one_path "$path"
    done
}

write_manifest() {
    local manifest="$staging_dir/MANIFEST.txt"
    local path

    {
        echo "created_at=$(date -Iseconds)"
        echo "mode=$mode"
        echo "asterinas_dir=$asterinas_dir"
        echo "exfat_refactor_dir=$exfat_refactor_dir"
        echo
        echo "[explicit-paths]"
        for path in "${archive_paths[@]}"; do
            echo "$path"
        done
    } > "$manifest"
}

write_tar_list() {
    local tar_list="$staging_dir/tar-list.txt"
    local path

    : > "$tar_list"
    for path in "${archive_paths[@]}"; do
        printf '%s\n' "$path" >> "$tar_list"
    done
    printf '%s\n' "MANIFEST.txt" >> "$tar_list"
}

stage_manifest_for_tar() {
    cp "$staging_dir/MANIFEST.txt" "$asterinas_dir/MANIFEST.txt.agent_state_pack_tmp"
    trap 'rm -f "$asterinas_dir/MANIFEST.txt.agent_state_pack_tmp"; cleanup' EXIT
    sed -i 's|^MANIFEST.txt$|MANIFEST.txt.agent_state_pack_tmp|' "$staging_dir/tar-list.txt"
}

create_archive() {
    local output=$1
    local normalized_output
    local tar_list="$staging_dir/tar-list.txt"

    normalized_output=$(normalize_archive_name "$output")
    if [ "$normalized_output" != "$output" ]; then
        output=$normalized_output
    fi

    write_manifest
    write_tar_list

    if [ "$dry_run" -eq 1 ]; then
        cat "$staging_dir/MANIFEST.txt"
        echo
        echo "[archive]"
        echo "$output"
        return
    fi

    stage_manifest_for_tar

    case "$output" in
        *.tar)
            tar -C "$asterinas_dir" \
                --transform='s|^MANIFEST[.]txt[.]agent_state_pack_tmp$|MANIFEST.txt|' \
                -cf "$output" -T "$tar_list"
            ;;
        *.tar.gz|*.tgz)
            tar -C "$asterinas_dir" \
                --transform='s|^MANIFEST[.]txt[.]agent_state_pack_tmp$|MANIFEST.txt|' \
                -czf "$output" -T "$tar_list"
            ;;
    esac

    rm -f "$asterinas_dir/MANIFEST.txt.agent_state_pack_tmp"
    trap cleanup EXIT
    echo "$output"
}

run_create() {
    resolve_paths
    create_archive "$dest"
}

run_push() {
    local output_name
    local output_path

    resolve_paths
    if [ -n "$archive_name" ]; then
        output_name=$(normalize_archive_name "$archive_name")
    else
        output_name="exfat-agent-state-${timestamp}.tar.gz"
    fi
    output_path="${TMPDIR:-/tmp}/$output_name"

    create_archive "$output_path"
    if [ "$dry_run" -eq 1 ]; then
        echo
        echo "[scp]"
        echo "scp $output_path $dest"
        return
    fi

    scp "$output_path" "$dest"
}

run_pull() {
    if [ "$dry_run" -eq 1 ]; then
        echo "scp $src $dest"
        return
    fi

    scp "$src" "$dest"
}

main() {
    parse_args "$@"
    make_staging_dir
    trap cleanup EXIT

    case "$mode" in
        create) run_create ;;
        push) run_push ;;
        pull) run_pull ;;
    esac
}

main "$@"
