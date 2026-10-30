//! The scene's parameter sets and the blend that walks the day and plays the XMB's start, which
//! put them into the sparkles' and the wave's settings. The sets and timings are faquino's
//! `scene-themes.js`, read out of the firmware's override/<set>/*.mnu and custom_render_plugin,
//! as Nagare's `scene.rs` carries them (MIT, see `LICENSE-MIT`); the music, the game launches,
//! Theme Settings' brightness and values set by hand, which the bar has none of, are left out.

use std::collections::HashMap;

use super::settings::{ParticleSettings, Params, WaveSettings};

type Set = &'static [(&'static str, f64)];

// The particle side. Some numbered sets repeat there: welcome_2's particles equal welcome_1's, coldboot2's equal
// coldboot1's, and gameboot4 is gameboot3 with `global alpha` 0, like black is music with it - so the repeats are left
// out.
const PARTICLE_THEMES: &[(&str, Set)] = &[
    ("yoake", &[("farFocus", 12.0064), ("farFocusDist", 4.02735), ("glare", 0.180536)]),
    ("day", &[("sizeMiddle", 0.0464771), ("farFocus", 12.2008), ("farFocusDist", 4.02735), ("glare", 0.201367)]),
    ("higure", &[("sizeMiddle", 0.0464771), ("farFocus", 12.6869), ("farFocusDist", 4.02735), ("glare", 0.18748)]),
    ("night", &[("farFocusDist", 4.09678)]),
    ("music", &[("sizeMiddle", 0.0464771), ("farFocus", 12.0064), ("glare", 0.201367)]),
    ("black", &[("globalAlpha", 0.0), ("sizeMiddle", 0.0464771), ("farFocus", 12.0064), ("glare", 0.201367)]),
    ("coldboot", PARTICLE_COLDBOOT),
    ("gameboot", PARTICLE_GAMEBOOT),
    ("welcome", &[
        ("emitVelMin", 0.131537), ("emitVelMul", 4.16622), ("emitVelZscale", 0.208311), ("emitPerFrame", 70.5765),
        ("emitProb", 0.493003), ("agingSpeed", 0.00101112), ("agingVariance", 0.5), ("friction", 0.0619405),
        ("spinTimeScale", 2.99327), ("deltaTime", 0.0172207), ("windDirX", 0.361019), ("windDirY", -0.0347719),
        ("windScale", 0.000544572), ("brownianScale", 0.0794061), ("specularPower", 29.3486),
        ("sizeMiddle", 0.073865), ("sizeNear", 0.0143576), ("sizeFar", 0.00746469), ("nearFocusDist", 1.59705),
        ("nearFocusPow", 2.22209), ("nearDarkness", 5.49606), ("farFocus", 7.0), ("farFocusDist", 6.31877),
        ("farFocusPow", 1.63882), ("farDarkness", 4.46526), ("nearAlign", 0.118043), ("sizeAlign", 14.1652),
        ("glare", 0.222198), ("glareScale", 6.22156), ("glareP1", 1.6344), ("glareP2", 8.41834),
    ]),
];
const PARTICLE_COLDBOOT: Set = &[
    ("emitVelMul", 3.7496), ("globalAlpha", 0.0), ("sizeMiddle", 0.0464771), ("farFocus", 12.0064), ("glare", 0.201367),
];
const PARTICLE_GAMEBOOT: Set = &[
    ("emitVelMin", 0.029756), ("emitVelMul", 1.52761), ("agingSpeed", 0.0034476), ("agingVariance", 0.0),
    ("deltaTime", 0.08), ("windDirX", -5.34058e-05), ("windScale", 0.000152351), ("brownianScale", 0.0556335),
    ("sizeMiddle", 0.073865), ("sizeNear", 0.186308), ("sizeFar", 0.0389094), ("nearFocus", 7.0),
    ("nearFocusDist", 2.4303), ("nearFocusPow", 3.05), ("nearDarkness", 6.63954), ("farFocus", 9.81914),
    ("farFocusDist", 0.138874), ("farFocusPow", 1.72), ("glare", 0.423566),
];

