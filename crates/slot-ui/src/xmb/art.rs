//! The bar's icons and text, drawn from original shapes and an open-licensed font.

use crate::art::render_svg;
use crate::CartFace;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum XmbIcon {
    Recents,
    Favorites,
    Gba,
    Gbc,
    Gb,
    Settings,
    General,
    Theme,
    Screen,
    Gameplay,
    Controls,
    About,
    Resume,
    Undo,
    States,
    Screenshots,
    Exit,
    Up,
    Down,
    /// The round badge every line of a settings list carries.
    Row,
    /// The small marker beside the item a list was opened from.
    Back,
    /// The status bar's battery, with no bars to three.
    Battery0,
    Battery1,
    Battery2,
    Battery3,
}

pub const XMB_ICONS: usize = 25;

impl XmbIcon {
    pub const ALL: [XmbIcon; XMB_ICONS] = [
        XmbIcon::Recents,
        XmbIcon::Favorites,
        XmbIcon::Gba,
        XmbIcon::Gbc,
        XmbIcon::Gb,
        XmbIcon::Settings,
        XmbIcon::General,
        XmbIcon::Theme,
        XmbIcon::Screen,
        XmbIcon::Gameplay,
        XmbIcon::Controls,
        XmbIcon::About,
        XmbIcon::Resume,
        XmbIcon::Undo,
        XmbIcon::States,
        XmbIcon::Screenshots,
        XmbIcon::Exit,
        XmbIcon::Up,
        XmbIcon::Down,
        XmbIcon::Row,
        XmbIcon::Back,
        XmbIcon::Battery0,
        XmbIcon::Battery1,
        XmbIcon::Battery2,
        XmbIcon::Battery3,
    ];

    pub fn index(self) -> usize {
        self as usize
    }
}

/// Icons are rasterised at this size and scaled down when drawn.
const XMB_ICON_PX: u32 = 128;

fn face(rgba: Option<Vec<u8>>, w: u32, h: u32) -> CartFace {
    CartFace {
        rgba: rgba.unwrap_or_else(|| vec![0u8; (w * h * 4) as usize]),
        w,
        h,
    }
}

/// A wrench lying corner to corner across a 96-unit box: a ring head with an open jaw at the
/// upper right and a rounded handle down to the lower left.
fn wrench(cx: f32, cy: f32) -> String {
    // Built pointing straight up, then turned 45 degrees.
    let pts_handle = [(-5.0, -2.5), (5.0, -2.5), (5.0, 24.0), (-5.0, 24.0)];
    let turn = |(x, y): (f32, f32)| {
        let a = std::f32::consts::FRAC_PI_4;
        (cx + x * a.cos() - y * a.sin(), cy + x * a.sin() + y * a.cos())
    };
    let handle: Vec<(f32, f32)> = pts_handle.iter().map(|p| turn(*p)).collect();
    let (hx, hy) = turn((0.0, -14.0));
    let jaw: Vec<(f32, f32)> = [(-4.0, -30.0), (4.0, -30.0), (4.0, -16.0), (-4.0, -16.0)]
        .iter()
        .map(|p| turn(*p))
        .collect();
    // The jaw is drawn as part of the cut-out, so it reads as the gap between the ring's ends.
    format!(
        "{}{}{}",
        poly(&handle),
        circle(hx, hy, 12.0),
        poly(&jaw)
    )
}

fn rrect(x: f32, y: f32, w: f32, h: f32, r: f32) -> String {
    format!(
        "M{} {y} H{} A{r} {r} 0 0 1 {} {} V{} A{r} {r} 0 0 1 {} {} H{} A{r} {r} 0 0 1 {x} {} V{} A{r} {r} 0 0 1 {} {y} Z ",
        x + r,
        x + w - r,
        x + w,
        y + r,
        y + h - r,
        x + w - r,
        y + h,
        x + r,
        y + h - r,
        y + r,
        x + r,
    )
}

fn circle(cx: f32, cy: f32, r: f32) -> String {
    format!(
        "M{} {cy} A{r} {r} 0 1 0 {} {cy} A{r} {r} 0 1 0 {} {cy} Z ",
        cx - r,
        cx + r,
        cx - r
    )
}

