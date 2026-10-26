//! The Modern background: the PS3's wave, its sparkles and its backdrop, run on the CPU and handed
//! to `slot_gfx`'s Modern pass a frame at a time. Everything the console's code decides - the
//! lines' springs, the lattice, the surface, the sparkles' task and emitter, the day's parameter
//! sets and the XMB's start - is ported from faquino's reverse engineering of the firmware
//! (github.com/faquino/PlayStation-3-XMB) by way of Nagare's Rust port
//! (github.com/bungholesurfers/Nagare), both MIT, see `LICENSE-MIT`.

// The ports keep their sources' shape, line for line where they can, so that they can be read
// against them.
#[allow(clippy::excessive_precision)]
pub mod months;
#[allow(
    clippy::neg_cmp_op_on_partial_ord,
    clippy::needless_range_loop,
    clippy::too_many_arguments,
    clippy::new_without_default
)]
pub mod particles;
pub mod scene;
pub mod settings;
#[allow(clippy::approx_constant, clippy::needless_range_loop, clippy::new_without_default)]
pub mod wave;

use slot_gfx::{
    ModernBackdrop, ModernFrame, ModernPost, ModernSparkles, ModernWave, MODERN_BACK_H, MODERN_BACK_W,
    MODERN_IRIDESCENT, MODERN_RECORD, MODERN_SPARKLES, MODERN_SPARKLE_VERTEX, MODERN_VERTICES,
};

use months::{month_texels, DAY, NIGHT};
use particles::{SimOptions, System, LIFE_MAX, LIFE_MIN, OUT_STRIDE};
use scene::{Scene, BOOT_EASE, BOOT_HOUR, TICK};
use settings::{ParticleSettings, WaveSettings};
use wave::{Wave, CAMERA};

/// The sparkles' thin-film colours, a fit of the firmware's proc_iridescent.tga
/// (faquino's `particles-settings.js`).
const IRIDESCENT_PALETTE: [u32; 16] = [
    0xf7f9bd, 0xf3dc60, 0xdc993c, 0xc06a49, 0xd4488a, 0xce34be, 0x964cde, 0x638af4, 0x357ce9, 0x496de7,
    0x6d84ec, 0xa482f5, 0xda85d8, 0xc58b73, 0xb6a856, 0xb3bba0,
];

/// The lines the console's output has: on a standard-definition one, as the handheld's 480 lines
/// are, the sparkles are drawn larger and fainter (PARTICLES_SPE.mnu).
const VIDEO_LINES: i32 = 480;

static TEXTURES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static MOMENTS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// The months' lengths as the scene keeps them: February has 28 days in every year.
const MONTH_DAYS: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

/// What the backdrop shows: the console's own month textures, by the clock, or one picture.
#[derive(Clone, Debug, PartialEq)]
pub enum ModernColour {
    Months,
    /// `MODERN_BACK_W` by `MODERN_BACK_H` RGBA, its first row at the bottom of the screen. Like
    /// the console's Theme Settings colour, it holds the scene at noon.
    Picture(Vec<u8>),
}

/// A moment handed to the backdrop: the month (0 to 11), its day, the hour, and how long it eases in.
#[derive(Clone, Copy, Debug)]
struct Moment {
    month: usize,
    day: u32,
    hour: f64,
    seconds: f64,
}

/// The wall clock as the scene reads it.
#[derive(Clone, Copy, Debug)]
pub struct Wall {
    pub month: usize,
    pub day: u32,
    pub hour: f64,
}

impl Wall {
    /// From seconds since 1970 on the local clock.
    pub fn of(secs: f64) -> Wall {
        let days = (secs / 86_400.0).floor();
        let hour = (secs - days * 86_400.0) / 3600.0;
        let (_, m, d) = crate::xmb::civil(secs.floor() as i64);
        Wall {
            month: (m as usize).clamp(1, 12) - 1,
            day: d,
            hour,
        }
    }
}