// The wave's side, LINE1.mnu, then HDR.mnu and BACKGROUND.mnu, under the same names: `music` is music_1, `coldboot`
// coldboot1, `gameboot` gameboot3 and `welcome` welcome_1. THINNESS is 1 in every set. The base BACKGROUND.mnu leaves
// out the backdrop's day and night parameters, so the code's defaults stand for them, and the sets that leave them
// out too (music_1, black, gameboot2) keep those.
const DAY_BACKDROP: [(&str, f64); 7] = [
    ("nightBlend", 0.499947), ("night2dayBegin", 0.0), ("night2dayEnd", 5.16611), ("day2nightBegin", 18.498),
    ("day2nightEnd", 20.3312), ("dayspread", 2.68038), ("nightWhitBias", 0.486059),
];
const NO_COLOURS: [(&str, f64); 12] = [
    ("colour1Red", 0.0), ("colour1Green", 0.0), ("colour1Blue", 0.0), ("colour2Red", 0.0), ("colour2Green", 0.0),
    ("colour2Blue", 0.0), ("colour3Red", 0.0), ("colour3Green", 0.0), ("colour3Blue", 0.0), ("colour4Red", 0.0),
    ("colour4Green", 0.0), ("colour4Blue", 0.0),
];
const NIGHT_HDR: [(&str, f64); 8] = [
    ("exposure", 1.41), ("whiteLevel", 0.999878), ("glareLevel", 1.33), ("glareThresh", 0.699812),
    ("gaussianRadR", 1.1), ("gaussianRadG", 1.36), ("gaussianRadB", 1.5), ("glareSumPow", 0.5),
];

