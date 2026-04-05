//! Cart-pole balancer evolutionary experiment: `--headless` or graphical (default).

mod evolution;
mod physics;
mod render;

use evolution::{Population, POP_SIZE, SAVE_FILENAME};
use macroquad::prelude::*;
use physics::{State, MAX_EPISODE_STEPS};
use ::rand::thread_rng;
use std::env;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Target graphics frame rate (real-time physics: ~one 60 Hz physics step per wall-clock frame at 60 Hz).
const GRAPHICS_FPS_NORMAL: f32 = 60.0;
/// With Shift held: target graphics FPS; each frame runs as many physics steps as fit in ~92% of `1/FASTFORWARD_FPS` seconds (CPU-limited).
const FASTFORWARD_FPS: f32 = 60.0;

const PHYSICS_FRAME_BUDGET_FRAC: f32 = 0.92;

fn headless_main() {
    let mut rng = thread_rng();
    let path = PathBuf::from(SAVE_FILENAME);
    let mut pop = Population::load_or_new(&path, &mut rng);

    loop {
        pop.run_generation(&mut rng);
        if let Err(e) = pop.save(&path) {
            eprintln!("save failed: {e}");
        }
        if pop.generation % 10 == 0 {
            eprintln!(
                "generation {} best fitness {:.3} s",
                pop.generation, pop.best_fitness
            );
        }
    }
}

fn graphical_conf() -> macroquad::conf::Conf {
    macroquad::conf::Conf {
        miniquad_conf: macroquad::miniquad::conf::Conf {
            window_title: "Cart-pole evolution".to_owned(),
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
    rng: &mut ::rand::rngs::ThreadRng,
    path: &PathBuf,
) {
    let dt = physics::DT as f32;
    if sim.alive() && *episode_steps < MAX_EPISODE_STEPS {
        physics::step_state(sim, &pop.individuals[*eval_index], rng);
        *episode_elapsed += dt;
        *episode_steps += 1;
    }
    if !sim.alive() || *episode_steps >= MAX_EPISODE_STEPS {
        pop.fitness[*eval_index] = *episode_elapsed;
        *eval_index += 1;
        *sim = State::initial_fixed();
        *episode_elapsed = 0.0;
        *episode_steps = 0;

        if *eval_index == POP_SIZE {
            pop.breed_next_generation(rng);
            let _ = pop.save(path);
            *eval_index = 0;
        }
    }
}

async fn graphical_run() {
    let mut rng = thread_rng();
    let path = PathBuf::from(SAVE_FILENAME);
    let mut pop = Population::load_or_new(&path, &mut rng);

    let dt = physics::DT as f32;
    let mut sim_accum: f32 = 0.0;

    let mut eval_index: usize = 0;
    let mut sim = State::initial_fixed();
    let mut episode_elapsed: f32 = 0.0;
    let mut episode_steps: u32 = 0;

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
                    &mut rng,
                    &path,
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
                    &mut rng,
                    &path,
                );
            }
        }

        clear_background(Color::from_rgba(24, 28, 36, 255));
        render::draw_cartpole(sim);

        let gen_txt = format!("generation: {}", pop.generation);
        let run_txt = format!(
            "individual {}/{}  episode: {:.2} s",
            eval_index + 1,
            POP_SIZE,
            episode_elapsed
        );
        let fit_txt = format!("best fitness (last breed): {:.2} s", pop.best_fitness);
        draw_text(&gen_txt, 16.0, 24.0, 22.0, WHITE);
        draw_text(&run_txt, 16.0, 50.0, 22.0, LIGHTGRAY);
        draw_text(&fit_txt, 16.0, 76.0, 18.0, Color::from_rgba(180, 200, 220, 255));
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