pub struct ModernScene {
    wave: Wave,
    sparkles: System,
    scene: Scene,
    ws: WaveSettings,
    ps: ParticleSettings,
    /// Seconds since the scene began.
    time: f64,
    colour: ModernColour,
    /// The backdrop: the moment it was last handed, a count of the hand-overs, the time into its
    /// ease, and the clock's last tick.
    moment: Moment,
    serial: u64,
    ease: f64,
    tick: Option<i64>,
    boot_handed: bool,
    /// The textures for the backdrop and a count of their changes.
    textures: u64,
    months_for: Option<(usize, usize)>,
    /// The scene's brightness: where it stands and where it is going, per second.
    fade: f64,
    fade_to: f64,
    fade_rate: f64,
    aspect: f64,
    frame: ModernFrame,
}

impl ModernScene {
    /// A scene already running, as if the bar had been up for a while.
    pub fn new(seed: Option<i32>) -> Self {
        let mut wave = Wave::new();
        // Fresh from its start the lines sit in the console's baked state; a few seconds on, they
        // move as the scene's do at rest.
        let ws = WaveSettings::default();
        for _ in 0..240 {
            wave.update(&ws, 1.0 / 60.0, 4.0 / 3.0, false);
        }
        let mut frame = ModernFrame {
            mesh: vec![0.0; MODERN_VERTICES * MODERN_RECORD],
            sparkles: vec![0.0; MODERN_SPARKLES * 4 * MODERN_SPARKLE_VERTEX],
            iridescent: iridescent_texels(&IRIDESCENT_PALETTE),
            fade: 1.0,
            ..ModernFrame::default()
        };
        for m in frame.months.iter_mut() {
            *m = vec![0; MODERN_BACK_W * MODERN_BACK_H * 4];
        }
        ModernScene {
            wave,
            sparkles: System::new(MODERN_SPARKLES, seed),
            scene: Scene::new(),
            ws,
            ps: ParticleSettings::default(),
            time: 0.0,
            colour: ModernColour::Months,
            moment: Moment {
                month: 0,
                day: 1,
                hour: 12.0,
                seconds: 0.0,
            },
            serial: 0,
            ease: 0.0,
            tick: None,
            boot_handed: false,
            textures: 0,
            months_for: None,
            fade: 1.0,
            fade_to: 1.0,
            fade_rate: 0.0,
            aspect: 4.0 / 3.0,
            frame,
        }
    }

    /// Plays the XMB's start from here: black, the lines starting afresh, the wave rising out of
    /// the dark and the backdrop easing in at ten in the morning.
    pub fn boot(&mut self) {
        self.scene.boot(self.time);
        self.wave.reset();
        self.boot_handed = false;
    }

    pub fn booting(&self) -> bool {
        self.scene.booting()
    }

    /// Seconds since the scene began.
    pub fn time(&self) -> f64 {
        self.time
    }

    /// What the backdrop shows. A change comes in over the clock's next second.
    pub fn set_colour(&mut self, colour: ModernColour) {
        if colour != self.colour {
            self.colour = colour;
            self.months_for = None;
        }
    }

    /// Fades the whole scene towards `to`, 0 for black, over `seconds`.
    pub fn fade_to(&mut self, to: f64, seconds: f64) {
        self.fade_to = to.clamp(0.0, 1.0);
        self.fade_rate = if seconds > 0.0 { 1.0 / seconds } else { f64::INFINITY };
    }

    /// Moves the scene on by `dt` seconds at `wall`, the local clock. `steps` are the bar's moves
    /// since the last frame (0 left, 1 right, 2 up, 3 down) and `held` the direction held, which
    /// stir the sparkles as the XMB's icons do. `aspect` is the screen's width over its height.
    pub fn advance(&mut self, dt: f64, wall: Wall, steps: &[usize], held: Option<usize>, aspect: f64) {
        let dt = dt.clamp(0.0, 0.1);
        self.time += dt;
        self.aspect = aspect;
        let noon = matches!(self.colour, ModernColour::Picture(_));
        let hour = if noon { 12.0 } else { wall.hour };
        self.scene.apply(self.time, hour, &mut self.ps, &mut self.ws);
        self.hand_backdrop(wall, noon);
        self.wave.update(&self.ws, dt, aspect, true);
        self.sparkles.step_input(steps, held);
        let opts = SimOptions {
            video_height: VIDEO_LINES,
            vertical_turn: false,
        };
        self.sparkles
            .update(&self.ps, &opts, if self.scene.booting() { "coldboot" } else { "none" }, &mut self.wave, aspect, self.time, dt, None);
        if self.fade != self.fade_to {
            let step = self.fade_rate * dt;
            self.fade = if self.fade < self.fade_to {
                (self.fade + step).min(self.fade_to)
            } else {
                (self.fade - step).max(self.fade_to)
            };
        }
        if self.ease < 1.5 * self.moment.seconds {
            self.ease += dt;
        }
    }

