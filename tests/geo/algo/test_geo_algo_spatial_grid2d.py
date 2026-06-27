from raygeo.geo.algo.spatial_grid2d import SpatialGrid


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

    # ------------------------------------------------------------------
    # remove tests
    # ------------------------------------------------------------------

    def test_remove_single_item(self):
        sg = SpatialGrid(50.0)
        sg.insert(0, [0.0, 0.0, 10.0, 10.0])
        sg.remove(0, [0.0, 0.0, 10.0, 10.0])
        assert sg.query([0.0, 0.0, 20.0, 20.0]) == []

    def test_remove_one_of_many(self):
        sg = SpatialGrid(50.0)
        sg.insert(0, [0.0, 0.0, 10.0, 10.0])
        sg.insert(1, [5.0, 5.0, 15.0, 15.0])
        sg.insert(2, [100.0, 100.0, 110.0, 110.0])
        sg.remove(0, [0.0, 0.0, 10.0, 10.0])
        assert sorted(sg.query([0.0, 0.0, 20.0, 20.0])) == [1]
        assert sg.query([100.0, 100.0, 110.0, 110.0]) == [2]

    def test_remove_middle_item(self):
        sg = SpatialGrid(50.0)
        sg.insert(0, [0.0, 0.0, 10.0, 10.0])
        sg.insert(1, [5.0, 5.0, 15.0, 15.0])
        sg.insert(2, [100.0, 100.0, 110.0, 110.0])
        sg.remove(1, [5.0, 5.0, 15.0, 15.0])
        assert sorted(sg.query([0.0, 0.0, 20.0, 20.0])) == [0]
        assert sg.query([100.0, 100.0, 110.0, 110.0]) == [2]

    def test_remove_item_spanning_multiple_cells(self):
        sg = SpatialGrid(10.0)
        sg.insert(0, [0.0, 0.0, 25.0, 25.0])
        sg.insert(1, [50.0, 50.0, 55.0, 55.0])
        sg.remove(0, [0.0, 0.0, 25.0, 25.0])
        assert sg.query([0.0, 0.0, 30.0, 30.0]) == []
        assert sg.query([50.0, 50.0, 60.0, 60.0]) == [1]

    def test_remove_then_insert_same_index(self):
        sg = SpatialGrid(10.0)
        sg.insert(0, [0.0, 0.0, 5.0, 5.0])
        sg.remove(0, [0.0, 0.0, 5.0, 5.0])
        sg.insert(0, [100.0, 100.0, 105.0, 105.0])
        assert sg.query([0.0, 0.0, 10.0, 10.0]) == []
        assert sg.query([100.0, 100.0, 110.0, 110.0]) == [0]

    def test_remove_nonexistent_index(self):
        sg = SpatialGrid(50.0)
        sg.insert(0, [0.0, 0.0, 10.0, 10.0])
        sg.remove(99, [0.0, 0.0, 10.0, 10.0])
        assert sg.query([0.0, 0.0, 20.0, 20.0]) == [0]

    def test_remove_all_items(self):
        sg = SpatialGrid(50.0)
        sg.insert(0, [0.0, 0.0, 10.0, 10.0])
        sg.insert(1, [5.0, 5.0, 15.0, 15.0])
        sg.remove(0, [0.0, 0.0, 10.0, 10.0])
        sg.remove(1, [5.0, 5.0, 15.0, 15.0])
        assert sg.query([0.0, 0.0, 20.0, 20.0]) == []

    def test_remove_from_empty_grid(self):
        sg = SpatialGrid(50.0)
        sg.remove(0, [0.0, 0.0, 10.0, 10.0])
        assert sg.query([0.0, 0.0, 20.0, 20.0]) == []
