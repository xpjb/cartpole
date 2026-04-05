//! 2D cart–pole drawing (Macroquad).

use crate::physics::{State, L, TRACK_LIMIT};
use macroquad::prelude::*;

const POLE_SCREEN_LEN: f32 = 120.0;
const CART_W: f32 = 48.0;
const CART_H: f32 = 28.0;
const RAIL_MARGIN: f32 = 40.0;

pub fn draw_cartpole(state: State) {
    let w = screen_width();
    let h = screen_height();
    let cx = w * 0.5;
    let rail_y = h * 0.55;

    let meters_to_px = (w * 0.5 - RAIL_MARGIN) / TRACK_LIMIT as f32;
    let cart_cx = cx + state.x as f32 * meters_to_px;
    let cart_top = rail_y - CART_H * 0.5;

    draw_line(
        RAIL_MARGIN,
        rail_y + CART_H * 0.5 + 4.0,
        w - RAIL_MARGIN,
        rail_y + CART_H * 0.5 + 4.0,
        3.0,
        DARKGRAY,
    );

    let wall_l = cx - TRACK_LIMIT as f32 * meters_to_px;
    let wall_r = cx + TRACK_LIMIT as f32 * meters_to_px;
    draw_line(wall_l, rail_y - 60.0, wall_l, rail_y + 40.0, 2.0, MAROON);
    draw_line(wall_r, rail_y - 60.0, wall_r, rail_y + 40.0, 2.0, MAROON);

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
