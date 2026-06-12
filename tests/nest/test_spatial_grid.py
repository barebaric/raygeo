from raygeo.nest.spatial_grid import SpatialGrid


class TestSpatialGrid:
    def test_insert_and_query(self):
        sg = SpatialGrid(50.0)
        sg.insert(0, [0.0, 0.0, 10.0, 10.0])
        sg.insert(1, [5.0, 5.0, 15.0, 15.0])
        sg.insert(2, [100.0, 100.0, 110.0, 110.0])
        result = sg.query([0.0, 0.0, 20.0, 20.0])
        assert sorted(result) == [0, 1]

    def test_no_match(self):
        sg = SpatialGrid(50.0)
        sg.insert(0, [0.0, 0.0, 10.0, 10.0])
        result = sg.query([100.0, 100.0, 110.0, 110.0])
        assert result == []

    def test_clear(self):
        sg = SpatialGrid(50.0)
        sg.insert(0, [0.0, 0.0, 10.0, 10.0])
        sg.clear()
        result = sg.query([0.0, 0.0, 20.0, 20.0])
        assert result == []

    def test_custom_cell_size(self):
        sg = SpatialGrid(10.0)
        sg.insert(0, [0.0, 0.0, 5.0, 5.0])
        sg.insert(1, [100.0, 100.0, 105.0, 105.0])
        result = sg.query([0.0, 0.0, 5.0, 5.0])
        assert result == [0]

    def test_repr(self):
        sg = SpatialGrid(25.0)
        assert "25" in repr(sg)
