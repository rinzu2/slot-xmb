//! Drives the cross-media bar through its screens and renders them to PNGs in software, for
//! review without a GPU: the background through `slot_gfx::backdrop_shade`, the CPU twin of
//! its shader. Run with `XMB_SHOTS=dir cargo test -p slot --features xmb --test xmb_shots`;
//! without `XMB_SHOTS` it still checks every screen and the navigation between them.
#![cfg(feature = "xmb")]

mod common;

use std::path::PathBuf;

use common::{app_booting_at, app_playing_in, write_gb_cart, CLOCK_IS_SET};
use slot::app::App;
use slot_gfx::TexId;
use slot_input::{Action, Btn};
use slot_store::{write_slot_state, SlotState};
use slot_ui::{cart_face, Draw, Icon, QuickRow, HUD_ICON_PX};

struct Soft {
    tex: Vec<(u32, u32, Vec<u8>)>,
    shots: Option<PathBuf>,
}

impl Soft {
    fn new() -> Self {
        let shots = std::env::var_os("XMB_SHOTS").map(PathBuf::from);
        if let Some(dir) = &shots {
            std::fs::create_dir_all(dir).unwrap();
        }
        Soft {
            tex: Vec::new(),
            shots,
        }
    }

    fn add(&mut self, w: u32, h: u32, rgba: Vec<u8>) -> TexId {
        self.tex.push((w, h, rgba));
        TexId::from_raw(self.tex.len() - 1)
    }

    fn faces(&mut self, app: &mut App) {
        let faces: Vec<_> = app
            .carts()
            .map(cart_face)
            .collect::<Vec<_>>()
            .into_iter()
            .map(|f| self.add(f.w, f.h, f.rgba))
            .collect();
        app.set_faces(faces);
        let icons = Icon::ALL
            .iter()
            .map(|i| {
                let f = slot_ui::icon_face_soft(*i, HUD_ICON_PX, [0xff; 3]);
                self.add(f.w, f.h, f.rgba)
            })
            .collect();
        app.set_icon_faces(icons);
    }

    fn sync(&mut self, app: &mut App) {
        let mut art = slot_ui::XmbArt::default();
        for icon in slot_ui::XmbIcon::ALL {
            let f = slot_ui::xmb_icon_face(icon);
            art.icons[icon.index()] = Some(self.add(f.w, f.h, f.rgba));
        }
        let g = slot_ui::xmb_glow_face();
        art.glow = Some(self.add(g.w, g.h, g.rgba));
        let f = slot_ui::xmb_frame_glow_face();
        art.frame = Some(self.add(f.w, f.h, f.rgba));
        let f = slot_ui::xmb_flare_face(slot_ui::xmb_pale(app.xmb_theme_rgb()));
        art.flare = Some(self.add(f.w, f.h, f.rgba));
        let f = slot_ui::xmb_foot_face(slot_ui::xmb_pale(app.xmb_theme_rgb()));
        art.foot = Some(self.add(f.w, f.h, f.rgba));
        let f = slot_ui::xmb_flare_face([0xff; 3]);
        art.glint = Some(self.add(f.w, f.h, f.rgba));
        let f = slot_ui::xmb_ring_face();
        art.ring = Some(self.add(f.w, f.h, f.rgba));
        for k in 0..slot_ui::XMB_HALOS {
            let (f, margin) = slot_ui::xmb_cart_halo_face(k);
            art.halos[k] = Some((self.add(f.w, f.h, f.rgba), margin));
        }
        let (f, at) = slot_ui::xmb_title_face(app.canvas());
        art.title = Some((self.add(f.w, f.h, f.rgba), at));
        let mut curves = Vec::new();
        slot_ui::xmb_curves_at(app.xmb_wave_style(), app.xmb_time(), app.xmb_intro_waves(), &mut curves);
        art.curves = Some(self.add(
            slot_gfx::BACKDROP_COLS as u32,
            slot_gfx::BACKDROP_ROWS as u32,
            curves,
        ));
        app.set_xmb_art(art);
        // Twice: the first pass renders what the screen asks for, the second what that reveals.
        for _ in 0..2 {
            let mut texts = std::collections::HashMap::new();
            for key in app.xmb_texts_wanted() {
                let (size, text) = slot_ui::XmbSize::of_key(&key).unwrap();
                let f = slot_ui::xmb_text_face(text, size);
                let id = self.add(f.w, f.h, f.rgba);
                texts.insert(key, (id, f.w, f.h));
            }
            app.set_xmb_texts(texts);
        }
    }