fn plus(cx: f32, cy: f32, len: f32, t: f32) -> String {
    let (a, b) = (len / 2.0, t / 2.0);
    format!(
        "M{} {} H{} V{} H{} V{} H{} V{} H{} V{} H{} V{} H{} Z ",
        cx - b,
        cy - a,
        cx + b,
        cy - b,
        cx + a,
        cy + b,
        cx + b,
        cy + a,
        cx - b,
        cy + b,
        cx - a,
        cy - b,
        cx - b
    )
}

fn star(cx: f32, cy: f32, outer: f32, inner: f32) -> String {
    let mut d = String::new();
    for k in 0..10 {
        let r = if k % 2 == 0 { outer } else { inner };
        let a = (-90.0 + 36.0 * k as f32).to_radians();
        let (x, y) = (cx + r * a.cos(), cy + r * a.sin());
        d.push_str(&format!("{}{x:.2} {y:.2} ", if k == 0 { "M" } else { "L" }));
    }
    d.push('Z');
    d
}

/// A cog: `teeth` square teeth on a ring with a hole.
#[allow(dead_code)]
fn cog(cx: f32, cy: f32, outer: f32, inner: f32, hole: f32, teeth: u32) -> String {
    let mut d = String::new();
    let n = teeth * 4;
    for k in 0..n {
        let r = if (k / 2) % 2 == 0 { outer } else { inner };
        let a = std::f32::consts::TAU * (k as f32 + 0.5) / n as f32;
        let (x, y) = (cx + r * a.cos(), cy + r * a.sin());
        d.push_str(&format!("{}{x:.2} {y:.2} ", if k == 0 { "M" } else { "L" }));
    }
    d.push_str("Z ");
    d.push_str(&circle(cx, cy, hole));
    d
}

fn poly(points: &[(f32, f32)]) -> String {
    let mut d = String::new();
    for (i, (x, y)) in points.iter().enumerate() {
        d.push_str(&format!("{}{x} {y} ", if i == 0 { "M" } else { "L" }));
    }
    d.push_str("Z ");
    d
}

