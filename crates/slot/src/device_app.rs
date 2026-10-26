use std::path::PathBuf;
use std::time::{Duration, Instant};

use slot::frontend::Frontend;
use slot::input::DeviceInput;
use slot::panel_turn::{panel_turn, PanelTurn};
use slot_gfx::{Compositor, FbdevSurface, Surface};
use slot_power::{trace_first_frame, DevicePlatform};

const CARD: &str = "/mnt/sdcard";

const MIN_FRAME: Duration = Duration::from_millis(12);

const STEP_TIMEOUT: Duration = Duration::from_millis(12);

const MARGIN: Duration = Duration::from_micros(1500);

const LOG_SYNC: Duration = Duration::from_secs(1);

/// How often the bar is drawn while nothing but its slow waves moves: 20 frames a second, a
/// third of the work, which keeps the handheld cool on the home screen.
const CALM_FRAME: Duration = Duration::from_millis(50);
/// Modern's sparkles drift on even when the bar is still, so it rests at thirty frames.
const CALM_FRAME_MODERN: Duration = Duration::from_millis(33);

/// How often input is still read between those frames.
const CALM_POLL: Duration = Duration::from_millis(12);

struct Pacer {
    last_return: Option<Instant>,
    period: Duration,
    work: Duration,
    works: [Duration; WORK_WINDOW],
    next: usize,
}

const WORK_WINDOW: usize = 32;

impl Pacer {
    fn new() -> Self {
        Pacer {
            last_return: None,
            period: Duration::from_micros(16_760),
            work: Duration::from_millis(4),
            works: [Duration::from_millis(4); WORK_WINDOW],
            next: 0,
        }
    }

    fn wait(&self) {
        let Some(last) = self.last_return else {
            return;
        };
        let delay = self.period.saturating_sub(self.work + MARGIN);
        if let Some(left) = (last + delay).checked_duration_since(Instant::now()) {
            std::thread::sleep(left);
        }
    }

    fn swapped(&mut self, work: Duration, blocked: bool) -> bool {
        let now = Instant::now();
        let mut missed = false;
        if let Some(last) = self.last_return {
            let seen = now - last;
            missed = seen > self.period.mul_f64(1.5);
            if blocked && seen > Duration::from_millis(12) && seen < Duration::from_millis(22) {
                self.period = self.period.mul_f64(0.95) + seen.mul_f64(0.05);
            }
        }
        self.works[self.next] = work;
        self.next = (self.next + 1) % WORK_WINDOW;
        self.work = self.works.iter().copied().max().unwrap_or(work);
        self.last_return = Some(now);
        missed
    }
}

