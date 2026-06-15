---
title: raygeo.geo.algo.cylindrical
sidebar_label: raygeo.geo.algo.cylindrical
sidebar_position: 6
---

## Functions

### `transform_to_cylinder()`

`transform_to_cylinder(verts: numpy.NDArray[numpy.float32], diameter: float, colors: numpy.NDArray[numpy.float32] | None = None, degrees_input: bool = False) -> tuple[numpy.NDArray[numpy.float32], numpy.NDArray[numpy.float32] | None, numpy.NDArray[numpy.int32]]`

Transform flat vertex pairs to cylindrical coordinates.

The input has angular values on axis 1 (Y) and linear position on axis 0 (X). The output maps to: X
= linear position along cylinder Y = r _ sin(theta) Z = r _ cos(theta)

Line segments are subdivided as needed to follow the cylinder surface instead of cutting through the
interior.

              Vertices are in pairs (line segments for GL_LINES).
                      directly. If False (default), they are in motor
                      units (mu) and converted via mu_to_degrees.

**Returns:** Tuple of (transformed_vertices, expanded_colors, cum_subs). cum_subs is a cumulative
subdivision count array.

| Parameter       | Type                                                                                                        | Description                                            |
| --------------- | ----------------------------------------------------------------------------------------------------------- | ------------------------------------------------------ |
| `verts`         | `numpy.NDArray[numpy.float32]`                                                                              | Float32 array of shape (N, 3) with X, Y, Z per vertex. |
| `diameter`      | `float`                                                                                                     | Cylinder diameter in mm.                               |
| `colors`        | `numpy.NDArray[numpy.float32] &#124; None = None`                                                           | Optional float32 array of shape (N, 4) RGBA colors.    |
| `degrees_input` | `bool = False`                                                                                              | If True, Y values are in degrees and converted         |
| _Returns_       | `tuple[numpy.NDArray[numpy.float32], numpy.NDArray[numpy.float32] &#124; None, numpy.NDArray[numpy.int32]]` |                                                        |
| _Complexity_    |                                                                                                             | O(N \* subdivisions)                                   |

![Flat vertex pairs wrapped onto a cylinder surface](images/cylindrical-transform.png)

_Flat vertex pairs wrapped onto a cylinder surface_
