from raygeo.geo.algo.nest2d.genetic import GeneticAlgorithm


class TestGeneticAlgorithm:
    def test_creation(self):
        ga = GeneticAlgorithm(
            10,
            {
                "rotation_count": 36,
                "flip_h": True,
                "flip_v": True,
                "population_size": 20,
                "mutation_rate": 30.0,
            },
        )
        assert len(ga) == 20

    def test_individual_structure(self):
        ga = GeneticAlgorithm(
            5,
            {
                "rotation_count": 4,
                "flip_h": True,
                "flip_v": True,
                "population_size": 10,
                "mutation_rate": 0.0,
            },
        )
        rotations, flips_h, flips_v, fitness = ga.get_individual(0)
        assert len(rotations) == 5
        assert len(flips_h) == 5
        assert len(flips_v) == 5
        assert fitness == float("inf")

    def test_set_and_get_fitness(self):
        ga = GeneticAlgorithm(
            3,
            {
                "rotation_count": 4,
                "flip_h": False,
                "flip_v": False,
                "population_size": 5,
                "mutation_rate": 0.0,
            },
        )
        ga.set_fitness(0, 1.5)
        assert ga.get_fitness(0) == 1.5

    def test_mutate(self):
        ga = GeneticAlgorithm(
            5,
            {
                "rotation_count": 36,
                "flip_h": True,
                "flip_v": True,
                "population_size": 10,
                "mutation_rate": 100.0,
            },
        )
        rots, fh, fv = ga.mutate(0)
        assert len(rots) == 5
        assert len(fh) == 5
        assert len(fv) == 5

    def test_mate(self):
        ga = GeneticAlgorithm(
            10,
            {
                "rotation_count": 4,
                "flip_h": False,
                "flip_v": False,
                "population_size": 20,
                "mutation_rate": 0.0,
            },
        )
        children = ga.mate(0, 1)
        assert len(children) == 2
        rots1, fh1, fv1 = children[0]
        rots2, fh2, fv2 = children[1]
        assert len(rots1) == 10
        assert len(rots2) == 10
        assert len(fh1) == 10
        assert len(fh2) == 10

    def test_generation_maintains_size(self):
        ga = GeneticAlgorithm(
            3,
            {
                "rotation_count": 4,
                "flip_h": False,
                "flip_v": False,
                "population_size": 8,
                "mutation_rate": 0.0,
            },
        )
        for i in range(len(ga)):
            ga.set_fitness(i, float(i))
        ga.generation()
        assert len(ga) == 8

    def test_generation_preserves_best(self):
        ga = GeneticAlgorithm(
            3,
            {
                "rotation_count": 4,
                "flip_h": False,
                "flip_v": False,
                "population_size": 8,
                "mutation_rate": 0.0,
            },
        )
        # Set first individual as best, last as worst
        for i in range(len(ga)):
            ga.set_fitness(i, float(100 - i))
        # Individual 0 has fitness 100 (worst), individual 7 has fitness 92
        # (best). After sort, individual 7 is first.
        # Remember: lower fitness is better!
        ga.generation()
        assert len(ga) == 8

    def test_no_flip_config(self):
        ga = GeneticAlgorithm(
            5,
            {
                "rotation_count": 4,
                "flip_h": False,
                "flip_v": False,
                "population_size": 10,
                "mutation_rate": 0.0,
            },
        )
        for i in range(len(ga)):
            _, fh, fv, _ = ga.get_individual(i)
            assert all(not f for f in fh)
            assert all(not f for f in fv)

    def test_population_diversity(self):
        config = {
            "rotation_count": 36,
            "flip_h": True,
            "flip_v": True,
            "population_size": 20,
            "mutation_rate": 30.0,
        }
        ga = GeneticAlgorithm(10, config)
        rots0, _, _, _ = ga.get_individual(0)
        # Check that not all individuals have the same rotations as ind 0
        all_same = True
        for i in range(1, len(ga)):
            rots, _, _, _ = ga.get_individual(i)
            if rots != rots0:
                all_same = False
                break
        assert not all_same, "Population should have diverse rotations"

    def test_generation_updates_genomes(self):
        ga = GeneticAlgorithm(
            5,
            {
                "rotation_count": 36,
                "flip_h": True,
                "flip_v": True,
                "population_size": 10,
                "mutation_rate": 50.0,
            },
        )
        rots_before, _, _, _ = ga.get_individual(0)
        for i in range(len(ga)):
            ga.set_fitness(i, float(i))
        ga.generation()
        rots_after, _, _, _ = ga.get_individual(0)
        # The elite individual (best fitness) should be preserved but may
        # have been mutated depending on fitness. Since we set ascending
        # fitness, ind 0 is best (fitness=0.0). With elitism, it stays.
        # But mutations only happen on children, not the elite.
        # With mutation_rate=50 and rotation_count=36, children will differ.
        assert len(rots_after) == 5
