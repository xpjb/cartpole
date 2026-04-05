//! Cart–pole dynamics (semi-implicit Euler): equations from the project spec.

use rand::Rng;

/// Fixed simulation timestep (60 Hz).
pub const DT: f64 = 1.0 / 60.0;

pub const M_CART: f64 = 1.0;
pub const M_POLE: f64 = 0.1;
pub const L: f64 = 0.5;
pub const G: f64 = 9.81;

/// Finish line: episode succeeds when `x >= RACE_DISTANCE_M`.
pub const RACE_DISTANCE_M: f64 = 20.0;

/// Any successful finish beats any failure (`fitness <= RACE_DISTANCE_M` as f32).
pub const BASE_SUCCESS: f32 = 1000.0;
const FITNESS_TIME_EPS: f32 = 1e-6;

pub const ANGLE_LIMIT_RAD: f64 = std::f64::consts::PI / 180.0 * 30.0;

/// Uniform noise half-width on θ each step (radians).
pub const THETA_NOISE: f64 = 1e-3;

pub const MAX_ABS_FORCE: f64 = 50.0;

pub type Genome = [f32; 4];

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

    /// Still contending: pole up and not left of the start line.
    #[inline]
    pub fn alive(self) -> bool {
        self.theta.abs() <= ANGLE_LIMIT_RAD && self.x >= 0.0
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
    let f = g[0] as f64 * state.x
        + g[1] as f64 * state.x_dot
        + g[2] as f64 * state.theta
        + g[3] as f64 * state.theta_dot;
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
            fitness: *max_x,
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
            fitness: BASE_SUCCESS + 1.0 / t_clamped,
            success: true,
            time_s: *t,
            max_x: *max_x,
        });
    }
    if !s.alive() {
        return Some(EpisodeOutcome {
            fitness: *max_x,
            success: false,
            time_s: *t,
            max_x: *max_x,
        });
    }
    None
}

/// Race fitness: distance when failing, speed (via `1/time`) when reaching the finish.
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
