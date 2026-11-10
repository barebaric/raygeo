---
title: raygeo.cnc.plan.tapping
sidebar_label: raygeo.cnc.plan.tapping
---

CNC rigid-tapping plan builder.

Produces a plan that rigid-taps internal threads: one spindle- synchronized cycle per pilot bore,
all sharing the same thread parameters. Bores come either from an explicit center list or are
detected automatically as circular island contours of the face and validated against the thread
geometry. Tapping feeds derive from the thread itself — every synchronized move travels pitch * rpm
mm/min — so the spindle speed is mandatory.

## Functions

### `plan_tapping()`

```python
plan_tapping(
    part: ops.part.Part,
    face_id: str = '',
    nominal_diameter: float = 6,
    pitch: float = 1,
    target_z: float = -12,
    z_surface: float = 0,
    rapid_z: float = 1,
    safe_z: float = 5,
    handedness: str = 'rh',
    peck: Optional[float] = None,
    dwell_ms: float = 0,
    spindle_rpm: Optional[int] = None,
    coolant: Optional[str] = None,
    holes: Optional[Sequence[tuple[float, float]]] = None,
    hole_tolerance: float = 0.1,
    hole_order: str = 'detection',
) -> cnc.plan.Plan
```

Build a plan that rigid-taps internal threads on one face.

One spindle-synchronized cycle per pilot bore; all bores share the thread parameters. Bores come
from `holes` when given (explicit centers), otherwise they are detected as circular island contours
of the face and validated: a bore at or above the nominal diameter has nothing left to tap. The
synchronized moves travel `pitch * spindle_rpm` mm/min, so `spindle_rpm` is mandatory. Bores needing
different parameters need a separate call.

| Parameter          | Type                                             | Description                                                                                                       |
| ------------------ | ------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------- |
| `part`             | `ops.part.Part`                                  | part whose face is tapped.                                                                                        |
| `face_id`          | `str = ''`                                       | face to tap ("" is the default face).                                                                             |
| `nominal_diameter` | `float = 6`                                      | nominal (major) thread diameter (mm).                                                                             |
| `pitch`            | `float = 1`                                      | thread pitch (lead) (mm).                                                                                         |
| `target_z`         | `float = -12`                                    | depth of the thread (default -12).                                                                                |
| `z_surface`        | `float = 0`                                      | surface Z the holes start at (default 0).                                                                         |
| `rapid_z`          | `float = 1`                                      | rapid plane above the surface (default 1).                                                                        |
| `safe_z`           | `float = 5`                                      | safe rapid height (default 5).                                                                                    |
| `handedness`       | `str = 'rh'`                                     | `"rh"` (default) or `"lh"`.                                                                                       |
| `peck`             | `Optional[float] = None`                         | optional peck increment; full synchronized retract between pecks (default None).                                  |
| `dwell_ms`         | `float = 0`                                      | dwell at the bottom before the spindle reversal (default 0).                                                      |
| `spindle_rpm`      | `Optional[int] = None`                           | spindle speed (rpm); required.                                                                                    |
| `coolant`          | `Optional[str] = None`                           | `"flood"`, `"mist"` or `"off"`; None leaves the plan's coolant unset.                                             |
| `holes`            | `Optional[Sequence[tuple[float, float]]] = None` | explicit pilot-bore centers `[(x, y), ...]`; None detects circular islands on the face.                           |
| `hole_tolerance`   | `float = 0.1`                                    | max distance (mm) between an island contour sample and the fitted circle for it to count as a bore (default 0.1). |
| `hole_order`       | `str = 'detection'`                              | `"detection"` or `"nearest_neighbor"`.                                                                            |
| _Returns_          | `cnc.plan.Plan`                                  | the tapping plan, one step per bore.                                                                              |
