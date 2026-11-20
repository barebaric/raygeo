---
title: raygeo.sketch.geometry
sidebar_label: raygeo.sketch.geometry
---

## Functions

### `document_geometry()`

```python
document_geometry(
    registry: registry.EntityRegistry,
    constraints: Sequence[Any],
    resolved_text: Optional[Mapping[int, Optional[str]]] = None,
) -> geo.Geometry
```

Converts a sketch registry into geometry, linking chainable entities into continuous paths.

| Parameter       | Type                                           | Description |
| --------------- | ---------------------------------------------- | ----------- |
| `registry`      | `registry.EntityRegistry`                      |             |
| `constraints`   | `Sequence[Any]`                                |             |
| `resolved_text` | `Optional[Mapping[int, Optional[str]]] = None` |             |
| _Returns_       | `geo.Geometry`                                 |             |
