import argparse

from raygeo.cli.commands.inspect_cmd import register as register_inspect
from raygeo.cli.commands.print_cmd import register as register_print
from raygeo.cli.commands.profile_cleared_area_cmd import (
    register as register_profile_cleared_area,
)
from raygeo.cli.commands.profile_cmd import register as register_profile
from raygeo.cli.commands.profile_wavefront_cmd import (
    register as register_profile_wavefront,
)
from raygeo.cli.commands.trace_cmd import register as register_trace


def main() -> None:
    parser = argparse.ArgumentParser(
        prog="raygeo",
        description="RayGeo — 2D/3D geometry engine for CAD/CAM applications.",
    )
    sub = parser.add_subparsers(dest="command", required=True)

    register_trace(sub)
    register_inspect(sub)
    register_print(sub)
    register_profile(sub)
    register_profile_cleared_area(sub)
    register_profile_wavefront(sub)

    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