fn wave_themes() -> Vec<(&'static str, Vec<(&'static str, f64)>)> {
    let cat = |parts: &[&[(&'static str, f64)]]| -> Vec<(&'static str, f64)> {
        parts.iter().flat_map(|p| p.iter().copied()).collect()
    };
    vec![
        ("yoake", cat(&[&[
            ("spacing", 407.671), ("brightness", 1.12108), ("mipmapBias", 1.86511), ("fresnel", 0.463044),
            ("falloff", 0.294154), ("timestep", 2.0), ("perturbation", 0.1), ("posX", -8.2), ("ffdScale2X", 3.2),
            ("ffdScale2Y", 1.27473), ("ffdScale2Z", 2.88939), ("ffdParam1", -1.34164),
            ("exposure", 1.1), ("whiteLevel", 0.999878), ("glareLevel", 1.5), ("glareThresh", 0.8),
            ("gaussianRadR", 1.23638), ("gaussianRadG", 1.43047), ("gaussianRadB", 1.55683), ("glareSumPow", 0.5),
        ], &DAY_BACKDROP])),
        ("day", cat(&[&[("timestep", 2.0), ("posX", -8.2), ("ffdScale2X", 3.2)], &DAY_BACKDROP])),
        ("higure", cat(&[&[
            ("spacing", 408.298), ("brightness", 1.06), ("mipmapBias", 1.75802), ("fresnel", 0.501002),
            ("falloff", 0.048), ("timestep", 2.0), ("perturbation", 0.1), ("posX", -8.2), ("ffdScale2X", 3.2),
            ("ffdScale2Y", 1.2), ("ffdScale2Z", 3.0), ("ffdParam1", -1.83395),
            ("exposure", 1.2), ("whiteLevel", 0.999878), ("glareLevel", 1.1), ("glareThresh", 0.8),
            ("gaussianRadR", 1.1), ("gaussianRadG", 1.36), ("gaussianRadB", 1.5), ("glareSumPow", 0.5),
        ], &DAY_BACKDROP])),
        ("night", cat(&[&[
            ("spacing", 408.298), ("brightness", 0.9), ("mipmapBias", 1.75802), ("fresnel", 0.52), ("falloff", 0.048),
            ("timestep", 2.0), ("perturbation", 0.1), ("posX", -8.2), ("angRot", 10.0), ("ffdScale2X", 3.2),
            ("ffdScale2Y", 1.2), ("ffdScale2Z", 3.0), ("ffdParam1", -1.83395),
        ], &NIGHT_HDR, &DAY_BACKDROP])),
        ("music", cat(&[&[
            ("timestep", 3.72102), ("posX", -7.5), ("posY", 0.0), ("posZ", -5.2), ("angY", 0.796751),
            ("angZ", 0.190364), ("angRot", 13.1208), ("ffdScale1X", 5.13725), ("ffdScale2X", 3.2),
            ("ffdScale2Y", 0.99579), ("ffdScale2Z", 3.41782),
            ("exposure", 1.51), ("whiteLevel", 0.999878), ("glareLevel", 2.46), ("glareThresh", 0.260814),
            ("gaussianRadR", 2.0), ("gaussianRadG", 2.2), ("gaussianRadB", 2.5), ("glareSumPow", 0.738001),
            ("colour1Red", 0.579004), ("colour1Green", 0.435001), ("colour1Blue", 0.472), ("colour2Red", 0.0),
            ("colour2Green", 0.0), ("colour2Blue", 0.0), ("colour3Red", 1.2), ("colour3Green", 1.0),
            ("colour3Blue", 1.1), ("colour4Red", 0.5), ("colour4Green", 0.0), ("colour4Blue", 0.5),
            ("colourShader", 1.0),
        ]])),
        ("black", cat(&[&[
            ("damping", 0.0003), ("spacing", 402.611), ("brightness", 0.506754), ("mipmapBias", 2.72688),
            ("fresnel", 1.05552), ("falloff", 2.27705), ("perturbation", 0.1), ("ffdScale1Y", 1.01926),
            ("ffdScale2Y", 1.94378), ("ffdScale2Z", 1.99926), ("ffdParam1", -0.908607),
        ], &NIGHT_HDR, &NO_COLOURS])),
        // The cold boot starts black: coldboot1's colours put the backdrop at a five-hundredth and the wave at nothing.
        ("coldboot", cat(&[&[
            ("spacing", 357.46), ("brightness", 0.512099), ("mipmapBias", 1.90518), ("fresnel", 2.0),
            ("falloff", 0.900768), ("timestep", 2.0), ("perturbation", 0.0), ("posZ", -7.0), ("ffdScale2Y", 0.999999),
            ("ffdScale2Z", 2.85341), ("ffdParam1", -2.0),
            ("exposure", 1.64), ("whiteLevel", 1.0), ("glareLevel", 1.74), ("glareThresh", 0.2),
            ("colour1Red", 0.0021302), ("colour1Green", 0.00213025), ("colour1Blue", 0.00213032),
            ("colour2Red", 0.00213051), ("colour2Green", 0.00213051), ("colour2Blue", 0.00213082),
            ("colour3Red", 0.0), ("colour3Green", 0.0), ("colour3Blue", 0.0), ("colour4Red", 0.0),
            ("colour4Green", 0.0), ("colour4Blue", 0.0),
            ("day2nightBegin", 18.498), ("day2nightEnd", 24.0), ("dayspread", 2.73593), ("nightWhitBias", 0.513834),
        ]])),
        ("gameboot", cat(&[&[
            ("brightness", 0.0), ("fresnel", 0.0), ("timestep", 2.0), ("posZ", -2.40287),
            ("exposure", 1.0), ("whiteLevel", 1.0), ("glareLevel", 1.0), ("glareThresh", 1.0), ("gaussianRadR", 1.1),
            ("gaussianRadG", 1.36), ("gaussianRadB", 1.5), ("glareSumPow", 0.5),
        ], &NO_COLOURS, &[("nightBlend", 0.0), ("dayspread", 1.0), ("nightWhitBias", 0.513834)]])),
        // Its HDR.mnu also changes the flags the page leaves out: TEX SIZE and TEX MAX MIP 7, TONEBEFORE 0, BLUR 1.
        ("welcome", cat(&[&[
            ("timestep", 2.0), ("posX", -8.2), ("ffdScale2X", 3.2),
            ("exposure", 3.40362), ("whiteLevel", 1000.0), ("glareLevel", 100.0), ("glareThresh", 100.0),
            ("gaussianRadR", 2.9), ("gaussianRadG", 2.9), ("gaussianRadB", 2.9), ("glareSumPow", 100.0),
            ("colour1Red", 0.343163), ("colour1Green", 1.15655), ("colour1Blue", 10.0), ("colour2Red", 0.440692),
            ("colour2Green", 1.81381), ("colour2Blue", 10.0), ("colour3Red", 0.557667), ("colour3Green", 0.624199),
            ("colour3Blue", 0.859806), ("colour4Red", 0.774938), ("colour4Green", 1.81381), ("colour4Blue", 0.696515),
            ("colourShader", 1.0), ("nightBlend", 0.499947), ("night2dayBegin", 0.0), ("night2dayEnd", 5.16611),
            ("day2nightBegin", 18.498), ("day2nightEnd", 20.3312), ("nightWhitBias", 0.486059),
        ]])),
    ]
}

const CYCLE: [(&str, &str, f64, f64); 4] =
    [("night", "yoake", 1.0, 5.0), ("yoake", "day", 7.0, 11.0), ("day", "higure", 13.0, 17.0), ("higure", "night", 19.0, 23.0)];


/// A step of the XMB's start: a set put into every layer at a time into it, over `seconds` along
/// qgl_base's blend mode 2, a smoothstep - or, under mode 1, each frame taking the parameters
/// `approach` of the way to the set.
#[derive(Clone, Copy)]
struct Step {
    at: f64,
    set: &'static str,
    seconds: f64,
    approach: Option<f64>,
}

// anim_coldboot2: BootBG2 at 0 starts the wave's lines afresh, puts coldboot1 in at once and
// coldboot2 under mode 1; NormalBG2 at 4 s puts mode 2 back and the day's set in over 7.5 s,
// which ShowGUI cuts short: it lets go of the scene's clock at 5.5 s, and its next tick ends the
// start, the day's set then coming in over a second from where the parameters stand.
const COLDBOOT: [Step; 3] = [
    Step { at: 0.0, set: "coldboot", seconds: 0.0, approach: None },
    Step { at: 0.0, set: "coldboot2", seconds: 0.0, approach: Some(0.01) },
    Step { at: 4.0, set: "theme", seconds: 7.5, approach: None },
];
const FRAME_HZ: f64 = 60.0;
/// ShowGUI, 5.5 s into the XMB's start, lets go of the scene's clock.
pub const BOOT_CLOCK_HOLD: f64 = 5.5;
/// As it begins, the XMB's start hands the backdrop 10:00 of the day, over 7.5 s.
pub const BOOT_HOUR: f64 = 10.0;
pub const BOOT_EASE: f64 = 7.5;
/// The scene's clock ticks every second and puts the moment it shows in over a second.
pub const TICK: f64 = 1.0;

/// DAYSPREAD's range, which the scene holds the parameter to whenever it sets it: its default,
/// 0, lies below it.
fn held(name: &str, value: f64) -> f64 {
    if name == "dayspread" {
        value.clamp(1.0, 3.0)
    } else {
        value
    }
}

/// COLOUR SHADER is a whole number, which a set puts in at once rather than blending.
fn whole(name: &str) -> bool {
    name == "colourShader"
}

fn smoothstep(u: f64) -> f64 {
    let t = u.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// What a goal asks for: one set, or `mix` of the way from one to another.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Goal {
    pub from: &'static str,
    pub to: &'static str,
    pub mix: f64,
}

/// The day's set at `hour`, 0 to 24.
pub fn day_cycle(hour: f64) -> Goal {
    let mut last = CYCLE[CYCLE.len() - 1].1; // before the first window of the day, last night's set holds
    for (from, to, start, end) in CYCLE {
        if hour >= start && hour < end {
            return Goal { from, to, mix: smoothstep((hour - start) / (end - start)) };
        }
        if hour >= end {
            last = to;
        }
    }
    Goal { from: last, to: last, mix: 0.0 }
}

/// A fade sent by the scene's event 0: the grey the scene fades to, over how long, and a count
/// of them.
#[derive(Clone, Debug)]
pub struct Sent {
    pub target: f64,
    pub seconds: f64,
    pub serial: u64,
}

/// One layer's sets, and the defaults the sets leave alone.
struct Layer {
    themes: HashMap<&'static str, Vec<(&'static str, f64)>>,
    keys: Vec<&'static str>,
    base: HashMap<&'static str, f64>,
    from: HashMap<&'static str, f64>,
}

impl Layer {
    fn new<T: Params>(defaults: &T, themes: Vec<(&'static str, Vec<(&'static str, f64)>)>) -> Self {
        let mut keys = Vec::new();
        for (_, set) in &themes {
            for (name, _) in set {
                if !keys.contains(name) {
                    keys.push(*name);
                }
            }
        }
        let base = keys.iter().map(|&k| (k, defaults.get(k).unwrap_or(0.0))).collect();
        Layer { themes: themes.into_iter().collect(), keys, base, from: HashMap::new() }
    }

    /// A setting's value in one of the sets; 'base', and a set that leaves it out, give the default.
    fn value_of(&self, name: &str, set: &str) -> f64 {
        self.themes
            .get(set)
            .and_then(|vs| vs.iter().find(|(k, _)| *k == name))
            .map_or(self.base[name], |v| v.1)
    }

    fn goal_value(&self, name: &str, goal: &Goal) -> f64 {
        if whole(name) {
            return self.value_of(name, if goal.mix > 0.0 { goal.to } else { goal.from });
        }
        let a = self.value_of(name, goal.from);
        a + (self.value_of(name, goal.to) - a) * goal.mix
    }

    /// Takes where every setting stands as what the next blend starts from.
    fn mark<T: Params>(&mut self, live: &T) {
        self.from = self.keys.iter().map(|&k| (k, live.get(k).unwrap_or(0.0))).collect();
    }

    /// Writes `k` of the way from where the blend started towards `goal`.
    fn write<T: Params>(&self, live: &mut T, goal: &Goal, k: f64) {
        for &name in &self.keys {
            let b = self.goal_value(name, goal);
            let v = match (k >= 1.0 || whole(name), self.from.get(name)) {
                (false, Some(&from)) => from + (b - from) * k,
                _ => b,
            };
            if let Some(slot) = live.get_mut(name) {
                *slot = held(name, v);
            }
        }
    }
}

