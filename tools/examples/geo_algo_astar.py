"""Visualise A* pathfinding avoiding obstacles."""

import matplotlib.pyplot as plt
from matplotlib.patches import Polygon as MplPolygon

from raygeo.geo.algo.astar import find_path


def generate_simple_path():
    """A* from left to right in an open rectangle."""
    free = [[(0, 0), (100, 0), (100, 50), (0, 50)]]
    result = find_path(
        from_=(10, 25),
        to=(90, 25),
        free_space=free,
        obstacles=[],
        cell_size=1.0,
    )

    fig, ax = plt.subplots(figsize=(8, 5))
    ax.set_aspect("equal")

    free_patch = MplPolygon(
        free[0], facecolor="#e0f0e0", edgecolor="green", linewidth=2, alpha=0.5
    )
    ax.add_patch(free_patch)

    if result:
        xs = [p[0] for p in result.waypoints]
        ys = [p[1] for p in result.waypoints]
        ax.plot(xs, ys, "b-", linewidth=2, label="A* path")
        ax.plot(xs[0], ys[0], "go", markersize=10, label="Start")
        ax.plot(xs[-1], ys[-1], "ro", markersize=10, label="Goal")

    ax.set_xlim(-5, 105)
    ax.set_ylim(-5, 55)
    ax.set_title("A* pathfinding — Open rectangle")
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.legend(fontsize=10)
    ax.grid(True, alpha=0.3)
    fig.tight_layout()
    return fig


def generate_obstacle_detour():
    """A* route around a central obstacle."""
    free = [[(0, 0), (100, 0), (100, 50), (0, 50)]]
    obstacle = [[(35, 10), (65, 10), (65, 40), (35, 40)]]
    result = find_path(
        from_=(10, 25),
        to=(90, 25),
        free_space=free,
        obstacles=obstacle,
        obstacle_margin=2.0,
        cell_size=1.0,
    )

    fig, ax = plt.subplots(figsize=(8, 5))
    ax.set_aspect("equal")

    free_patch = MplPolygon(
        free[0], facecolor="#e0f0e0", edgecolor="green", linewidth=2, alpha=0.3
    )
    ax.add_patch(free_patch)
    obs_patch = MplPolygon(
        obstacle[0],
        facecolor="#f0c0c0",
        edgecolor="red",
        linewidth=2,
        alpha=0.7,
    )
    ax.add_patch(obs_patch)

    if result:
        xs = [p[0] for p in result.waypoints]
        ys = [p[1] for p in result.waypoints]
        ax.plot(xs, ys, "b-", linewidth=2, label="A* path")
        ax.plot(xs[0], ys[0], "go", markersize=10, label="Start")
        ax.plot(xs[-1], ys[-1], "ro", markersize=10, label="Goal")

    ax.set_xlim(-5, 105)
    ax.set_ylim(-5, 55)
    ax.set_title("A* pathfinding — Obstacle detour")
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.legend(fontsize=10)
    ax.grid(True, alpha=0.3)
    fig.tight_layout()
    return fig


def generate_multiple_obstacles():
    """A* path threading between several obstacles."""
    free = [[(0, 0), (120, 0), (120, 60), (0, 60)]]
    obstacles = [
        [(15, 15), (35, 15), (35, 35), (15, 35)],
        [(50, 5), (70, 5), (70, 25), (50, 25)],
        [(85, 30), (105, 30), (105, 50), (85, 50)],
    ]
    result = find_path(
        from_=(5, 50),
        to=(115, 5),
        free_space=free,
        obstacles=obstacles,
        obstacle_margin=2.0,
        cell_size=1.0,
    )

    fig, ax = plt.subplots(figsize=(9, 5))
    ax.set_aspect("equal")

    free_patch = MplPolygon(
        free[0], facecolor="#e0f0e0", edgecolor="green", linewidth=2, alpha=0.2
    )
    ax.add_patch(free_patch)
    for obs in obstacles:
        obs_patch = MplPolygon(
            obs, facecolor="#f0c0c0", edgecolor="red", linewidth=1.5, alpha=0.7
        )
        ax.add_patch(obs_patch)

    if result:
        xs = [p[0] for p in result.waypoints]
        ys = [p[1] for p in result.waypoints]
        ax.plot(xs, ys, "b-", linewidth=2, label="A* path")
        ax.plot(xs[0], ys[0], "go", markersize=10, label="Start")
        ax.plot(xs[-1], ys[-1], "ro", markersize=10, label="Goal")

    ax.set_xlim(-5, 125)
    ax.set_ylim(-5, 65)
    ax.set_title("A* pathfinding — Multiple obstacles")
    ax.set_xlabel("X")
    ax.set_ylabel("Y")
    ax.legend(fontsize=10)
    ax.grid(True, alpha=0.3)
    fig.tight_layout()
    return fig


def generate_obstacle_margin():
    """Effect of obstacle margin on path clearance."""
    free = [[(0, 0), (100, 0), (100, 50), (0, 50)]]
    obstacle = [[(40, 15), (60, 15), (60, 35), (40, 35)]]

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 5))

    for ax, margin, title in [
        (ax1, 0.0, "Margin = 0"),
        (ax2, 8.0, "Margin = 8"),
    ]:
        ax.set_aspect("equal")
        free_patch = MplPolygon(
            free[0],
            facecolor="#e0f0e0",
            edgecolor="green",
            linewidth=2,
            alpha=0.2,
        )
        ax.add_patch(free_patch)
        obs_patch = MplPolygon(
            obstacle[0],
            facecolor="#f0c0c0",
            edgecolor="red",
            linewidth=1.5,
            alpha=0.7,
        )
        ax.add_patch(obs_patch)

        result = find_path(
            from_=(5, 25),
            to=(95, 25),
            free_space=free,
            obstacles=obstacle,
            obstacle_margin=margin,
            cell_size=1.0,
        )
        if result:
            xs = [p[0] for p in result.waypoints]
            ys = [p[1] for p in result.waypoints]
            ax.plot(
                xs,
                ys,
                "b-",
                linewidth=2,
                label=f"Path (visited {result.visited})",
            )
            ax.plot(xs[0], ys[0], "go", markersize=8)
            ax.plot(xs[-1], ys[-1], "ro", markersize=8)

        ax.set_xlim(-5, 105)
        ax.set_ylim(-5, 55)
        ax.set_title(title)
        ax.set_xlabel("X")
        ax.set_ylabel("Y")
        ax.legend(fontsize=9)
        ax.grid(True, alpha=0.3)

    fig.suptitle(
        "Effect of obstacle margin on path clearance", fontsize=14, y=1.02
    )
    fig.tight_layout()
    return fig


__docs_target__ = ["raygeo.geo.algo.astar.md"]
__images__ = [
    {
        "heading": None,
        "caption": (
            "A* pathfinding in an open rectangle — the shortest path"
            " is a straight line from start to goal"
        ),
        "function": generate_simple_path,
    },
    {
        "heading": "find_path",
        "caption": (
            "A* finds a path around a central obstacle when the direct"
            " route is blocked"
        ),
        "function": generate_obstacle_detour,
    },
    {
        "heading": "find_path",
        "caption": (
            "A* threading a path between multiple disconnected obstacles"
            " — the algorithm explores the free cells and finds an"
            " optimal route"
        ),
        "function": generate_multiple_obstacles,
    },
    {
        "heading": "find_path",
        "caption": (
            "Increasing the obstacle margin (right) pushes the path"
            " further from obstacles compared to no margin (left)"
        ),
        "function": generate_obstacle_margin,
    },
]
