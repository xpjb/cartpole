//! Cart–pole dynamics (semi-implicit Euler): equations from the project spec.

use rand::Rng;

/// Fixed simulation timestep (60 Hz).
pub const DT: f64 = 1.0 / 60.0;

pub const M_CART: f64 = 1.0;
pub const M_POLE: f64 = 0.1;
pub const L: f64 = 0.5;
pub const G: f64 = 9.81;

/// Finish line: episode succeeds when `x >= RACE_DISTANCE_M`.
pub const RACE_DISTANCE_M: f64 = 100.0;
/// Episode fails when `x` goes past this left bound (meters). Allows brief backward motion.
pub const LEFT_BOUND_M: f64 = -1.0;

/// Weight on survival time in failure fitness (`W_TIME * time_s + W_DIST * max_x`).
pub const W_TIME: f32 = 0.04;
/// Weight on forward distance in failure fitness.
pub const W_DIST: f32 = 0.02;
/// Scale for inverse finish time on success (`BASE_SUCCESS + W_SPEED / time_s`).
pub const W_SPEED: f32 = 100.0;

/// Any successful finish must score above any failure. With linear failure fitness,
/// worst case is roughly `W_TIME * (MAX_EPISODE_STEPS * DT) + W_DIST * RACE_DISTANCE_M` (≈ 667 < 1000).
pub const BASE_SUCCESS: f32 = 1000.0;
const FITNESS_TIME_EPS: f32 = 1e-6;

#[inline]
fn fitness_fail(time_s: f32, max_x: f32) -> f32 {
    W_TIME * time_s.min(3.0) + W_DIST * max_x
}

pub const ANGLE_LIMIT_RAD: f64 = std::f64::consts::PI / 180.0 * 30.0;

/// Uniform noise half-width on θ each step (radians).
pub const THETA_NOISE: f64 = 1e-3;

pub const MAX_ABS_FORCE: f64 = 500.0;

/// Controller: 4 linear weights on `(x, x_dot, theta, theta_dot)`, then 6 weights on
/// products `v_i * v_j` for `i < j` in that variable order (strict upper triangle).
pub const GENOME_LEN: usize = 10;
pub type Genome = [f32; GENOME_LEN];

#[derive(Clone, Copy, Debug, Default)]
pub struct State {
    pub x: f64,
    pub x_dot: f64,
    pub theta: f64,
    pub theta_dot: f64,
}

impl State {
    pub fn initial_fixed() -> Self {
        Self {
            x: 0.0,
            x_dot: 0.0,
            theta: 0.0,
            theta_dot: 0.0,
        }
    }

    /// Still contending: pole up and cart not left of `LEFT_BOUND_M`.
    #[inline]
    pub fn alive(self) -> bool {
        self.theta.abs() <= ANGLE_LIMIT_RAD && self.x >= LEFT_BOUND_M
    }
}

#[derive(Clone, Copy, Debug)]
pub struct EpisodeOutcome {
    pub fitness: f32,
    pub success: bool,
    pub time_s: f32,
    pub max_x: f32,
}

#[inline]
fn force_from_genome(state: &State, g: &Genome) -> f64 {
    let x = state.x;
    let xd = state.x_dot;
    let th = state.theta;
    let td = state.theta_dot;
    let v = [x, xd, th, td];
    let mut f = g[0] as f64 * x
        + g[1] as f64 * xd
        + g[2] as f64 * th
        + g[3] as f64 * td;
    let mut k = 4usize;
    for i in 0..4 {
        for j in (i + 1)..4 {
            f += g[k] as f64 * v[i] * v[j];
            k += 1;
        }
    }
    debug_assert_eq!(k, GENOME_LEN);
    f.clamp(-MAX_ABS_FORCE, MAX_ABS_FORCE)
}

/// Semi-implicit Euler: update angular velocity and cart velocity, then positions; then θ noise.
pub fn step_state(state: &mut State, genome: &Genome, rng: &mut impl Rng) {
    let m = M_CART;
    let m_p = M_POLE;
    let l = L;
    let grav = G;
    let sum_m = m + m_p;

    let force = force_from_genome(state, genome);
    let theta = state.theta;
    let theta_dot = state.theta_dot;
    let sin_t = theta.sin();
    let cos_t = theta.cos();

    let numer = -force - m_p * l * theta_dot * theta_dot * sin_t;
    let theta_ddot = (grav * sin_t + cos_t * (numer / sum_m))
        / (l * (4.0 / 3.0 - m_p * cos_t * cos_t / sum_m));
    let x_ddot =
        (force + m_p * l * (theta_dot * theta_dot * sin_t - theta_ddot * cos_t)) / sum_m;

    state.theta_dot += theta_ddot * DT;
    state.x_dot += x_ddot * DT;
    state.theta += state.theta_dot * DT;
    state.x += state.x_dot * DT;

    let n: f64 = rng.gen_range(-THETA_NOISE..THETA_NOISE);
    state.theta += n;
}

/// Max physics steps per episode (matches headless `evaluate` cap).
pub const MAX_EPISODE_STEPS: u32 = 1_000_000;

/// One physics step of an episode. Returns `Some` when the episode ends.
pub fn episode_step(
    s: &mut State,
    t: &mut f32,
    steps: &mut u32,
    max_x: &mut f32,
    genome: &Genome,
    rng: &mut impl Rng,
) -> Option<EpisodeOutcome> {
    if *steps >= MAX_EPISODE_STEPS {
        return Some(EpisodeOutcome {
            fitness: fitness_fail(*t, *max_x),
            success: false,
            time_s: *t,
            max_x: *max_x,
        });
    }

    step_state(s, genome, rng);
    *t += DT as f32;
    *steps += 1;
    *max_x = (*max_x).max(s.x as f32);

    if s.x >= RACE_DISTANCE_M {
        let t_clamped = (*t).max(FITNESS_TIME_EPS);
        return Some(EpisodeOutcome {
            fitness: BASE_SUCCESS + W_SPEED / t_clamped,
            success: true,
            time_s: *t,
            max_x: *max_x,
        });
    }
    if !s.alive() {
        return Some(EpisodeOutcome {
            fitness: fitness_fail(*t, *max_x),
            success: false,
            time_s: *t,
            max_x: *max_x,
        });
    }
    None
}

/// Race fitness: `W_TIME*time + W_DIST*max_x` when failing; `BASE_SUCCESS + W_SPEED/time` on finish.
pub fn evaluate(genome: &Genome, rng: &mut impl Rng) -> f32 {
    let mut s = State::initial_fixed();
    let mut t = 0.0f32;
    let mut steps = 0u32;
    let mut max_x = 0.0f32;
    loop {
        if let Some(out) = episode_step(&mut s, &mut t, &mut steps, &mut max_x, genome, rng) {
            return out.fitness;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_beats_worst_failure_fitness() {
        let t_cap = MAX_EPISODE_STEPS as f32 * DT as f32;
        let worst_fail = W_TIME * t_cap + W_DIST * RACE_DISTANCE_M as f32;
        assert!(
            BASE_SUCCESS > worst_fail,
            "BASE_SUCCESS must exceed any linear failure score (here ~{worst_fail})"
        );
        let slow_finish = BASE_SUCCESS + W_SPEED / t_cap;
        assert!(slow_finish > worst_fail);
    }
}
