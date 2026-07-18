<!-- SPDX-License-Identifier: MPL-2.0 -->

# xfstests Prebuilt-Image Guide

Use this guide only if the adopting workspace chooses a prebuilt-image xfstests lane for early smoke or bounded remount validation.

Expected local decisions:
- define where reusable base images live under `.agents/xfstests/`
- define how per-run overlays or copies are created
- define how QEMU logs, suite results, and reproduce commands are archived before reuse
- define the filesystem-type proof that the intended `OverlayFs` mount actually ran

If the workspace does not use a prebuilt-image lane, remove references to this guide from its local skills and packets.
