//! The start-up and switch-off animations, measured frame by frame off a capture of the
//! handheld's own video output.
//!
//! From the moment the screen lights (seconds):
//! - 0.00 three dim grey hairlines appear across the middle of the black screen at once, two
//!   close together above the middle and one below, with a line of lettering set on them: its
//!   capitals stand on the top line, its small letters on the second, and the lowest runs just
//!   under its foot. Here the lettering is the name, already up from the boot picture; it fades
//!   from 1.7 to 2.05.
//! - 0.33 a faint glow in the theme's colour starts to rise from the foot of the screen,
//!   growing until the picture comes.
//! - 1.40 the lines brighten and take on the theme's colour; a lens of light gathers around
//!   them, widest in the middle.
//! - 2.48 the lens swells into a great ellipse that fills the screen with the theme's colour,
//!   dim, by 2.58.
//! - 2.58 the picture brightens out of that to full strength by about 3.1, the waves surfacing
//!   in the first fifth of a second and running fast to the right, slowing to their usual drift
//!   over the next two seconds.
//! - 3.6 the clock fades in, then the bar.
//!
//! Switching off runs the middle of it backwards: the bar fades, the picture dims into the
//! lens, the lens draws back to the lines, and the screen goes dark.

use std::sync::OnceLock;

use fontdue::{Font, FontSettings};
use slot_gfx::{Backdrop, Canvas, Draw, TexId};

use crate::CartFace;

/// A broad geometric sans, near the face the handheld's start-up is lettered in.
const TITLE_TTF: &[u8] = include_bytes!("../../assets/intro.ttf");
/// What the start-up spells out.
pub const XMB_INTRO_TITLE: &str = "slot. XMB";
/// The lettering's grey, measured off the capture.
const TITLE_GREY: u8 = 221;
/// Capital height and baseline as fractions of the screen's height, measured off the capture.
const TITLE_CAP: f32 = 0.0361;
const TITLE_BASE: f32 = 0.5139;
/// Letter spacing, as a fraction of the capital height, and the word space as a fraction of
/// the face's own: the lettering is set tight.
const TITLE_TRACK: f32 = -0.02;
const TITLE_SPACE: f32 = 0.7;
const TITLE_FADE: f32 = 1.7;
const TITLE_FADE_SECS: f32 = 0.35;

pub const XMB_INTRO_BLOOM: f32 = 2.48;
/// When the waves start their fast run, for `xmb_curves_at`.
pub const XMB_INTRO_WAVES: f32 = 2.55;
pub const XMB_INTRO_CLOCK: f32 = 3.6;
pub const XMB_INTRO_BAR: f32 = 3.9;
pub const XMB_INTRO_END: f32 = 4.5;
pub const XMB_OUTRO_SECS: f32 = 1.3;
/// When Modern's own start, the PS3's, begins under the boot picture: from black, as the
/// console's does, the hairlines giving way as its wave comes up.
pub const XMB_MODERN_START: f32 = 0.6;

fn ramp(x: f32, from: f32, span: f32) -> f32 {
    ((x - from) / span).clamp(0.0, 1.0)
}