/// The filled shape of each icon, holes cut with even-odd filling, and any stroked detail.
fn icon_shape(icon: XmbIcon) -> (String, &'static str) {
    const CLOCK_HANDS: &str = "M48 26 V48 L62 58";
    const UNDO_ARROW: &str = "M30 40 A24 24 0 1 1 30 66";
    match icon {
        XmbIcon::Recents => (
            format!("{}{}", circle(48.0, 48.0, 36.0), circle(48.0, 48.0, 29.0)),
            CLOCK_HANDS,
        ),
        XmbIcon::Favorites => (star(48.0, 51.0, 40.0, 17.0), ""),
        XmbIcon::Gba => (
            // A generic landscape handheld: a wide screen between a d-pad and a diamond of four
            // face buttons, with two small keys beneath the screen.
            format!(
                "{}{}{}{}{}{}{}{}{}",
                rrect(5.0, 24.0, 86.0, 48.0, 14.0),
                rrect(29.0, 30.0, 38.0, 28.0, 3.0),
                plus(17.0, 46.0, 15.0, 5.4),
                circle(78.5, 40.7, 2.8),
                circle(83.8, 46.0, 2.8),
                circle(78.5, 51.3, 2.8),
                circle(73.2, 46.0, 2.8),
                rrect(38.5, 62.5, 7.5, 3.2, 1.6),
                rrect(50.0, 62.5, 7.5, 3.2, 1.6)
            ),
            "",
        ),
        XmbIcon::Gb => (
            format!(
                "M28 8 H68 A6 6 0 0 1 74 14 V68 A20 20 0 0 1 54 88 H28 A6 6 0 0 1 22 82 V14 A6 6 0 0 1 28 8 Z {}{}{}{}{}{}",
                rrect(28.5, 15.0, 39.0, 31.0, 3.0),
                plus(35.0, 62.0, 14.0, 4.6),
                circle(57.0, 65.0, 4.0),
                circle(66.0, 59.0, 4.0),
                rrect(39.0, 77.0, 7.0, 2.6, 1.3),
                rrect(48.5, 77.0, 7.0, 2.6, 1.3)
            ),
            "",
        ),
        XmbIcon::Gbc => (
            format!(
                "{}{}{}{}{}",
                rrect(24.0, 6.0, 48.0, 84.0, 11.0),
                rrect(30.0, 14.0, 36.0, 30.0, 6.0),
                plus(36.5, 60.0, 13.5, 4.6),
                circle(57.5, 63.0, 4.0),
                circle(65.5, 56.0, 4.0)
            ),
            "",
        ),
        XmbIcon::Settings => (
            // A toolbox: handle, lid seam and latch.
            format!(
                "M34 14 H62 A6 6 0 0 1 68 20 V28 H62 V21 H34 V28 H28 V20 A6 6 0 0 1 34 14 Z {}{}{}",
                rrect(8.0, 28.0, 80.0, 22.0, 6.0),
                rrect(8.0, 54.0, 80.0, 30.0, 6.0),
                rrect(42.0, 46.0, 12.0, 12.0, 2.0)
            ),
            "",
        ),
        XmbIcon::Row => (
            // A disc with a wrench cut out of it, laid corner to corner.
            format!("{}{}", circle(48.0, 48.0, 40.0), wrench(48.0, 48.0)),
            "",
        ),
        XmbIcon::Back => (poly(&[(70.0, 20.0), (70.0, 76.0), (26.0, 48.0)]), ""),
        XmbIcon::General => (
            // Three sliders.
            // Each track stops short of its knob so the shapes never overlap.
            [(22.0, 28.0), (45.0, 56.0), (68.0, 36.0)]
                .iter()
                .map(|&(y, knob)| {
                    format!(
                        "{}{}{}",
                        rrect(10.0, y, knob - 13.0, 6.0, 3.0),
                        rrect(knob + 15.0, y, 86.0 - knob - 15.0, 6.0, 3.0),
                        rrect(knob - 6.0, y - 8.0, 12.0, 22.0, 4.0)
                    )
                })
                .collect::<String>(),
            "",
        ),
        XmbIcon::Theme => (
            // A painter's palette.
            format!(
                "M48 10 C24 10 8 28 8 48 C8 70 26 86 46 86 C54 86 56 80 52 74 C48 68 52 62 60 62 H70 C80 62 88 56 88 44 C88 24 70 10 48 10 Z {}{}{}{}",
                circle(30.0, 40.0, 7.0),
                circle(46.0, 26.0, 7.0),
                circle(66.0, 30.0, 7.0),
                circle(28.0, 62.0, 7.0)
            ),
            "",
        ),
        XmbIcon::Screen => (
            format!(
                "{}{}{}",
                rrect(8.0, 16.0, 80.0, 54.0, 6.0),
                rrect(15.0, 23.0, 66.0, 40.0, 2.0),
                poly(&[(36.0, 70.0), (60.0, 70.0), (66.0, 82.0), (30.0, 82.0)])
            ),
            "",
        ),
        XmbIcon::Gameplay => (
            // A controller.
            format!(
                "M22 28 H74 C86 28 92 40 92 54 C92 70 84 78 76 78 C68 78 64 68 58 66 H38 C32 68 28 78 20 78 C12 78 4 70 4 54 C4 40 10 28 22 28 Z {}{}{}",
                plus(26.0, 50.0, 18.0, 6.0),
                circle(68.0, 44.0, 5.0),
                circle(78.0, 54.0, 5.0)
            ),
            "",
        ),
        XmbIcon::Controls => (
            // A d-pad and two buttons, bare.
            format!(
                "{}{}{}",
                plus(30.0, 48.0, 40.0, 13.0),
                circle(66.0, 40.0, 9.0),
                circle(80.0, 58.0, 9.0)
            ),
            "",
        ),
        XmbIcon::About => (
            format!(
                "{}{}{}{}",
                circle(48.0, 48.0, 38.0),
                circle(48.0, 48.0, 31.0),
                circle(48.0, 30.0, 5.0),
                rrect(43.5, 41.0, 9.0, 30.0, 3.0)
            ),
            "",
        ),
        XmbIcon::Resume => (poly(&[(28.0, 16.0), (80.0, 48.0), (28.0, 80.0)]), ""),
        XmbIcon::Undo => (
            poly(&[(14.0, 30.0), (40.0, 22.0), (34.0, 50.0)]),
            UNDO_ARROW,
        ),
        XmbIcon::States => (
            // A stack of cards.
            format!(
                "{}{}{}",
                rrect(20.0, 12.0, 56.0, 12.0, 3.0),
                rrect(14.0, 28.0, 68.0, 12.0, 3.0),
                rrect(8.0, 44.0, 80.0, 40.0, 4.0)
            ),
            "",
        ),
        XmbIcon::Screenshots => (
            format!(
                "M10 30 H30 L36 20 H60 L66 30 H86 V78 H10 Z {}",
                circle(48.0, 52.0, 15.0)
            ),
            "",
        ),
        XmbIcon::Exit => (
            format!(
                "{}{}",
                poly(&[(48.0, 14.0), (84.0, 54.0), (12.0, 54.0)]),
                rrect(12.0, 64.0, 72.0, 12.0, 3.0)
            ),
            "",
        ),
        XmbIcon::Up => (poly(&[(48.0, 22.0), (82.0, 72.0), (14.0, 72.0)]), ""),
        XmbIcon::Down => (poly(&[(14.0, 24.0), (82.0, 24.0), (48.0, 74.0)]), ""),
        XmbIcon::Battery0 | XmbIcon::Battery1 | XmbIcon::Battery2 | XmbIcon::Battery3 => {
            // A cell lying on its side, its terminal to the left; bars fill from the right.
            let bars = match icon {
                XmbIcon::Battery1 => 1,
                XmbIcon::Battery2 => 2,
                XmbIcon::Battery3 => 3,
                _ => 0,
            };
            let mut d = format!(
                "{}{}{}",
                rrect(16.0, 28.0, 76.0, 40.0, 7.0),
                rrect(22.0, 34.0, 64.0, 28.0, 3.0),
                rrect(6.0, 39.0, 12.0, 18.0, 3.0)
            );
            for k in 0..bars {
                let x = 66.0 - k as f32 * 20.0;
                d.push_str(&rrect(x, 38.0, 16.0, 20.0, 2.0));
            }
            (d, "")
        }
    }
}

