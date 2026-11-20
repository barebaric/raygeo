"""Public type vocabulary of the sketch document model.

The registry assigns entity and point ids; `EntityID` names that id
space for callers, so annotations can say what a value means instead
of a bare `int`.
"""

from typing import TypeAlias

EntityID: TypeAlias = int
