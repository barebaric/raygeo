"""The standalone machine-type probe: ``query_machine_type()``
against the xTool WiFi firmware emulator."""

import pytest

from raygeo.driver.xtool import query_machine_type
from raygeo.driver.xtool.emulator import MACHINE_TYPE, XtoolEmulator


@pytest.fixture
async def emulator():
    emu = XtoolEmulator()
    http_port, ws_port = await emu.start()
    try:
        yield emu
    finally:
        await emu.stop()


@pytest.fixture
async def emulator_of_type(request):
    emu = XtoolEmulator(machine_type=request.param)
    http_port, ws_port = await emu.start()
    try:
        yield emu
    finally:
        await emu.stop()


class TestQueryMachineType:
    async def test_returns_reported_type(self, emulator):
        machine_type = await query_machine_type(
            "127.0.0.1", emulator.http_port, timeout=2.0
        )
        assert machine_type == MACHINE_TYPE

    @pytest.mark.parametrize(
        "emulator_of_type, expected",
        [
            ("FakePrinter", None),
            ("FakeXTool", "FakeXTool"),
        ],
        indirect=["emulator_of_type"],
    )
    async def test_identification_is_case_insensitive(
        self, emulator_of_type, expected
    ):
        machine_type = await query_machine_type(
            "127.0.0.1", emulator_of_type.http_port, timeout=2.0
        )
        assert machine_type == expected

    async def test_none_for_dead_endpoint(self):
        assert await query_machine_type("127.0.0.1", 1) is None
