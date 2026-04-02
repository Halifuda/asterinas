<!-- SPDX-License-Identifier: MPL-2.0 -->

# Task Packet Archive

This directory stores the actual task packets sent to ordinary subagents.

Packets are workflow artifacts, not disposable prompts.
They exist so later review can answer questions such as:

- what prior slices the main agent actually delivered,
- what files a delegated role was allowed to read or edit,
- whether the stop condition was explicit,
- whether a role was given the right execution environment,
- whether a quality-prior slice was sent without over-specifying another role's job.

## Layout

Archive packets under a per-component directory:

```text
.agents/subagent-tasks/<component-id>/
  00_architect_packet.md
  01_designer_core_packet.md
  02_designer_async_packet.md
  03_designer_ktest_packet.md
  10_creator_serial_packet.md
  11_checker_serial_packet.md
  12_advisor_serial_packet.md
  30_reviewer_report_packet.md
  31_checker_final_packet.md
```

If a packet must be reissued for the same step, keep the old file and write a new one with a suffix such as `_v2`.

## Packet Trace

Every delegated role artifact should cite the archived packet path in its metadata.
If a step was done locally by the main agent instead of a delegated subagent, the artifact may say so explicitly instead of citing a packet file.
