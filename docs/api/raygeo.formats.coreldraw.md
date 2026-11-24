---
title: raygeo.formats.coreldraw
sidebar_label: raygeo.formats.coreldraw
---

![Two drawings parsed natively: page frame, layers and geometry](images/formats-coreldraw-pages.png)

*Two drawings parsed natively: page frame, layers and geometry* CorelDRAW (`.cdr`) import and
export.

Parses the drawing natively: RIFF and ZIP containers, compressed chunk lists, object records and
their transforms are all read in Rust, so no external tool or C library is needed. Text is drawn as
glyph outlines with the system's closest font; bitmaps and object kinds the reader does not know are
reported in **CdrDocument.issues** instead of being converted.

**write** creates a version 12 drawing from a geometry, so a result can be opened in CorelDRAW and
other vector editors.

## CdrDocument

The geometry of a CorelDRAW page.

### `issues`

```python
issues: list[str]
```

Content the reader did not convert, one note per category.

### `layers`

```python
layers: list[tuple[str, geo.Geometry]]
```

The layers of the first page, bottom to top, as `(name, geometry)` pairs. The geometry is in inches,
relative to the page's bottom-left corner.

### `page_count`

```python
page_count: int
```

The number of non-master pages in the document.

### `page_height`

```python
page_height: float
```

The page height in inches.

### `page_width`

```python
page_width: float
```

The page width in inches.

### `units_to_mm`

```python
units_to_mm: float
```

Multiplier converting the native unit (inches) to millimetres.

## CdrError

Base class of the CorelDRAW reader errors.

## CdrFormat

The kind of Corel document a byte stream holds.

**Values:**

- `CDR` — A CorelDRAW drawing (`.cdr`).
- `CMX` — A Corel Presentation Exchange file (`.cmx`).

## CdrParseError

Raised when a CorelDRAW container, record or argument table cannot be read.

## CdrUnsupportedFormat

Raised when the data is not a CorelDRAW or CMX document.

## Functions

### `detect_format()`

```python
detect_format(data: bytes) -> formats.coreldraw.CdrFormat | None
```

Return the kind of Corel document in `data`.

Recognizes a bare RIFF drawing, a ZIP package holding one, and the earliest `WL` format; CMX files
are recognized but not read.

| Parameter    | Type                                      | Description                                                         |
| ------------ | ----------------------------------------- | ------------------------------------------------------------------- |
| `data`       | `bytes`                                   | The first bytes of the file, or the whole file.                     |
| _Returns_    | `formats.coreldraw.CdrFormat &#124; None` | The document kind, or `None` when the data is not a Corel document. |
| _Complexity_ |                                           | O(n) where n = size of the data                                     |

![Container and version recognized from the leading bytes](images/formats-coreldraw-detect-format.png)

*Container and version recognized from the leading bytes*

### `parse()`

```python
parse(data: bytes) -> formats.coreldraw.CdrDocument
```

Read a CorelDRAW drawing into the geometry of its first page.

The whole file is parsed in Rust: RIFF and ZIP containers, compressed chunk lists, object records
and their transforms. Drawing geometry and text are extracted; text becomes glyph outlines of the
system's closest font, since CorelDRAW's embedded fonts are not used. Bitmaps and unknown object
kinds are listed in `issues`. The layers of the first page come bottom to top, their geometry in
inches with the page origin at the bottom-left corner.

**Raises:** `CdrUnsupportedFormat` — When the data is not a CorelDRAW

```
document.
```

**Raises:** `CdrParseError` — When the container, a record or an argument

```
table cannot be read, or when the file is a CMX file.
```

| Parameter    | Type                            | Description                                          |
| ------------ | ------------------------------- | ---------------------------------------------------- |
| `data`       | `bytes`                         | The document's bytes.                                |
| _Returns_    | `formats.coreldraw.CdrDocument` | The page size, its layers and the conversion issues. |
| _Complexity_ |                                 | O(n) where n = size of the document                  |

![A two-page document: page one is returned, its text drawn as glyph outlines](images/formats-coreldraw-parse.png)

*A two-page document: page one is returned, its text drawn as glyph outlines*

### `write()`

```python
write(geometry: geo.Geometry) -> bytes
```

Write geometry as a CorelDRAW version 12 drawing.

The page is the geometry's bounding box in millimetres, and the drawing is moved to the page's
bottom-left corner. It goes into one object on one layer, drawn with a black hairline outline and no
fill, so the file opens in CorelDRAW and other vector editors.

**Raises:** `RuntimeError` — When the geometry is empty.

| Parameter    | Type           | Description                            |
| ------------ | -------------- | -------------------------------------- |
| `geometry`   | `geo.Geometry` | The geometry to write, in millimetres. |
| _Returns_    | `bytes`        | The document's bytes.                  |
| _Complexity_ |                | O(n) where n = number of commands      |

![A geometry written to a version 12 file and read back](images/formats-coreldraw-write.png)

*A geometry written to a version 12 file and read back*
