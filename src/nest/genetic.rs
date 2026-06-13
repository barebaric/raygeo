use rand::rngs::ThreadRng;
use rand::Rng;

/// Probability of flipping a part horizontally or vertically when generating
/// a random individual.
const RANDOM_FLIP_PROBABILITY: f64 = 0.5;

/// Base probability for mutating a part's rotation angle.
const ROTATION_MUTATION_BASE: f64 = 0.01;

/// Probability of toggling a flip during mutation.
const FLIP_MUTATION_PROBABILITY: f64 = 0.05;

/// A single individual in the genetic algorithm population.
#[derive(Clone, Debug)]
pub struct Individual {
    pub rotation: Vec<f64>,
    pub flip_h: Vec<bool>,
    pub flip_v: Vec<bool>,
    pub fitness: f64,
}

/// Configuration for the genetic algorithm.
#[derive(Clone, Debug)]
pub struct GeneticConfig {
    pub rotation_count: usize,
    pub flip_h: bool,
    pub flip_v: bool,
    pub population_size: usize,
    pub mutation_rate: f64,
}

/// Genetic algorithm for nesting optimization.
#[derive(Clone, Debug)]
pub struct GeneticAlgorithm {
    pub population: Vec<Individual>,
    pub config: GeneticConfig,
    num_parts: usize,
    angle_step: f64,
}

/// Build the initial population with diverse individuals.
fn random_individual(
    num_parts: usize,
    config: &GeneticConfig,
    angle_step: f64,
    rng: &mut ThreadRng,
) -> Individual {
    let mut rot = Vec::with_capacity(num_parts);
    let mut fh = Vec::with_capacity(num_parts);
    let mut fv = Vec::with_capacity(num_parts);
    for _ in 0..num_parts {
        let angle_idx = if config.rotation_count > 0 {
            rng.random_range(0..config.rotation_count)
        } else {
            0
        };
        rot.push(angle_idx as f64 * angle_step);
        fh.push(config.flip_h && rng.random_bool(RANDOM_FLIP_PROBABILITY));
        fv.push(config.flip_v && rng.random_bool(RANDOM_FLIP_PROBABILITY));
    }
    Individual {
        rotation: rot,
        flip_h: fh,
        flip_v: fv,
        fitness: f64::INFINITY,
    }
}

fn create_initial_population(
    num_parts: usize,
    config: &GeneticConfig,
    rng: &mut ThreadRng,
) -> Vec<Individual> {
    let angle_step = if config.rotation_count > 0 {
        360.0 / config.rotation_count as f64
    } else {
        0.0
    };

    let mut population: Vec<Individual> = Vec::new();

    // Individual 0: identity (all zero)
    population.push(Individual {
        rotation: vec![0.0; num_parts],
        flip_h: vec![false; num_parts],
        flip_v: vec![false; num_parts],
        fitness: f64::INFINITY,
    });

    // Compute a "large" rotation angle for diversity (guaranteed > 90 deg)
    let large_angle = if config.rotation_count >= 2 {
        let half_count = config.rotation_count / 2;
        angle_step * half_count as f64
    } else {
        angle_step
    };

    // Individual 1: all parts at a large angle (180 or nearest)
    if config.rotation_count > 0 {
        let mut rot = vec![large_angle; num_parts];
        // mix in some zeros for variety
        for i in (0..num_parts).step_by(3) {
            rot[i] = 0.0;
        }
        population.push(Individual {
            rotation: rot,
            flip_h: vec![false; num_parts],
            flip_v: vec![false; num_parts],
            fitness: f64::INFINITY,
        });
    } else {
        population.push(Individual {
            rotation: vec![0.0; num_parts],
            flip_h: vec![false; num_parts],
            flip_v: vec![false; num_parts],
            fitness: f64::INFINITY,
        });
    }

    // Individual 2: small angle (one step) for half the parts
    if config.rotation_count > 0 {
        let mut rot = vec![0.0; num_parts];
        for i in (0..num_parts).step_by(2) {
            rot[i] = angle_step;
        }
        population.push(Individual {
            rotation: rot,
            flip_h: vec![false; num_parts],
            flip_v: vec![false; num_parts],
            fitness: f64::INFINITY,
        });
    } else {
        population.push(Individual {
            rotation: vec![0.0; num_parts],
            flip_h: vec![false; num_parts],
            flip_v: vec![false; num_parts],
            fitness: f64::INFINITY,
        });
    }

    // Individual 3: random rotations and flips
    population.push(random_individual(num_parts, config, angle_step, rng));

    // Individual 4: another random individual (different seed -> different
    // rolls)
    if config.population_size > 4 {
        population.push(random_individual(num_parts, config, angle_step, rng));
    }

    // Individual 5: all horizontal flips
    if config.flip_h {
        population.push(Individual {
            rotation: vec![0.0; num_parts],
            flip_h: vec![true; num_parts],
            flip_v: vec![false; num_parts],
            fitness: f64::INFINITY,
        });
    }

    // Individual 6: all vertical flips
    if config.flip_v {
        population.push(Individual {
            rotation: vec![0.0; num_parts],
            flip_h: vec![false; num_parts],
            flip_v: vec![true; num_parts],
            fitness: f64::INFINITY,
        });
    }

    // Individual 7: both flips
    if config.flip_h && config.flip_v {
        population.push(Individual {
            rotation: vec![0.0; num_parts],
            flip_h: vec![true; num_parts],
            flip_v: vec![true; num_parts],
            fitness: f64::INFINITY,
        });
    }

    population
}