fn ease(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// How the background is shown at `t`: squeeze, brightness, smoothing, softness (see
/// `Backdrop::shape`). Before the lens swells there is no background.
pub fn xmb_intro_shape(t: f32) -> Option<[f32; 4]> {
    if t < XMB_INTRO_BLOOM {
        return None;
    }
    // The picture opens out from the lines to the top and bottom of the screen, its edges
    // softening as they go so that the colour comes into the top and bottom as a gradient.
    let open = ease(ramp(t, XMB_INTRO_BLOOM + 0.02, 0.34));
    let reach = lerp(0.02, 0.72, open);
    let soft = lerp(0.04, 0.2, open);
    let flat = 1.0 - ease(ramp(t, 2.5, 0.1));
    let bright = lerp(0.2, 0.38, open) + (1.0 - 0.38) * ease(ramp(t, 2.58, 0.52));
    Some([reach, bright, flat, soft])
}

/// Where switching off is in the animation, `p` running 0 to 1.
pub fn xmb_outro_t(p: f32) -> f32 {
    lerp(3.1, 2.3, ease(ramp(p, 0.1, 0.5)))
}

/// The lines and lens take on a pale form of the theme's colour.
fn pale(theme: [f32; 3]) -> [f32; 3] {
    theme.map(|c| 0.55 * c + 0.45)
}

/// The pale form of a theme colour the lines and the lens are drawn in.
pub fn xmb_pale(rgb: [u8; 3]) -> [u8; 3] {
    rgb.map(|c| (0.55 * c as f32 + 0.45 * 255.0) as u8)
}

/// A soft ellipse of light, its middle nearly white, fading to the colour and then to nothing:
/// stretched wide it is the glow between the lines and the flood.
pub fn xmb_flare_face(rgb: [u8; 3]) -> CartFace {
    let (w, h) = (128u32, 64u32);
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let dx = (x as f32 + 0.5) / w as f32 * 2.0 - 1.0;
            let dy = (y as f32 + 0.5) / h as f32 * 2.0 - 1.0;
            let d2 = dx * dx * 0.35 + dy * dy;
            let a = (-d2 * 4.5).exp() * (1.0 - d2.min(1.0));
            let core = (-d2 * 22.0).exp();
            let i = ((y * w + x) * 4) as usize;
            for c in 0..3 {
                rgba[i + c] = (rgb[c] as f32 + (255.0 - rgb[c] as f32) * core * 0.1) as u8;
            }
            rgba[i + 3] = (a.clamp(0.0, 1.0) * 255.0) as u8;
        }
    }
    CartFace { rgba, w, h }
}

/// How far up the screen the glow from the foot reaches, and how strong it is at the foot.
const FOOT_REACH: f32 = 0.8;
const FOOT_PEAK: f32 = 0.225;

/// The glow rising from the foot of the screen: the theme's pale colour, fading upward as one
/// smooth gradient (stretched to the bottom `FOOT_REACH` of the screen).
pub fn xmb_foot_face(rgb: [u8; 3]) -> CartFace {
    let (w, h) = (4u32, 512u32);
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        // Height above the foot, as a fraction of the screen.
        let d = FOOT_REACH * (1.0 - (y as f32 + 0.5) / h as f32);
        // Falls off from the foot and eases to nothing at the top, with no edge.
        let top = 1.0 - (d / FOOT_REACH).powi(4);
        let a = (-d / 0.13).exp() * top;
        for _ in 0..w {
            rgba.extend_from_slice(&[rgb[0], rgb[1], rgb[2], (a * 255.0).round() as u8]);
        }
    }
    CartFace { rgba, w, h }
}

/// A ring, for the loading mark.
pub fn xmb_ring_face() -> CartFace {
    let s = 64u32;
    let mut rgba = vec![0u8; (s * s * 4) as usize];
    for y in 0..s {
        for x in 0..s {
            let dx = (x as f32 + 0.5) / s as f32 * 2.0 - 1.0;
            let dy = (y as f32 + 0.5) / s as f32 * 2.0 - 1.0;
            let r = (dx * dx + dy * dy).sqrt();
            let a = (1.0 - ((r - 0.78).abs() / 0.12)).clamp(0.0, 1.0);
            let i = ((y * s + x) * 4) as usize;
            rgba[i..i + 4].copy_from_slice(&[255, 255, 255, (a.powf(0.8) * 255.0) as u8]);
        }
    }
    CartFace { rgba, w: s, h: s }
}

fn title_font() -> Option<&'static Font> {
    static FONT: OnceLock<Option<Font>> = OnceLock::new();
    FONT.get_or_init(|| Font::from_bytes(TITLE_TTF, FontSettings::default()).ok())
        .as_ref()
}