    fn settle(&mut self, app: &mut App, frames: usize) {
        for _ in 0..frames {
            app.update(1.0 / 30.0);
        }
        self.sync(app);
    }

    fn frame(&mut self, app: &mut App) -> Vec<Draw> {
        self.sync(app);
        let mut out = Vec::new();
        app.draw(&mut out);
        out
    }

    fn shot(&mut self, app: &mut App, name: &str) -> Vec<Draw> {
        self.settle(app, 24);
        self.shot_now(app, name)
    }

    fn shot_now(&mut self, app: &mut App, name: &str) -> Vec<Draw> {
        let out = self.frame(app);
        assert!(
            out.iter().any(|d| matches!(d, Draw::Tex { .. } | Draw::Part { .. })),
            "{name} drew nothing textured"
        );
        self.write_shot(app, name, &out);
        out
    }

    /// A frame written out with no check that it drew anything: a moment of the start-up
    /// animation can be lines and black alone.
    fn shot_bare(&mut self, app: &mut App, name: &str) {
        let out = self.frame(app);
        self.write_shot(app, name, &out);
    }

    fn write_shot(&self, app: &App, name: &str, out: &[Draw]) {
        let Some(dir) = self.shots.clone() else {
            return;
        };
        let c = app.canvas();
        let (w, h) = (c.w as usize, c.h as usize);
        let mut px = vec![[0.0f32; 3]; w * h];
        for d in out {
            self.raster(d, w, h, &mut px);
        }
        let bytes: Vec<u8> = px
            .iter()
            .flat_map(|p| p.map(|v| (v.clamp(0.0, 1.0) * 255.0) as u8))
            .collect();
        let file = std::fs::File::create(dir.join(format!("{name}.png"))).unwrap();
        let mut e = png::Encoder::new(std::io::BufWriter::new(file), w as u32, h as u32);
        e.set_color(png::ColorType::Rgb);
        e.set_depth(png::BitDepth::Eight);
        e.write_header().unwrap().write_image_data(&bytes).unwrap();
    }

    fn raster(&self, d: &Draw, w: usize, h: usize, px: &mut [[f32; 3]]) {
        let whole = [0.0, 0.0, 1.0, 1.0];
        let (x, y, dw, dh, tex, alpha, uv, colour) = match *d {
            Draw::Rect { x, y, w, h, colour } => (x, y, w, h, None, colour[3], whole, colour),
            Draw::Tex {
                x,
                y,
                w,
                h,
                tex,
                alpha,
            }
            | Draw::Turned {
                x,
                y,
                w,
                h,
                tex,
                alpha,
                ..
            } => (x, y, w, h, Some(tex), alpha, whole, [1.0; 4]),
            Draw::Part {
                x,
                y,
                w,
                h,
                tex,
                alpha,
                uv,
            } => (x, y, w, h, Some(tex), alpha, uv, [1.0; 4]),
            Draw::Backdrop(b) => {
                let curves = &self.tex[tex_index(b.curves)].2;
                for (i, p) in px.iter_mut().enumerate() {
                    let (u, v) = ((i % w) as f32 + 0.5, (i / w) as f32 + 0.5);
                    let c = slot_gfx::backdrop_shade(
                        curves,
                        b.style,
                        &b.colours,
                        &b.edges,
                        b.shape,
                        u / w as f32,
                        v / h as f32,
                        h as f32,
                    );
                    for k in 0..3 {
                        p[k] = p[k] * (1.0 - b.alpha) + c[k] * b.alpha;
                    }
                }
                return;
            }
            Draw::Game => {
                // A stand-in for the game's picture.
                for (i, p) in px.iter_mut().enumerate() {
                    let (gx, gy) = (i % w, i / w);
                    let check = ((gx / 32) + (gy / 32)) % 2 == 0;
                    *p = if check { [0.25, 0.55, 0.35] } else { [0.85, 0.75, 0.3] };
                }
                return;
            }
            _ => return,
        };
        let x0 = x.max(0.0).floor() as usize;
        let y0 = y.max(0.0).floor() as usize;
        let x1 = ((x + dw).min(w as f32)).ceil().max(0.0) as usize;
        let y1 = ((y + dh).min(h as f32)).ceil().max(0.0) as usize;
        for py in y0..y1 {
            for qx in x0..x1 {
                let (fx, fy) = ((qx as f32 + 0.5 - x) / dw, (py as f32 + 0.5 - y) / dh);
                if !(0.0..1.0).contains(&fx) || !(0.0..1.0).contains(&fy) {
                    continue;
                }
                let (rgb, a) = match tex {
                    None => ([colour[0], colour[1], colour[2]], alpha),
                    Some(t) => {
                        let (tw, th, data) = &self.tex[tex_index(t)];
                        let u = (uv[0] + fx * uv[2]) * *tw as f32 - 0.5;
                        let v = (uv[1] + fy * uv[3]) * *th as f32 - 0.5;
                        let s = bilinear(data, *tw, *th, u, v);
                        ([s[0], s[1], s[2]], s[3] * alpha)
                    }
                };
                let p = &mut px[py * w + qx];
                for k in 0..3 {
                    p[k] = p[k] * (1.0 - a) + rgb[k] * a;
                }
            }
        }
    }
}