pub fn run() {
    slot::boot_time::mark("start");
    let _ = std::thread::Builder::new()
        .name("slot-fonts".into())
        .spawn(slot_ui::warm_fonts);
    let audio = std::thread::Builder::new()
        .name("slot-audio-open".into())
        .spawn(slot::audio::opened_sink);
    let root = std::env::var_os("SLOT_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(CARD));
    let mut surface = match FbdevSurface::new() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("slot: {e}");
            return;
        }
    };
    slot::boot_time::mark("surface");
    let mut compositor = match Compositor::new(&surface) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("slot: {e}");
            return;
        }
    };
    slot::boot_time::mark("compositor");
    let panel = surface.window_size();
    let env = |key| std::env::var(key).ok();
    let turn = panel_turn(
        panel,
        slot::baseos::var("BASEOS_PANEL_ROTATION_CCW").as_deref(),
        env("SLOT_ROTATE").as_deref(),
    );
    let turned = turn != PanelTurn::None;
    let window = if turned { (panel.1, panel.0) } else { panel };
    eprintln!(
        "slot: baseos target {}, model {}, rotation ccw {}",
        slot::baseos::var("BASEOS_TARGET").unwrap_or_default(),
        slot::baseos::var("BASEOS_MODEL").unwrap_or_default(),
        slot::baseos::var("BASEOS_PANEL_ROTATION_CCW").unwrap_or_default()
    );
    eprintln!(
        "slot: panel {}x{}, canvas {}x{}, turn {turn:?}",
        panel.0, panel.1, window.0, window.1
    );
    if turned {
        if let Err(e) = compositor.set_panel_turn(panel, turn == PanelTurn::Ccw) {
            eprintln!("slot: {e}");
            return;
        }
    }
    // With the bar, the start-up animation is the first thing drawn, the moment the screen
    // is ready; the platform, the sound and everything else load behind it.
    let mut intro = BootIntro::begin(&root, &mut compositor, window);
    let platform_root = root.clone();
    let boot = move |intro: Option<(Instant, std::path::PathBuf, u8)>| {
        let platform = DevicePlatform::new(platform_root);
        eprintln!("slot: {}", platform.report());
        platform.trace_boot();
        slot::boot_time::mark("platform");
        let sink = match audio.map(|h| h.join()) {
            Ok(Ok(sink)) => sink,
            _ => slot::audio::opened_sink(),
        };
        slot::boot_time::mark("audio");
        if let Some((started, root, volume)) = intro {
            BootIntro::chime(&root, &*sink, started, volume);
        }
        Frontend::boot_with(Box::new(platform), sink)
    };
    let mut frontend = match &mut intro {
        Some(intro) => {
            let (tx, rx) = std::sync::mpsc::channel();
            let chime = Some((intro.started, root.clone(), intro.volume));
            let booting = std::thread::Builder::new()
                .name("slot-boot".into())
                .spawn(move || {
                    let _ = tx.send(Booted(boot(chime)));
                });
            match booting {
                Ok(_) => loop {
                    if let Ok(Booted(f)) = rx.try_recv() {
                        break f;
                    }
                    intro.frame(&mut compositor, &mut surface, true);
                },
                Err(e) => {
                    eprintln!("slot: boot thread: {e}");
                    return;
                }
            }
        }
        None => boot(None),
    };
    slot::boot_time::mark("frontend");
    let step = |intro: &mut Option<BootIntro>, compositor: &mut Compositor, surface: &mut FbdevSurface| {
        if let Some(intro) = intro {
            intro.frame(compositor, surface, true);
        }
    };
    let target = slot::baseos::var("BASEOS_TARGET").unwrap_or_default();
    frontend.set_left_stick(slot::input::evdev::has_left_stick(&target));
    if let Some(intro) = &intro {
        frontend.xmb_intro_played(intro.played_ms());
    }
    frontend.upload_boot_faces(&mut compositor);
    slot::boot_time::mark("faces");
    step(&mut intro, &mut compositor, &mut surface);
    frontend.upload_bezel(&mut compositor, window);
    slot::boot_time::mark("bezel");
    step(&mut intro, &mut compositor, &mut surface);
    let mut input = DeviceInput::open(&root);
    slot::boot_time::mark("input");
    if let Some(intro) = &mut intro {
        frontend.xmb_intro_played(intro.played_ms());
        if let Some((scene, booted)) = intro.modern.take() {
            frontend.adopt_modern(slot::frontend::ModernSync::new(scene, booted));
        }
    }
    let card = root.clone();
    let _ = std::thread::Builder::new()
        .name("slot-bootlogo".into())
        .spawn(move || {
            slot::bootlogo::refresh(&card);
            slot::root::sweep_temps(&card);
        });
    let _ = std::thread::Builder::new()
        .name("slot-logsync".into())
        .spawn(sync_log_writes);
    let mut drawn = false;
    frontend.drive_emulator();
    let mut pacer = Pacer::new();
    let (mut frames, mut missed, mut early) = (0u32, 0u32, 0u32);
    let mut last_frame: Option<Instant> = None;
    loop {
        pacer.wait();
        let began = Instant::now();
        input.steer_with_stick(frontend.app().left_stick_dpad());
        frontend.advance(&mut input);
        if frontend.restarting() {
            frontend.restart();
        }
        if frontend.repairing() {
            frontend.repair();
        }
        if frontend.powering_off() {
            frontend.poweroff();
            return;
        }
        if slot::crash::stopping() {
            eprintln!("slot: told to stop, saving first");
            frontend.stop();
            return;
        }
        let calm = match frontend.app().xmb_modern() {
            true => CALM_FRAME_MODERN,
            false => CALM_FRAME,
        };
        if frontend.app().xmb_calm() && last_frame.is_some_and(|at| at.elapsed() < calm) {
            std::thread::sleep(CALM_POLL);
            continue;
        }
        frontend.step_emulator(pacer.period, STEP_TIMEOUT);
        frontend.render(&mut compositor, window);
        last_frame = Some(Instant::now());
        let swap = Instant::now();
        let work = swap - began;
        if let Err(e) = surface.swap() {
            eprintln!("slot: {e}");
            frontend.stop();
            return;
        }
        let swap_took = swap.elapsed();
        slot::latency::swapped(swap_took.as_secs_f64() * 1000.0);
        let dropped = pacer.swapped(work, swap_took > Duration::from_millis(1));
        if slot::latency::tracing() {
            frames += 1;
            missed += u32::from(dropped);
            early += u32::from(swap_took > pacer.period / 2);
            if frames == 600 {
                eprintln!(
                    "slot: pace: 600 frames, {missed} missed a latch, {early} drawn early, work {:.1} ms, period {:.2} ms",
                    pacer.work.as_secs_f64() * 1e3,
                    pacer.period.as_secs_f64() * 1e3
                );
                (frames, missed, early) = (0, 0, 0);
            }
        }
        if !drawn {
            drawn = true;
            slot::boot_time::mark("first frame");
            trace_first_frame();
        }
        if let Some(left) = MIN_FRAME.checked_sub(began.elapsed()) {
            std::thread::sleep(left);
        }
    }
}

