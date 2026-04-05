//! 2D cart–pole drawing (Macroquad): camera follows cart; world-space distance grid.

use crate::physics::{State, L, MAX_ABS_FORCE, RACE_DISTANCE_M};
use macroquad::prelude::*;

const POLE_SCREEN_LEN: f32 = 120.0;
const CART_W: f32 = 48.0;
const CART_H: f32 = 28.0;
const RAIL_MARGIN: f32 = 40.0;
/// Horizontal cart anchor as fraction of screen width (camera follow).
const CAMERA_ANCHOR_FRAC: f32 = 0.4;
/// Nominal meters visible to one side for zoom (similar density to old 3.6 m half-width).
const ZOOM_REF_M: f32 = 8.0;

#[inline]
fn world_to_screen_x(world_x: f64, cart_x: f64, w: f32, meters_to_px: f32) -> f32 {
    w * CAMERA_ANCHOR_FRAC + (world_x - cart_x) as f32 * meters_to_px
}

pub fn draw_cartpole(state: State) {
    let w = screen_width();
    let h = screen_height();
    let rail_y = h * 0.55;
    let cart_x = state.x;

    let meters_to_px = (w * 0.5 - RAIL_MARGIN) / ZOOM_REF_M;

    let cart_cx = world_to_screen_x(cart_x, cart_x, w, meters_to_px);
    let cart_top = rail_y - CART_H * 0.5;

    let world_left = cart_x - (w * CAMERA_ANCHOR_FRAC) as f64 / meters_to_px as f64;
    let world_right =
        cart_x + (w * (1.0 - CAMERA_ANCHOR_FRAC)) as f64 / meters_to_px as f64;
    let k_min = world_left.floor() as i32 - 1;
    let k_max = world_right.ceil() as i32 + 2;
    let finish_k = RACE_DISTANCE_M.ceil() as i32 + 1;

    let rail_y_top = rail_y - 70.0;
    let rail_y_bot = rail_y + CART_H * 0.5 + 4.0;

    for k in k_min..=k_max.max(finish_k) {
        let xw = k as f64;
        let sx = world_to_screen_x(xw, cart_x, w, meters_to_px);
        if sx < -40.0 || sx > w + 40.0 {
            continue;
        }
        let is_finish = (xw - RACE_DISTANCE_M).abs() < 1e-6;
        let is_start = k == 0;
        let col = if is_finish {
            Color::from_rgba(60, 200, 120, 255)
        } else if is_start {
            MAROON
        } else {
            Color::from_rgba(55, 62, 75, 255)
        };
        let thick = if is_finish || is_start { 2.5 } else { 1.0 };
        draw_line(sx, rail_y_top, sx, rail_y_bot + 36.0, thick, col);

        if k % 5 == 0 && !is_finish {
            let label = format!("{}", k);
            draw_text(
                &label,
                sx - 6.0,
                rail_y_bot + 42.0,
                14.0,
                Color::from_rgba(130, 140, 155, 255),
            );
        }
    }

    let rail_x0 = world_to_screen_x(-1.0, cart_x, w, meters_to_px);
    let rail_x1 = world_to_screen_x(RACE_DISTANCE_M + 2.0, cart_x, w, meters_to_px);
    draw_line(rail_x0, rail_y_bot, rail_x1, rail_y_bot, 3.0, DARKGRAY);

    draw_rectangle(
        cart_cx - CART_W * 0.5,
        cart_top,
        CART_W,
        CART_H,
        SKYBLUE,
    );

    let scale = POLE_SCREEN_LEN / L as f32;
    let sin_t = state.theta.sin() as f32;
    let cos_t = state.theta.cos() as f32;
    let tip_x = cart_cx + L as f32 * scale * sin_t;
    let tip_y = cart_top - L as f32 * scale * cos_t;

    draw_line(cart_cx, cart_top, tip_x, tip_y, 5.0, GOLD);
    draw_circle(tip_x, tip_y, 7.0, ORANGE);
}

/// Bottom-center: horizontal force, linear in `|f| / MAX_ABS_FORCE`; full scale = 25% of screen each way from center.
pub fn draw_control_force_arrow(force: f64) {
    let w = screen_width();
    let h = screen_height();
    let cx = w * 0.5;
    let cy = h - 52.0;
    let red = Color::from_rgba(220, 55, 55, 255);

    let len = (force.abs() / MAX_ABS_FORCE) * (w as f64 * 10.0);
    if len < 1.0 {
        draw_circle(cx, cy, 3.0, red);
        return;
    }

    let dir = force.signum() as f32;
    let tip_x = (cx as f64 + force.signum() * len) as f32;
    let head = 6.0f32;
    let wing = 4.0f32;
    let base_x = tip_x - dir * head;

    draw_line(cx, cy, tip_x, cy, 2.5, red);
    draw_line(tip_x, cy, base_x, cy + wing, 2.5, red);
    draw_line(tip_x, cy, base_x, cy - wing, 2.5, red);
}
