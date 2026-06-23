#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0

set -euo pipefail

usage() {
    cat <<'USAGE'
Usage: render_terminal_screenshot.sh --input FILE --output PNG [options]

Options:
  --title TEXT         Optional title line rendered above the body.
  --prologue TEXT      Optional text block inserted before the rendered body.
  --start REGEX        Keep content starting from the first matching line.
  --end REGEX          Stop content at the first matching line after --start.
  --tail N             Keep only the last N lines after filtering.
  --width PX           Render width in pixels. Default: 1600.
  --pointsize N        Font size. Default: 20.
  -h, --help           Show this help text.

The script strips ANSI escape sequences and carriage returns before rendering.
USAGE
}

INPUT_FILE=""
OUTPUT_FILE=""
TITLE=""
PROLOGUE=""
START_REGEX=""
END_REGEX=""
TAIL_LINES=""
WIDTH=1600
POINTSIZE=20

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
        --title)
            TITLE="${2:?missing --title value}"
            shift 2
            ;;
        --prologue)
            PROLOGUE="${2:?missing --prologue value}"
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

if [ -z "${INPUT_FILE}" ] || [ -z "${OUTPUT_FILE}" ]; then
    usage >&2
    exit 2
fi

if [ ! -f "${INPUT_FILE}" ]; then
    echo "input file does not exist: ${INPUT_FILE}" >&2
    exit 1
fi

if ! command -v convert >/dev/null 2>&1; then
    echo "missing dependency: convert" >&2
    exit 1
fi
if ! command -v fc-match >/dev/null 2>&1; then
    echo "missing dependency: fc-match" >&2
    exit 1
fi

case "${WIDTH}" in
    ''|*[!0-9]*)
        echo "--width must be a positive integer" >&2
        exit 2
        ;;
esac

case "${POINTSIZE}" in
    ''|*[!0-9]*)
        echo "--pointsize must be a positive integer" >&2
        exit 2
        ;;
esac

if [ -n "${TAIL_LINES}" ]; then
    case "${TAIL_LINES}" in
        ''|*[!0-9]*)
            echo "--tail must be a non-negative integer" >&2
            exit 2
            ;;
    esac
fi

OUTPUT_DIR=$(dirname "${OUTPUT_FILE}")
mkdir -p "${OUTPUT_DIR}"

TMP_DIR=$(mktemp -d)
cleanup() {
    rm -rf -- "${TMP_DIR}"
}
trap cleanup EXIT

SANITIZED_FILE="${TMP_DIR}/sanitized.txt"
FILTERED_FILE="${TMP_DIR}/filtered.txt"
RENDER_FILE="${TMP_DIR}/render.txt"

perl -pe 's/\e\[[0-9;?]*[ -\/]*[@-~]//g; s/\r//g' "${INPUT_FILE}" > "${SANITIZED_FILE}"

awk -v start_regex="${START_REGEX}" -v end_regex="${END_REGEX}" '
BEGIN {
    if (start_regex == "") {
        started = 1;
    } else {
        started = 0;
    }
}
{
    if (!started && $0 ~ start_regex) {
        started = 1;
    }
    if (!started) {
        next;
    }
    print;
    if (end_regex != "" && $0 ~ end_regex) {
        exit;
    }
}
' "${SANITIZED_FILE}" > "${FILTERED_FILE}"

if [ -n "${TAIL_LINES}" ]; then
    tail -n "${TAIL_LINES}" "${FILTERED_FILE}" > "${RENDER_FILE}"
else
    cp -- "${FILTERED_FILE}" "${RENDER_FILE}"
fi

if [ -n "${TITLE}" ]; then
    {
        printf '%s\n' "${TITLE}"
        printf '%s\n' "$(printf '=%.0s' $(seq 1 ${#TITLE}))"
        printf '\n'
        if [ -n "${PROLOGUE}" ]; then
            printf '%s\n\n' "${PROLOGUE}"
        fi
        cat "${RENDER_FILE}"
    } > "${TMP_DIR}/with-title.txt"
    mv -- "${TMP_DIR}/with-title.txt" "${RENDER_FILE}"
elif [ -n "${PROLOGUE}" ]; then
    {
        printf '%s\n\n' "${PROLOGUE}"
        cat "${RENDER_FILE}"
    } > "${TMP_DIR}/with-prologue.txt"
    mv -- "${TMP_DIR}/with-prologue.txt" "${RENDER_FILE}"
fi

if [ ! -s "${RENDER_FILE}" ]; then
    echo "render content is empty after filtering" >&2
    exit 1
fi

RENDER_TEXT=$(cat "${RENDER_FILE}")
FONT_FILE=$(fc-match -f '%{file}\n' monospace | head -n 1)

if [ -z "${FONT_FILE}" ] || [ ! -f "${FONT_FILE}" ]; then
    echo "failed to resolve a monospace font file" >&2
    exit 1
fi

convert \
    -background '#111827' \
    -fill '#e5e7eb' \
    -font "${FONT_FILE}" \
    -pointsize "${POINTSIZE}" \
    -size "${WIDTH}x" \
    "caption:${RENDER_TEXT}" \
    -bordercolor '#0b1220' \
    -border 24 \
    "${OUTPUT_FILE}"

if [ ! -f "${OUTPUT_FILE}" ]; then
    echo "failed to create output file: ${OUTPUT_FILE}" >&2
    exit 1
fi

echo "${OUTPUT_FILE}"
