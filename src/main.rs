//! Cart-pole balancer evolutionary experiment: `--headless` or graphical (default).

mod evolution;
mod physics;
mod render;

use evolution::{Population, POP_SIZE, SAVE_FILENAME};
use macroquad::prelude::*;
use physics::{control_force, episode_step, State};
use ::rand::thread_rng;
use std::env;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Target graphics frame rate (real-time physics: ~one 60 Hz physics step per wall-clock frame at 60 Hz).
const GRAPHICS_FPS_NORMAL: f32 = 60.0;
/// With Shift held: target graphics FPS; each frame runs as many physics steps as fit in ~92% of `1/FASTFORWARD_FPS` seconds (CPU-limited).
const FASTFORWARD_FPS: f32 = 60.0;

const PHYSICS_FRAME_BUDGET_FRAC: f32 = 0.92;

/// When true, load `SAVE_FILENAME` on start and save after each generation / breed.
const PERSIST_POPULATION: bool = false;

fn headless_main() {
    let mut rng = thread_rng();
    let save_path = PERSIST_POPULATION.then(|| PathBuf::from(SAVE_FILENAME));
    let mut pop = match &save_path {
        Some(p) => Population::load_or_new(p, &mut rng),
        None => Population::new_random(&mut rng),
    };

    loop {
        pop.run_generation(&mut rng);
        if let Some(ref path) = save_path {
            if let Err(e) = pop.save(path) {
                eprintln!("save failed: {e}");
            }
        }
        if pop.generation % 10 == 0 {
            eprintln!(
                "generation {} best fitness (race) {:.3}",
                pop.generation, pop.best_fitness
            );
        }
    }
}

