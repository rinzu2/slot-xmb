//! Where the waves run, column by column, at a moment in time. The GPU shader in
//! `slot_gfx::Backdrop` lights the screen from these curves.
//!
//! Original has two sheets whose edges run close together across the middle of the screen, one
//! opaque and one a thin veil, crossing and parting as they roll. Classic has four waves: an
//! upper pair that cross each other a third of the way down, and a lower pair that run close
//! together near the bottom.

use slot_gfx::{
    backdrop_put, BACKDROP_COLS, BACKDROP_LINES, BACKDROP_ROWS, ROW_EDGES, ROW_LINES, STYLE_CLASSIC,
};

const TAU: f32 = std::f32::consts::TAU;

/// The sheet edge and the veil edge at `u` (0..1 across), as fractions of the screen height.
pub fn original_edges(u: f32, t: f32) -> (f32, f32) {
    // The crest drifts to the right, as on the handheld.
    let main = 0.62
        + 0.11 * (TAU * 0.62 * u - 0.25 * t + 4.145).sin()
        + 0.035 * (TAU * 1.3 * u - 0.18 * t + 1.7).sin();
    let veil = main + 0.05 * (TAU * 0.85 * u - 0.33 * t + 2.4).sin() - 0.012;
    (main, veil)
}

/// Each Classic wave: (height, tilt across the screen, swell, phase, speed).
const WAVES: [(f32, f32, f32, f32, f32); BACKDROP_LINES] = [
    (0.33, 0.10, 0.060, 0.0, 0.16),
    (0.36, -0.12, 0.050, 2.2, -0.13),
    (0.80, 0.04, 0.045, 1.1, 0.12),
    (0.855, -0.03, 0.04, 4.0, -0.15),
];

/// Classic's four waves at `u`: the upper pair first, then the lower pair.
pub fn classic_lines(u: f32, t: f32) -> [f32; BACKDROP_LINES] {
    WAVES.map(|(mid, tilt, swell, phase, speed)| {
        mid + tilt * (u - 0.5)
            + swell * (TAU * 0.55 * u + speed * t + phase).sin()
            + 0.3 * swell * (TAU * 1.15 * u - 0.7 * speed * t + 1.7 * phase).sin()
    })
}

/// Fills `out` with the curve texture for `style` at time `t` (seconds), `BACKDROP_COLS` by
/// `BACKDROP_ROWS` RGBA texels.
pub fn xmb_curves(style: u8, t: f32, out: &mut Vec<u8>) {
    xmb_curves_at(style, t, None, out);
}

/// How much further the waves have run `s` seconds after the start-up's fast run began, in
/// seconds of their usual drift. Measured off the handheld: the crest crosses the screen about
/// twenty times its usual speed for a second and a quarter, brakes hard, and eases back to its
/// drift over the next few seconds.
pub fn xmb_wave_rush(s: f32) -> f32 {
    const DRIFT: f32 = 0.25;
    const FAST: f32 = 4.7;
    const TAIL: f32 = 0.75;
    const HOLD: f32 = 1.25;
    let s = s.max(0.0);
    let held = s.min(HOLD);
    let after = (s - HOLD).max(0.0);
    let rad = (FAST + TAIL) * held
        + FAST * 0.15 * (1.0 - (-after / 0.15).exp())
        + TAIL * 1.6 * (1.0 - (-after / 1.6).exp());
    rad / DRIFT
}

/// Where the start-up's hairlines lie, as fractions of the screen height: the waves grow out
/// of them. Two close together above the middle, one below.
pub const XMB_INTRO_LINES: [f32; 3] = [0.479, 0.49, 0.5167];

fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Classic's waves fling out of the hairlines like a plucked string: past where they rest by
/// nearly half as much again, back, and settled within a second. The left of the screen
/// leads the right by a tenth of a second.
fn flung(u: f32, s: f32) -> f32 {
    let x = s + 0.05 - 0.1 * u;
    if x <= 0.0 {
        return 0.0;
    }
    // The left swings furthest, the waves fanning out from the right.
    let lift = 0.1 + 0.7 * (1.0 - u);
    let w = x * std::f32::consts::TAU / 0.6;
    1.0 - (-x / 0.3).exp() * (w.cos() - lift * w.sin())
}

/// How far Original's edges have grown out of the hairlines: the right of the screen first,
/// the left a tenth of a second behind.
fn grown(u: f32, s: f32) -> f32 {
    ease((s + 0.08 - 0.1 * (1.0 - u)) / 0.2)
}

/// As `xmb_curves`, `rush` seconds after the start-up's fast run began when given: the waves
/// start as the start-up's hairlines and grow out of them.
pub fn xmb_curves_at(style: u8, t: f32, rush: Option<f32>, out: &mut Vec<u8>) {
    out.clear();
    out.resize(BACKDROP_COLS * BACKDROP_ROWS * 4, 0);
    let t = t + rush.map_or(0.0, xmb_wave_rush);
    let [high, mid, low] = XMB_INTRO_LINES;
    for col in 0..BACKDROP_COLS {
        let u = (col as f32 + 0.5) / BACKDROP_COLS as f32;
        if style == STYLE_CLASSIC {
            let mut lines = classic_lines(u, t);
            if let Some(s) = rush {
                let g = flung(u, s);
                let from = [high, mid, low, (mid + low) / 2.0];
                for (y, f) in lines.iter_mut().zip(from) {
                    *y = f + (*y - f) * g;
                }
            }
            for (i, y) in lines.into_iter().enumerate() {
                backdrop_put(out, col, ROW_LINES + i / 2, i % 2 == 1, y);
            }
        } else {
            let (mut main, mut veil) = original_edges(u, t);
            if let Some(s) = rush {
                let g = grown(u, s);
                main = mid + (main - mid) * g;
                veil = high + (veil - high) * g;
            }
            backdrop_put(out, col, ROW_EDGES, false, main);
            backdrop_put(out, col, ROW_EDGES, true, veil);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_edges_stay_on_screen_and_cross() {
        let mut crossed = false;
        for step in 0..400 {
            let t = step as f32 * 0.25;
            let mut sign = None;
            for c in 0..64 {
                let (a, b) = original_edges(c as f32 / 63.0, t);
                assert!((0.3..0.9).contains(&a) && (0.25..0.95).contains(&b));
                let s = a > b;
                crossed |= sign.is_some_and(|p| p != s);
                sign = Some(s);
            }
        }
        assert!(crossed, "the two Original edges never cross");
    }

    #[test]
    fn classic_has_four_waves_and_the_upper_pair_cross() {
        let mut crossed = false;
        for step in 0..200 {
            let t = step as f32 * 0.5;
            let at = |u| classic_lines(u, t);
            crossed |= (at(0.0)[0] > at(0.0)[1]) != (at(1.0)[0] > at(1.0)[1]);
            for c in 0..64 {
                let l = at(c as f32 / 63.0);
                assert!(
                    l[0] < 0.5 && l[1] < 0.5 && l[2] > 0.7 && l[3] > 0.7,
                    "{l:?}"
                );
            }
        }
        assert!(crossed);
    }
}
