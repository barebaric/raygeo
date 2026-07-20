---
title: raygeo.pipeline.stage
sidebar_label: raygeo.pipeline.stage
---

Stage specification types.

## StageSpec

Stage specification for one node of the intent tree.

`Compute` — a leaf node that produces Ops from geometry via an assembler. `Aggregate` — an interior
node that concatenates and transforms Ops from one or more upstream nodes.

### Aggregate

An aggregate interior node.

### Compute

A compute leaf node.