/// The scene: the sets each layer can take, the XMB's start while it plays, and a change of set
/// on its way.
pub struct Scene {
    particles: Layer,
    wave: Layer,
    /// When the XMB's start began, while it plays, and the step it is on.
    boot: Option<(f64, usize)>,
    /// A change of set on its way: when it started, and how long it takes.
    fade: Option<(f64, f64)>,
    applied: Option<Goal>,
    /// The scene's time when it was last applied.
    now: f64,
}

impl Scene {
    /// Each layer's sets, and the ones only the start puts in: the sparkles' coldboot2 is their
    /// coldboot1, the wave's day's LINE1.mnu with the base HDR.mnu and BACKGROUND.mnu.
    pub fn new() -> Self {
        let mut particle_themes: Vec<_> = PARTICLE_THEMES.iter().map(|(n, s)| (*n, s.to_vec())).collect();
        particle_themes.push(("coldboot2", PARTICLE_COLDBOOT.to_vec()));
        let mut waves = wave_themes();
        let day = waves.iter().find(|(n, _)| *n == "day").map(|d| d.1.clone()).unwrap_or_default();
        waves.push(("coldboot2", day));
        Scene {
            particles: Layer::new(&ParticleSettings::default(), particle_themes),
            wave: Layer::new(&WaveSettings::default(), waves),
            boot: None,
            fade: None,
            applied: None,
            now: 0.0,
        }
    }

