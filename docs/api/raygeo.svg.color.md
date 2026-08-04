---
title: raygeo.svg.color
sidebar_label: raygeo.svg.color
---

SVG color attribute selection.

Selects which color attribute (fill, stroke, fill-else-stroke, or any) determines the color bucket
of a shape.

## ColorAttr

Which color attribute of a shape determines its color bucket.

`FILL_ELSE_STROKE` uses the fill color when present, otherwise the stroke color. `ANY` buckets a
shape by both its fill and its stroke when they differ, producing two layers (one per color).

**Values:**

- `ANY` — Bucket by both fill and stroke when they differ, producing two layers (one per color).
- `FILL` — Bucket by the resolved `fill` paint.
- `FILL_ELSE_STROKE` — Use the fill paint when present, otherwise the stroke paint.
- `STROKE` — Bucket by the resolved `stroke` paint.