    /// The moment the backdrop is handed: the clock's, a second at a time while nothing holds the
    /// clock, or ten in the morning as the XMB's start begins.
    fn hand_backdrop(&mut self, wall: Wall, noon: bool) {
        let tick = self.time.floor() as i64;
        let shown = Moment {
            month: wall.month,
            day: wall.day,
            hour: if noon { 12.0 } else { wall.hour },
            seconds: TICK,
        };
        let hand = |m: Moment, me: &mut Self| {
            me.moment = m;
            me.serial = MOMENTS.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            me.ease = 0.0;
        };
        if self.scene.booting() && !self.boot_handed {
            self.boot_handed = true;
            hand(
                Moment {
                    hour: BOOT_HOUR,
                    seconds: BOOT_EASE,
                    ..shown
                },
                self,
            );
        } else if self.tick.is_none() {
            hand(Moment { seconds: 0.0, ..shown }, self);
        } else if Some(tick) != self.tick && !self.scene.clock_held(self.time) {
            hand(shown, self);
        }
        self.tick = Some(tick);
    }

    /// The frame `frame` last built.
    pub fn last_frame(&self) -> &ModernFrame {
        &self.frame
    }

    /// The frame to draw, built from where the scene stands.
    pub fn frame(&mut self) -> &ModernFrame {
        let f = &mut self.frame;
        f.mesh.copy_from_slice(&self.wave.mesh);
        let s = &self.ws;
        f.wave = ModernWave {
            mipmap_bias: s.mipmap_bias as f32,
            brightness: s.brightness as f32,
            fresnel: s.fresnel as f32,
            spacing: s.spacing as f32,
            thinness: s.thinness as f32,
        };
        f.post = post_uniforms(s, 1.0);
        f.fade = self.fade as f32;
        // The backdrop.
        let m = self.moment;
        let want = match &self.colour {
            ModernColour::Months => (m.month, (m.month + 1) % 12),
            ModernColour::Picture(_) => (12, 12),
        };
        if self.months_for != Some(want) {
            self.months_for = Some(want);
            // Counted across every scene, so that a new one's textures are never taken for old ones.
            self.textures = TEXTURES.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            match &self.colour {
                ModernColour::Months => {
                    f.months[0] = month_texels(&DAY[want.0]);
                    f.months[1] = month_texels(&DAY[want.1]);
                    f.months[2] = month_texels(&NIGHT[want.0]);
                    f.months[3] = month_texels(&NIGHT[want.1]);
                }
                ModernColour::Picture(p) => {
                    for t in f.months.iter_mut() {
                        t.clone_from(p);
                    }
                }
            }
        }
        f.backdrop = backdrop_uniforms(s, &m, self.ease, self.serial, self.textures);
        // The sparkles: each a quad of four corners.
        let n = self.sparkles.count.min(MODERN_SPARKLES);
        let out = &self.sparkles.output;
        for i in 0..n {
            let rec = &out[i * OUT_STRIDE..i * OUT_STRIDE + 8];
            for (k, corner) in [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]].iter().enumerate() {
                let o = (4 * i + k) * MODERN_SPARKLE_VERTEX;
                f.sparkles[o] = corner[0];
                f.sparkles[o + 1] = corner[1];
                f.sparkles[o + 2..o + 10].copy_from_slice(rec);
            }
        }
        f.sparkle_count = n;
        f.particles = sparkle_uniforms(&self.sparkles.effective, self.aspect, self.sparkles.color_fade.value);
        &self.frame
    }
}

fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    if x < edge0 {
        return 0.0;
    }
    if x > edge1 {
        return 1.0;
    }
    let u = (x - edge0) / (edge1 - edge0);
    u * u * (3.0 - 2.0 * u)
}

/// What the scene hands the backdrop's program (custom_render_plugin's 0x52ad8): _DayTime 2400
/// times a cubic of the time of day whose slope at both ends is DAYSPREAD, _NightTime 2400 times
/// the time since noon, _NightDayBlend from NIGHT BLEND and the day's two smoothsteps, and the
/// month's position, 0 to 30.
fn backdrop_uniforms(s: &WaveSettings, m: &Moment, ease: f64, serial: u64, textures: u64) -> ModernBackdrop {
    let t = m.hour / 24.0;
    let sp = s.dayspread.clamp(1.0, 3.0);
    let hermite = -2.0 * t * t * t + 3.0 * t * t + sp * (2.0 * t * t * t - 3.0 * t * t + t);
    let up = smoothstep(s.night2day_begin / 24.0, s.night2day_end / 24.0, t);
    let down = smoothstep(s.day2night_begin / 24.0, s.day2night_end / 24.0, t);
    let days = MONTH_DAYS[m.month.min(11)];
    let day = m.day.clamp(1, days);
    let shader = if s.colour_shader.round() as i32 == 1 { 1 } else { 0 };
    let alpha = if m.seconds <= 0.0 || ease >= m.seconds {
        1.0
    } else {
        smoothstep(0.0, m.seconds, ease)
    };
    ModernBackdrop {
        day_time: if shader == 1 {
            (t * 1000.0 / 24.0) as f32
        } else {
            (2400.0 * hermite) as f32
        },
        night_time: (2400.0 * ((t + 0.5) % 1.0)) as f32,
        month_time: (30.0 * (day as f64 - 1.0) / days as f64) as f32,
        night_brightness: s.night_whit_bias as f32,
        night_day_blend: (1.0 - s.night_blend * (1.0 - up * (1.0 - down))) as f32,
        shader,
        alpha: alpha as f32,
        serial,
        textures,
    }
}

/// HDR.mnu's and BACKGROUND.mnu's settings as the passes after the wave take them: the tone
/// curve's white, each channel's Gaussian normalised over |k| <= floor(3 RAD - 1), the six
/// levels' weights GLARE LEVEL² SUM POW^l / (1 + ... + SUM POW^5), and the four colours.
#[allow(clippy::needless_range_loop)]
fn post_uniforms(s: &WaveSettings, gamma: f64) -> ModernPost {
    let mut gauss = [0.0f32; 24];
    for (channel, sigma) in [s.gaussian_rad_r, s.gaussian_rad_g, s.gaussian_rad_b].into_iter().enumerate() {
        let reach = if sigma > 0.0 {
            (3.0 * sigma - 1.0).floor().clamp(0.0, 7.0) as usize
        } else {
            0
        };
        let mut w = [0.0f64; 8];
        let mut total = 0.0;
        for k in 0..8 {
            w[k] = if k <= reach && sigma > 0.0 {
                (-((k * k) as f64) / (2.0 * sigma * sigma)).exp()
            } else if k == 0 {
                1.0
            } else {
                0.0
            };
            total += if k == 0 { w[k] } else { 2.0 * w[k] };
        }
        for k in 0..8 {
            gauss[3 * k + channel] = (w[k] / total) as f32;
        }
    }
    let pow = s.glare_sum_pow;
    let sum: f64 = (0..6).map(|l| pow.powi(l)).sum();
    let finest = s.glare_level * s.glare_level / sum;
    let mut levels = [0.0f32; 6];
    for (l, v) in levels.iter_mut().enumerate() {
        *v = (finest * pow.powi(l as i32)) as f32;
    }
    let c = |r: f64, g: f64, b: f64| [r as f32, g as f32, b as f32];
    ModernPost {
        exposure: s.exposure as f32,
        white_sqr_rcp: if s.white_level != 0.0 {
            (1.0 / (s.white_level * s.white_level)) as f32
        } else {
            0.0
        },
        threshold: s.glare_thresh as f32,
        gamma: gamma as f32,
        gauss,
        levels,
        glare_weight: 1.0,
        colours: [
            c(s.colour1_red, s.colour1_green, s.colour1_blue),
            c(s.colour2_red, s.colour2_green, s.colour2_blue),
            c(s.colour3_red, s.colour3_green, s.colour3_blue),
            c(s.colour4_red, s.colour4_green, s.colour4_blue),
        ],
    }
}

