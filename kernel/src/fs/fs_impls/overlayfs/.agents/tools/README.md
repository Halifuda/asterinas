# Tooling Expectations

This bundle does not ship the original workspace helper scripts.
If you want to keep the same command-lane contract, provide compatible versions of:
- `checker_lock.sh`
- `checker_run.sh`
- `xfstests_run.sh` (optional, only if the workspace uses xfstests)

Rust code navigation is handled by the `ra-code-nav` skill at the repository
root (`.agents/skills/ra-code-nav/`), which queries a pre-generated
rust-analyzer LSIF index with shell + `jq`. No `ra_code_nav.py` script is
needed.

Update local protocol text if your adopting workspace uses different helper names or lanes.