    /// Starts the XMB's start at `now`.
    pub fn boot(&mut self, now: f64) {
        self.boot = Some((now, usize::MAX));
        self.fade = None;
        self.applied = None;
    }

    /// Whether the XMB's start is playing.
    pub fn booting(&self) -> bool {
        self.boot.is_some_and(|(start, _)| self.now < Self::boot_end(start))
    }

    /// When the start's last step, the day's set coming in, is over. The console cuts it short
    /// when it lets go of the clock, bringing the rest in over a second; here it runs its course,
    /// so that the colours come up in one smooth sweep.
    fn tail_end(start: f64) -> f64 {
        let last = COLDBOOT[COLDBOOT.len() - 1];
        start + last.at + last.seconds
    }

    /// When the start ends: on the scene clock's first tick once ShowGUI has let go of it.
    fn boot_end(start: f64) -> f64 {
        (start + BOOT_CLOCK_HOLD).ceil()
    }

    /// Whether the scene's clock is held at `now`, so that its ticks hand the backdrop nothing.
    pub fn clock_held(&self, now: f64) -> bool {
        self.boot.is_some_and(|(start, _)| now - start < BOOT_CLOCK_HOLD)
    }

    /// Writes the day's set at `hour`, or the start's step, into both layers. A change of set
    /// eases in from where the parameters stood, on qgl_base's smoothstep.
    pub fn apply(&mut self, now: f64, hour: f64, p: &mut ParticleSettings, w: &mut WaveSettings) {
        let theme = day_cycle(hour);
        self.now = now;
        if let Some((start, _)) = self.boot {
            if now >= Self::tail_end(start) {
                self.boot = None;
                self.fade = None;
                self.applied = None;
            }
        }
        if let Some((start, mut step)) = self.boot {
            let t = now - start;
            // A step starts from where the one before it stood at the step's own time.
            while step.wrapping_add(1) < COLDBOOT.len() && t >= COLDBOOT[step.wrapping_add(1)].at {
                if step != usize::MAX {
                    let at = COLDBOOT[step.wrapping_add(1)].at;
                    self.write_step(&COLDBOOT[step], theme, at, p, w);
                }
                step = step.wrapping_add(1);
                self.particles.mark(p);
                self.wave.mark(w);
            }
            self.boot = Some((start, step));
            if step != usize::MAX {
                self.write_step(&COLDBOOT[step], theme, t, p, w);
            }
            return;
        }
        let goal = Goal { mix: (theme.mix * 500.0).round() / 500.0, ..theme };
        let moved = self.applied != Some(goal);
        self.applied = Some(goal);
        if !moved && self.fade.is_none() {
            return;
        }
        let k = self.fade.map_or(1.0, |(start, seconds)| smoothstep((now - start) / seconds));
        self.particles.write(p, &goal, k);
        self.wave.write(w, &goal, k);
        if k >= 1.0 {
            self.fade = None;
        }
    }

    fn write_step(&self, step: &Step, theme: Goal, t: f64, p: &mut ParticleSettings, w: &mut WaveSettings) {
        let since = t - step.at;
        let mix = if let Some(approach) = step.approach {
            1.0 - (1.0 - approach).powf(since * FRAME_HZ)
        } else if step.seconds > 0.0 {
            smoothstep(since / step.seconds)
        } else {
            1.0
        };
        let goal = if step.set == "theme" { theme } else { Goal { from: step.set, to: step.set, mix: 0.0 } };
        let k = mix.min(1.0 - 1e-12);
        self.particles.write(p, &goal, if mix >= 1.0 { 1.0 } else { k });
        self.wave.write(w, &goal, if mix >= 1.0 { 1.0 } else { k });
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}
