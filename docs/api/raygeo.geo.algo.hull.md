---
title: raygeo.geo.algo.hull
sidebar_label: raygeo.geo.algo.hull
---

Hull computation from binary images.

Provides convex and concave (shrink-wrap) hull generation from boolean images, using contour tracing
and a vacuum-like pull of the hull toward the content. Coordinates are returned in image pixel space
(y increases downward).

## Functions

### `get_concave_hull()`

```python
get_concave_hull(
    boolean_image: numpy.ndarray,
    gravity: float = 0.1,
    allow_self_intersections: bool = False,
) -> geo.Geometry | None
```

Compute a concave (shrink-wrap) hull around the content.

The band behaves like a membrane under vacuum: each point is pulled along the inward normal of the
convex hull toward the content, tension keeps the band smooth, and pinch points stop it where it
would fold through itself or through the content.

| Parameter                  | Type                       | Description                                                                                                                     |
| -------------------------- | -------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| `boolean_image`            | `numpy.ndarray`            | 2D boolean array.                                                                                                               |
| `gravity`                  | `float = 0.1`              | Shrink-wrap factor 0.0-1.0. 0 gives convex hull.                                                                                |
| `allow_self_intersections` | `bool = False`             | When False, the band stops at pinch points instead of crossing itself. Set to True when a self-intersecting outline is desired. |
| _Returns_                  | `geo.Geometry &#124; None` | Concave hull as Geometry in pixel coords, or None.                                                                              |
| _Complexity_               |                            | O(w*h + n log n) time, O(w*h) space where w\*h is the image size and n the number of contour points                             |

![Concave vs convex hull](images/geo-algo-hull-concave.png)

*Concave vs convex hull*

### `get_enclosing_hull()`

```python
get_enclosing_hull(boolean_image: numpy.ndarray) -> geo.Geometry | None
```

Compute a single convex hull enclosing all content.

| Parameter       | Type                       | Description                                                                                      |
| --------------- | -------------------------- | ------------------------------------------------------------------------------------------------ |
| `boolean_image` | `numpy.ndarray`            | 2D boolean array.                                                                                |
| _Returns_       | `geo.Geometry &#124; None` | Convex hull as Geometry in pixel coords, or None.                                                |
| _Complexity_    |                            | O(w*h + n log n) time, O(n) space where w*h is the image size and n the number of contour points |

### `get_hulls_from_image()`

```python
get_hulls_from_image(boolean_image: numpy.ndarray) -> list[geo.Geometry]
```

Compute a separate convex hull for each distinct component.

| Parameter       | Type                 | Description                                                                                            |
| --------------- | -------------------- | ------------------------------------------------------------------------------------------------------ |
| `boolean_image` | `numpy.ndarray`      | 2D boolean array.                                                                                      |
| _Returns_       | `list[geo.Geometry]` | List of Geometry objects in pixel coords.                                                              |
| _Complexity_    |                      | O(w*h + n log n) time, O(n) space where w*h is the image size and n the total number of contour points |