/// The sparkles' uniforms, and the camera's matrix: the eye at (0, 0, 2) looking down -z.
fn sparkle_uniforms(s: &ParticleSettings, aspect: f64, colour: f64) -> ModernSparkles {
    let f = 1.0 / (CAMERA.fovy / 2.0).tan();
    let (near, far) = (CAMERA.near, CAMERA.far);
    let mut mvp = [0.0f32; 16];
    // proj * view, view a move by -eye.
    let p22 = (far + near) / (near - far);
    let p32 = 2.0 * far * near / (near - far);
    let eye = CAMERA.eye;
    mvp[0] = (f / aspect) as f32;
    mvp[5] = f as f32;
    mvp[10] = p22 as f32;
    mvp[11] = -1.0;
    mvp[12] = (-(f / aspect) * eye[0]) as f32;
    mvp[13] = (-f * eye[1]) as f32;
    mvp[14] = (-p22 * eye[2] + p32) as f32;
    mvp[15] = eye[2] as f32;
    let v = |x: f64| x as f32;
    ModernSparkles {
        mvp,
        eye: eye.map(v),
        spot: [v(s.spot_pos_x), v(s.spot_pos_y), v(s.spot_pos_z)],
        attn: [v(s.spot_attn_x), v(s.spot_attn_y), v(s.spot_attn_z)],
        light: [v(s.lambert_coeff), v(s.specular_coeff), v(s.exposure), v(s.specular_power)],
        life_min: LIFE_MIN,
        life_max: LIFE_MAX,
        focus: [
            v(s.near_focus),
            v(s.near_focus + s.near_focus_dist),
            v(s.far_focus),
            v(s.far_focus + s.far_focus_dist),
        ],
        focus_curves: [v(s.near_focus_pow), v(s.far_focus_pow), v(CAMERA.fovy)],
        size: [v(s.size_middle), v(s.size_near), v(s.size_far)],
        darkness: [v(s.near_darkness), v(s.far_darkness)],
        near: [v(s.size_align), v(s.near_fuzziness), v(s.near_align)],
        transparency: [v(s.fresnel), v(s.global_alpha)],
        glare: [v(s.glare), v(s.glare_scale), v(s.glare_p1), v(s.glare_p2)],
        iridescent_exp: v(s.iridescent_exp),
        color_control: v(s.color_control),
        colour: v(colour),
        gamma: 1.0,
    }
}

/// proc_iridescent.tga rebuilt from a palette: its colour hangs only on a spiral's phase about
/// the centre, read from the palette as a periodic Catmull-Rom loop (Nagare's `particles.rs`).
fn iridescent_texels(palette: &[u32]) -> Vec<u8> {
    let size = MODERN_IRIDESCENT;
    let n = palette.len() as i64;
    let rgb: Vec<[f64; 3]> = palette
        .iter()
        .map(|c| [((c >> 16) & 255) as f64, ((c >> 8) & 255) as f64, (c & 255) as f64])
        .collect();
    let at = |k: i64| &rgb[k.rem_euclid(n) as usize];
    let c = (size - 1) as f64 / 2.0;
    let mut px = vec![0u8; size * size * 4];
    for j in 0..size {
        for i in 0..size {
            let dx = i as f64 - c;
            let dy = c - j as f64;
            let r = dx.hypot(dy);
            let phase = (dy.atan2(dx) - 0.025 * r - 0.00006 * r * r) / (2.0 * std::f64::consts::PI);
            let u = (phase - phase.floor()) * n as f64 - 0.5;
            let k = u.floor() as i64;
            let fr = u - k as f64;
            let (p0, p1, p2, p3) = (at(k - 1), at(k), at(k + 1), at(k + 2));
            let o = (j * size + i) * 4;
            for ch in 0..3 {
                let v = 0.5
                    * (2.0 * p1[ch]
                        + (p2[ch] - p0[ch]) * fr
                        + (2.0 * p0[ch] - 5.0 * p1[ch] + 4.0 * p2[ch] - p3[ch]) * fr * fr
                        + (3.0 * p1[ch] - p0[ch] - 3.0 * p2[ch] + p3[ch]) * fr * fr * fr);
                px[o + ch] = v.round().clamp(0.0, 255.0) as u8;
            }
            px[o + 3] = 255;
        }
    }
    px
}