fn tex_index(t: TexId) -> usize {
    // TexId is a newtype over its index; its Debug form is the only window onto it.
    format!("{t:?}")
        .trim_start_matches("TexId(")
        .trim_end_matches(')')
        .parse()
        .unwrap()
}

fn bilinear(data: &[u8], w: u32, h: u32, u: f32, v: f32) -> [f32; 4] {
    let at = |x: i32, y: i32| {
        let x = x.clamp(0, w as i32 - 1) as usize;
        let y = y.clamp(0, h as i32 - 1) as usize;
        let i = (y * w as usize + x) * 4;
        let a = data[i + 3] as f32 / 255.0;
        // Premultiply so transparent texels don't bleed their colour.
        [
            data[i] as f32 / 255.0 * a,
            data[i + 1] as f32 / 255.0 * a,
            data[i + 2] as f32 / 255.0 * a,
            a,
        ]
    };
    let (x0, y0) = (u.floor(), v.floor());
    let (fx, fy) = (u - x0, v - y0);
    let (x0, y0) = (x0 as i32, y0 as i32);
    let mut out = [0.0; 4];
    for (dx, dy, wgt) in [
        (0, 0, (1.0 - fx) * (1.0 - fy)),
        (1, 0, fx * (1.0 - fy)),
        (0, 1, (1.0 - fx) * fy),
        (1, 1, fx * fy),
    ] {
        let s = at(x0 + dx, y0 + dy);
        for k in 0..4 {
            out[k] += s[k] * wgt;
        }
    }
    if out[3] > 0.0 {
        for k in 0..3 {
            out[k] /= out[3];
        }
    }
    out
}

fn press(a: &mut App, btn: Btn) {
    a.apply(Action::GbaDown(btn));
    a.apply(Action::GbaUp(btn));
}

const GAMES: [&str; 12] = [
    "Advance Wars",
    "Castlevania Aria of Sorrow",
    "Fire Emblem",
    "Golden Sun",
    "Kirby Nightmare in Dream Land",
    "Mario Kart Super Circuit",
    "Metroid Fusion",
    "Minish Cap",
    "Pokemon Emerald",
    "Sonic Advance",
    "Wario Ware",
    "Zone of the Enders",
];

fn root(state: SlotState) -> tempfile::TempDir {
    let d = common::tmp_root_with_carts(&GAMES);
    for (stem, title) in [("Tetris", "TETRIS"), ("Kirby's Dream Land", "KIRBY DREAM")] {
        write_gb_cart(&d, stem, title);
    }
    write_slot_state(
        d.path(),
        &SlotState {
            clock_set: true,
            ..state
        },
    )
    .unwrap();
    d
}

fn home(state: SlotState) -> (tempfile::TempDir, App, Soft) {
    let (d, mut a, soft) = home_starting(state);
    a.skip_xmb_intro();
    (d, a, soft)
}

/// The home screen as it boots, its start-up animation still to play.
fn home_starting(state: SlotState) -> (tempfile::TempDir, App, Soft) {
    let d = root(state);
    let (mut a, _clock) = app_booting_at(d.path(), CLOCK_IS_SET);
    a.set_panel((640, 480));
    let mut soft = Soft::new();
    soft.faces(&mut a);
    (d, a, soft)
}