impl GeneticAlgorithm {
    pub fn new(num_parts: usize, config: GeneticConfig) -> Self {
        let mut rng = rand::rng();
        let angle_step = if config.rotation_count > 0 {
            360.0 / config.rotation_count as f64
        } else {
            0.0
        };

        let mut population =
            create_initial_population(num_parts, &config, &mut rng);

        // Fill remaining population with mutated copies
        while population.len() < config.population_size {
            let donor_idx = rng.random_range(0..population.len());
            let mutant = mutate_internal(
                &population[donor_idx],
                &config,
                angle_step,
                num_parts,
                &mut rng,
            );
            population.push(mutant);
        }

        GeneticAlgorithm {
            population,
            config,
            num_parts,
            angle_step,
        }
    }

    pub fn mutate(&self, idx: usize) -> Individual {
        let mut rng = rand::rng();
        mutate_internal(
            &self.population[idx],
            &self.config,
            self.angle_step,
            self.num_parts,
            &mut rng,
        )
    }

    pub fn mate(
        &self,
        male_idx: usize,
        female_idx: usize,
    ) -> (Individual, Individual) {
        let male = &self.population[male_idx];
        let female = &self.population[female_idx];

        if self.num_parts <= 1 {
            return (male.clone(), female.clone());
        }

        let mut rng = rand::rng();
        let cutpoint = rng.random_range(1..self.num_parts);

        let child1_rot: Vec<f64> = male.rotation[..cutpoint]
            .iter()
            .chain(&female.rotation[cutpoint..])
            .copied()
            .collect();
        let child1_h: Vec<bool> = male.flip_h[..cutpoint]
            .iter()
            .chain(&female.flip_h[cutpoint..])
            .copied()
            .collect();
        let child1_v: Vec<bool> = male.flip_v[..cutpoint]
            .iter()
            .chain(&female.flip_v[cutpoint..])
            .copied()
            .collect();

        let child2_rot: Vec<f64> = female.rotation[..cutpoint]
            .iter()
            .chain(&male.rotation[cutpoint..])
            .copied()
            .collect();
        let child2_h: Vec<bool> = female.flip_h[..cutpoint]
            .iter()
            .chain(&male.flip_h[cutpoint..])
            .copied()
            .collect();
        let child2_v: Vec<bool> = female.flip_v[..cutpoint]
            .iter()
            .chain(&male.flip_v[cutpoint..])
            .copied()
            .collect();

        (
            Individual {
                rotation: child1_rot,
                flip_h: child1_h,
                flip_v: child1_v,
                fitness: f64::INFINITY,
            },
            Individual {
                rotation: child2_rot,
                flip_h: child2_h,
                flip_v: child2_v,
                fitness: f64::INFINITY,
            },
        )
    }

    pub fn generation(&mut self) {
        self.population.sort_by(|a, b| {
            a.fitness
                .partial_cmp(&b.fitness)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut rng = rand::rng();
        let mut new_population: Vec<Individual> =
            Vec::with_capacity(self.population.len());

        // Elitism: keep the best individual
        new_population.push(self.population[0].clone());

        while new_population.len() < self.population.len() {
            let male_idx =
                random_weighted_individual(&self.population, &mut rng, None);
            let female_idx = random_weighted_individual(
                &self.population,
                &mut rng,
                Some(male_idx),
            );

            let (child1, child2) = self.mate(male_idx, female_idx);

            let child1 = mutate_internal(
                &child1,
                &self.config,
                self.angle_step,
                self.num_parts,
                &mut rng,
            );
            new_population.push(child1);

            if new_population.len() < self.population.len() {
                let child2 = mutate_internal(
                    &child2,
                    &self.config,
                    self.angle_step,
                    self.num_parts,
                    &mut rng,
                );
                new_population.push(child2);
            }
        }

        self.population = new_population;
    }
}

fn random_weighted_individual(
    population: &[Individual],
    rng: &mut ThreadRng,
    exclude_idx: Option<usize>,
) -> usize {
    let indices: Vec<usize> = (0..population.len())
        .filter(|&i| Some(i) != exclude_idx)
        .collect();

    if indices.is_empty() {
        return 0;
    }

    let len = indices.len();
    // Linear ranking: rank 0 (best) gets weight len, rank len-1 (worst)
    // gets weight 1. Sum = len*(len+1)/2.
    let denom = (len * (len + 1) / 2) as f64;
    let rand_float: f64 = rng.random();
    let mut cumulative = 0.0;

    for (pos, &idx) in indices.iter().enumerate() {
        let weight = (len - pos) as f64 / denom;
        cumulative += weight;
        if rand_float < cumulative {
            return idx;
        }
    }

    indices[len - 1]
}

fn mutate_internal(
    individual: &Individual,
    config: &GeneticConfig,
    angle_step: f64,
    num_parts: usize,
    rng: &mut ThreadRng,
) -> Individual {
    let mut clone = individual.clone();

    for i in 0..num_parts {
        // Random rotation change
        if config.rotation_count > 0
            && rng.random_bool(ROTATION_MUTATION_BASE * config.mutation_rate)
        {
            let angle_idx = rng.random_range(0..config.rotation_count);
            clone.rotation[i] = angle_idx as f64 * angle_step;
        }

        // Horizontal flip toggle
        if config.flip_h && rng.random_bool(FLIP_MUTATION_PROBABILITY) {
            clone.flip_h[i] = !clone.flip_h[i];
        }

        // Vertical flip toggle
        if config.flip_v && rng.random_bool(FLIP_MUTATION_PROBABILITY) {
            clone.flip_v[i] = !clone.flip_v[i];
        }
    }

    clone.fitness = f64::INFINITY;
    clone
}
