import pytest

from raygeo.pipeline.execute import clear_cache


@pytest.fixture(autouse=True)
def _clear_pipeline_cache():
    clear_cache()
    yield
    clear_cache()