/// The rows of games drawn: (y, height) of every cart face on screen, top to bottom.
fn game_rows(out: &[Draw]) -> Vec<(f32, f32)> {
    let mut rows: Vec<(f32, f32)> = out
        .iter()
        .filter_map(|d| match *d {
            // Faces sit in the column on the left; titles start to their right.
            Draw::Part { x, y, h, uv, .. } if uv == [0.0, 0.0, 1.0, 1.0] && h > 30.0 && x < 200.0 => {
                Some((y, h))
            }
            _ => None,
        })
        .collect();
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    rows
}

#[test]
fn home_and_settings() {
    let (_d, mut a, mut soft) = home(SlotState::default());
    for _ in 0..3 {
        press(&mut a, Btn::Down);
    }
    soft.shot(&mut a, "01_home_gba");

    press(&mut a, Btn::Start);
    soft.shot(&mut a, "02_options");
    press(&mut a, Btn::B);

    a.apply(Action::QuickMenu);
    soft.shot(&mut a, "03_settings");
    assert!(a.quick_menu().is_some());

    // About ends the column, Date & Time Settings and Theme Settings above it; Background
    // opens first, Color under it.
    press(&mut a, Btn::Up);
    assert_eq!(a.quick_menu(), Some(QuickRow::About));
    press(&mut a, Btn::Up);
    press(&mut a, Btn::Up);
    assert_eq!(a.quick_menu(), Some(QuickRow::XmbTheme));
    press(&mut a, Btn::A);
    assert_eq!(a.quick_menu(), Some(QuickRow::XmbWave));
    press(&mut a, Btn::Down);
    assert_eq!(a.quick_menu(), Some(QuickRow::XmbColour));
    soft.shot(&mut a, "04_theme_settings");

    // Open the colour strip, step through a few and keep one.
    press(&mut a, Btn::A);
    for _ in 0..5 {
        press(&mut a, Btn::Down);
    }
    soft.shot(&mut a, "05_colour_picker");
    press(&mut a, Btn::A);
    assert_eq!(a.xmb_colour(), 5, "five down from By Month is Magenta");

    // Back undoes a preview.
    press(&mut a, Btn::A);
    press(&mut a, Btn::Down);
    press(&mut a, Btn::B);
    assert_eq!(a.xmb_colour(), 5, "B put the colour back");

    // Background to Classic, icons to labels.
    press(&mut a, Btn::Up);
    assert_eq!(a.quick_menu(), Some(QuickRow::XmbWave));
    press(&mut a, Btn::A);
    press(&mut a, Btn::Down);
    soft.shot(&mut a, "06_background_picker");
    press(&mut a, Btn::A);
    assert_eq!(a.xmb_wave_style(), slot_ui::XMB_WAVE_CLASSIC);
    press(&mut a, Btn::Down);
    press(&mut a, Btn::Down);
    assert_eq!(a.quick_menu(), Some(QuickRow::XmbIcons));
    press(&mut a, Btn::A);
    press(&mut a, Btn::Down);
    press(&mut a, Btn::A);
    // Back out of Theme Settings and the bar, onto the games.
    press(&mut a, Btn::Left);
    press(&mut a, Btn::Right);
    assert!(a.quick_menu().is_none(), "Right on the Settings column leaves it");
    soft.shot(&mut a, "07_classic_labels");

    a.apply(Action::QuickMenu);
    press(&mut a, Btn::Down);
    assert_eq!(a.quick_menu(), Some(QuickRow::Screen));
    press(&mut a, Btn::A);
    press(&mut a, Btn::A);
    soft.shot(&mut a, "08_screen_general");
}

