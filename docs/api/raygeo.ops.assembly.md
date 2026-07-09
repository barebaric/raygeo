---
title: raygeo.ops.assembly
sidebar_label: raygeo.ops.assembly
---

Motion-path assembly: turning raw geometry primitives into Ops.

Functions in this module compose geo-layer primitives (polylines, arcs, polygons) into complete
motion sequences represented as Ops objects. They decide traversal order, linking strategy,
lead-in/out, overscan, and tab insertion — concerns that belong to motion assembly rather than pure
geometry.
