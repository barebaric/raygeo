---
title: raygeo.geo.algo
sidebar_label: raygeo.geo.algo
sidebar_position: 3
---

Geometric algorithms for path processing.

This module provides algorithms that operate on geometry paths and point sequences. It covers
several categories of geometric processing:

Clipping — intersect and clip line segments against rectangles and polygon regions. Also includes
coordinate conversion between floating point and Clipper's integer grid system for boolean-accuracy
clipping.

Fitting — reconstruct curves (arcs, lines, beziers) from unordered point sequences. Includes
recursive primitive fitting, circle fitting, polyline linearization, and deviation analysis to
evaluate fit quality.

Minkowski sums — compute Minkowski sums, convolutions, and no-fit polygons for 2D toolpath
generation, nesting, and packing algorithms.

Nesting — 2D packing algorithms for sheet/plate layout (NFP, IFP, placement, gravity, genetic
optimization).

Simplification — reduce the number of points in a polyline while preserving shape within a tolerance
(Ramer-Douglas-Peucker).

Smoothing — apply Gaussian kernel smoothing to polylines with configurable corner-angle thresholds
to preserve sharp features.
