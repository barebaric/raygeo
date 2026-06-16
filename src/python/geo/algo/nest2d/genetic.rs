use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::geo::algo::nest2d::genetic;

pyo3_stub_gen::module_doc!("raygeo.geo.algo.nest2d.genetic", "{}", MODULE_DOC);

pub(crate) const MODULE_DOC: &str = "\
Genetic algorithm for nesting optimization.

Provides a GeneticAlgorithm class that manages a population of
placement configurations (rotations, flips) and evolves them
via mutation, crossover, and selection.
";

// ---------------------------------------------------------------------------
// GeneticAlgorithm class
// ---------------------------------------------------------------------------

#[gen_stub_pyclass(module = "raygeo.geo.algo.nest2d.genetic")]
#[pyclass]
pub struct GeneticAlgorithm {
    inner: genetic::GeneticAlgorithm,
}

#[gen_stub_pymethods]
#[pymethods]
impl GeneticAlgorithm {
    #[new]
    pub fn new(num_parts: usize, config: &Bound<'_, PyDict>) -> PyResult<Self> {
        let rotation_count = config
            .get_item("rotation_count")?
            .map(|v| v.extract::<usize>())
            .transpose()?
            .unwrap_or(36);
        let flip_h = config
            .get_item("flip_h")?
            .map(|v| v.extract::<bool>())
            .transpose()?
            .unwrap_or(false);
        let flip_v = config
            .get_item("flip_v")?
            .map(|v| v.extract::<bool>())
            .transpose()?
            .unwrap_or(false);
        let population_size = config
            .get_item("population_size")?
            .map(|v| v.extract::<usize>())
            .transpose()?
            .unwrap_or(10);
        let mutation_rate = config
            .get_item("mutation_rate")?
            .map(|v| v.extract::<f64>())
            .transpose()?
            .unwrap_or(30.0);

        let config = genetic::GeneticConfig {
            rotation_count,
            flip_h,
            flip_v,
            population_size,
            mutation_rate,
        };

        Ok(GeneticAlgorithm {
            inner: genetic::GeneticAlgorithm::new(num_parts, config),
        })
    }

    fn __len__(&self) -> usize {
        self.inner.population.len()
    }

    /// Returns (rotations, flips_h, flips_v, fitness) for individual at idx.
    ///
    /// :complexity: O(1)
    pub fn get_individual(
        &self,
        idx: usize,
    ) -> (Vec<f64>, Vec<bool>, Vec<bool>, f64) {
        let ind = &self.inner.population[idx];
        (
            ind.rotation.clone(),
            ind.flip_h.clone(),
            ind.flip_v.clone(),
            ind.fitness,
        )
    }

    /// Set the fitness for individual at idx.
    ///
    /// :complexity: O(1)
    pub fn set_fitness(&mut self, idx: usize, fitness: f64) {
        if idx < self.inner.population.len() {
            self.inner.population[idx].fitness = fitness;
        }
    }

    /// Returns the fitness of individual at idx.
    ///
    /// :complexity: O(1)
    pub fn get_fitness(&self, idx: usize) -> f64 {
        self.inner.population[idx].fitness
    }

    /// Evolve one generation.
    ///
    /// :complexity: O(p * n) where p = population size, n = num_parts
    pub fn generation(&mut self) {
        self.inner.generation();
    }

    /// Mutate and return a copy of individual at idx.
    ///
    /// :complexity: O(n) where n = num_parts
    pub fn mutate(&self, idx: usize) -> (Vec<f64>, Vec<bool>, Vec<bool>) {
        let ind = self.inner.mutate(idx);
        (ind.rotation, ind.flip_h, ind.flip_v)
    }

    /// Mate two individuals and return the two children.
    ///
    /// :complexity: O(n) where n = num_parts
    pub fn mate(
        &self,
        male_idx: usize,
        female_idx: usize,
    ) -> Vec<(Vec<f64>, Vec<bool>, Vec<bool>)> {
        let (c1, c2) = self.inner.mate(male_idx, female_idx);
        vec![
            (c1.rotation, c1.flip_h, c1.flip_v),
            (c2.rotation, c2.flip_h, c2.flip_v),
        ]
    }
}

// ---------------------------------------------------------------------------
// Register
// ---------------------------------------------------------------------------

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<GeneticAlgorithm>()?;
    Ok(())
}
