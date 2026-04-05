//! Population, selection, crossover, mutation, save/load.

use crate::physics::{evaluate, Genome, GENOME_LEN};
use rand::rngs::ThreadRng;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

pub const POP_SIZE: usize = 100;
pub const ELITE_COUNT: usize = 2;
pub const MUTATE_PROB: f64 = 0.15;
pub const MUTATE_STD: f32 = 0.4;
pub const INIT_WEIGHT_RANGE: f32 = 2.0;
/// Per-rank success probability scanning sorted population best→worst; first success is the parent.
pub const PARENT_SCAN_P: f64 = 0.3;

pub const SAVE_FILENAME: &str = "best_population.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SavedPopulation {
    pub generation: u64,
    pub individuals: Vec<Genome>,
    pub best: Genome,
    pub best_fitness: f32,
}

pub struct Population {
    pub generation: u64,
    /// Parallel to `fitness` after evaluation.
    pub individuals: Vec<Genome>,
    pub fitness: Vec<f32>,
    pub best: Genome,
    pub best_fitness: f32,
}

impl Population {
    pub fn new_random(rng: &mut ThreadRng) -> Self {
        let mut individuals = Vec::with_capacity(POP_SIZE);
        for _ in 0..POP_SIZE {
            individuals.push(random_genome(rng));
        }
        let best = individuals[0];
        Self {
            generation: 0,
            individuals,
            fitness: vec![0.0; POP_SIZE],
            best,
            best_fitness: 0.0,
        }
    }

    pub fn load_or_new(path: &Path, rng: &mut ThreadRng) -> Self {
        match load(path) {
            Ok(saved) => Self::from_saved(saved),
            Err(e) => {
                eprintln!("starting new population ({e})");
                Self::new_random(rng)
            }
        }
    }

    fn from_saved(saved: SavedPopulation) -> Self {
        let mut individuals = saved.individuals;
        if individuals.len() != POP_SIZE {
            eprintln!(
                "saved population size {} != {POP_SIZE}, truncating or padding",
                individuals.len()
            );
            individuals.truncate(POP_SIZE);
            while individuals.len() < POP_SIZE {
                individuals.push(saved.best);
            }
        }
        Self {
            generation: saved.generation,
            individuals,
            fitness: vec![0.0; POP_SIZE],
            best: saved.best,
            best_fitness: saved.best_fitness,
        }
    }

    pub fn run_generation(&mut self, rng: &mut ThreadRng) {
        for i in 0..POP_SIZE {
            self.fitness[i] = evaluate(&self.individuals[i], rng);
        }
        self.breed_next_generation(rng);
    }

    /// Use after `self.fitness` holds a full evaluation of `self.individuals` (same as headless `run_generation`).
    pub fn breed_next_generation(&mut self, rng: &mut ThreadRng) {
        let mut order: Vec<usize> = (0..POP_SIZE).collect();
        order.sort_by(|&a, &b| {
            self.fitness[b]
                .partial_cmp(&self.fitness[a])
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let best_i = order[0];
        self.best = self.individuals[best_i];
        self.best_fitness = self.fitness[best_i];

        let mut next = Vec::with_capacity(POP_SIZE);
        for &i in order.iter().take(ELITE_COUNT) {
            next.push(self.individuals[i]);
        }

        while next.len() < POP_SIZE {
            let ia = pick_parent_rank(rng);
            let ib = pick_parent_rank(rng);
            let mut child = crossover(
                &self.individuals[order[ia]],
                &self.individuals[order[ib]],
                rng,
            );
            mutate(&mut child, rng);
            next.push(child);
        }

        self.individuals = next;
        self.generation += 1;
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        let saved = SavedPopulation {
            generation: self.generation,
            individuals: self.individuals.clone(),
            best: self.best,
            best_fitness: self.best_fitness,
        };
        let json = serde_json::to_string_pretty(&saved)?;
        fs::write(path, json)
    }
}

fn pick_parent_rank(rng: &mut ThreadRng) -> usize {
    for r in 0..POP_SIZE {
        if rng.gen_bool(PARENT_SCAN_P) {
            return r;
        }
    }
    0
}

fn random_genome(rng: &mut ThreadRng) -> Genome {
    let mut g = [0.0f32; GENOME_LEN];
    for i in 0..GENOME_LEN {
        g[i] = rng.gen_range(-INIT_WEIGHT_RANGE..INIT_WEIGHT_RANGE);
    }
    g
}

fn crossover(a: &Genome, b: &Genome, rng: &mut ThreadRng) -> Genome {
    let mut c = [0.0f32; GENOME_LEN];
    for i in 0..GENOME_LEN {
        c[i] = if rng.gen_bool(0.5) { a[i] } else { b[i] };
    }
    c
}

fn mutate(g: &mut Genome, rng: &mut ThreadRng) {
    for i in 0..GENOME_LEN {
        if rng.gen_bool(MUTATE_PROB) {
            g[i] += rng.gen_range(-MUTATE_STD..MUTATE_STD);
        }
    }
}

fn load(path: &Path) -> Result<SavedPopulation, String> {
    let s = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&s).map_err(|e| e.to_string())
}
