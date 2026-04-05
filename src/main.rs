//! Cart-pole balancer evolutionary experiment: `--headless` or graphical (default).

mod evolution;
mod physics;
mod render;

use evolution::{Population, POP_SIZE, SAVE_FILENAME};
use macroquad::prelude::*;
use physics::{episode_step, State, RACE_DISTANCE_M};
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

fn graphical_physics_step(
    pop: &mut Population,
    eval_index: &mut usize,
    sim: &mut State,
    episode_elapsed: &mut f32,
    episode_steps: &mut u32,
    episode_max_x: &mut f32,
    last_episode: &mut Option<physics::EpisodeOutcome>,
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
                    &mut rng,
                    save_path.as_ref(),
                );
            }
        }

        clear_background(Color::from_rgba(24, 28, 36, 255));
        render::draw_cartpole(sim);

        let dist_rem = (RACE_DISTANCE_M as f32 - sim.x as f32).max(0.0);
        let gen_txt = format!("generation: {}", pop.generation);
        let run_txt = format!(
            "individual {}/{}  t={:.2}s  x={:.2}m  to finish: {:.2} m",
            eval_index + 1,
            POP_SIZE,
            episode_elapsed,
            sim.x,
            dist_rem
        );
        let fit_txt = format!(
            "best fitness (race, last breed): {:.3}",
            pop.best_fitness
        );
        draw_text(&gen_txt, 16.0, 24.0, 22.0, WHITE);
        draw_text(&run_txt, 16.0, 50.0, 22.0, LIGHTGRAY);
        draw_text(&fit_txt, 16.0, 76.0, 18.0, Color::from_rgba(180, 200, 220, 255));
        if let Some(last) = last_episode {
            let last_txt = if last.success {
                format!("last episode: FINISH in {:.3} s", last.time_s)
            } else {
                format!(
                    "last episode: out at {:.2} m  (t={:.2} s)",
                    last.max_x, last.time_s
                )
            };
            draw_text(
                &last_txt,
                16.0,
                102.0,
                16.0,
                Color::from_rgba(160, 175, 195, 255),
            );
        }
        let mode_txt = if shift {
            format!(
                "Shift: max physics until {:.0}% of a {:.0} Hz frame — release for 1:1 real time",
                PHYSICS_FRAME_BUDGET_FRAC * 100.0,
                FASTFORWARD_FPS
            )
        } else {
            format!(
                "Hold Shift: max physics per frame @ {:.0} Hz (see FASTFORWARD_FPS); {:.0} Hz real-time now",
                FASTFORWARD_FPS, GRAPHICS_FPS_NORMAL
            )
        };
        draw_text(&mode_txt, 16.0, screen_height() - 44.0, 16.0, GRAY);
        draw_text(
            "use --headless to train without a window",
            16.0,
            screen_height() - 24.0,
            16.0,
            DARKGRAY,
        );

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