/// Icons are drawn at twice their texture size and averaged down, so their edges stay smooth
/// when the bar scales them.
const SUPERSAMPLE: u32 = 2;

fn icon_svg(icon: XmbIcon) -> String {
    let (shape, stroke) = icon_shape(icon);
    let lines = match stroke {
        "" => String::new(),
        d => format!(
            r##"<path d="{d}" fill="none" stroke="url(#g)" stroke-width="7" stroke-linecap="round" stroke-linejoin="round"/>"##
        ),
    };
    // White lit from above, falling to a pale grey, as the handheld's icons are.
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="96" height="96" viewBox="0 0 96 96">
<defs><linearGradient id="g" x1="0" y1="0" x2="0" y2="96" gradientUnits="userSpaceOnUse">
<stop offset="0" stop-color="#ffffff"/><stop offset="0.55" stop-color="#f6f6f6"/><stop offset="1" stop-color="#d9d9d9"/>
</linearGradient></defs>
<path d="{shape}" fill="url(#g)" fill-rule="evenodd"/>{lines}
</svg>"##
    )
}

/// Averages `k` by `k` blocks of an RGBA image, weighting colour by coverage.
fn downsample(rgba: &[u8], w: u32, h: u32, k: u32) -> Vec<u8> {
    let (ow, oh) = (w / k, h / k);
    let mut out = vec![0u8; (ow * oh * 4) as usize];
    for y in 0..oh {
        for x in 0..ow {
            let mut acc = [0f32; 4];
            for dy in 0..k {
                for dx in 0..k {
                    let i = (((y * k + dy) * w + x * k + dx) * 4) as usize;
                    let a = rgba[i + 3] as f32;
                    for c in 0..3 {
                        acc[c] += rgba[i + c] as f32 * a;
                    }
                    acc[3] += a;
                }
            }
            let o = ((y * ow + x) * 4) as usize;
            let a = acc[3] / (k * k) as f32;
            for c in 0..3 {
                out[o + c] = if acc[3] > 0.0 { (acc[c] / acc[3]).round() as u8 } else { 255 };
            }
            out[o + 3] = a.round() as u8;
        }
    }
    out
}

