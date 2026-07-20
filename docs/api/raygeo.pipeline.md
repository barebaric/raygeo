---
title: raygeo.pipeline
sidebar_label: raygeo.pipeline
---

Runtime intent-tree pipeline.

Executes trees of NodeRequests on a rayon thread pool. Each node carries an opaque stage that the
CNC layer interprets.

Submodules:

- **request** — `NodeRequest`: one node of the intent tree.
- **completed** — `CompletedNode`: completion record.
- **execute** — `execute_stages`: the rayon-scoped runner.
