#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0

set -euo pipefail

usage() {
    cat <<'USAGE'
Usage:
  capture_nixos_side_screenshot.sh --log FILE [options]
  capture_nixos_side_screenshot.sh --run-id ID [options]

Options:
  --log FILE           Render an existing guest-side log file.
  --run-id ID          Resolve log file as .agents/xfstests/logs/ID/qemu.log.
  --output PNG         Output PNG path. Default: .agents/tmp/nixos-side-<timestamp>.png
  --title TEXT         Optional title. Default: NixOS Side.
  --command TEXT       Command line to show in the screenshot.
  --start REGEX        Optional start regex passed to the renderer.
  --end REGEX          Optional end regex passed to the renderer.
  --tail N             Last-N-lines fallback. Default: 120 when no regex is selected.
  --width PX           Render width in pixels. Default: 1600.
  --pointsize N        Font size. Default: 20.
  -h, --help           Show this help text.

If neither --start/--end nor --tail is given, the script first tries to crop to
well-known probe blocks such as `FSCK_PROBE_OUTPUT_BEGIN/END`, then falls back
to the last 120 lines.
USAGE
}

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
AGENTS_DIR=$(cd "${SCRIPT_DIR}/.." && pwd)
TMP_DIR="${AGENTS_DIR}/tmp"
LOGS_DIR="${AGENTS_DIR}/xfstests/logs"
RENDER_SCRIPT="${SCRIPT_DIR}/render_terminal_screenshot.sh"

LOG_FILE=""
RUN_ID=""
OUTPUT_FILE=""
TITLE="NixOS Side"
COMMAND_TEXT=""
START_REGEX=""
END_REGEX=""
TAIL_LINES=""
WIDTH=1600
POINTSIZE=20

while [ "$#" -gt 0 ]; do
    case "$1" in
        --log)
            LOG_FILE="${2:?missing --log value}"
            shift 2
            ;;
        --run-id)
            RUN_ID="${2:?missing --run-id value}"
            shift 2
            ;;
        --output)
            OUTPUT_FILE="${2:?missing --output value}"
            shift 2
            ;;
        --title)
            TITLE="${2:?missing --title value}"
            shift 2
            ;;
        --command)
            COMMAND_TEXT="${2:?missing --command value}"
            shift 2
            ;;
        --start)
            START_REGEX="${2:?missing --start value}"
            shift 2
            ;;
        --end)
            END_REGEX="${2:?missing --end value}"
            shift 2
            ;;
        --tail)
            TAIL_LINES="${2:?missing --tail value}"
            shift 2
            ;;
        --width)
            WIDTH="${2:?missing --width value}"
            shift 2
            ;;
        --pointsize)
            POINTSIZE="${2:?missing --pointsize value}"
            shift 2
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

if [ -n "${LOG_FILE}" ] && [ -n "${RUN_ID}" ]; then
    echo "--log and --run-id are mutually exclusive" >&2
    exit 2
fi

if [ -z "${LOG_FILE}" ] && [ -z "${RUN_ID}" ]; then
    usage >&2
    exit 2
fi

if [ -n "${RUN_ID}" ]; then
    LOG_FILE="${LOGS_DIR}/${RUN_ID}/qemu.log"
fi

if [ ! -f "${LOG_FILE}" ]; then
    echo "log file does not exist: ${LOG_FILE}" >&2
    exit 1
fi

mkdir -p "${TMP_DIR}"

timestamp=$(date +%Y%m%d-%H%M%S)
if [ -z "${OUTPUT_FILE}" ]; then
    OUTPUT_FILE="${TMP_DIR}/nixos-side-${timestamp}.png"
fi

if [ -z "${START_REGEX}" ] && [ -z "${END_REGEX}" ] && [ -z "${TAIL_LINES}" ]; then
    if rg -q 'FSCK_PROBE_OUTPUT_BEGIN' "${LOG_FILE}"; then
        START_REGEX='FSCK_PROBE_OUTPUT_BEGIN'
        END_REGEX='FSCK_PROBE_OUTPUT_END'
        if [ -z "${COMMAND_TEXT}" ]; then
            COMMAND_TEXT='fsck.exfat -n /dev/vdb'
        fi
    elif rg -q 'BLOCK_PROBE_BEGIN' "${LOG_FILE}"; then
        START_REGEX='BLOCK_PROBE_BEGIN'
        END_REGEX='BLOCK_PROBE_DONE'
    elif rg -q 'BLKPBSZGET_PROBE_BEGIN' "${LOG_FILE}"; then
        START_REGEX='BLKPBSZGET_PROBE_BEGIN'
        END_REGEX='BLKPBSZGET_PROBE_DONE'
    elif rg -q 'PREAD_ROOT_FAT_ENTRY_PROBE_BEGIN' "${LOG_FILE}"; then
        START_REGEX='PREAD_ROOT_FAT_ENTRY_PROBE_BEGIN'
        END_REGEX='PREAD_ROOT_FAT_ENTRY_PROBE_DONE'
    else
        TAIL_LINES=120
    fi
fi

renderer_args=(
    --input "${LOG_FILE}"
    --output "${OUTPUT_FILE}"
    --title "${TITLE}"
    --width "${WIDTH}"
    --pointsize "${POINTSIZE}"
)

if [ -n "${COMMAND_TEXT}" ]; then
    renderer_args+=(--prologue "root@asterinas: ${COMMAND_TEXT}")
fi
if [ -n "${START_REGEX}" ]; then
    renderer_args+=(--start "${START_REGEX}")
fi
if [ -n "${END_REGEX}" ]; then
    renderer_args+=(--end "${END_REGEX}")
fi
if [ -n "${TAIL_LINES}" ]; then
    renderer_args+=(--tail "${TAIL_LINES}")
fi

"${RENDER_SCRIPT}" "${renderer_args[@]}"
