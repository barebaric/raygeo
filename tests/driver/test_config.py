"""Typed session config classes and the factory boundary."""

import pytest

from raygeo.driver.config import (
    Esp3dConfig,
    GrblSerialConfig,
    GrblTelnetConfig,
    M2Config,
    MarlinConfig,
    RemoteEndpoint,
    SmoothieConfig,
    XtoolConfig,
)
from raygeo.driver.grbl import GrblSerialDriver
from raygeo.driver.remote.client import create_session


def test_grbl_serial_config_to_dict():
    config = GrblSerialConfig(port="/dev/ttyUSB0", baudrate=250000)
    data = config.to_dict()
    assert data["port"] == "/dev/ttyUSB0"
    assert data["baudrate"] == 250000
    assert "cached_rx_buffer_size" not in data


def test_grbl_telnet_config_to_dict():
    data = GrblTelnetConfig(host="grbl.local", tcp_port=23).to_dict()
    assert data["host"] == "grbl.local"
    assert data["tcp_port"] == 23


def test_esp3d_config_to_dict():
    data = Esp3dConfig(
        host="fluidnc.local",
        http_port=80,
        ws_port=80,
        protocol="FluidNC",
    ).to_dict()
    assert data["host"] == "fluidnc.local"
    assert data["http_port"] == 80
    assert data["protocol"] == "FluidNC"


def test_other_protocol_configs():
    assert SmoothieConfig(host="smoothie").to_dict()["tcp_port"] == 23
    assert MarlinConfig(port="/dev/ttyACM0").to_dict()["baudrate"] == 115200
    assert XtoolConfig(host="xtool").to_dict()["http_port"] == 8080
    assert M2Config(host="m2").to_dict()["port"] == 28900


def test_remote_endpoint_to_dict():
    data = RemoteEndpoint(remote_host="pi.local", token="secret").to_dict()
    assert data == {
        "remote_host": "pi.local",
        "remote_port": 8765,
        "token": "secret",
    }


def test_grbl_options_defaults_are_flattened():
    data = GrblTelnetConfig(host="grbl.local").to_dict()
    assert data["stall_timeout_default"] == 30.0
    assert data["poll_response_attempts"] == 10
    assert "cached_rx_buffer_size" not in data


def test_configs_are_keyword_only():
    with pytest.raises(TypeError):
        GrblSerialConfig("/dev/null")  # pyright: ignore[reportCallIssue]


def test_session_accepts_typed_config():
    session = GrblSerialDriver(
        config=GrblSerialConfig(port="/dev/null"),
    )
    assert session.resource_uri == "serial:///dev/null"


def test_factory_accepts_typed_config():
    session = create_session(
        "GrblSerialDriver",
        GrblSerialConfig(port="/dev/null"),
        {},
        None,
    )
    assert session.resource_uri == "serial:///dev/null"
