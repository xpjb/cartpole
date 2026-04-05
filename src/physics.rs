//! Cart–pole dynamics (semi-implicit Euler): equations from the project spec.

use rand::Rng;

/// Fixed simulation timestep (60 Hz).
pub const DT: f64 = 1.0 / 60.0;

pub const M_CART: f64 = 1.0;
pub const M_POLE: f64 = 0.1;
pub const L: f64 = 0.5;
pub const G: f64 = 9.81;

pub const TRACK_LIMIT: f64 = 3.6;
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

    #[inline]
    pub fn alive(self) -> bool {
        self.x.abs() <= TRACK_LIMIT && self.theta.abs() <= ANGLE_LIMIT_RAD
    }
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

/// Survival time in seconds for one episode (fixed start, run until failure).
pub fn evaluate(genome: &Genome, rng: &mut impl Rng) -> f32 {
    let mut s = State::initial_fixed();
    let mut t = 0.0f32;
    let mut steps = 0u32;
    while s.alive() && steps < MAX_EPISODE_STEPS {
        step_state(&mut s, genome, rng);
        t += DT as f32;
        steps += 1;
    }
    t
}
