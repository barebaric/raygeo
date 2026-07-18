---
title: raygeo.ops.transform.bidir_scan_offset
sidebar_label: raygeo.ops.transform.bidir_scan_offset
---

## BidirScanOffsetSpec

Parameters for the `BidirScanOffset` transformer.

Construct with `BidirScanOffsetSpec(offset_mm)`. An offset of 0.0 is a legitimate no-op spec.

### `offset_mm`

```python
offset_mm: float
```

X offset in millimeters applied to right-to-left raster passes.
