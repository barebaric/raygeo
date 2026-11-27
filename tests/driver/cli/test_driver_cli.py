"""Tests for the raydriver command line interface."""

import sys
from importlib.metadata import PackageNotFoundError

import pytest

from raygeo.driver.cli import main as cli_main
from raygeo.driver.cli.main import amain, build_parser


@pytest.fixture
def gcode_file(tmp_path):
    def write(text):
        path = tmp_path / "job.gcode"
        path.write_text(text)
        return str(path)

    return write


@pytest.fixture
def rpascript_file(tmp_path):
    def write(text):
        path = tmp_path / "job.rds"
        path.write_text(text)
        return str(path)

    return write


class TestLegacyPackage:
    def test_main_rejects_installed_raydriver(self, monkeypatch, capsys):
        class LegacyDistribution:
            version = "1.23.1"

        monkeypatch.setattr(
            cli_main,
            "distribution",
            lambda name: LegacyDistribution(),
        )
        monkeypatch.setattr(sys, "argv", ["raydriver", "--help"])
        with pytest.raises(SystemExit) as exc:
            cli_main.main()
        assert exc.value.code == 1
        err = capsys.readouterr().err
        assert "pip uninstall raydriver" in err

    def test_main_runs_without_raydriver(self, monkeypatch, capsys):
        def missing(name):
            raise PackageNotFoundError(name)

        monkeypatch.setattr(cli_main, "distribution", missing)
        monkeypatch.setattr(sys, "argv", ["raydriver", "--help"])
        with pytest.raises(SystemExit) as exc:
            cli_main.main()
        assert exc.value.code == 0
        capsys.readouterr()


class TestRunCommand:
    async def test_successful_job_shows_progress(self, gcode_file, capsys):
        path = gcode_file("G1 X10 F1000\nG1 Y5 F1000\nG1 X0 F1000\n")
        rc = await amain(
            ["run", path, "--emulator", "--speed-factor", "100000"]
        )
        assert rc == 0
        out = capsys.readouterr().out
        assert "3/3" in out
        assert "done" in out
        assert "Connecting to emulated device" in out
        assert "[VER:" in out

    async def test_progress_lines_terminated_when_redirected(
        self, gcode_file, capsys
    ):
        """Non-TTY progress updates must be newline-terminated so
        redirected logs never share a physical line with progress."""
        path = gcode_file("G1 X10 F1000\nG1 Y5 F1000\nG1 X0 F1000\n")
        rc = await amain(
            ["run", path, "--emulator", "--speed-factor", "100000"]
        )
        assert rc == 0
        out = capsys.readouterr().out
        assert "\r" not in out
        assert out.endswith("\n")

    async def test_log_file_receives_diagnostics(
        self, gcode_file, tmp_path, monkeypatch, capsys
    ):
        log_path = tmp_path / "session.log"
        monkeypatch.setenv("RUST_LOG", "info")
        path = gcode_file("G1 X10 F1000\nG1 Y5 F1000\nG1 X0 F1000\n")
        rc = await amain(
            [
                "run",
                path,
                "--emulator",
                "--speed-factor",
                "100000",
                "--log-file",
                str(log_path),
            ]
        )
        assert rc == 0
        content = log_path.read_text()
        assert "Connected to GRBL" in content
        # Progress goes to stdout, diagnostics to the file: the
        # captured stdout must not contain log lines.
        out = capsys.readouterr().out
        assert "Connected to GRBL" not in out

    async def test_progress_percentages_render(self, gcode_file, capsys):
        lines = "\n".join(f"G1 X{i % 30} F1000" for i in range(50))
        path = gcode_file(lines)
        rc = await amain(
            ["run", path, "--emulator", "--speed-factor", "20000"]
        )
        assert rc == 0
        out = capsys.readouterr().out
        assert "100.0%" in out
        assert "50/50" in out

    async def test_device_error_aborts_with_exit_code(
        self, gcode_file, capsys
    ):
        path = gcode_file("G1 X10 F1000\nG999\nG1 X0 F1000\n")
        rc = await amain(["run", path, "--emulator", "--speed-factor", "100"])
        assert rc == 1
        out = capsys.readouterr().out
        assert "ABORTED" in out
        # The abort line must carry the failure reason, not just
        # "job did not complete".
        assert "Unsupported Command" in out

    async def test_empty_file_is_rejected(self, gcode_file, capsys):
        path = gcode_file("; only comments\n\n")
        rc = await amain(["run", path, "--emulator"])
        assert rc == 2
        assert "no G-code lines" in capsys.readouterr().err

    async def test_missing_device_fails_cleanly(self, gcode_file):
        path = gcode_file("G1 X10 F1000\n")
        with pytest.raises(ConnectionError):
            # No emulator and a serial port that cannot answer.
            await amain(
                [
                    "run",
                    path,
                    "--port",
                    "/dev/null-does-not-exist",
                    "--timeout",
                    "0.3",
                ]
            )


class TestRunRuida:
    async def test_job_sent_without_wait(self, rpascript_file, capsys):
        path = rpascript_file("START_JOB\nEND_JOB\nEOF\n")
        rc = await amain(
            [
                "run",
                path,
                "--protocol",
                "ruida",
                "--emulator",
                "--no-wait",
            ]
        )
        assert rc == 0
        out = capsys.readouterr().out
        assert "emulated Ruida controller" in out
        assert "Device: RDC6442S" in out
        assert "job sent" in out

    async def test_job_waits_for_idle(
        self, rpascript_file, capsys, monkeypatch
    ):
        import raygeo.driver.cli.main as cli

        monkeypatch.setattr(cli, "_RUIDA_START_GRACE", 0.05)
        path = rpascript_file("START_JOB\nEND_JOB\nEOF\n")
        rc = await amain(["run", path, "--protocol", "ruida", "--emulator"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "done: controller idle" in out

    async def test_requires_a_source(self, rpascript_file, capsys):
        path = rpascript_file("END_JOB\n")
        rc = await amain(["run", path, "--protocol", "ruida"])
        assert rc == 2
        assert "Ruida needs" in capsys.readouterr().err

    async def test_rejects_gcode_input(self, gcode_file, capsys):
        path = gcode_file("G1 X10 F1000\n")
        rc = await amain(
            [
                "run",
                path,
                "--protocol",
                "ruida",
                "--emulator",
                "--no-wait",
            ]
        )
        assert rc == 1
        assert "unknown rpascript" in capsys.readouterr().err

    async def test_empty_file_is_rejected(self, rpascript_file, capsys):
        path = rpascript_file("\n   \n")
        rc = await amain(["run", path, "--protocol", "ruida", "--emulator"])
        assert rc == 2
        assert "no rpascript lines" in capsys.readouterr().err


def test_run_parser_accepts_ruida_options():
    args = build_parser().parse_args(
        [
            "run",
            "job.rds",
            "--protocol",
            "ruida",
            "--host",
            "192.168.1.50",
            "--net-port",
            "50201",
            "--transport",
            "tcp",
            "--magic",
            "0x99",
            "--no-wait",
            "--wait-timeout",
            "30",
        ]
    )
    assert args.protocol == "ruida"
    assert args.host == "192.168.1.50"
    assert args.net_port == 50201
    assert args.transport == "tcp"
    assert args.magic == 0x99
    assert args.no_wait is True
    assert args.wait_timeout == 30.0


class TestStatusCommand:
    async def test_status_reports_printed(self, capsys):
        rc = await amain(["status", "--emulator", "--seconds", "0.5"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "IDLE" in out
        assert "MPos:" in out