/// The start-up's lettering, rendered for a screen `canvas` tall, and where it goes: x, y, w, h
/// in whole pixels so it lands on the screen's pixels as it does in the boot picture.
pub fn xmb_title_face(canvas: Canvas) -> (CartFace, [f32; 4]) {
    let empty = (
        CartFace {
            rgba: vec![0; 4],
            w: 1,
            h: 1,
        },
        [0.0, 0.0, 0.0, 0.0],
    );
    let Some(font) = title_font() else {
        return empty;
    };
    let cap = TITLE_CAP * canvas.h;
    let unit = font.metrics('H', 100.0).height as f32 / 100.0;
    if unit <= 0.0 {
        return empty;
    }
    let px = cap / unit;
    // Lay the glyphs out along a baseline at y = 0 and gather their ink.
    let mut glyphs = Vec::new();
    let mut pen = 0.0f32;
    for ch in XMB_INTRO_TITLE.chars() {
        let (m, cov) = font.rasterize(ch, px);
        if ch == ' ' {
            pen += m.advance_width * TITLE_SPACE;
            continue;
        }
        let gx = (pen + m.xmin as f32).round() as i32;
        let gy = -(m.height as i32) - m.ymin;
        glyphs.push((gx, gy, m.width, m.height, cov));
        pen += m.advance_width + TITLE_TRACK * cap;
    }
    let left = glyphs.iter().map(|g| g.0).min().unwrap_or(0);
    let right = glyphs.iter().map(|g| g.0 + g.2 as i32).max().unwrap_or(1);
    let top = glyphs.iter().map(|g| g.1).min().unwrap_or(-1);
    let bottom = glyphs.iter().map(|g| g.1 + g.3 as i32).max().unwrap_or(0);
    // Room for the faint bloom about the letters.
    let pad = 2;
    let (w, h) = ((right - left + 2 * pad) as u32, (bottom - top + 2 * pad) as u32);
    let mut alpha = vec![0u8; (w * h) as usize];
    for (gx, gy, gw, gh, cov) in &glyphs {
        for y in 0..*gh {
            for x in 0..*gw {
                let (dx, dy) = (gx - left + pad + x as i32, gy - top + pad + y as i32);
                let i = (dy as u32 * w + dx as u32) as usize;
                alpha[i] = alpha[i].max(cov[y * gw + x]);
            }
        }
    }
    // The letters glow faintly, as they do on the handheld's screen.
    let bloom = soften(&soften(&alpha, w, h), w, h);
    let mut rgba = Vec::with_capacity(alpha.len() * 4);
    for (a, b) in alpha.iter().zip(&bloom) {
        let a = (*a as f32 + *b as f32 * 0.3).min(255.0) as u8;
        rgba.extend_from_slice(&[TITLE_GREY, TITLE_GREY, TITLE_GREY, a]);
    }
    let x = ((canvas.w - w as f32) / 2.0).round();
    let y = (TITLE_BASE * canvas.h).round() + (top - pad) as f32;
    (CartFace { rgba, w, h }, [x, y, w as f32, h as f32])
}

/// A three-by-three box blur.
fn soften(src: &[u8], w: u32, h: u32) -> Vec<u8> {
    let (w, h) = (w as i32, h as i32);
    let mut out = vec![0u8; src.len()];
    for y in 0..h {
        for x in 0..w {
            let mut sum = 0u32;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let (sx, sy) = ((x + dx).clamp(0, w - 1), (y + dy).clamp(0, h - 1));
                    sum += src[(sy * w + sx) as usize] as u32;
                }
            }
            out[(y * w + x) as usize] = (sum / 9) as u8;
        }
    }
    out
}

/// How strongly the lettering shows at `t`: it is up from the start and fades before the lens.
pub fn xmb_title_alpha(t: f32) -> f32 {
    1.0 - ease(ramp(t, TITLE_FADE, TITLE_FADE_SECS))
}

/// The hairlines at `t` as rectangles: y, height and colour.
fn hairlines(h: f32, t: f32, theme: [f32; 3]) -> Vec<(f32, f32, [f32; 4])> {
    let mut out = Vec::new();
    // Up from the very first frame, as in the boot picture.
    let shown = 1.0 - ease(ramp(t, XMB_INTRO_BLOOM, 0.12));
    if t < 0.0 || shown <= 0.0 {
        return out;
    }
    let (k, cy) = (h / 480.0, h * 0.5);
    let pale = pale(theme);
    let warm = ease(ramp(t, 1.4, 0.9));
    let c = [
        lerp(0.85, pale[0], warm),
        lerp(0.85, pale[1], warm),
        lerp(0.85, pale[2], warm),
    ];
    // (offset from the middle, brightness)
    for (dy, bright) in [(-10.0, 0.15), (-4.8, 0.13), (8.0, 0.16)] {
        let a = shown * bright * lerp(1.0, 2.6, warm);
        let y = cy + dy * k;
        out.push((
            (y - 2.0 * k).round(),
            (4.0 * k).round().max(2.0),
            [c[0], c[1], c[2], (0.45 * a).min(1.0)],
        ));
        out.push((
            (y - 0.75 * k).round(),
            (1.5 * k).round().max(1.0),
            [c[0], c[1], c[2], a.min(1.0)],
        ));
    }
    out
}

