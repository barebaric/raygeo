---
title: raygeo.image.composite
sidebar_label: raygeo.image.composite
---

## Functions

### `composite_views_into()`

```python
composite_views_into(target: Any, views: Any) -> None
```

Composite multiple ARGB32 bitmaps into a target buffer with per-view positioning and scaling.

Each source is placed at (dst_x, dst_y) in target pixel coordinates, scaled by (scale_x, scale_y).
Nearest-neighbour sampling is used. Alpha blending follows the pre-multiplied `over` operator.

| Parameter | Type   | Description                                                                                            |
| --------- | ------ | ------------------------------------------------------------------------------------------------------ |
| `target`  | `Any`  | `(H, W, 4)` `numpy.uint8` target buffer (zero-initialised ARGB32 premultiplied).                       |
| `views`   | `Any`  | List of `(source, dst_x, dst_y, scale_x, scale_y)` tuples where *source* is `(H, W, 4)` `numpy.uint8`. |
| _Returns_ | `None` | `None`                                                                                                 |