#[test]
fn scrolling_fast_keeps_the_column_whole() {
    let (_d, mut a, mut soft) = home(SlotState::default());
    soft.shot(&mut a, "09_before_scroll");
    // Hold down for a while: the selection runs ahead of the animation.
    a.apply(Action::GbaDown(Btn::Down));
    for _ in 0..14 {
        a.update(1.0 / 60.0);
        a.update(1.0 / 60.0);
        a.update(1.0 / 60.0);
    }
    let out = soft.shot_now(&mut a, "10_mid_scroll");
    a.apply(Action::GbaUp(Btn::Down));
    let rows = game_rows(&out);
    assert!(rows.len() >= 4, "only {} rows on screen mid-scroll", rows.len());
    // Rows below the bar are evenly spaced and in order: no gaps, no pile-ups.
    let below: Vec<f32> = rows.iter().map(|r| r.0 + r.1 / 2.0).filter(|y| *y > 260.0).collect();
    for pair in below.windows(2) {
        let gap = pair[1] - pair[0];
        assert!((40.0..140.0).contains(&gap), "uneven rows mid-scroll: {below:?}");
    }
    soft.shot(&mut a, "11_after_scroll");
}

#[test]
fn game_boy_shelf_with_labels() {
    let (_d, mut a, mut soft) = home(SlotState {
        xmb_labels: true,
        xmb_colour: 26,
        ..SlotState::default()
    });
    press(&mut a, Btn::Right);
    soft.shot(&mut a, "12_gb_labels");
}

#[test]
fn levels_show_as_segments() {
    let (_d, mut a, mut soft) = home(SlotState::default());
    a.apply(Action::VolumeUp);
    let out = soft.shot_now(&mut a, "13_volume");
    // Ticks reached are upright bars, the rest dots, each with a shadow.
    let bars = out
        .iter()
        .filter(|d| matches!(d, Draw::Rect { h, .. } if (20.0..26.0).contains(h)))
        .count();
    let dots = out
        .iter()
        .filter(|d| matches!(d, Draw::Rect { w, h, .. } if *h <= 6.0 && *w <= 6.0))
        .count();
    assert!(bars >= 2 && dots >= 2, "{bars} bars and {dots} dots");
    // A tick for every press: from silence, one press lights exactly one of twenty.
    for _ in 0..40 {
        a.apply(Action::VolumeDown);
    }
    a.apply(Action::VolumeUp);
    let out = soft.shot_now(&mut a, "13c_volume_one");
    let white = |d: &&Draw| matches!(d, Draw::Rect { colour, .. } if colour[0] == 1.0 && colour[3] > 0.0);
    let lit = out
        .iter()
        .filter(white)
        .filter(|d| matches!(d, Draw::Rect { h, .. } if (20.0..26.0).contains(h)))
        .count();
    let unlit = out
        .iter()
        .filter(white)
        .filter(|d| matches!(d, Draw::Rect { w, h, .. } if *h <= 6.0 && *w <= 6.0))
        .count();
    assert_eq!((lit, unlit), (1, 19), "one press from silence is one tick of twenty");
    a.apply(Action::VolumeDown);
    // Brightness has fewer steps, so a shorter gauge.
    soft.settle(&mut a, 200);
    a.apply(Action::BrightnessDown);
    soft.shot_now(&mut a, "13b_brightness");
}

#[test]
fn in_game_menu() {
    let d = root(SlotState::default());
    let mut a = app_playing_in(d.path(), "Golden Sun");
    a.set_panel((640, 480));
    let mut soft = Soft::new();
    soft.faces(&mut a);
    soft.settle(&mut a, 120);
    a.apply(Action::QuickMenu);
    soft.settle(&mut a, 30);
    assert!(a.in_game_menu().is_some());
    soft.shot(&mut a, "14_in_game");
    // Resume, Settings, Exit Game: Settings, then its Screen list.
    press(&mut a, Btn::Down);
    assert_eq!(a.in_game_menu(), Some(QuickRow::Settings));
    press(&mut a, Btn::A);
    press(&mut a, Btn::A);
    press(&mut a, Btn::A);
    soft.shot(&mut a, "15_in_game_shader_picker");
    press(&mut a, Btn::B);
    assert!(a.in_game_menu().is_some(), "still in the menu");
    press(&mut a, Btn::Left);
    press(&mut a, Btn::Left);
    assert_eq!(a.in_game_menu(), Some(QuickRow::Settings));
    press(&mut a, Btn::B);
    soft.settle(&mut a, 10);
    assert!(a.in_game_menu().is_none(), "B on the column closes the menu");
}

/// Presses Down until the settings land on `row`.
fn down_to(a: &mut App, row: QuickRow) {
    for _ in 0..12 {
        if a.quick_menu() == Some(row) {
            return;
        }
        press(a, Btn::Down);
    }
    assert_eq!(a.quick_menu(), Some(row));
}