/// The picture the device shows from power-on until the animation takes over: its first
/// frame, the hairlines and the lettering on black, `w` by `h` pixels.
pub fn xmb_boot_picture(w: u32, h: u32) -> Vec<[u8; 3]> {
    let mut px = vec![[0.0f32; 3]; (w * h) as usize];
    let mut blend = |x: u32, y: u32, c: [f32; 3], a: f32| {
        let p = &mut px[(y * w + x) as usize];
        for i in 0..3 {
            p[i] = p[i] * (1.0 - a) + c[i] * a;
        }
    };
    for (y0, rh, c) in hairlines(h as f32, 0.0, [1.0; 3]) {
        let (y0, y1) = (y0.max(0.0) as u32, ((y0 + rh) as u32).min(h));
        for y in y0..y1 {
            for x in 0..w {
                blend(x, y, [c[0], c[1], c[2]], c[3]);
            }
        }
    }
    let canvas = Canvas {
        w: w as f32,
        h: h as f32,
    };
    let (face, [fx, fy, _, _]) = xmb_title_face(canvas);
    let a0 = xmb_title_alpha(0.0);
    for y in 0..face.h {
        for x in 0..face.w {
            let (dx, dy) = (fx as i64 + x as i64, fy as i64 + y as i64);
            if dx < 0 || dy < 0 || dx >= w as i64 || dy >= h as i64 {
                continue;
            }
            let i = ((y * face.w + x) * 4) as usize;
            let c = face.rgba[i] as f32 / 255.0;
            let a = face.rgba[i + 3] as f32 / 255.0 * a0;
            blend(dx as u32, dy as u32, [c; 3], a);
        }
    }
    px.iter()
        .map(|p| p.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8))
        .collect()
}

/// The animation at `t` seconds: black, the lines, the glow and the background.
pub struct XmbIntro {
    pub canvas: Canvas,
    pub t: f32,
    /// The background as it will rest; its shape is set here.
    pub backdrop: Option<Backdrop>,
    /// `xmb_flare_face` in the theme's colour, and in white.
    pub flare: Option<TexId>,
    pub glint: Option<TexId>,
    /// The theme's colour.
    pub theme: [f32; 3],
    /// `xmb_title_face` and where it goes.
    pub title: Option<(TexId, [f32; 4])>,
    /// `xmb_foot_face` in the theme's pale colour.
    pub foot: Option<TexId>,
    /// How much of a Modern background shows: 1 as it starts, falling as the bar switches off.
    pub modern: f32,
}

impl XmbIntro {
    pub fn draw(&self, out: &mut Vec<Draw>) {
        let (w, h, t) = (self.canvas.w, self.canvas.h, self.t);
        let k = h / 480.0;
        out.push(Draw::Rect {
            x: 0.0,
            y: 0.0,
            w,
            h,
            colour: [0.0, 0.0, 0.0, 1.0],
        });
        let modern = self.backdrop.is_some_and(|b| b.style == slot_gfx::STYLE_MODERN);
        if modern {
            // The PS3's start draws itself, from black; the scene is told when it began.
            if let (Some(mut b), true) = (self.backdrop, t >= XMB_MODERN_START && self.modern > 0.0) {
                b.alpha = self.modern.clamp(0.0, 1.0);
                out.push(Draw::Backdrop(b));
            }
            let fade = 1.0 - ease(ramp(t, 1.0, 1.3));
            for (y, rh, mut colour) in hairlines(h, t.min(1.4), self.theme) {
                colour[3] *= fade;
                if colour[3] > 0.0 {
                    out.push(Draw::Rect { x: 0.0, y, w, h: rh, colour });
                }
            }
            self.draw_title(out);
            return;
        }
        if let (Some(mut b), Some(shape)) = (self.backdrop, xmb_intro_shape(t)) {
            b.shape = shape;
            out.push(Draw::Backdrop(b));
        }
        // The glow rising from the foot, fading as the picture comes.
        let rise = ease(ramp(t, 0.33, 2.0)) * (1.0 - ease(ramp(t, 2.68, 0.25)));
        if let (Some(tex), true) = (self.foot, rise > 0.0) {
            out.push(Draw::Tex {
                x: 0.0,
                y: h * (1.0 - FOOT_REACH),
                w,
                h: h * FOOT_REACH,
                tex,
                alpha: FOOT_PEAK * rise,
            });
        }
        let cy = h * 0.5;
        // The lens gathering about the lines, swelling as the picture comes.
        let lens = ease(ramp(t, 1.3, 1.15)) * (1.0 - ease(ramp(t, 2.55, 0.12)));
        if lens > 0.0 {
            if let Some(tex) = self.flare {
                let swell = ease(ramp(t, XMB_INTRO_BLOOM - 0.04, 0.12));
                let lh = lerp(lerp(26.0, 70.0, ease(ramp(t, 1.3, 1.15))) * k, h * 1.7, swell);
                let lw = w * lerp(1.3, 2.0, swell);
                out.push(Draw::Tex {
                    x: (w - lw) / 2.0,
                    y: cy - lh / 2.0,
                    w: lw,
                    h: lh,
                    tex,
                    alpha: lens * lerp(0.4, 0.16, swell),
                });
            }
        }
        // The hairlines: dim grey, then brighter and coloured, gone as the lens swells.
        for (y, rh, colour) in hairlines(h, t, self.theme) {
            out.push(Draw::Rect {
                x: 0.0,
                y,
                w,
                h: rh,
                colour,
            });
        }
        self.draw_title(out);
    }