/// A soft shadow under the shape: its coverage blurred, dropped a little, and laid beneath.
fn with_shadow(rgba: &[u8], s: u32) -> Vec<u8> {
    let n = s as usize;
    let alpha: Vec<f32> = rgba.chunks(4).map(|p| p[3] as f32 / 255.0).collect();
    let r = (s / 40).max(1) as isize;
    let blur = |src: &[f32], horizontal: bool| -> Vec<f32> {
        let mut dst = vec![0f32; n * n];
        for y in 0..n as isize {
            for x in 0..n as isize {
                let mut sum = 0.0;
                for d in -r..=r {
                    let (sx, sy) = if horizontal { (x + d, y) } else { (x, y + d) };
                    if sx >= 0 && sy >= 0 && (sx as usize) < n && (sy as usize) < n {
                        sum += src[sy as usize * n + sx as usize];
                    }
                }
                dst[y as usize * n + x as usize] = sum / (2 * r + 1) as f32;
            }
        }
        dst
    };
    let soft = blur(&blur(&blur(&blur(&alpha, true), false), true), false);
    let drop = (s / 48) as usize;
    let mut out = rgba.to_vec();
    for y in 0..n {
        for x in 0..n {
            let i = y * n + x;
            let sh = if y >= drop { soft[(y - drop) * n + x] * 0.32 } else { 0.0 };
            let fa = alpha[i];
            let a = fa + sh * (1.0 - fa);
            if a <= 0.0 {
                continue;
            }
            for c in 0..3 {
                // The shadow is black: the shape's colour weighted by its share of the cover.
                out[i * 4 + c] = (rgba[i * 4 + c] as f32 * fa / a).round() as u8;
            }
            out[i * 4 + 3] = (a * 255.0).round() as u8;
        }
    }
    out
}

pub fn xmb_icon_face(icon: XmbIcon) -> CartFace {
    let s = XMB_ICON_PX;
    let big = s * SUPERSAMPLE;
    let Some(rgba) = render_svg(&icon_svg(icon), big, big) else {
        return face(None, s, s);
    };
    let small = downsample(&rgba, big, big, SUPERSAMPLE);
    face(Some(with_shadow(&small, s)), s, s)
}


/// A soft white glow that fades to nothing at its edge, for behind a highlighted icon.
/// A halo hugging a rectangle, for the highlighted game: drawn in nine slices so it keeps its
/// width along any size of cover. The inner `FRAME_INSET` texels each side are the falloff.
pub const FRAME_INSET: u32 = 24;

pub fn xmb_frame_glow_face() -> CartFace {
    let s = 64u32;
    let edge = FRAME_INSET as f32;
    let mut rgba = vec![0u8; (s * s * 4) as usize];
    for y in 0..s {
        for x in 0..s {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let ox = (edge - px).max(px - (s as f32 - edge)).max(0.0);
            let oy = (edge - py).max(py - (s as f32 - edge)).max(0.0);
            let d = (ox * ox + oy * oy).sqrt() / edge;
            let a = (1.0 - d).clamp(0.0, 1.0).powf(1.6);
            let i = ((y * s + x) * 4) as usize;
            rgba[i..i + 4].copy_from_slice(&[255, 255, 255, (a * 255.0) as u8]);
        }
    }
    CartFace { rgba, w: s, h: s }
}

pub fn xmb_glow_face() -> CartFace {
    let s = 64u32;
    let mut rgba = vec![0u8; (s * s * 4) as usize];
    for y in 0..s {
        for x in 0..s {
            let (dx, dy) = ((x as f32 + 0.5) / s as f32 - 0.5, (y as f32 + 0.5) / s as f32 - 0.5);
            // A rounded square falloff, so a wide cover glows evenly along its sides.
            let d = (dx.abs().powf(3.0) + dy.abs().powf(3.0)).powf(1.0 / 3.0) * 2.0;
            let a = (1.0 - d).clamp(0.0, 1.0);
            let a = a * a * (3.0 - 2.0 * a);
            let i = ((y * s + x) * 4) as usize;
            rgba[i..i + 4].copy_from_slice(&[255, 255, 255, (a * 255.0) as u8]);
        }
    }
    CartFace { rgba, w: s, h: s }
}
