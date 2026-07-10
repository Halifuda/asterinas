#!/usr/bin/env python3
#
# SPDX-License-Identifier: MPL-2.0

"""Thin rust-analyzer LSP client for subagent code navigation."""

from __future__ import annotations

import argparse
import fnmatch
import json
import os
from pathlib import Path
import queue
import shlex
import shutil
import subprocess
import sys
import threading
import time
from typing import Any
from urllib.parse import unquote, urlparse
from urllib.request import pathname2url


DEFAULT_EXCLUDE_GLOBS = [
    "kernel/src/fs/fs_impls/exfat/**",
]
DEFAULT_CONTAINER_NAME = "codex-asterinas-dev"
DEFAULT_CONTAINER_REPO_DIR = "/root/asterinas"
DEFAULT_CONTAINER_RUST_ANALYZER_RELATIVE_PATH = Path(
    "kernel/src/fs/fs_impls/exfat_refactor/.agents/tmp/ra_code_nav/bin/rust-analyzer"
)

SYMBOL_KIND_NAMES = {
    1: "File",
    2: "Module",
    3: "Namespace",
    4: "Package",
    5: "Class",
    6: "Method",
    7: "Property",
    8: "Field",
    9: "Constructor",
    10: "Enum",
    11: "Interface",
    12: "Function",
    13: "Variable",
    14: "Constant",
    15: "String",
    16: "Number",
    17: "Boolean",
    18: "Array",
    19: "Object",
    20: "Key",
    21: "Null",
    22: "EnumMember",
    23: "Struct",
    24: "Event",
    25: "Operator",
    26: "TypeParameter",
}


def find_repo_root(start: Path) -> Path:
    current = start.resolve()
    for candidate in [current, *current.parents]:
        if (candidate / ".git").exists() and (candidate / "Cargo.toml").exists():
            return candidate
    return current


def path_to_uri(path: Path) -> str:
    return "file://" + pathname2url(str(path.resolve()))


def uri_to_path(uri: str) -> str:
    parsed = urlparse(uri)
    if parsed.scheme != "file":
        return uri
    return unquote(parsed.path)


def rel_path(path: str, root: Path) -> str:
    try:
        return str(Path(path).resolve().relative_to(root.resolve()))
    except ValueError:
        return path


def one_based_position(position: dict[str, Any]) -> tuple[int, int]:
    return int(position.get("line", 0)) + 1, int(position.get("character", 0)) + 1


def is_excluded_path(path: str, root: Path, exclude_globs: list[str]) -> bool:
    relative_path = rel_path(path, root)
    return any(fnmatch.fnmatch(relative_path, pattern) for pattern in exclude_globs)


def location_to_record(
    location: dict[str, Any],
    root: Path,
    exclude_globs: list[str] | None = None,
) -> dict[str, Any] | None:
    uri = location.get("uri") or location.get("targetUri") or ""
    range_obj = location.get("range") or location.get("targetSelectionRange") or {}
    start = range_obj.get("start", {})
    line, column = one_based_position(start)
    path = uri_to_path(uri)
    if exclude_globs and is_excluded_path(path, root, exclude_globs):
        return None
    return {
        "path": rel_path(path, root),
        "line": line,
        "column": column,
        "uri": uri,
    }


def print_jsonl(items: list[dict[str, Any]]) -> None:
    for item in items:
        print(json.dumps(item, ensure_ascii=False, sort_keys=True))


def active_toolchain_channel(root: Path) -> str | None:
    toolchain_file = root / "rust-toolchain.toml"
    if not toolchain_file.exists():
        return None
    for line in toolchain_file.read_text().splitlines():
        stripped = line.strip()
        if stripped.startswith("channel"):
            _, _, value = stripped.partition("=")
            return value.strip().strip('"')
    return None