fn sync_log_writes() {
    let mut synced = 0;
    loop {
        std::thread::sleep(LOG_SYNC);
        let mut st: libc::stat = unsafe { std::mem::zeroed() };
        if unsafe { libc::fstat(2, &mut st) } != 0 {
            return;
        }
        if st.st_size != synced {
            #[cfg(target_os = "linux")]
            unsafe {
                libc::syncfs(2);
            }
            synced = st.st_size;
        }
    }
}

/// The booted frontend, handed back from the thread that loaded it. It holds no game yet, and
/// so nothing tied to the thread it was made on.
struct Booted(Frontend);

// SAFETY: the one part of a frontend that is not `Send` is the snapshot of a running game, which
// is only set once a game starts; a frontend fresh from `boot_with` has none.
unsafe impl Send for Booted {}

/// The bar's start-up animation, drawn by the device before the frontend exists, from the
/// colour and background the card has saved. Once loading outlasts it, the waves stay up with
/// a breathing ring at the lower left.
struct BootIntro {
    started: Instant,
    volume: u8,
    window: (u32, u32),
    style: u8,
    colour: u32,
    curves: slot_gfx::TexId,
    flare: slot_gfx::TexId,
    glint: slot_gfx::TexId,
    ring: slot_gfx::TexId,
    title: (slot_gfx::TexId, [f32; 4]),
    foot: slot_gfx::TexId,
    ground: Option<slot_gfx::TexId>,
    /// Modern's scene, and whether its start has begun, until the frontend takes it on.
    modern: Option<(slot_ui::xmb_modern::ModernScene, bool)>,
    buf: Vec<u8>,
    draws: Vec<slot_gfx::Draw>,
    last: Option<Instant>,
    /// Where the animation is, in seconds. It follows the clock but never jumps more than a
    /// frame's worth, so a pause in the loading holds the picture rather than skipping on.
    clock: f32,
    /// Where it was when the Modern scene last moved on.
    modern_at: f32,
    utc_offset_min: i32,
}

