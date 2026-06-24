import pytest

from raygeo.ops import Ops


def test_translate_layers_default_applied_to_all():
    ops = Ops()
    ops.move_to(10.0, 20.0, 0.0)
    ops.line_to(30.0, 40.0, 5.0)
    ops.translate_layers((1.0, 2.0, 3.0))
    assert ops.endpoint(0) == (9.0, 18.0, -3.0)
    assert ops.endpoint(1) == (29.0, 38.0, 2.0)


def test_translate_layers_layer_overrides_default():
    ops = Ops()
    ops.layer_start("lyr1")
    ops.move_to(100.0, 0.0)
    ops.layer_end("lyr1")
    ops.translate_layers((5.0, 5.0, 0.0), {"lyr1": (1.0, 1.0, 0.0)})
    assert ops.endpoint(1) == (99.0, -1.0, 0.0)


def test_translate_layers_multiple_layers():
    ops = Ops()
    ops.layer_start("lyrA")
    ops.move_to(0.0, 0.0)
    ops.layer_end("lyrA")
    ops.layer_start("lyrB")
    ops.move_to(10.0, 10.0)
    ops.layer_end("lyrB")
    ops.translate_layers(
        (2.0, 2.0, 0.0),
        {"lyrA": (10.0, 10.0, 0.0), "lyrB": (2.0, 2.0, 0.0)},
    )
    assert ops.endpoint(1) == (-10.0, -10.0, 0.0)
    assert ops.endpoint(4) == (8.0, 8.0, 0.0)


def test_translate_layers_between_layers_gets_default():
    ops = Ops()
    ops.layer_start("lyrA")
    ops.move_to(0.0, 0.0)
    ops.layer_end("lyrA")
    ops.move_to(50.0, 50.0)
    ops.layer_start("lyrB")
    ops.move_to(10.0, 10.0)
    ops.layer_end("lyrB")
    ops.translate_layers(
        (3.0, 3.0, 0.0),
        {"lyrA": (10.0, 10.0, 0.0), "lyrB": (1.0, 1.0, 0.0)},
    )
    assert ops.endpoint(1) == (-10.0, -10.0, 0.0)  # layer A
    assert ops.endpoint(3) == (47.0, 47.0, 0.0)  # between layers
    assert ops.endpoint(5) == (9.0, 9.0, 0.0)  # layer B


def test_translate_layers_after_last_layer_gets_default():
    ops = Ops()
    ops.layer_start("lyrA")
    ops.move_to(0.0, 0.0)
    ops.layer_end("lyrA")
    ops.move_to(60.0, 60.0)
    ops.translate_layers((5.0, 5.0, 0.0), {"lyrA": (2.0, 2.0, 0.0)})
    assert ops.endpoint(1) == (-2.0, -2.0, 0.0)
    assert ops.endpoint(3) == (55.0, 55.0, 0.0)


def test_translate_layers_empty_dict():
    ops = Ops()
    ops.layer_start("lyr1")
    ops.move_to(10.0, 10.0)
    ops.layer_end("lyr1")
    ops.translate_layers((4.0, 4.0, 0.0), {})
    assert ops.endpoint(1) == (6.0, 6.0, 0.0)


def test_translate_layers_layer_not_in_dict_gets_default():
    ops = Ops()
    ops.layer_start("unknown")
    ops.move_to(10.0, 10.0)
    ops.layer_end("unknown")
    ops.translate_layers((4.0, 4.0, 0.0), {"other": (1.0, 1.0, 0.0)})
    assert ops.endpoint(1) == (6.0, 6.0, 0.0)


def test_translate_layers_commands_before_first_layer():
    ops = Ops()
    ops.move_to(100.0, 100.0)
    ops.layer_start("lyr1")
    ops.move_to(0.0, 0.0)
    ops.layer_end("lyr1")
    ops.translate_layers((10.0, 10.0, 0.0), {"lyr1": (2.0, 2.0, 0.0)})
    assert ops.endpoint(0) == (90.0, 90.0, 0.0)
    assert ops.endpoint(2) == (-2.0, -2.0, 0.0)


def test_transform_layers_callback_receives_layer_uid():
    ops = Ops()
    ops.layer_start("lyr1")
    ops.move_to(0.0, 0.0)
    ops.layer_end("lyr1")
    ops.layer_start("lyr2")
    ops.move_to(10.0, 10.0)
    ops.layer_end("lyr2")

    seen = []
    ops.transform_layers(lambda uid, sub: seen.append(uid))
    assert seen == ["lyr1", "lyr2"]


def test_transform_layers_callback_receives_layer_commands():
    ops = Ops()
    ops.layer_start("lyr1")
    ops.move_to(10.0, 20.0)
    ops.line_to(30.0, 40.0)
    ops.layer_end("lyr1")

    def check(uid, sub):
        assert sub.len() == 4
        assert sub.endpoint(1)[:2] == (10.0, 20.0)
        assert sub.endpoint(2)[:2] == (30.0, 40.0)

    ops.transform_layers(check)


def test_transform_layers_callback_modifies_in_place():
    ops = Ops()
    ops.layer_start("lyr1")
    ops.move_to(100.0, 0.0)
    ops.layer_end("lyr1")

    def flip_y(uid, sub):
        def _flip(end, extra):
            end[1] = -end[1]

        sub.transform_moving(_flip)

    ops.transform_layers(flip_y)
    assert ops.endpoint(1) == (100.0, 0.0, 0.0)


def test_transform_layers_commands_before_first_layer_skipped():
    ops = Ops()
    ops.move_to(99.0, 99.0)
    ops.layer_start("lyr1")
    ops.move_to(10.0, 10.0)
    ops.layer_end("lyr1")
    ops.move_to(88.0, 88.0)  # between layers — collected with lyr1
    ops.layer_start("lyr2")
    ops.move_to(20.0, 20.0)
    ops.layer_end("lyr2")
    ops.move_to(77.0, 77.0)  # after last layer — collected with lyr2

    counts = []

    ops.transform_layers(lambda uid, sub: counts.append(sub.len()))
    assert counts == [3, 3]


def test_transform_layers_no_layers():
    ops = Ops()
    ops.move_to(0.0, 0.0)

    called = False

    def record(uid, sub):
        nonlocal called
        called = True

    ops.transform_layers(record)
    assert not called


def test_layer_uid():
    ops = Ops()
    ops.layer_start("layer_a")
    assert ops.layer_uid(0) == "layer_a"
    ops.layer_end("layer_a")
    assert ops.layer_uid(1) == "layer_a"


def test_layer_uid_wrong_type():
    ops = Ops()
    ops.move_to(0, 0)
    with pytest.raises(TypeError):
        ops.layer_uid(0)
