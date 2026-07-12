from enum import Enum


class CutSide(str, Enum):
    CENTERLINE = "centerline"
    INSIDE = "inside"
    OUTSIDE = "outside"


class CutOrder(str, Enum):
    INSIDE_OUTSIDE = "inside_outside"
    OUTSIDE_INSIDE = "outside_inside"
