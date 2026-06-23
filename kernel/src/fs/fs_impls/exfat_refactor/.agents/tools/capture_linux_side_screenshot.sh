#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0

set -euo pipefail

usage() {
    cat <<'USAGE'
Usage:
  capture_linux_side_screenshot.sh --input FILE [options]
  capture_linux_side_screenshot.sh -- [COMMAND [ARG ...]]

Options:
  --input FILE         Render an existing Linux-side output file.
  --output PNG         Output PNG path. Default: .agents/tmp/linux-side-<timestamp>.png
  --raw-output FILE    Save command stdout/stderr here in command mode.
  --title TEXT         Optional title. Default: Linux Side.
  --command TEXT       Command line to show in the screenshot.
  --start REGEX        Optional start regex passed to the renderer.
  --end REGEX          Optional end regex passed to the renderer.
  --tail N             Optional last-N-lines filter passed to the renderer.
  --width PX           Render width in pixels. Default: 1600.
  --pointsize N        Font size. Default: 20.
  -h, --help           Show this help text.

If no --input is given, everything after `--` is executed locally and the
captured stdout/stderr is rendered into a terminal-style PNG.
USAGE
}

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
AGENTS_DIR=$(cd "${SCRIPT_DIR}/.." && pwd)
TMP_DIR="${AGENTS_DIR}/tmp"
RENDER_SCRIPT="${SCRIPT_DIR}/render_terminal_screenshot.sh"

INPUT_FILE=""
OUTPUT_FILE=""
RAW_OUTPUT_FILE=""
TITLE="Linux Side"
COMMAND_TEXT=""
START_REGEX=""
END_REGEX=""
TAIL_LINES=""
WIDTH=1600
POINTSIZE=20
RUN_COMMAND=0

while [ "$#" -gt 0 ]; do
    case "$1" in
        --input)
            INPUT_FILE="${2:?missing --input value}"
            shift 2
            ;;
        --output)
            OUTPUT_FILE="${2:?missing --output value}"
            shift 2
            ;;
        --raw-output)
            RAW_OUTPUT_FILE="${2:?missing --raw-output value}"
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
        --)
            RUN_COMMAND=1
            shift
            break
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

mkdir -p "${TMP_DIR}"

timestamp=$(date +%Y%m%d-%H%M%S)
if [ -z "${OUTPUT_FILE}" ]; then
    OUTPUT_FILE="${TMP_DIR}/linux-side-${timestamp}.png"
fi

if [ -z "${INPUT_FILE}" ] && [ "${RUN_COMMAND}" -eq 0 ]; then
    usage >&2
    exit 2
fi

if [ -n "${INPUT_FILE}" ] && [ "${RUN_COMMAND}" -eq 1 ]; then
    echo "--input and command mode are mutually exclusive" >&2
    exit 2
fi

if [ "${RUN_COMMAND}" -eq 1 ]; then
    if [ "$#" -eq 0 ]; then
        echo "missing command after --" >&2
        exit 2
    fi
    if [ -z "${RAW_OUTPUT_FILE}" ]; then
        RAW_OUTPUT_FILE="${TMP_DIR}/linux-side-${timestamp}.txt"
    fi
    if [ -z "${COMMAND_TEXT}" ]; then
        COMMAND_TEXT="$*"
    fi
    "$@" > "${RAW_OUTPUT_FILE}" 2>&1
    INPUT_FILE="${RAW_OUTPUT_FILE}"
fi

renderer_args=(
    --input "${INPUT_FILE}"
    --output "${OUTPUT_FILE}"
    --title "${TITLE}"
    --width "${WIDTH}"
    --pointsize "${POINTSIZE}"
)

if [ -n "${COMMAND_TEXT}" ]; then
    renderer_args+=(--prologue "\$ ${COMMAND_TEXT}")
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

if [ -n "${RAW_OUTPUT_FILE}" ]; then
    echo "raw_output=${RAW_OUTPUT_FILE}"
fi
