---
title: raygeo.geo.algo.trace
sidebar_label: raygeo.geo.algo.trace
---

Contour extraction from binary images.

Provides boundary tracing of foreground regions in a boolean image, returning ordered point loops
around each component in pixel coordinates (y increases downward).

## Functions

### `find_external_contours()`

```python
find_external_contours(
    boolean_image: numpy.ndarray,
) -> list[list[tuple[float, float]]]
```

Trace the outer boundary of each foreground region.

Pixels with value 0 are treated as background; non-zero values are foreground. Each contour is an
ordered loop of (x, y) points in pixel coordinates; contours with fewer than 3 points are dropped.

| Parameter       | Type                              | Description                                            |
| --------------- | --------------------------------- | ------------------------------------------------------ |
| `boolean_image` | `numpy.ndarray`                   | 2D boolean array.                                      |
| _Returns_       | `list[list[tuple[float, float]]]` | List of contours, each a list of (x, y) points.        |
| _Complexity_    |                                   | O(w*h) time, O(w*h) space where w\*h is the image size |