def rustup_which(binary: str) -> Path | None:
    rustup = shutil.which("rustup")
    if rustup is None:
        return None
    result = subprocess.run(
        [rustup, "which", binary],
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        return None
    candidate = Path(result.stdout.strip())
    if not candidate.is_file():
        return None
    return candidate.resolve()


def fallback_rust_analyzer(root: Path) -> Path | None:
    channel = active_toolchain_channel(root)
    toolchains_dir = Path.home() / ".rustup" / "toolchains"
    if channel:
        preferred = toolchains_dir / f"{channel}-x86_64-unknown-linux-gnu" / "bin" / "rust-analyzer"
        if preferred.is_file():
            return preferred.resolve()
    matches = sorted(toolchains_dir.glob("*/bin/rust-analyzer"))
    if not matches:
        return None
    return matches[-1].resolve()


def resolve_local_rust_analyzer(root: Path, requested: str) -> str:
    requested_path = Path(requested).expanduser()
    if requested_path.is_absolute() or "/" in requested:
        if not requested_path.is_file():
            raise RuntimeError(f"rust-analyzer binary not found: {requested}")
        return str(requested_path.resolve())

    if requested == "rust-analyzer":
        rustup_candidate = rustup_which("rust-analyzer")
        if rustup_candidate is not None:
            return str(rustup_candidate)
        fallback = fallback_rust_analyzer(root)
        if fallback is not None:
            return str(fallback)
        raise RuntimeError("unable to locate a real rust-analyzer binary")

    binary_path = shutil.which(requested)
    if binary_path is None:
        raise RuntimeError(f"rust-analyzer binary not found in PATH: {requested}")
    return binary_path


def container_is_available(name: str) -> bool:
    docker = shutil.which("docker")
    if docker is None:
        return False
    result = subprocess.run(
        [docker, "exec", name, "true"],
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    return result.returncode == 0


def should_run_in_container(args: argparse.Namespace, root: Path) -> bool:
    if args.container_mode == "never":
        return False
    if Path("/.dockerenv").exists() or root.as_posix().startswith(args.container_repo_dir):
        return False
    if args.container_mode == "always":
        return True
    return container_is_available(args.container_name)


def container_repo_path(host_path: Path, host_root: Path, container_root: str) -> str:
    relative = host_path.resolve().relative_to(host_root.resolve())
    return str(Path(container_root) / relative)


def container_rust_analyzer_ld_library_path(host_binary: Path) -> str | None:
    try:
        toolchains_index = host_binary.parts.index("toolchains")
    except ValueError:
        return None
    if toolchains_index + 1 >= len(host_binary.parts):
        return None
    toolchain_name = host_binary.parts[toolchains_index + 1]
    return str(Path("/root/.rustup/toolchains") / toolchain_name / "lib")


def ensure_container_rust_analyzer(
    host_root: Path,
    container_root: str,
    requested: str,
) -> tuple[str, str | None]:
    host_binary = Path(resolve_local_rust_analyzer(host_root, requested))
    container_binary_path = host_root / DEFAULT_CONTAINER_RUST_ANALYZER_RELATIVE_PATH
    container_binary_path.parent.mkdir(parents=True, exist_ok=True)
    if (
        not container_binary_path.exists()
        or host_binary.stat().st_size != container_binary_path.stat().st_size
        or int(host_binary.stat().st_mtime) != int(container_binary_path.stat().st_mtime)
    ):
        shutil.copy2(host_binary, container_binary_path)
        container_binary_path.chmod(0o755)
    return (
        container_repo_path(container_binary_path, host_root, container_root),
        container_rust_analyzer_ld_library_path(host_binary),
    )


def filtered_forward_args(raw_args: list[str]) -> list[str]:
    stripped: list[str] = []
    skip_next = False
    flags_with_values = {
        "--container-mode",
        "--container-name",
        "--container-repo-dir",
        "--root",
        "--rust-analyzer",
    }
    for arg in raw_args:
        if skip_next:
            skip_next = False
            continue
        matched_flag = next(
            (flag for flag in flags_with_values if arg == flag or arg.startswith(f"{flag}=")),
            None,
        )
        if matched_flag is None:
            stripped.append(arg)
            continue
        if "=" not in arg:
            skip_next = True
    return stripped


def rerun_in_container(args: argparse.Namespace, root: Path) -> int:
    docker = shutil.which("docker")
    if docker is None:
        raise RuntimeError("docker is not available for container-backed ra_code_nav")

    container_rust_analyzer, container_ld_library_path = ensure_container_rust_analyzer(
        root,
        args.container_repo_dir,
        args.rust_analyzer,
    )
    container_script = container_repo_path(Path(__file__), root, args.container_repo_dir)
    forwarded_args = [
        "--container-mode=never",
        f"--root={args.container_repo_dir}",
        f"--rust-analyzer={container_rust_analyzer}",
        *filtered_forward_args(sys.argv[1:]),
    ]
    shell_command = "cd {repo} && ".format(repo=shlex.quote(args.container_repo_dir))
    if container_ld_library_path is not None:
        shell_command += "export LD_LIBRARY_PATH={ld}:$LD_LIBRARY_PATH && ".format(
            ld=shlex.quote(container_ld_library_path),
        )
    shell_command += "exec python3 {script} {args}".format(
        repo=shlex.quote(args.container_repo_dir),
        script=shlex.quote(container_script),
        args=" ".join(shlex.quote(arg) for arg in forwarded_args),
    )
    result = subprocess.run(
        [docker, "exec", "-i", args.container_name, "bash", "-lc", shell_command],
        check=False,
    )
    return result.returncode


class LspClient:
    def __init__(self, rust_analyzer: str, root: Path, timeout: float) -> None:
        self.root = root.resolve()
        self.timeout = timeout
        self.next_id = 1
        self.responses: dict[int, dict[str, Any]] = {}
        self.messages: "queue.Queue[dict[str, Any]]" = queue.Queue()
        env = os.environ.copy()
        env.setdefault("RA_LOG", "error")
        self.process = subprocess.Popen(
            [rust_analyzer],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            cwd=str(self.root),
            env=env,
        )
        assert self.process.stdin is not None
        assert self.process.stdout is not None
        self.stdin = self.process.stdin
        self.stdout = self.process.stdout
        self.reader = threading.Thread(target=self._read_loop, daemon=True)
        self.reader.start()

    def _read_loop(self) -> None:
        while True:
            headers: dict[str, str] = {}
            while True:
                line = self.stdout.readline()
                if not line:
                    return
                decoded = line.decode("ascii", errors="replace").strip()
                if decoded == "":
                    break
                name, _, value = decoded.partition(":")
                headers[name.lower()] = value.strip()

            content_length = int(headers.get("content-length", "0"))
            if content_length == 0:
                continue
            body = self.stdout.read(content_length)
            try:
                message = json.loads(body.decode("utf-8"))
            except json.JSONDecodeError:
                continue
            self.messages.put(message)

    def _send(self, payload: dict[str, Any]) -> None:
        body = json.dumps(payload, separators=(",", ":")).encode("utf-8")
        header = f"Content-Length: {len(body)}\r\n\r\n".encode("ascii")
        self.stdin.write(header + body)
        self.stdin.flush()

    def request(self, method: str, params: dict[str, Any] | None = None) -> Any:
        request_id = self.next_id
        self.next_id += 1
        payload: dict[str, Any] = {
            "jsonrpc": "2.0",
            "id": request_id,
            "method": method,
        }
        if params is not None:
            payload["params"] = params
        self._send(payload)

        deadline = time.monotonic() + self.timeout
        while time.monotonic() < deadline:
            try:
                message = self.messages.get(timeout=0.1)
            except queue.Empty:
                if self.process.poll() is not None:
                    raise RuntimeError("rust-analyzer exited before responding")
                continue
            if message.get("id") == request_id:
                if "error" in message:
                    raise RuntimeError(json.dumps(message["error"], ensure_ascii=False))
                return message.get("result")
        raise TimeoutError(f"timed out waiting for {method}")

    def notify(self, method: str, params: dict[str, Any] | None = None) -> None:
        payload: dict[str, Any] = {"jsonrpc": "2.0", "method": method}
        if params is not None:
            payload["params"] = params
        self._send(payload)

    def initialize(self) -> None:
        result = self.request(
            "initialize",
            {
                "processId": os.getpid(),
                "rootUri": path_to_uri(self.root),
                "workspaceFolders": [
                    {
                        "uri": path_to_uri(self.root),
                        "name": self.root.name,
                    }
                ],
                "clientInfo": {
                    "name": "exfat-refactor-ra-code-nav",
                    "version": "0.1",
                },
                "capabilities": {
                    "workspace": {
                        "symbol": {
                            "symbolKind": {
                                "valueSet": list(SYMBOL_KIND_NAMES.keys()),
                            }
                        }
                    },
                    "textDocument": {
                        "documentSymbol": {
                            "hierarchicalDocumentSymbolSupport": True,
                            "symbolKind": {
                                "valueSet": list(SYMBOL_KIND_NAMES.keys()),
                            },
                        },
                        "definition": {"linkSupport": True},
                        "implementation": {"linkSupport": True},
                        "references": {},
                        "hover": {"contentFormat": ["markdown", "plaintext"]},
                    },
                },
                "initializationOptions": {
                    "checkOnSave": False,
                    "cargo": {
                        "buildScripts": {"enable": False},
                    },
                    "procMacro": {"enable": False},
                },
            },
        )
        if result is None:
            raise RuntimeError("rust-analyzer returned no initialize result")
        self.notify("initialized", {})

    def shutdown(self) -> None:
        try:
            self.request("shutdown", {})
            self.notify("exit")
        except Exception:
            self.process.terminate()

    def open_document(self, path: Path) -> None:
        text = path.read_text()
        self.notify(
            "textDocument/didOpen",
            {
                "textDocument": {
                    "uri": path_to_uri(path),
                    "languageId": "rust",
                    "version": 1,
                    "text": text,
                }
            },
        )


def flatten_document_symbols(
    symbols: list[dict[str, Any]],
    root: Path,
    path: Path,
    container: str = "",
) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    for symbol in symbols:
        selection_range = symbol.get("selectionRange") or symbol.get("range") or {}
        start = selection_range.get("start", {})
        line, column = one_based_position(start)
        name = symbol.get("name", "")
        kind = SYMBOL_KIND_NAMES.get(symbol.get("kind"), str(symbol.get("kind")))
        records.append(
            {
                "name": name,
                "kind": kind,
                "path": rel_path(str(path), root),
                "line": line,
                "column": column,
                "container": container,
                "detail": symbol.get("detail", ""),
            }
        )
        child_container = f"{container}::{name}" if container else name
        records.extend(
            flatten_document_symbols(
                symbol.get("children", []),
                root,
                path,
                child_container,
            )
        )
    return records


def run_symbols(client: LspClient, args: argparse.Namespace) -> None:
    if args.settle_seconds > 0:
        time.sleep(args.settle_seconds)
    result = client.request("workspace/symbol", {"query": args.query}) or []
    records = []
    for symbol in result:
        location = symbol.get("location") or {}
        record = location_to_record(location, client.root, args.exclude_glob)
        if record is None:
            continue
        records.append(
            {
                "name": symbol.get("name", ""),
                "kind": SYMBOL_KIND_NAMES.get(symbol.get("kind"), str(symbol.get("kind"))),
                "container": symbol.get("containerName", ""),
                **record,
            }
        )
        if len(records) >= args.limit:
            break
    print_jsonl(records)


def run_file_symbols(client: LspClient, args: argparse.Namespace) -> None:
    path = (client.root / args.path).resolve()
    client.open_document(path)
    result = client.request(
        "textDocument/documentSymbol",
        {"textDocument": {"uri": path_to_uri(path)}},
    ) or []
    if result and "location" in result[0]:
        records = []
        for symbol in result:
            record = location_to_record(symbol["location"], client.root, args.exclude_glob)
            if record is None:
                continue
            records.append(
                {
                    "name": symbol.get("name", ""),
                    "kind": SYMBOL_KIND_NAMES.get(symbol.get("kind"), str(symbol.get("kind"))),
                    "container": symbol.get("containerName", ""),
                    **record,
                }
            )
    else:
        records = flatten_document_symbols(result[: args.limit], client.root, path)
        records = records[: args.limit]
    print_jsonl(records)


def text_document_position(client: LspClient, args: argparse.Namespace) -> dict[str, Any]:
    path = (client.root / args.path).resolve()
    client.open_document(path)
    if args.settle_seconds > 0:
        time.sleep(args.settle_seconds)
    return {
        "textDocument": {"uri": path_to_uri(path)},
        "position": {
            "line": args.line - 1,
            "character": args.column - 1,
        },
    }


def run_locations(
    client: LspClient,
    method: str,
    params: dict[str, Any],
    limit: int,
    exclude_globs: list[str],
) -> None:
    result = client.request(method, params)
    if result is None:
        print_jsonl([])
        return
    if isinstance(result, dict):
        result = [result]
    records = []
    for location in result:
        record = location_to_record(location, client.root, exclude_globs)
        if record is None:
            continue
        records.append(record)
        if len(records) >= limit:
            break
    print_jsonl(records)


def run_hover(client: LspClient, args: argparse.Namespace) -> None:
    result = client.request("textDocument/hover", text_document_position(client, args))
    print(json.dumps(result or {}, ensure_ascii=False, sort_keys=True))


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Query rust-analyzer for symbol-aware Asterinas code navigation.",
    )
    parser.add_argument(
        "--root",
        default=None,
        help="Repository root. Defaults to nearest parent with .git and Cargo.toml.",
    )
    parser.add_argument(
        "--rust-analyzer",
        default="rust-analyzer",
        help="rust-analyzer binary path.",
    )
    parser.add_argument(
        "--container-mode",
        choices=("auto", "always", "never"),
        default="auto",
        help="Run the LSP helper inside the dev container when available. Default: auto.",
    )
    parser.add_argument(
        "--container-name",
        default=DEFAULT_CONTAINER_NAME,
        help=f"Docker container name. Default: {DEFAULT_CONTAINER_NAME}.",
    )
    parser.add_argument(
        "--container-repo-dir",
        default=DEFAULT_CONTAINER_REPO_DIR,
        help=f"Repository path inside the container. Default: {DEFAULT_CONTAINER_REPO_DIR}.",
    )
    parser.add_argument(
        "--timeout",
        type=float,
        default=60.0,
        help="Per-request timeout in seconds.",
    )
    parser.add_argument(
        "--settle-seconds",
        type=float,
        default=3.0,
        help="Delay after initialization before workspace-wide queries.",
    )
    parser.add_argument(
        "--include-legacy-exfat",
        action="store_true",
        help="Include legacy kernel/src/fs/fs_impls/exfat results. Excluded by default.",
    )
    parser.add_argument(
        "--exclude-glob",
        action="append",
        default=[],
        help="Additional repository-relative glob to exclude from results.",
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    symbols = subparsers.add_parser("symbols", help="Workspace symbol search by name.")
    symbols.add_argument("query")
    symbols.add_argument("--limit", type=int, default=50)

    file_symbols = subparsers.add_parser("file-symbols", help="List symbols in one file.")
    file_symbols.add_argument("path")
    file_symbols.add_argument("--limit", type=int, default=200)

    definition = subparsers.add_parser("definition", help="Go to definition at 1-based line/column.")
    definition.add_argument("path")
    definition.add_argument("line", type=int)
    definition.add_argument("column", type=int)
    definition.add_argument("--limit", type=int, default=20)

    references = subparsers.add_parser("references", help="Find references at 1-based line/column.")
    references.add_argument("path")
    references.add_argument("line", type=int)
    references.add_argument("column", type=int)
    references.add_argument("--include-declaration", action="store_true")
    references.add_argument("--limit", type=int, default=100)

    implementation = subparsers.add_parser(
        "implementation",
        help="Find implementations at 1-based line/column.",
    )
    implementation.add_argument("path")
    implementation.add_argument("line", type=int)
    implementation.add_argument("column", type=int)
    implementation.add_argument("--limit", type=int, default=100)

    hover = subparsers.add_parser("hover", help="Show hover/type info at 1-based line/column.")
    hover.add_argument("path")
    hover.add_argument("line", type=int)
    hover.add_argument("column", type=int)

    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    root = Path(args.root).resolve() if args.root else find_repo_root(Path.cwd())
    if not args.include_legacy_exfat:
        args.exclude_glob = [*DEFAULT_EXCLUDE_GLOBS, *args.exclude_glob]

    if should_run_in_container(args, root):
        return rerun_in_container(args, root)

    client = LspClient(resolve_local_rust_analyzer(root, args.rust_analyzer), root, args.timeout)
    try:
        client.initialize()
        if args.command == "symbols":
            run_symbols(client, args)
        elif args.command == "file-symbols":
            run_file_symbols(client, args)
        elif args.command == "definition":
            params = text_document_position(client, args)
            run_locations(client, "textDocument/definition", params, args.limit, args.exclude_glob)
        elif args.command == "references":
            params = text_document_position(client, args)
            params["context"] = {"includeDeclaration": args.include_declaration}
            run_locations(client, "textDocument/references", params, args.limit, args.exclude_glob)
        elif args.command == "implementation":
            params = text_document_position(client, args)
            run_locations(
                client,
                "textDocument/implementation",
                params,
                args.limit,
                args.exclude_glob,
            )
        elif args.command == "hover":
            run_hover(client, args)
        else:
            parser.error(f"unknown command {args.command}")
    finally:
        client.shutdown()
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (RuntimeError, TimeoutError, BrokenPipeError) as err:
        print(f"ra_code_nav.py: {err}", file=sys.stderr)
        raise SystemExit(1)
