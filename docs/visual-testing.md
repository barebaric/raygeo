# Visual Testing

raygeo includes a Streamlit-based interactive playground for visually exploring geometry operations,
rasterization, nesting, and more.

## Running

```bash
pip install -e ".[visual]"
make visual
```

This launches a web UI where you can interact with raygeo operations in real time through a
sidebar-navigated set of pages.

## Pages

### Geometry

A full interactive geometry editor with four tabs:

- **Build** — Choose preset shapes (rectangle, circle, regular polygon, star) or enter raw points.
  The geometry is plotted live with optional control-point display.
- **Transform** — Apply operations interactively: grow/shrink (offset), flip X/Y, simplify,
  linearize, split contours, or remove inner edges.
- **Analyze** — Shows command count, signed area, total path distance, bounding box, and closed
  status.
- **Curve Fitting** — Fit arcs or arcs+beziers back to linearized geometry at a configurable
  tolerance.

All tabs update the same geometry in sequence, so you can build, then transform, then analyze, then
fit curves on the same path.

### Polygon Boolean

Interactive boolean operations between two polygons (union, intersection, difference). Each shape
can be a square or circle with adjustable size and relative offset. The result polygon is overlaid
in green.

### Polygon Offset

Offset (inflate/deflate) a polygon by a configurable amount. The original and offset polygon(s) are
shown superimposed.

### Image Processing

Two sub-sections:

- **sRGB Round-Trip** — Generate a test pattern (gradient, checkered, circle, noise) and view the
  sRGB → linear → sRGB round-trip side by side.
- **Dithering** — Apply Floyd-Steinberg or Bayer 4×4 dithering to the grayscale image, with optional
  inversion.

### SVG Parsing

Test the SVG path data parser. Choose a preset (rectangle+circle, star) or paste custom SVG path
data. The parsed geometries are displayed with color-coded sub-paths.

### Tab Operations

Apply tabs (gaps or power reductions) to a shape's outline. Choose rectangle, circle, or rounded
rectangle as the base contour, set the number and width of tabs, and toggle between gap mode and
power-reduction mode. The original and modified command streams are compared side by side.

### Merge Lines

Demonstrates the `merge_overlapping_lines` operation with several presets: near-duplicate lines,
identical duplicates, overlapping collinear segments, adjacent rectangles, and triangles sharing an
edge. Also supports custom line input. The original lines are shown in translucent red, the merged
result in green.

### Overscan

Visualizes `apply_overscan` on raster scan lines. Presets include horizontal raster lines,
bidirectional raster, diagonal lines, variable power scanlines, and mixed raster+vector. Original
lines are shown in translucent red, overscanned output in green, and travel moves in gray.

### Lead-In/Out

Demonstrates `apply_lead_in_out` on closed contours (rectangle, triangle, circle, multiple contours)
and open paths. Lead-in and lead-out segments are shown in blue (zero power), cut segments in green,
and travel in gray.

### Concave Hull (Shrink-Wrap)

Compute convex and concave hulls from rasterized shapes. Choose a preset shape (two squares,
hourglass, L-shape, circle, three dots) or upload a custom SVG. Adjustable resolution and gravity
parameters let you control the shrink-wrap tightness. Per-component hulls are also computed and
displayed.

### Rasterization

Full-featured rasterization playground supporting all four modes:

- **Power Modulation** — Sample grayscale values and emit scan lines with per-sample power data.
- **Mask Scan** — Rasterize a binary mask into scan lines (one line per pixel row/column).
- **Mask Lines** — Rasterize a binary mask into individual line segments outlining the mask
  boundary.
- **Multi-Pass** — Slice grayscale into depth levels and emit a separate raster pass for each level.

Each mode supports segmented and full-sweep scan patterns, adjustable line interval, angle, and
pixels-per-mm. Scan-line power is rendered with a colormap in the output plot.

### Nesting

Interactive part nesting using the NFP (no-fit polygon) placement engine and optional genetic
algorithm optimization. Configure part shape and size, sheet dimensions, spacing, rotation limits,
and flip settings. The genetic algorithm evolves placement order and orientation across multiple
generations. Results show placed parts per sheet with optional gravity settling post-placement.
