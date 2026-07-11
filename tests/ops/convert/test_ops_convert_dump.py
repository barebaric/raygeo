import io
from contextlib import redirect_stdout

import pytest

from raygeo.ops import Ops


@pytest.fixture
def sample_ops():
    ops = Ops()
    ops.set_power(1.0)
    ops.move_to(0.0, 0.0, 0.0)
    ops.line_to(10.0, 10.0, 0.0)
    ops.line_to(20.0, 0.0, 0.0)
    return ops


def test_dump(sample_ops):
    f = io.StringIO()
    with redirect_stdout(f):
        sample_ops.dump()
    output = f.getvalue()
    assert "MOVE_TO" in output
    assert "LINE_TO" in output
