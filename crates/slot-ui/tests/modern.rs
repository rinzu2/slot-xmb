//! The Modern background, drawn by the GPU programs the handheld runs, through whatever GLES 2
//! the machine has. With `MODERN_SHOTS=dir` the frames are written out as PNGs.
#![cfg(target_os = "linux")]

use slot_gfx::{Canvas, Compositor, Draw, HeadlessSurface};
use slot_ui::xmb_modern::{xmb_modern_picture, ModernColour, ModernScene, Wall};

fn gl() -> Option<(HeadlessSurface, Compositor)> {
    let surface = HeadlessSurface::new().ok()?;
    let compositor = Compositor::new(&surface).ok()?;
    Some((surface, compositor))
}

fn shot(c: &mut Compositor, scene: &mut ModernScene, window: (u32, u32), name: &str) -> Vec<u8> {
    c.fit(window);
    c.set_modern(scene.frame());
    c.begin_frame();
    let curves = c.create_texture_nearest(
        slot_gfx::BACKDROP_COLS as u32,
        slot_gfx::BACKDROP_ROWS as u32,
        &vec![0; slot_gfx::BACKDROP_COLS * slot_gfx::BACKDROP_ROWS * 4],
    );
    c.draw_list(&[Draw::Backdrop(slot_gfx::Backdrop {
        curves,
        style: slot_gfx::STYLE_MODERN,
        colours: [[0; 3]; 15],
        edges: [[0.0; 4]; 3],
        shape: slot_gfx::BACKDROP_PLAIN,
        alpha: 1.0,
        ground: None,
    })]);
    let frame = c.read_frame();
    if let Some(dir) = std::env::var_os("MODERN_SHOTS") {
        let dir = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).unwrap();
        let (w, h) = (window.0 as usize, window.1 as usize);
        let mut rows = Vec::with_capacity(w * h * 3);
        for y in (0..h).rev() {
            for x in 0..w {
                let o = (y * w + x) * 4;
                rows.extend_from_slice(&frame[o..o + 3]);
            }
        }
        let file = std::fs::File::create(dir.join(format!("{name}.png"))).unwrap();
        let mut e = png::Encoder::new(std::io::BufWriter::new(file), w as u32, h as u32);
        e.set_color(png::ColorType::Rgb);
        e.set_depth(png::BitDepth::Eight);
        e.write_header().unwrap().write_image_data(&rows).unwrap();
    }
    frame
}

fn run(scene: &mut ModernScene, seconds: f64, wall: Wall, aspect: f64) {
    let frames = (seconds * 60.0).round() as usize;
    for _ in 0..frames {
        scene.advance(1.0 / 60.0, wall, &[], None, aspect);
    }
}

fn mean(frame: &[u8]) -> [f64; 3] {
    let n = (frame.len() / 4) as f64;
    let mut sum = [0.0; 3];
    for p in frame.chunks(4) {
        for i in 0..3 {
            sum[i] += p[i] as f64;
        }
    }
    sum.map(|s| s / n)
}

#[test]
fn the_wave_lights_the_screen_over_the_month_backdrop() {
    let Some((_s, mut c)) = gl() else {
        eprintln!("no gles here; skipped");
        return;
    };
    let window = (640, 480);
    let aspect = 640.0 / 480.0;
    for (name, month, hour) in [("day_oct", 9, 13.0), ("night_oct", 9, 23.0), ("dawn_mar", 2, 6.0), ("day_jun", 5, 12.0)] {
        let mut scene = ModernScene::new(Some(7));
        let wall = Wall {
            month,
            day: 26,
            hour,
        };
        run(&mut scene, 6.0, wall, aspect);
        let f = shot(&mut c, &mut scene, window, name);
        let m = mean(&f);
        assert!(m.iter().any(|&v| v > 4.0), "{name} came out black: {m:?}");
    }
}

#[test]
fn a_colour_of_the_bars_own_takes_the_backdrop() {
    let Some((_s, mut c)) = gl() else {
        return;
    };
    let window = (640, 480);
    for colour in [9u32, 21, 29, 34] {
        let mut scene = ModernScene::new(Some(3));
        scene.set_colour(ModernColour::Picture(xmb_modern_picture(colour, None)));
        let wall = Wall {
            month: 9,
            day: 26,
            hour: 22.0,
        };
        run(&mut scene, 4.0, wall, 4.0 / 3.0);
        shot(&mut c, &mut scene, window, &format!("colour_{colour}"));
    }
}

#[test]
fn the_start_rises_out_of_black() {
    let Some((_s, mut c)) = gl() else {
        return;
    };
    let window = (640, 480);
    let wall = Wall {
        month: 9,
        day: 26,
        hour: 13.0,
    };
    let mut scene = ModernScene::new(Some(11));
    scene.boot();
    let mut last = 0.0;
    for (i, t) in [0.05, 0.5, 1.0, 1.5, 2.0, 3.0, 4.0, 5.0, 6.5, 8.0].iter().enumerate() {
        run(&mut scene, t - last, wall, 4.0 / 3.0);
        last = *t;
        let f = shot(&mut c, &mut scene, window, &format!("boot_{i:02}_{t}"));
        if i == 0 {
            let m = mean(&f);
            assert!(m.iter().all(|&v| v < 3.0), "the start began lit: {m:?}");
        }
    }
    let _ = Canvas::of(window);
}

#[test]
#[ignore]
fn bench_the_scene() {
    let mut scene = ModernScene::new(Some(1));
    let wall = Wall { month: 9, day: 26, hour: 13.0 };
    run(&mut scene, 5.0, wall, 4.0 / 3.0);
    let n = 600;
    let t0 = std::time::Instant::now();
    for _ in 0..n {
        scene.advance(1.0 / 60.0, wall, &[], None, 4.0 / 3.0);
    }
    let t1 = std::time::Instant::now();
    for _ in 0..n {
        scene.frame();
    }
    let t2 = std::time::Instant::now();
    eprintln!(
        "advance {:.3} ms, frame {:.3} ms",
        (t1 - t0).as_secs_f64() * 1000.0 / n as f64,
        (t2 - t1).as_secs_f64() * 1000.0 / n as f64
    );
}