#[test]
fn wallpaper_clock_and_levels() {
    let (_d, mut a, mut soft) = home(SlotState::default());
    a.apply(Action::QuickMenu);

    // Wallpaper is a Background, and only then is its own row in Theme Settings.
    for _ in 0..3 {
        press(&mut a, Btn::Up);
    }
    press(&mut a, Btn::A);
    assert_eq!(a.quick_menu(), Some(QuickRow::XmbWave));
    assert!(a.hidden_rows().contains(&QuickRow::XmbWallpaper));
    assert!(!a.hidden_rows().contains(&QuickRow::XmbColour));
    press(&mut a, Btn::A);
    press(&mut a, Btn::Down);
    press(&mut a, Btn::Down);
    press(&mut a, Btn::Down);
    press(&mut a, Btn::A);
    // The wallpaper takes the colour's place.
    assert!(!a.hidden_rows().contains(&QuickRow::XmbWallpaper));
    assert!(a.hidden_rows().contains(&QuickRow::XmbColour));
    press(&mut a, Btn::Down);
    assert_eq!(a.quick_menu(), Some(QuickRow::XmbWallpaper));
    soft.shot(&mut a, "16_wallpaper_row");
    // Back to Original, and the row goes again.
    press(&mut a, Btn::Up);
    press(&mut a, Btn::A);
    press(&mut a, Btn::Up);
    press(&mut a, Btn::Up);
    press(&mut a, Btn::A);
    assert!(a.hidden_rows().contains(&QuickRow::XmbWallpaper));
    assert!(!a.hidden_rows().contains(&QuickRow::XmbColour));
    press(&mut a, Btn::Left);

    // Date & Time Settings, its own page below Theme Settings.
    press(&mut a, Btn::Down);
    assert_eq!(a.quick_menu(), Some(QuickRow::XmbClock));
    press(&mut a, Btn::A);
    assert_eq!(a.quick_menu(), Some(QuickRow::XmbSetClock));
    soft.shot(&mut a, "17_clock_settings");
    press(&mut a, Btn::A);
    assert!(a.picker().is_some());
    // Month first: right moves to the day.
    press(&mut a, Btn::Right);
    assert_eq!(a.picker().map(|p| p.cursor()), Some(slot_ui::Field::Day));
    soft.shot(&mut a, "18_clock_edit");
    press(&mut a, Btn::B);
    assert_eq!(a.quick_menu(), Some(QuickRow::XmbSetClock));

    // 24-hour clock.
    down_to(&mut a, QuickRow::XmbTimeFormat);
    press(&mut a, Btn::A);
    press(&mut a, Btn::Down);
    press(&mut a, Btn::A);
    soft.shot(&mut a, "19_clock_24h");

    // Battery percentage beside the battery, in General.
    press(&mut a, Btn::Left);
    for _ in 0..5 {
        press(&mut a, Btn::Up);
    }
    assert_eq!(a.quick_menu(), Some(QuickRow::General));
    press(&mut a, Btn::A);
    down_to(&mut a, QuickRow::XmbBattery);
    press(&mut a, Btn::A);
    press(&mut a, Btn::Down);
    press(&mut a, Btn::A);
    press(&mut a, Btn::Left);

    // About scrolls.
    press(&mut a, Btn::Up);
    assert_eq!(a.quick_menu(), Some(QuickRow::About));
    press(&mut a, Btn::A);
    press(&mut a, Btn::Down);
    soft.shot(&mut a, "21_about");
    press(&mut a, Btn::Left);

    // Blue light at Off lists Off first and steps down into the levels.
    for _ in 0..5 {
        press(&mut a, Btn::Up);
    }
    assert_eq!(a.quick_menu(), Some(QuickRow::Screen));
    press(&mut a, Btn::A);
    down_to(&mut a, QuickRow::ScreenGeneral);
    press(&mut a, Btn::A);
    down_to(&mut a, QuickRow::BlueLight);
    assert_eq!(a.quick_value(QuickRow::BlueLight), Some(slot_ui::QuickValue::Off));
    press(&mut a, Btn::A);
    press(&mut a, Btn::Up);
    assert_eq!(a.quick_value(QuickRow::BlueLight), Some(slot_ui::QuickValue::Off));
    press(&mut a, Btn::Down);
    soft.shot(&mut a, "20_blue_light");
    assert_eq!(a.quick_value(QuickRow::BlueLight), Some(slot_ui::QuickValue::level(1)));
}