/// A colour of the bar's own as the backdrop's picture: Classic's ground for it - its top row
/// across the screen easing into its bottom one - or the handheld's own background picture for
/// it, `ground` (w, h, RGBA, top row first), when the card has one.
pub fn xmb_modern_picture(colour: u32, ground: Option<(u32, u32, &[u8])>) -> Vec<u8> {
    xmb_modern_picture_of(&crate::xmb::xmb_backdrop_colours(colour), ground)
}

/// As `xmb_modern_picture`, from the fifteen colours the bar's own backdrop takes.
pub fn xmb_modern_picture_of(colours: &slot_gfx::BackdropColours, ground: Option<(u32, u32, &[u8])>) -> Vec<u8> {
    let c = colours.map(|p| p.map(|v| v as f32 / 255.0));
    let mix = |a: [f32; 3], b: [f32; 3], t: f32| [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t);
    let soft = |x: f32, a: f32| {
        let d = x - a;
        a + 0.5 * (d + (d * d + 0.01).sqrt())
    };
    let step = |a: f32, b: f32, x: f32| {
        let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    };
    let side = |part: usize, u: f32| {
        let (l, m, r) = (c[part * 3], c[part * 3 + 1], c[part * 3 + 2]);
        let a = mix(l, m, soft((u - 0.234) / 0.349, -0.3));
        let b = mix(m, r, ((u - 0.583) / 0.396).clamp(-1.0, 1.06));
        mix(a, b, step(0.433, 0.733, u)).map(|x| x.clamp(0.0, 1.0))
    };
    let mut out = Vec::with_capacity(MODERN_BACK_W * MODERN_BACK_H * 4);
    for j in 0..MODERN_BACK_H {
        // Rows from the bottom of the screen; `v` from its top, as the bar's shader has it.
        let v = 1.0 - (j as f32 + 0.5) / MODERN_BACK_H as f32;
        for i in 0..MODERN_BACK_W {
            let u = (i as f32 + 0.5) / MODERN_BACK_W as f32;
            let px = match ground {
                Some((w, h, rgba)) if w > 0 && h > 0 && rgba.len() >= (w * h * 4) as usize => {
                    let fx = (u * w as f32 - 0.5).clamp(0.0, (w - 1) as f32);
                    let fy = (v * h as f32 - 0.5).clamp(0.0, (h - 1) as f32);
                    let (x0, y0) = (fx.floor() as u32, fy.floor() as u32);
                    let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
                    let (tx, ty) = (fx - x0 as f32, fy - y0 as f32);
                    let at = |x: u32, y: u32| {
                        let o = ((y * w + x) * 4) as usize;
                        [rgba[o], rgba[o + 1], rgba[o + 2]].map(|b| b as f32 / 255.0)
                    };
                    let top = mix(at(x0, y0), at(x1, y0), tx);
                    let bottom = mix(at(x0, y1), at(x1, y1), tx);
                    mix(top, bottom, ty).map(|x| (x * 1.046 + 0.04).clamp(0.0, 1.0))
                }
                _ => mix(side(0, u), side(4, u), step(0.11, 0.97, v)),
            };
            out.extend(px.map(|x| (x * 255.0).round() as u8));
            out.push(255);
        }
    }
    out
}