    /// The lettering, over the lines.
    fn draw_title(&self, out: &mut Vec<Draw>) {
        let a = xmb_title_alpha(self.t);
        if let (Some((tex, [x, y, tw, th])), true) = (self.title, a > 0.0) {
            out.push(Draw::Tex {
                x,
                y,
                w: tw,
                h: th,
                tex,
                alpha: a,
            });
        }
    }
}

/// The loading mark: a ring at the lower left, breathing.
pub fn xmb_loading_ring(canvas: Canvas, ring: TexId, time: f32, alpha: f32, out: &mut Vec<Draw>) {
    let k = canvas.h / 480.0;
    let beat = (time * std::f32::consts::TAU / 1.2).sin() * 0.5 + 0.5;
    let size = (26.0 + 6.0 * beat) * k;
    let (cx, cy) = (canvas.w * 0.06 + 14.0 * k, canvas.h * 0.92);
    out.push(Draw::Tex {
        x: cx - size / 2.0,
        y: cy - size / 2.0,
        w: size,
        h: size,
        tex: ring,
        alpha: alpha * (0.45 + 0.5 * beat),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_flood_opens_and_settles() {
        assert!(xmb_intro_shape(1.0).is_none());
        let s = xmb_intro_shape(2.5).unwrap();
        assert!(s[0] < 0.5 && s[1] < 0.4 && s[2] > 0.5);
        let s = xmb_intro_shape(4.5).unwrap();
        assert!(s[0] > 0.6 && (s[1] - 1.0).abs() < 1e-3 && s[2] < 1e-3);
        assert!(xmb_outro_t(1.0) < XMB_INTRO_BLOOM);
    }

    #[test]
    fn the_boot_picture_is_the_first_frame() {
        let (w, h) = (640u32, 480u32);
        let px = xmb_boot_picture(w, h);
        assert_eq!(px.len(), (w * h) as usize);
        assert_eq!(px[0], [0, 0, 0]);
        // A hairline runs right across, under the lettering's capitals.
        let line = (h as f32 * 0.5 - 10.0).round() as u32;
        assert!(px[(line * w) as usize][0] > 10);
        assert!(px[(line * w + w - 1) as usize][0] > 10);
        // The lettering is bright, centred and sits between the lines.
        let canvas = Canvas { w: w as f32, h: h as f32 };
        let (_, [x, y, tw, th]) = xmb_title_face(canvas);
        assert!(((x + tw / 2.0) - w as f32 / 2.0).abs() <= 1.0);
        assert!(y > 220.0 && y + th < 256.0);
        let bright = (y as u32..(y + th) as u32)
            .flat_map(|yy| (x as u32..(x + tw) as u32).map(move |xx| (yy, xx)))
            .filter(|&(yy, xx)| px[(yy * w + xx) as usize][0] > 200)
            .count();
        assert!(bright > 100);
        assert_eq!(xmb_title_alpha(0.0), 1.0);
        assert_eq!(xmb_title_alpha(2.1), 0.0);
    }
}