#[test]
fn start_up_and_switch_off() {
    // The colour of the handheld's capture.
    let (_d, mut a, mut soft) = home_starting(SlotState {
        xmb_colour: 19,
        ..SlotState::default()
    });
    let start = a.now() + 10;
    a.tick_ms(start);
    assert_eq!(a.take_xmb_sound(), Some(slot::audio::XmbSound::Opening));
    for ms in [30, 930, 1430, 1930, 2380, 2430, 2480, 2530, 2580, 2630, 2730, 2830, 2930, 3080, 3430, 3930] {
        a.tick_ms(start + ms);
        soft.shot_bare(&mut a, &format!("30_intro_{ms:04}"));
    }
    // Classic, in blue as in the second video.
    let (_d2, mut c, mut soft2) = home_starting(SlotState {
        xmb_colour: 30,
        xmb_wave: 1,
        ..SlotState::default()
    });
    let begun = c.now() + 10;
    c.tick_ms(begun);
    for ms in [2430, 2530, 2580, 2630, 2680, 2730, 2780, 2830, 2880, 2980, 3080, 3280] {
        c.tick_ms(begun + ms);
        soft2.shot_bare(&mut c, &format!("31_classic_{ms:04}"));
    }
    a.tick_ms(start + 5000);
    // Moving makes the cursor sound; choosing, the confirm.
    press(&mut a, Btn::Down);
    assert_eq!(a.take_xmb_sound(), Some(slot::audio::XmbSound::Cursor));
    press(&mut a, Btn::A);
    assert_eq!(a.take_xmb_sound(), Some(slot::audio::XmbSound::Confirm));
    // Switching off: the bar goes, the colour sinks back into the lines and goes dark.
    let (_d3, mut o, mut soft3) = home(SlotState {
        xmb_colour: 19,
        ..SlotState::default()
    });
    let off = o.now() + 10;
    o.tick_ms(off);
    o.on_battery(slot_power::Battery {
        percent: 0,
        charge: slot_power::Charge::Discharging,
    });
    for ms in [100, 300, 500, 700, 900, 1100, 1250] {
        o.tick_ms(off + ms);
        soft3.shot_bare(&mut o, &format!("32_off_{ms:04}"));
    }
}

#[test]
fn shoulders_jump_a_letter_and_the_place_survives_a_stop() {
    let (d, mut a, mut soft) = home(SlotState::default());
    let first = a.selected_stem().map(str::to_string);
    a.apply(Action::NextShelf);
    let next = a.selected_stem().map(str::to_string);
    assert_ne!(first, next, "R1 moves to the next letter");
    assert_ne!(
        first.as_deref().map(slot_store::initial),
        next.as_deref().map(slot_store::initial),
        "and lands on a different letter"
    );
    assert!(a.quick_menu().is_none(), "R1 stays on the games");
    a.apply(Action::PreviousShelf);
    assert_eq!(a.selected_stem().map(str::to_string), first, "L1 goes back a letter");
    // Hover a game, go into Settings, and be stopped from outside: the game is remembered.
    for _ in 0..3 {
        press(&mut a, Btn::Down);
    }
    let want = a.selected_stem().map(str::to_string);
    press(&mut a, Btn::Left);
    assert!(a.quick_menu().is_some());
    soft.settle(&mut a, 2);
    let st = slot_store::read_slot_state(d.path());
    assert_eq!(st.shelf_cart.map(|c| c.stem), want, "remembered on the way into Settings");
    let (b, _c) = app_booting_at(d.path(), CLOCK_IS_SET);
    assert_eq!(b.selected_stem().map(str::to_string), want);
}