impl BootIntro {
    fn begin(root: &std::path::Path, compositor: &mut Compositor, window: (u32, u32)) -> Option<Self> {
        if !slot::app::XMB {
            return None;
        }
        let state = slot_store::read_slot_state(root);
        // A first start sets the clock before anything else; the bar comes later.
        if !state.clock_set {
            return None;
        }
        // A game left running comes straight back, from the boot picture to the game.
        if state.cart.is_some() {
            return None;
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let wall = now + i64::from(state.utc_offset_min) * 60;
        let colour = slot_ui::xmb_colour_index(state.xmb_colour, wall);
        let style = match state.xmb_wave {
            1 => slot_gfx::STYLE_CLASSIC,
            3 => slot_gfx::STYLE_MODERN,
            _ => slot_gfx::STYLE_ORIGINAL,
        };
        let mut buf = Vec::new();
        slot_ui::xmb_curves_at(style, 0.0, Some(0.0), &mut buf);
        let (w, h) = (slot_gfx::BACKDROP_COLS as u32, slot_gfx::BACKDROP_ROWS as u32);
        let curves = compositor.create_texture_nearest(w, h, &buf);
        let f = slot_ui::xmb_flare_face(slot_ui::xmb_pale(slot_ui::xmb_swatch(colour)));
        let flare = compositor.create_texture(f.w, f.h, &f.rgba);
        let f = slot_ui::xmb_flare_face([0xff; 3]);
        let glint = compositor.create_texture(f.w, f.h, &f.rgba);
        let f = slot_ui::xmb_foot_face(slot_ui::xmb_pale(slot_ui::xmb_swatch(colour)));
        let foot = compositor.create_texture(f.w, f.h, &f.rgba);
        let f = slot_ui::xmb_ring_face();
        let ring = compositor.create_texture(f.w, f.h, &f.rgba);
        let grounds = slot::xmb_ground::load(root);
        let ground = slot::xmb_ground::face(&grounds, colour).map(|f| compositor.create_texture(f.w, f.h, &f.rgba));
        let (f, at) = slot_ui::xmb_title_face(slot_gfx::Canvas::of(window));
        let title = (compositor.create_texture(f.w, f.h, &f.rgba), at);
        let modern = (style == slot_gfx::STYLE_MODERN).then(|| {
            use slot_ui::xmb_modern::{xmb_modern_picture, ModernColour, ModernScene};
            let mut scene = ModernScene::new(None);
            if state.xmb_colour != 0 {
                let face = slot::xmb_ground::face(&grounds, colour).map(|f| (f.w, f.h, &f.rgba[..]));
                scene.set_colour(ModernColour::Picture(xmb_modern_picture(colour, face)));
            }
            (scene, false)
        });
        Some(BootIntro {
            started: Instant::now(),
            volume: state.volume,
            window,
            style,
            colour,
            curves,
            flare,
            glint,
            ring,
            title,
            foot,
            ground,
            modern,
            buf,
            draws: Vec::new(),
            last: None,
            clock: 0.0,
            modern_at: 0.0,
            utc_offset_min: i32::from(state.utc_offset_min),
        })
    }

    /// The opening chime, in step with the animation: once the sound device is open, the part
    /// of the chime already due is skipped.
    fn chime(root: &std::path::Path, sink: &dyn slot::audio::AudioSink, started: Instant, volume: u8) {
        let ring = sink.ring();
        let rate = ring.sample_rate();
        if rate == 0 {
            return;
        }
        let sounds = slot::audio::XmbSounds::load(root);
        let mut samples = sounds.render(slot::audio::XmbSound::Opening, rate);
        let late = (started.elapsed().as_secs_f64() * rate as f64) as usize * 2;
        if late >= samples.len() {
            return;
        }
        samples.drain(..late);
        slot::audio::volume::apply(&mut samples, volume);
        ring.mix(&samples);
    }

    fn played_ms(&self) -> slot_input::Millis {
        (self.clock * 1000.0) as slot_input::Millis
    }

    /// Draws the next frame, at most sixty a second.
    fn frame(&mut self, compositor: &mut Compositor, surface: &mut FbdevSurface, loading: bool) {
        if let Some(last) = self.last {
            if let Some(left) = Duration::from_micros(16_000).checked_sub(last.elapsed()) {
                std::thread::sleep(left);
            }
            self.clock += last.elapsed().as_secs_f32().min(0.05);
        }
        self.last = Some(Instant::now());
        let t = self.clock;
        if let Some((scene, booted)) = &mut self.modern {
            if !*booted && t >= slot_ui::XMB_MODERN_START {
                scene.boot();
                *booted = true;
            }
            let wall = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.0)
                + self.utc_offset_min as f64 * 60.0;
            let (w, h) = self.window;
            let dt = (t - self.modern_at) as f64;
            self.modern_at = t;
            scene.advance(
                dt,
                slot_ui::xmb_modern::Wall::of(wall),
                &[],
                None,
                w as f64 / h.max(1) as f64,
            );
            compositor.set_modern(scene.frame());
        }
        slot_ui::xmb_curves_at(self.style, t, Some(t - slot_ui::XMB_INTRO_WAVES), &mut self.buf);
        let (w, h) = (slot_gfx::BACKDROP_COLS as u32, slot_gfx::BACKDROP_ROWS as u32);
        compositor.update_texture(self.curves, w, h, &self.buf);
        let canvas = slot_gfx::Canvas::of(self.window);
        self.draws.clear();
        slot_ui::XmbIntro {
            canvas,
            t,
            backdrop: Some(slot_gfx::Backdrop {
                curves: self.curves,
                style: self.style,
                colours: slot_ui::xmb_backdrop_colours(self.colour),
                edges: slot_ui::xmb_backdrop_edges(self.colour),
                shape: slot_gfx::BACKDROP_PLAIN,
                alpha: 1.0,
                ground: self.ground,
            }),
            flare: Some(self.flare),
            glint: Some(self.glint),
            theme: slot_ui::xmb_swatch(self.colour).map(|v| v as f32 / 255.0),
            title: Some(self.title),
            foot: Some(self.foot),
            modern: 1.0,
        }
        .draw(&mut self.draws);
        if loading && t > slot_ui::XMB_INTRO_BAR {
            let a = ((t - slot_ui::XMB_INTRO_BAR) / 0.4).clamp(0.0, 1.0);
            slot_ui::xmb_loading_ring(canvas, self.ring, t, a, &mut self.draws);
        }
        compositor.fit(self.window);
        compositor.begin_frame();
        compositor.draw_list(&self.draws);
        compositor.end_frame(self.window);
        if let Err(e) = surface.swap() {
            eprintln!("slot: {e}");
        }
    }
}
