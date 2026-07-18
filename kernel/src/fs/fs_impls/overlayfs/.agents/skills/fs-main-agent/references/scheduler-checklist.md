# Scheduler Checklist

Use this note when resuming or reshaping the filesystem implementation/refactor board for the current workspace.

## Core ownership

- Read `README.md`, `PROTOCOL.md`, `SYSTEM_BLUEPRINT.md`, and the latest main-agent handoff first.
- Treat `.agents/PROTOCOL.md` as the normative scheduler law.
- Only the main agent updates `SYSTEM_BLUEPRINT.md` or official component state.
- Only the main agent decides protocol acceptance, rejection, stale-lock clearing, and escalation.

## Gate model

Normal path:

```text
Planned -> Architected -> Specified
  -> One or more creator/checker pass loops
  -> Independent meso integration checker pass(es)
  -> Reviewer
  -> Optional final checker
  -> Accepted
```

- `Architected` requires the Architect artifact expected by the current packet or blueprint phase.
- `Specified` requires the Designer spec and Designer validation contract.
- Creator/checker pass pairs must name the same parent meso-component and covered micro-features.
- `Accepted` requires structural acceptance of artifacts plus upstream-approved validation evidence when runtime execution was required.

## Lane model

- There is one serialized execution lane.
- Checker owns command-producing runtime verification.
- Checker must not add or recommend filesystem-local ktests or test-support code under `kernel/src/fs/fs_impls/`; filesystem behavior validation should use the upstream-approved lane, currently expected to be NixOS xfstests.
- Command-free roles should continue in parallel when dependencies and write-sets allow.
- If a command-free delegation stalls, repair the packet or routing before absorbing the work into the main thread.