#[test]
fn a_game_fades_up_from_black_without_the_television_effect() {
    let d = root(SlotState::default());
    write_slot_state(
        d.path(),
        &SlotState { cart: Some("Golden Sun".into()), clock_set: true, ..SlotState::default() },
    )
    .unwrap();
    let mut a = App::boot(d.path());
    a.set_panel((640, 480));
    a.set_snapshot(common::StubSnapshot::boxed());
    a.on_core_ready();
    for _ in 0..20 {
        a.update(1.0 / 60.0);
    }
    // No picture yet: the screen stays black, with no lit panel.
    assert!(a.blank_panel().is_none());
    a.set_game_ready(true);
    a.update(0.1);
    // Full size all the way: the television effect is held at its end.
    assert_eq!(a.screen_power(), 1.0);
    let mut out = Vec::new();
    a.draw(&mut out);
    let game = out.iter().position(|d| matches!(d, Draw::Game)).expect("the game is drawn");
    let fade = out[game..].iter().find_map(|d| match d {
        Draw::Rect { colour, .. } if colour[..3] == [0.0, 0.0, 0.0] => Some(colour[3]),
        _ => None,
    });
    assert!(fade.is_some_and(|a| a > 0.0 && a < 1.0), "black fading off over the game: {fade:?}");
    for _ in 0..30 {
        a.update(1.0 / 30.0);
    }
    let mut out = Vec::new();
    a.draw(&mut out);
    let game = out.iter().position(|d| matches!(d, Draw::Game)).expect("the game is drawn");
    assert!(
        !out[game..].iter().any(|d| matches!(d, Draw::Rect { colour, .. } if colour[..3] == [0.0, 0.0, 0.0])),
        "faded up after a second"
    );
}

#[test]
fn leaving_a_game_fades_it_down_into_black() {
    let d = root(SlotState::default());
    let mut a = app_playing_in(d.path(), "Golden Sun");
    a.set_panel((640, 480));
    a.set_game_ready(true);
    for _ in 0..30 {
        a.update(1.0 / 30.0);
    }
    a.apply(Action::Eject);
    a.update(0.1);
    assert_eq!(a.screen_power(), 1.0, "no switch-off effect");
    let mut out = Vec::new();
    a.draw(&mut out);
    let game = out.iter().position(|d| matches!(d, Draw::Game)).expect("the game still shows");
    let fade = out[game..].iter().find_map(|d| match d {
        Draw::Rect { colour, .. } if colour[..3] == [0.0, 0.0, 0.0] => Some(colour[3]),
        _ => None,
    });
    assert!(fade.is_some_and(|a| a > 0.0 && a < 1.0), "black fading over the game: {fade:?}");
}

#[test]
fn the_paused_game_menu_is_always_dark() {
    for colour in [5u8, 20, 30] {
        let d = root(SlotState::default());
        write_slot_state(
            d.path(),
            &SlotState { cart: Some("Golden Sun".into()), clock_set: true, xmb_colour: colour, ..SlotState::default() },
        )
        .unwrap();
        let mut a = App::boot(d.path());
        a.set_panel((640, 480));
        a.set_snapshot(common::StubSnapshot::boxed());
        a.on_core_ready();
        a.set_game_ready(true);
        let mut soft = Soft::new();
        soft.faces(&mut a);
        soft.settle(&mut a, 60);
        a.apply(Action::QuickMenu);
        let out = soft.shot(&mut a, "pause_check");
        let b = out
            .iter()
            .find_map(|d| match d {
                Draw::Backdrop(b) => Some(*b),
                _ => None,
            })
            .expect("a backdrop behind the menu");
        assert_eq!(b.colours, slot_ui::xmb_pause_colours(), "colour {colour} goes dark");
        assert!(b.ground.is_none());
    }
}

#[test]
fn the_dark_pause_menu_can_be_turned_off() {
    let d = root(SlotState::default());
    write_slot_state(
        d.path(),
        &SlotState { cart: Some("Golden Sun".into()), clock_set: true, xmb_colour: 20, xmb_pause_dark: false, ..SlotState::default() },
    )
    .unwrap();
    let mut a = App::boot(d.path());
    a.set_panel((640, 480));
    a.set_snapshot(common::StubSnapshot::boxed());
    a.on_core_ready();
    a.set_game_ready(true);
    let mut soft = Soft::new();
    soft.faces(&mut a);
    soft.settle(&mut a, 60);
    a.apply(Action::QuickMenu);
    let out = soft.shot(&mut a, "pause_colour_check");
    let b = out
        .iter()
        .find_map(|d| match d {
            Draw::Backdrop(b) => Some(*b),
            _ => None,
        })
        .expect("a backdrop behind the menu");
    assert_eq!(b.colours, slot_ui::xmb_backdrop_colours(20), "off: the bar's own colour");
    assert!(SlotState::default().xmb_pause_dark, "on by default");
}