fn graphical_conf() -> macroquad::conf::Conf {
    macroquad::conf::Conf {
        miniquad_conf: macroquad::miniquad::conf::Conf {
            window_title: "Cart-pole race".to_owned(),
            high_dpi: true,
            window_width: 900,
            window_height: 600,
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Row label column (left).
const STAT_LABEL_W: usize = 14;
/// Padded numeric columns: `distance`, `time`, `fitness`.
const STAT_DIST_W: usize = 8;
const STAT_TIME_W: usize = 8;
const STAT_FIT_W: usize = 9;

fn stat_table_header_line() -> String {
    format!(
        "{:<lw$}{:>dw$}  {:>tw$}  {:>fw$}",
        "",
        "distance",
        "time",
        "fitness",
        lw = STAT_LABEL_W,
        dw = STAT_DIST_W,
        tw = STAT_TIME_W,
        fw = STAT_FIT_W,
    )
}

fn stat_table_row(label: &str, cells: Option<(f32, f32, f32)>) -> String {
    match cells {
        Some((d, t, f)) => format!(
            "{:<lw$}{:>dw$.2}  {:>tw$.2}  {:>fw$.3}",
            label,
            d,
            t,
            f,
            lw = STAT_LABEL_W,
            dw = STAT_DIST_W,
            tw = STAT_TIME_W,
            fw = STAT_FIT_W,
        ),
        None => format!(
            "{:<lw$}{:>dw$}  {:>tw$}  {:>fw$}",
            label,
            "---",
            "---",
            "---",
            lw = STAT_LABEL_W,
            dw = STAT_DIST_W,
            tw = STAT_TIME_W,
            fw = STAT_FIT_W,
        ),
    }
}

fn graphical_physics_step(
    pop: &mut Population,
    eval_index: &mut usize,
    sim: &mut State,
    episode_elapsed: &mut f32,
    episode_steps: &mut u32,
    episode_max_x: &mut f32,
    last_episode: &mut Option<physics::EpisodeOutcome>,
    gen_best: &mut Option<(f32, f32, f32)>,
    session_best: &mut Option<(f32, f32, f32)>,
    rng: &mut ::rand::rngs::ThreadRng,
    save_path: Option<&PathBuf>,
) {
    if let Some(outcome) = episode_step(
        sim,
        episode_elapsed,
        episode_steps,
        episode_max_x,
        &pop.individuals[*eval_index],
        rng,
    ) {
        *last_episode = Some(outcome);
        pop.fitness[*eval_index] = outcome.fitness;

        let better = |a: f32, b: f32| {
            a.partial_cmp(&b)
                .unwrap_or(std::cmp::Ordering::Equal)
                == std::cmp::Ordering::Greater
        };
        if gen_best.map_or(true, |(_, _, f)| better(outcome.fitness, f)) {
            *gen_best = Some((outcome.max_x, outcome.time_s, outcome.fitness));
        }
        if session_best.map_or(true, |(_, _, f)| better(outcome.fitness, f)) {
            *session_best = Some((outcome.max_x, outcome.time_s, outcome.fitness));
        }

        *eval_index += 1;
        *sim = State::initial_fixed();
        *episode_elapsed = 0.0;
        *episode_steps = 0;
        *episode_max_x = 0.0;

        if *eval_index == POP_SIZE {
            pop.breed_next_generation(rng);
            if let Some(path) = save_path {
                let _ = pop.save(path);
            }
            *eval_index = 0;
            *gen_best = None;
        }
    }
}

async fn graphical_run() {
    let mut rng = thread_rng();
    let save_path = PERSIST_POPULATION.then(|| PathBuf::from(SAVE_FILENAME));
    let mut pop = match &save_path {
        Some(p) => Population::load_or_new(p, &mut rng),
        None => Population::new_random(&mut rng),
    };

    let dt = physics::DT as f32;
    let mut sim_accum: f32 = 0.0;

    let mut eval_index: usize = 0;
    let mut sim = State::initial_fixed();
    let mut episode_elapsed: f32 = 0.0;
    let mut episode_steps: u32 = 0;
    let mut episode_max_x: f32 = 0.0;
    let mut last_episode: Option<physics::EpisodeOutcome> = None;
    let mut gen_best: Option<(f32, f32, f32)> = None;
    let mut session_best: Option<(f32, f32, f32)> = None;

    loop {
        let frame_dt = get_frame_time();
        let shift = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);

        if shift {
            let gfx_fps = FASTFORWARD_FPS.clamp(1.0, 480.0);
            let frame_period = Duration::from_secs_f32((1.0 / gfx_fps) * PHYSICS_FRAME_BUDGET_FRAC);
            let t0 = Instant::now();
            let mut steps_this_frame: u32 = 0;
            const MAX_STEPS_PER_FRAME: u32 = 5_000_000;

            while t0.elapsed() < frame_period && steps_this_frame < MAX_STEPS_PER_FRAME {
                graphical_physics_step(
                    &mut pop,
                    &mut eval_index,
                    &mut sim,
                    &mut episode_elapsed,
                    &mut episode_steps,
                    &mut episode_max_x,
                    &mut last_episode,
                    &mut gen_best,
                    &mut session_best,
                    &mut rng,
                    save_path.as_ref(),
                );
                steps_this_frame += 1;
            }
        } else {
            sim_accum += frame_dt;
            sim_accum = sim_accum.min(0.25);

            while sim_accum >= dt {
                sim_accum -= dt;
                graphical_physics_step(
                    &mut pop,
                    &mut eval_index,
                    &mut sim,
                    &mut episode_elapsed,
                    &mut episode_steps,
                    &mut episode_max_x,
                    &mut last_episode,
                    &mut gen_best,
                    &mut session_best,
                    &mut rng,
                    save_path.as_ref(),
                );
            }
        }

        clear_background(Color::from_rgba(24, 28, 36, 255));
        render::draw_cartpole(sim);
        let f = control_force(&sim, &pop.individuals[eval_index]);
        render::draw_control_force_arrow(f);

        let gen_ep_line = format!(
            "gen {}  episode {}/{}",
            pop.generation,
            eval_index + 1,
            POP_SIZE
        );
        let header_line = stat_table_header_line();
        let prev_line = stat_table_row(
            "prev:",
            last_episode.map(|e| (e.max_x, e.time_s, e.fitness)),
        );
        let gen_line = stat_table_row("best of gen:", gen_best);
        let seen_line = stat_table_row("best seen:", session_best);

        let y0 = 20.0;
        let line_h = 22.0;
        let size_title = 20.0;
        let size_table = 18.0;
        draw_text(&gen_ep_line, 16.0, y0, size_title, WHITE);
        draw_text(
            &header_line,
            16.0,
            y0 + line_h,
            size_table,
            Color::from_rgba(150, 170, 195, 255),
        );
        let stat_color = Color::from_rgba(200, 215, 235, 255);
        draw_text(&prev_line, 16.0, y0 + 2.0 * line_h, size_table, stat_color);
        draw_text(&gen_line, 16.0, y0 + 3.0 * line_h, size_table, stat_color);
        draw_text(&seen_line, 16.0, y0 + 4.0 * line_h, size_table, stat_color);
        let mode_txt = 
            format!(
                "Shift: Super Fast Forward"
            );
        draw_text(&mode_txt, 16.0, screen_height() - 44.0, 16.0, GRAY);
        next_frame().await;
    }
}

fn main() {
    let headless = env::args().skip(1).any(|a| a == "--headless");
    if headless {
        headless_main();
        return;
    }
    macroquad::Window::from_config(graphical_conf(), graphical_run());
}
